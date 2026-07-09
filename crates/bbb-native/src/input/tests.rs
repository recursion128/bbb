use super::*;
use bbb_item_model::NativeItemRuntime;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_BAT_ID, VANILLA_ENTITY_TYPE_BLAZE_ID,
    VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_BREEZE_ID,
    VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COD_ID,
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_CREEPER_ID,
    VANILLA_ENTITY_TYPE_DOLPHIN_ID, VANILLA_ENTITY_TYPE_DROWNED_ID,
    VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID,
    VANILLA_ENTITY_TYPE_ENDERMITE_ID, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
    VANILLA_ENTITY_TYPE_EVOKER_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_GHAST_ID,
    VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, VANILLA_ENTITY_TYPE_GOAT_ID,
    VANILLA_ENTITY_TYPE_GUARDIAN_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_HUSK_ID, VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
    VANILLA_ENTITY_TYPE_INTERACTION_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
    VANILLA_ENTITY_TYPE_OCELOT_ID, VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_PARROT_ID,
    VANILLA_ENTITY_TYPE_PHANTOM_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
    VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
    VANILLA_ENTITY_TYPE_PUFFERFISH_ID, VANILLA_ENTITY_TYPE_RABBIT_ID,
    VANILLA_ENTITY_TYPE_RAVAGER_ID, VANILLA_ENTITY_TYPE_SALMON_ID, VANILLA_ENTITY_TYPE_SHEEP_ID,
    VANILLA_ENTITY_TYPE_SHULKER_ID, VANILLA_ENTITY_TYPE_SILVERFISH_ID,
    VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_SLIME_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
    VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, VANILLA_ENTITY_TYPE_SPIDER_ID, VANILLA_ENTITY_TYPE_SQUID_ID,
    VANILLA_ENTITY_TYPE_STRAY_ID, VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TADPOLE_ID,
    VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID, VANILLA_ENTITY_TYPE_VEX_ID,
    VANILLA_ENTITY_TYPE_VINDICATOR_ID, VANILLA_ENTITY_TYPE_WITCH_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
    VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::packets::BlockEntityData;
use bbb_protocol::packets::{
    AddEntity, AdvancementDisplaySummary, AdvancementFrameType, AdvancementIconSummary,
    AdvancementSummary, BlockEntityTagQuery, BlockPos as ProtocolBlockPos,
    BlockUpdate as ProtocolBlockUpdate, ChatCommand, CommandArgumentParser, CommandNode,
    CommandNodeType, CommandSuggestion, CommandSuggestionRequest, CommandSuggestions, Commands,
    CommonPlayerSpawnInfo, ContainerClick, ContainerCloseRequest, ContainerInput,
    ContainerSetContent as ProtocolContainerSetContent, DataComponentPatchSummary, DeleteChat,
    DialogHolder, EntityDataEnumSerializer, EntityDataRegistryHolder,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
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

    fn with_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
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

    fn get_debug_clipboard_text(&mut self) -> Option<String> {
        self.text.clone()
    }
}

struct VariableDebugOptionsSearchTextMeasurer;

impl DebugOptionsSearchTextMeasurer for VariableDebugOptionsSearchTextMeasurer {
    fn debug_options_search_display_start_for_width(
        &self,
        search_text: &str,
        scroll_to: usize,
        width: i32,
    ) -> usize {
        let scroll_to = scroll_to.min(search_text.chars().count());
        if variable_debug_options_search_prefix_width(search_text, scroll_to) <= width {
            return 0;
        }

        let chars = search_text.chars().take(scroll_to).collect::<Vec<_>>();
        let mut start = chars.len();
        let mut used_width = 0;
        while start > 0 {
            let advance = variable_debug_options_search_char_advance(chars[start - 1]);
            if used_width + advance > width {
                break;
            }
            used_width += advance;
            start -= 1;
        }
        start
    }

    fn debug_options_search_cursor_for_text_offset_from_display_start(
        &self,
        search_text: &str,
        display_start: usize,
        offset: i32,
    ) -> usize {
        let display_start = display_start.min(search_text.chars().count());
        let tail = search_text.chars().skip(display_start).collect::<String>();
        display_start + variable_debug_options_search_cursor_for_text_offset(&tail, offset)
    }
}

fn variable_debug_options_search_cursor_for_text_offset(search_text: &str, offset: i32) -> usize {
    let mut width = 0;
    let mut cursor = 0;
    for ch in search_text.chars() {
        let advance = variable_debug_options_search_char_advance(ch);
        if width + advance > offset {
            break;
        }
        width += advance;
        cursor += 1;
    }
    cursor
}

fn variable_debug_options_search_char_advance(ch: char) -> i32 {
    match ch {
        'i' => 2,
        'w' => 7,
        ' ' => 4,
        _ => DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE,
    }
}

fn variable_debug_options_search_prefix_width(search_text: &str, char_count: usize) -> i32 {
    search_text
        .chars()
        .take(char_count)
        .map(variable_debug_options_search_char_advance)
        .sum()
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
fn shift_f3_i_with_permission_copies_local_creeper_save_nbt_to_clipboard() {
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
                data_id: ENTITY_CUSTOM_NAME_DATA_ID,
                serializer_id: 6,
                value: EntityDataValueKind::OptionalComponent(Some("Bob \"Prime\"".to_string())),
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
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: CREEPER_POWERED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: CREEPER_IGNITED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
             {Motion: [0.25d, -0.5d, 0.75d], Rotation: [45.0f, 10.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 123s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CustomName: 'Bob \"Prime\"', \
             CustomNameVisible: 1b, Silent: 1b, NoGravity: 1b, Glowing: 1b, TicksFrozen: 42, \
             CanPickUpLoot: 0b, PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             powered: 1b, Fuse: 30s, ExplosionRadius: 3b, ignited: 1b}"
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
fn shift_f3_i_with_permission_copies_local_creaking_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_CREAKING_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: CREAKING_HOME_POS_DATA_ID,
                serializer_id: 11,
                value: EntityDataValueKind::OptionalBlockPos(Some(ProtocolBlockPos {
                    x: 4,
                    y: 5,
                    z: 6,
                })),
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
            "/summon minecraft:creaking 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, home_pos: [I; 4, 5, 6]}"
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
fn shift_f3_i_with_permission_copies_local_shulker_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_SHULKER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: SHULKER_ATTACH_FACE_DATA_ID,
                serializer_id: 12,
                value: EntityDataValueKind::Direction(5),
            },
            ProtocolEntityDataValue {
                data_id: SHULKER_PEEK_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(30),
            },
            ProtocolEntityDataValue {
                data_id: SHULKER_COLOR_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(11),
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
            "/summon minecraft:shulker 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, AttachFace: 5b, Peek: 30b, \
             Color: 11b}"
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
fn shift_f3_i_with_permission_copies_local_slime_family_save_nbt_to_clipboard() {
    for (entity_type_id, entity_type) in [
        (VANILLA_ENTITY_TYPE_SLIME_ID, "minecraft:slime"),
        (VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, "minecraft:magma_cube"),
    ] {
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        world.apply_add_entity(AddEntity {
            id: 50,
            uuid: Uuid::from_u128(50),
            entity_type_id,
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
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: 50,
            values: vec![
                ProtocolEntityDataValue {
                    data_id: MOB_FLAGS_DATA_ID,
                    serializer_id: 0,
                    value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
                },
                ProtocolEntityDataValue {
                    data_id: SLIME_SIZE_DATA_ID,
                    serializer_id: 1,
                    value: EntityDataValueKind::Int(4),
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

        let expected = format!(
            "/summon {entity_type} 0.00 0.00 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, Size: 3, wasOnGround: 0b}}"
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
}

#[test]
fn shift_f3_i_with_permission_copies_local_snow_golem_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: SNOW_GOLEM_PUMPKIN_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(0),
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
            "/summon minecraft:snow_golem 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, Pumpkin: 0b}"
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
fn shift_f3_i_with_permission_copies_local_bat_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_BAT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: BAT_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(1),
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
            "/summon minecraft:bat 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, BatFlags: 1b}"
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
fn shift_f3_i_with_permission_copies_local_blaze_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_BLAZE_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: MOB_FLAGS_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
        }],
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
            "/summon minecraft:blaze 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b}"
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
fn shift_f3_i_with_permission_copies_local_breeze_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_BREEZE_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: MOB_FLAGS_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
        }],
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
            "/summon minecraft:breeze 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b}"
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
fn shift_f3_i_with_permission_copies_local_cod_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 52,
        uuid: Uuid::from_u128(52),
        entity_type_id: VANILLA_ENTITY_TYPE_COD_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.45,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 52,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: ABSTRACT_FISH_FROM_BUCKET_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:cod 0.00 1.45 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, FromBucket: 1b}"
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
fn shift_f3_i_with_permission_copies_local_pufferfish_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 53,
        uuid: Uuid::from_u128(53),
        entity_type_id: VANILLA_ENTITY_TYPE_PUFFERFISH_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 53,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: ABSTRACT_FISH_FROM_BUCKET_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: PUFFERFISH_PUFF_STATE_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(2),
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
            "/summon minecraft:pufferfish 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             FromBucket: 1b, PuffState: 2}"
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
fn shift_f3_i_with_permission_copies_local_salmon_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 54,
        uuid: Uuid::from_u128(54),
        entity_type_id: VANILLA_ENTITY_TYPE_SALMON_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.2,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 54,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: ABSTRACT_FISH_FROM_BUCKET_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: SALMON_VARIANT_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(2),
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
            "/summon minecraft:salmon 0.00 1.20 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             FromBucket: 1b, type: \"large\"}"
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
fn shift_f3_i_with_permission_copies_local_tropical_fish_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 55,
        uuid: Uuid::from_u128(55),
        entity_type_id: VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.3,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: ABSTRACT_FISH_FROM_BUCKET_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: TROPICAL_FISH_VARIANT_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(0x0405_0001),
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
            "/summon minecraft:tropical_fish 0.00 1.30 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             FromBucket: 1b, Variant: 67436545}"
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
fn shift_f3_i_with_permission_copies_local_tadpole_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 56,
        uuid: Uuid::from_u128(56),
        entity_type_id: VANILLA_ENTITY_TYPE_TADPOLE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.4,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 56,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: TADPOLE_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:tadpole 0.00 1.40 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             FromBucket: 1b, Age: 0, AgeLocked: 1b}"
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
fn shift_f3_i_with_permission_copies_local_axolotl_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 57,
        uuid: Uuid::from_u128(57),
        entity_type_id: VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 57,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AXOLOTL_VARIANT_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(9),
            },
            ProtocolEntityDataValue {
                data_id: AXOLOTL_FROM_BUCKET_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:axolotl 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 6000s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, Variant: 0, FromBucket: 1b}"
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
fn shift_f3_i_with_permission_copies_local_dolphin_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 66,
        uuid: Uuid::from_u128(66),
        entity_type_id: VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 66,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: DOLPHIN_GOT_FISH_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: DOLPHIN_MOISTNESS_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(1234),
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
            "/summon minecraft:dolphin 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 4800s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 1b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, GotFish: 1b, Moistness: 1234}"
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
fn shift_f3_i_with_permission_copies_local_frog_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 67,
        uuid: Uuid::from_u128(67),
        entity_type_id: VANILLA_ENTITY_TYPE_FROG_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 67,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: FROG_VARIANT_DATA_ID,
                serializer_id: 27,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::FrogVariant,
                    id: 2,
                },
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
            "/summon minecraft:frog 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, variant: \"minecraft:cold\"}"
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
fn shift_f3_i_with_permission_copies_local_rabbit_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 58,
        uuid: Uuid::from_u128(58),
        entity_type_id: VANILLA_ENTITY_TYPE_RABBIT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.3,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 58,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: RABBIT_TYPE_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(99),
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
            "/summon minecraft:rabbit 0.00 1.30 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, RabbitType: 99, \
             MoreCarrotTicks: 0}"
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
fn shift_f3_i_with_permission_copies_local_ocelot_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 59,
        uuid: Uuid::from_u128(59),
        entity_type_id: VANILLA_ENTITY_TYPE_OCELOT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.3,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 59,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: OCELOT_TRUSTING_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:ocelot 0.00 1.30 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, Trusting: 1b}"
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
fn shift_f3_i_with_permission_copies_local_sheep_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 60,
        uuid: Uuid::from_u128(60),
        entity_type_id: VANILLA_ENTITY_TYPE_SHEEP_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.2,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 60,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: SHEEP_WOOL_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte((SHEEP_WOOL_SHEARED_FLAG | 14) as i8),
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
            "/summon minecraft:sheep 0.00 1.20 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, Sheared: 1b, Color: 14b}"
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
fn shift_f3_i_with_permission_copies_local_pig_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 61,
        uuid: Uuid::from_u128(61),
        entity_type_id: VANILLA_ENTITY_TYPE_PIG_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.25,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 61,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: PIG_VARIANT_DATA_ID,
                serializer_id: 28,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::PigVariant,
                    id: 2,
                },
            },
            ProtocolEntityDataValue {
                data_id: PIG_SOUND_VARIANT_DATA_ID,
                serializer_id: 29,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::PigSoundVariant,
                    id: 1,
                },
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
            "/summon minecraft:pig 0.00 1.25 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, variant: \"minecraft:cold\", \
             sound_variant: \"minecraft:big\"}"
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
fn shift_f3_i_with_permission_copies_local_chicken_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 62,
        uuid: Uuid::from_u128(62),
        entity_type_id: VANILLA_ENTITY_TYPE_CHICKEN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.25,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 62,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: CHICKEN_VARIANT_DATA_ID,
                serializer_id: 30,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::ChickenVariant,
                    id: 1,
                },
            },
            ProtocolEntityDataValue {
                data_id: CHICKEN_SOUND_VARIANT_DATA_ID,
                serializer_id: 31,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::ChickenSoundVariant,
                    id: 1,
                },
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
            "/summon minecraft:chicken 0.00 1.25 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, IsChickenJockey: 0b, \
             EggLayTime: 0, variant: \"minecraft:warm\", sound_variant: \"minecraft:picky\"}"
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
fn shift_f3_i_with_permission_copies_local_cow_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 63,
        uuid: Uuid::from_u128(63),
        entity_type_id: VANILLA_ENTITY_TYPE_COW_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 63,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: COW_VARIANT_DATA_ID,
                serializer_id: 23,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::CowVariant,
                    id: 2,
                },
            },
            ProtocolEntityDataValue {
                data_id: COW_SOUND_VARIANT_DATA_ID,
                serializer_id: 24,
                value: EntityDataValueKind::RegistryId {
                    serializer: EntityDataRegistryHolder::CowSoundVariant,
                    id: 1,
                },
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
            "/summon minecraft:cow 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, variant: \"minecraft:cold\", \
             sound_variant: \"minecraft:moody\"}"
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
fn shift_f3_i_with_permission_copies_local_mooshroom_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 64,
        uuid: Uuid::from_u128(64),
        entity_type_id: VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 64,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: MOOSHROOM_TYPE_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(1),
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
            "/summon minecraft:mooshroom 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, Type: \"brown\"}"
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
fn shift_f3_i_with_permission_copies_local_goat_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 65,
        uuid: Uuid::from_u128(65),
        entity_type_id: VANILLA_ENTITY_TYPE_GOAT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 65,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: GOAT_SCREAMING_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: GOAT_LEFT_HORN_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(false),
            },
            ProtocolEntityDataValue {
                data_id: GOAT_RIGHT_HORN_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:goat 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, IsScreamingGoat: 1b, \
             HasLeftHorn: 0b, HasRightHorn: 1b}"
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
fn shift_f3_i_with_permission_copies_local_polar_bear_save_nbt_to_clipboard() {
    const POLAR_BEAR_STANDING_DATA_ID: u8 = 18;

    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 68,
        uuid: Uuid::from_u128(68),
        entity_type_id: VANILLA_ENTITY_TYPE_POLAR_BEAR_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 68,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: POLAR_BEAR_STANDING_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:polar_bear 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, anger_end_time: 0L}"
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
fn shift_f3_i_with_permission_copies_local_panda_save_nbt_to_clipboard() {
    const PANDA_UNHAPPY_COUNTER_DATA_ID: u8 = 18;
    const PANDA_EAT_COUNTER_DATA_ID: u8 = 20;
    const PANDA_MAIN_GENE_DATA_ID: u8 = 21;
    const PANDA_HIDDEN_GENE_DATA_ID: u8 = 22;
    const PANDA_FLAGS_DATA_ID: u8 = 23;
    const PANDA_FLAG_ROLLING: i8 = 0x04;
    const PANDA_FLAG_SITTING: i8 = 0x08;
    const PANDA_FLAG_ON_BACK: i8 = 0x10;

    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 69,
        uuid: Uuid::from_u128(69),
        entity_type_id: VANILLA_ENTITY_TYPE_PANDA_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 69,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: PANDA_UNHAPPY_COUNTER_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(12),
            },
            ProtocolEntityDataValue {
                data_id: PANDA_EAT_COUNTER_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(4),
            },
            ProtocolEntityDataValue {
                data_id: PANDA_MAIN_GENE_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(4),
            },
            ProtocolEntityDataValue {
                data_id: PANDA_HIDDEN_GENE_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(1),
            },
            ProtocolEntityDataValue {
                data_id: PANDA_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(
                    PANDA_FLAG_ROLLING | PANDA_FLAG_SITTING | PANDA_FLAG_ON_BACK,
                ),
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
            "/summon minecraft:panda 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, MainGene: \"brown\", \
             HiddenGene: \"lazy\"}"
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
fn shift_f3_i_with_permission_copies_local_parrot_save_nbt_to_clipboard() {
    const TAMABLE_ANIMAL_FLAGS_DATA_ID: u8 = 18;
    const TAMABLE_ANIMAL_SITTING_FLAG: i8 = 0x01;
    const PARROT_VARIANT_DATA_ID: u8 = 20;

    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 71,
        uuid: Uuid::from_u128(71),
        entity_type_id: VANILLA_ENTITY_TYPE_PARROT_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 71,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: TAMABLE_ANIMAL_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(TAMABLE_ANIMAL_SITTING_FLAG),
            },
            ProtocolEntityDataValue {
                data_id: PARROT_VARIANT_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(3),
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
            "/summon minecraft:parrot 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, Sitting: 1b, Variant: 3}"
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
fn shift_f3_i_with_permission_copies_local_sniffer_save_nbt_to_clipboard() {
    const SNIFFER_STATE_DATA_ID: u8 = 18;
    const SNIFFER_DROP_SEED_AT_TICK_DATA_ID: u8 = 19;
    const SNIFFER_STATE_DIGGING_ID: i32 = 5;

    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 70,
        uuid: Uuid::from_u128(70),
        entity_type_id: VANILLA_ENTITY_TYPE_SNIFFER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 70,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: SNIFFER_STATE_DATA_ID,
                serializer_id: 35,
                value: EntityDataValueKind::EnumId {
                    serializer: EntityDataEnumSerializer::SnifferState,
                    id: SNIFFER_STATE_DIGGING_ID,
                },
            },
            ProtocolEntityDataValue {
                data_id: SNIFFER_DROP_SEED_AT_TICK_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(1234),
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
            "/summon minecraft:sniffer 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0}"
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
fn shift_f3_i_with_permission_copies_local_strider_save_nbt_to_clipboard() {
    const STRIDER_BOOST_TIME_DATA_ID: u8 = 18;
    const STRIDER_SUFFOCATING_DATA_ID: u8 = 19;

    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 72,
        uuid: Uuid::from_u128(72),
        entity_type_id: VANILLA_ENTITY_TYPE_STRIDER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 72,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: STRIDER_BOOST_TIME_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(40),
            },
            ProtocolEntityDataValue {
                data_id: STRIDER_SUFFOCATING_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:strider 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0}"
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
fn shift_f3_i_with_permission_copies_local_hoglin_save_nbt_to_clipboard() {
    const HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID: u8 = 18;

    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 73,
        uuid: Uuid::from_u128(73),
        entity_type_id: VANILLA_ENTITY_TYPE_HOGLIN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 73,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:hoglin 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, \
             IsImmuneToZombification: 1b, CannotBeHunted: 0b}"
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
fn shift_f3_i_with_permission_copies_local_piglin_brute_save_nbt_to_clipboard() {
    const PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID: u8 = 16;

    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 74,
        uuid: Uuid::from_u128(74),
        entity_type_id: VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 74,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:piglin_brute 0.00 1.00 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 1b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             IsImmuneToZombification: 1b, TimeInOverworld: 0}"
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
fn shift_f3_i_with_permission_copies_local_spider_family_save_nbt_to_clipboard() {
    for (entity_type_id, entity_type) in [
        (VANILLA_ENTITY_TYPE_SPIDER_ID, "minecraft:spider"),
        (VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, "minecraft:cave_spider"),
    ] {
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        world.apply_add_entity(AddEntity {
            id: 50,
            uuid: Uuid::from_u128(50),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.25,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: 50,
            values: vec![ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            }],
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

        let expected = format!(
            "/summon {entity_type} 0.00 1.25 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b}}"
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
}

#[test]
fn shift_f3_i_with_permission_copies_local_skeleton_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 55,
        uuid: Uuid::from_u128(55),
        entity_type_id: VANILLA_ENTITY_TYPE_SKELETON_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.25,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 55,
        values: vec![ProtocolEntityDataValue {
            data_id: MOB_FLAGS_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
        }],
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
            "/summon minecraft:skeleton 0.00 1.25 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             StrayConversionTime: -1}"
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
fn shift_f3_i_with_permission_copies_local_nonconverting_skeleton_family_save_nbt_to_clipboard() {
    for (entity_type_id, entity_type) in [
        (VANILLA_ENTITY_TYPE_STRAY_ID, "minecraft:stray"),
        (
            VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
            "minecraft:wither_skeleton",
        ),
    ] {
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        world.apply_add_entity(AddEntity {
            id: 56,
            uuid: Uuid::from_u128(56),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.25,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: 56,
            values: vec![ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            }],
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

        let expected = format!(
            "/summon {entity_type} 0.00 1.25 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b}}"
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
}

#[test]
fn shift_f3_i_with_permission_copies_local_zombie_family_save_nbt_to_clipboard() {
    const ZOMBIE_BABY_DATA_ID: u8 = 16;

    for (entity_type_id, entity_type) in [
        (VANILLA_ENTITY_TYPE_ZOMBIE_ID, "minecraft:zombie"),
        (VANILLA_ENTITY_TYPE_HUSK_ID, "minecraft:husk"),
        (VANILLA_ENTITY_TYPE_DROWNED_ID, "minecraft:drowned"),
    ] {
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        world.apply_add_entity(AddEntity {
            id: 57,
            uuid: Uuid::from_u128(57),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.25,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: 57,
            values: vec![
                ProtocolEntityDataValue {
                    data_id: MOB_FLAGS_DATA_ID,
                    serializer_id: 0,
                    value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
                },
                ProtocolEntityDataValue {
                    data_id: ZOMBIE_BABY_DATA_ID,
                    serializer_id: 8,
                    value: EntityDataValueKind::Boolean(true),
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

        let expected = format!(
            "/summon {entity_type} 0.00 1.25 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             IsBaby: 1b, CanBreakDoors: 0b, InWaterTime: -1, \
             DrownedConversionTime: -1}}"
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
}

#[test]
fn shift_f3_i_with_permission_copies_local_bogged_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_BOGGED_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: BOGGED_SHEARED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:bogged 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, sheared: 1b}"
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
fn shift_f3_i_with_permission_copies_local_end_crystal_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: END_CRYSTAL_BEAM_TARGET_DATA_ID,
                serializer_id: 11,
                value: EntityDataValueKind::OptionalBlockPos(Some(ProtocolBlockPos {
                    x: 1,
                    y: 2,
                    z: 3,
                })),
            },
            ProtocolEntityDataValue {
                data_id: END_CRYSTAL_SHOW_BOTTOM_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(false),
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
            "/summon minecraft:end_crystal 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, beam_target: [I; 1, 2, 3], \
             ShowBottom: 0b}"
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
fn shift_f3_i_with_permission_copies_local_endermite_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_ENDERMITE_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: MOB_FLAGS_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
        }],
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
            "/summon minecraft:endermite 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Lifetime: 0}"
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
fn shift_f3_i_with_permission_copies_local_enderman_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_ENDERMAN_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: MOB_FLAGS_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
        }],
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
            "/summon minecraft:enderman 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             anger_end_time: 0L}"
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
fn shift_f3_i_with_permission_copies_local_silverfish_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_SILVERFISH_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 2.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: MOB_FLAGS_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
        }],
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
            "/summon minecraft:silverfish 0.00 1.50 2.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b}"
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
fn shift_f3_i_with_permission_copies_local_guardian_family_save_nbt_to_clipboard() {
    for (entity_type_id, entity_type) in [
        (VANILLA_ENTITY_TYPE_GUARDIAN_ID, "minecraft:guardian"),
        (
            VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID,
            "minecraft:elder_guardian",
        ),
    ] {
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        world.apply_add_entity(AddEntity {
            id: 50,
            uuid: Uuid::from_u128(50),
            entity_type_id,
            position: ProtocolVec3d {
                x: 0.0,
                y: 1.0,
                z: 3.0,
            },
            delta_movement: ProtocolVec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: 50,
            values: vec![ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            }],
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

        let expected = format!(
            "/summon {entity_type} 0.00 1.00 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b}}"
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
}

#[test]
fn shift_f3_i_with_permission_copies_local_happy_ghast_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:happy_ghast 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: 1, \
             ForcedAge: 0, AgeLocked: 1b, InLove: 0, still_timeout: 0}"
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
fn shift_f3_i_with_permission_copies_local_ghast_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_GHAST_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: MOB_FLAGS_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
        }],
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
            "/summon minecraft:ghast 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, ExplosionPower: 1b}"
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
fn shift_f3_i_with_permission_copies_local_glow_squid_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: GLOW_SQUID_DARK_TICKS_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(77),
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
            "/summon minecraft:glow_squid 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: 1, \
             ForcedAge: 0, AgeLocked: 1b, DarkTicksRemaining: 77}"
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
fn shift_f3_i_with_permission_copies_local_squid_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 51,
        uuid: Uuid::from_u128(51),
        entity_type_id: VANILLA_ENTITY_TYPE_SQUID_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 51,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
            },
            ProtocolEntityDataValue {
                data_id: AGEABLE_MOB_AGE_LOCKED_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:squid 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, Age: -1, \
             ForcedAge: 0, AgeLocked: 1b}"
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
fn shift_f3_i_with_permission_copies_local_interaction_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_INTERACTION_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: INTERACTION_WIDTH_DATA_ID,
                serializer_id: 3,
                value: EntityDataValueKind::Float(1.5),
            },
            ProtocolEntityDataValue {
                data_id: INTERACTION_HEIGHT_DATA_ID,
                serializer_id: 3,
                value: EntityDataValueKind::Float(2.0),
            },
            ProtocolEntityDataValue {
                data_id: INTERACTION_RESPONSE_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:interaction 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, width: 1.5f, height: 2.0f, \
             response: 1b}"
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
fn shift_f3_i_with_permission_copies_local_iron_golem_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: IRON_GOLEM_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(IRON_GOLEM_PLAYER_CREATED_FLAG),
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
            "/summon minecraft:iron_golem 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, PlayerCreated: 1b, \
             anger_end_time: 0L}"
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
fn shift_f3_i_with_permission_copies_local_phantom_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_PHANTOM_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.5,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: PHANTOM_SIZE_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(5),
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
            "/summon minecraft:phantom 0.00 1.50 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, size: 5}"
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
fn shift_f3_i_with_permission_copies_local_ravager_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_RAVAGER_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: MOB_FLAGS_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
        }],
    }));
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: 50,
        event_id: 4,
    }));
    assert!(world.apply_entity_event(ProtocolEntityEvent {
        entity_id: 50,
        event_id: 39,
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
            "/summon minecraft:ravager 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, PatrolLeader: 0b, Patrolling: 0b, \
             Wave: 0, CanJoinRaid: 0b, AttackTick: 10, StunTick: 40, RoarTick: 0}"
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
fn shift_f3_i_with_permission_copies_local_witch_save_nbt_to_clipboard() {
    const WITCH_USING_ITEM_DATA_ID: u8 = 17;

    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 51,
        uuid: Uuid::from_u128(51),
        entity_type_id: VANILLA_ENTITY_TYPE_WITCH_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 51,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: WITCH_USING_ITEM_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:witch 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             PatrolLeader: 0b, Patrolling: 0b, Wave: 0, CanJoinRaid: 0b}"
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
fn shift_f3_i_with_permission_copies_local_spellcaster_illager_save_nbt_to_clipboard() {
    const SPELLCASTER_ILLAGER_CASTING_DATA_ID: u8 = 17;

    for (index, (entity_type_id, entity_type)) in [
        (VANILLA_ENTITY_TYPE_EVOKER_ID, "minecraft:evoker"),
        (VANILLA_ENTITY_TYPE_ILLUSIONER_ID, "minecraft:illusioner"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut input = ClientInputState::new(true);
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);
        let entity_id = 52 + i32::try_from(index).unwrap_or(0);
        world.apply_add_entity(AddEntity {
            id: entity_id,
            uuid: Uuid::from_u128(u128::try_from(entity_id).unwrap_or(52)),
            entity_type_id,
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
        assert!(world.apply_set_entity_data(ProtocolSetEntityData {
            id: entity_id,
            values: vec![
                ProtocolEntityDataValue {
                    data_id: MOB_FLAGS_DATA_ID,
                    serializer_id: 0,
                    value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
                },
                ProtocolEntityDataValue {
                    data_id: SPELLCASTER_ILLAGER_CASTING_DATA_ID,
                    serializer_id: 0,
                    value: EntityDataValueKind::Byte(2),
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

        let expected = format!(
            "/summon {entity_type} 0.00 0.00 3.00 \
             {{Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             PatrolLeader: 0b, Patrolling: 0b, Wave: 0, CanJoinRaid: 0b, \
             SpellTicks: 0}}"
        );
        assert_eq!(clipboard.text.as_deref(), Some(expected.as_str()));
        assert!(input.take_debug_recreate_server_query_requests().is_empty());
        let messages = &world.client_chat().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(
            messages[0].content,
            "[Debug]: Copied client-side entity data to clipboard"
        );
    }
}

#[test]
fn shift_f3_i_with_permission_copies_local_vindicator_save_nbt_to_clipboard() {
    const RAIDER_IS_CELEBRATING_DATA_ID: u8 = 16;

    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 54,
        uuid: Uuid::from_u128(54),
        entity_type_id: VANILLA_ENTITY_TYPE_VINDICATOR_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 54,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: RAIDER_IS_CELEBRATING_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:vindicator 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b, \
             PatrolLeader: 0b, Patrolling: 0b, Wave: 0, CanJoinRaid: 0b}"
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
fn shift_f3_i_with_permission_copies_local_vex_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_VEX_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![ProtocolEntityDataValue {
            data_id: MOB_FLAGS_DATA_ID,
            serializer_id: 0,
            value: EntityDataValueKind::Byte(MOB_FLAG_NO_AI | MOB_FLAG_LEFT_HANDED),
        }],
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
            "/summon minecraft:vex 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, NoAI: 1b}"
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
fn shift_f3_i_with_permission_copies_local_wither_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_WITHER_ID,
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
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: WITHER_INVULNERABLE_TICKS_DATA_ID,
                serializer_id: 1,
                value: EntityDataValueKind::Int(220),
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
            "/summon minecraft:wither 0.00 0.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, Invul: 220}"
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
fn shift_f3_i_with_permission_copies_local_zoglin_save_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_ZOGLIN_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 1.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: 50,
        values: vec![
            ProtocolEntityDataValue {
                data_id: MOB_FLAGS_DATA_ID,
                serializer_id: 0,
                value: EntityDataValueKind::Byte(MOB_FLAG_LEFT_HANDED),
            },
            ProtocolEntityDataValue {
                data_id: ZOGLIN_BABY_DATA_ID,
                serializer_id: 8,
                value: EntityDataValueKind::Boolean(true),
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
            "/summon minecraft:zoglin 0.00 1.00 3.00 \
             {Motion: [0.0d, 0.0d, 0.0d], Rotation: [0.0f, 0.0f], \
             fall_distance: 0.0d, Fire: 0s, Air: 300s, OnGround: 0b, \
             Invulnerable: 0b, PortalCooldown: 0, CanPickUpLoot: 0b, \
             PersistenceRequired: 0b, LeftHanded: 1b, IsBaby: 1b}"
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
fn debug_snbt_string_quotes_and_escapes_custom_names() {
    assert_eq!(debug_snbt_string("Bob \"Prime\""), "'Bob \"Prime\"'");
    assert_eq!(
        debug_snbt_string("Bob's \\ line\n"),
        "\"Bob's \\\\ line\\n\""
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
fn debug_options_screen_projects_vanilla_order_and_search_filter() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    let surface = winit::dpi::PhysicalSize::new(420, 240);

    let state = input
        .debug_options_screen_hud_state(surface, false)
        .expect("screen should project");
    assert_eq!(state.total_rows, 47);
    assert_eq!(state.visible_rows, 7);
    assert!(!state.default_profile_active);
    assert!(state.performance_profile_active);
    assert_eq!(
        state.rows[..3],
        [
            DebugOptionsScreenHudRow::Category {
                label: "Debug Screen Text".to_string()
            },
            DebugOptionsScreenHudRow::Entry {
                entry: DebugScreenEntryId::Biome,
                path: "biome".to_string(),
                status: DebugScreenEntryStatus::Never,
                hovered_status: None,
                allowed: true,
            },
            DebugOptionsScreenHudRow::Entry {
                entry: DebugScreenEntryId::ChunkGenerationStats,
                path: "chunk_generation_stats".to_string(),
                status: DebugScreenEntryStatus::Never,
                hovered_status: None,
                allowed: true,
            },
        ]
    );

    assert!(input.handle_debug_options_screen_text_input("chunk"));
    let filtered = input
        .debug_options_screen_hud_state(surface, false)
        .expect("screen should project after search");
    let labels = filtered
        .rows
        .iter()
        .map(|row| match row {
            DebugOptionsScreenHudRow::Category { label } => label.as_str(),
            DebugOptionsScreenHudRow::Entry { path, .. } => path.as_str(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "Debug Screen Text",
            "chunk_generation_stats",
            "chunk_render_stats",
            "chunk_source_stats",
            "Debug Renderers",
            "chunk_borders",
            "chunk_section_octree",
        ]
    );
}

#[test]
fn debug_options_screen_search_tracks_editbox_cursor_selection_and_word_keys() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();

    assert!(input.handle_debug_options_screen_text_input("chunk stats"));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_text_input("_"));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "chunk stat_s");
        assert_eq!(screen.search_cursor, 11);
        assert_eq!(screen.search_selection, 11);
    }

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "chunk stat_s");
        assert_eq!(screen.search_cursor, 9);
        assert_eq!(screen.search_selection, 11);
    }

    assert!(input.handle_debug_options_screen_text_input("X"));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "chunk staXs");
        assert_eq!(screen.search_cursor, 10);
        assert_eq!(screen.search_selection, 10);
    }

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    ));
    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::KeyA), ElementState::Pressed,));
    assert!(input.handle_debug_options_screen_text_input("alpha beta gamma"));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "alpha beta ");
        assert_eq!(screen.search_cursor, 11);
        assert_eq!(screen.search_selection, 11);
    }

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ArrowLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Released,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    ));
    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::Home), ElementState::Pressed,));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Delete),
        ElementState::Pressed,
    ));
    assert!(input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::End), ElementState::Pressed,));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Backspace),
        ElementState::Pressed,
    ));
    let state = input
        .debug_options_screen_hud_state(winit::dpi::PhysicalSize::new(420, 240), false)
        .unwrap();
    assert_eq!(state.search_text, "lpha");
    assert_eq!(state.search_cursor, 4);
    assert_eq!(state.search_selection, 4);
}

#[test]
fn debug_options_screen_search_uses_editbox_filter_and_default_max_length() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();

    assert!(input.handle_debug_options_screen_text_input(
        "abc\u{a7}\ndefghijklmnopqrstuvwxyz0123456789more"
    ));

    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_text, "abcdefghijklmnopqrstuvwxyz012345");
    assert_eq!(debug_options_search_len(&screen.search_text), 32);
    assert_eq!(screen.search_cursor, 32);
    assert_eq!(screen.search_selection, 32);
}

#[test]
fn debug_options_screen_search_handles_editbox_clipboard_shortcuts() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("chunk stats"));

    let mut clipboard = MockDebugClipboard::with_text("stale");
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyC),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert_eq!(clipboard.text.as_deref(), Some("chunk stats"));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "chunk stats");
        assert_eq!(screen.search_cursor, 11);
        assert_eq!(screen.search_selection, 0);
    }

    clipboard.text = Some("entity_hitboxes".to_string());
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "entity_hitboxes");
        assert_eq!(screen.search_cursor, 15);
        assert_eq!(screen.search_selection, 15);
    }

    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyX),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert_eq!(clipboard.text.as_deref(), Some("entity_hitboxes"));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_text, "");
        assert_eq!(screen.search_cursor, 0);
        assert_eq!(screen.search_selection, 0);
    }

    clipboard.text = Some("abc\u{a7}\ndef".to_string());
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyV),
        ElementState::Pressed,
        Some(&mut clipboard),
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    ));
    let state = input
        .debug_options_screen_hud_state(winit::dpi::PhysicalSize::new(420, 240), false)
        .unwrap();
    assert_eq!(state.search_text, "abcdef");
    assert_eq!(state.search_cursor, 6);
    assert_eq!(state.search_selection, 6);

    let mut rejecting_clipboard = MockDebugClipboard {
        text: None,
        accepts_text: false,
    };
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
        Some(&mut rejecting_clipboard),
    ));
    assert!(input.handle_debug_options_screen_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyX),
        ElementState::Pressed,
        Some(&mut rejecting_clipboard),
    ));
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Released,
    ));
    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_text, "abcdef");
    assert_eq!(screen.search_cursor, 6);
    assert_eq!(screen.search_selection, 0);
}

#[test]
fn debug_options_screen_search_handles_mouse_click_and_drag_selection() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("alpha beta"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 5),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_cursor, 5);
        assert_eq!(screen.search_selection, 5);
    }

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 9),
            f64::from(search_y + 18)
        )),
        surface,
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_cursor, 9);
        assert_eq!(screen.search_selection, 5);
    }

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Released,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 9),
            f64::from(search_y + 18)
        )),
        surface,
        false,
    ));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x),
            f64::from(search_y + 2)
        )),
        surface,
    ));
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_cursor, 9);
        assert_eq!(screen.search_selection, 5);
    }
}

#[test]
fn debug_options_screen_search_mouse_hit_testing_uses_variable_text_measurer() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("iwx"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;
    let measurer = VariableDebugOptionsSearchTextMeasurer;

    assert!(
        input.handle_debug_options_screen_mouse_input_with_text_measurer(
            winit::event::MouseButton::Left,
            ElementState::Pressed,
            Some(winit::dpi::PhysicalPosition::new(
                f64::from(text_x + 2),
                f64::from(search_y + 2)
            )),
            surface,
            false,
            &measurer,
        )
    );
    {
        let screen = input.debug_options_screen.as_ref().unwrap();
        assert_eq!(screen.search_cursor, 1);
        assert_eq!(screen.search_selection, 1);
    }

    assert!(
        input.handle_debug_options_screen_cursor_moved_with_text_measurer(
            Some(winit::dpi::PhysicalPosition::new(
                f64::from(text_x + 9),
                f64::from(search_y + 18)
            )),
            surface,
            &measurer,
        )
    );
    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_cursor, 2);
    assert_eq!(screen.search_selection, 1);
}

#[test]
fn debug_options_screen_search_mouse_hit_testing_uses_display_start() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("aaaaaaaaaaaaaaaaaaaa"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;
    let measurer = VariableDebugOptionsSearchTextMeasurer;

    assert!(
        input.handle_debug_options_screen_mouse_input_with_text_measurer(
            winit::event::MouseButton::Left,
            ElementState::Pressed,
            Some(winit::dpi::PhysicalPosition::new(
                f64::from(text_x),
                f64::from(search_y + 2)
            )),
            surface,
            false,
            &measurer,
        )
    );
    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_cursor, 2);
    assert_eq!(screen.search_selection, 2);
}

#[test]
fn debug_options_screen_search_double_click_selects_word() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("alpha beta"));
    input
        .debug_options_screen
        .as_mut()
        .unwrap()
        .last_left_click_at = Some(Instant::now());
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 8),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));
    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_selection, 6);
    assert_eq!(screen.search_cursor, 10);
}

#[test]
fn debug_options_screen_search_double_click_requires_vanilla_threshold() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("alpha beta"));
    input
        .debug_options_screen
        .as_mut()
        .unwrap()
        .last_left_click_at = Some(Instant::now() - DEBUG_OPTIONS_DOUBLE_CLICK_THRESHOLD);
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 8),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));
    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_cursor, 8);
    assert_eq!(screen.search_selection, 8);
}

#[test]
fn debug_options_screen_search_shift_click_extends_selection() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("chunk stats"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let (search_x, search_y, _, _) = debug_options_search_box_rect(surface);
    let text_x = search_x + DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;

    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::ShiftLeft),
        ElementState::Pressed,
    ));
    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(text_x + DEBUG_OPTIONS_SEARCH_CHAR_ADVANCE * 5),
            f64::from(search_y + 2)
        )),
        surface,
        false,
    ));

    let screen = input.debug_options_screen.as_ref().unwrap();
    assert_eq!(screen.search_cursor, 5);
    assert_eq!(screen.search_selection, 11);
}

#[test]
fn debug_options_screen_buttons_update_status_and_profiles() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("entity_hitboxes"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let content_x = debug_options_content_x(surface);
    let buttons_start = content_x + DEBUG_OPTIONS_ROW_WIDTH - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    let always_x = buttons_start + DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 2 + 2;
    let entry_y = DEBUG_OPTIONS_HEADER_HEIGHT + DEBUG_OPTIONS_ROW_HEIGHT + 2;

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(always_x),
            f64::from(entry_y)
        )),
        surface,
        false,
    ));
    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::EntityHitboxes),
        DebugScreenEntryStatus::AlwaysOn
    );
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(state.default_profile_active);
    assert!(state.performance_profile_active);

    let (_, performance_x, _) = debug_options_footer_button_xs(surface);
    let footer_y = debug_options_footer_button_y(surface) + 2;
    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(performance_x + 2),
            f64::from(footer_y)
        )),
        surface,
        false,
    ));
    assert_eq!(
        input.debug_screen_entry_status(DebugScreenEntryId::Fps),
        DebugScreenEntryStatus::AlwaysOn
    );
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(state.default_profile_active);
    assert!(!state.performance_profile_active);
}

#[test]
fn debug_options_screen_projects_button_hover_state() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("entity_hitboxes"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let content_x = debug_options_content_x(surface);
    let buttons_start = content_x + DEBUG_OPTIONS_ROW_WIDTH - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    let always_x = buttons_start + DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 2 + 2;
    let entry_y = DEBUG_OPTIONS_HEADER_HEIGHT + DEBUG_OPTIONS_ROW_HEIGHT + 2;

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(always_x),
            f64::from(entry_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    let hovered_status = state.rows.iter().find_map(|row| match row {
        DebugOptionsScreenHudRow::Entry {
            entry,
            hovered_status,
            ..
        } if *entry == DebugScreenEntryId::EntityHitboxes => *hovered_status,
        _ => None,
    });
    assert_eq!(hovered_status, Some(DebugScreenEntryStatus::AlwaysOn));
    assert!(!state.default_profile_hovered);
    assert!(!state.performance_profile_hovered);
    assert!(!state.done_hovered);

    let (default_x, performance_x, done_x) = debug_options_footer_button_xs(surface);
    let footer_y = debug_options_footer_button_y(surface) + 2;
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(default_x + 2),
            f64::from(footer_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(state.default_profile_hovered);
    assert!(!state.performance_profile_hovered);
    assert!(!state.done_hovered);

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(performance_x + 2),
            f64::from(footer_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(!state.default_profile_hovered);
    assert!(state.performance_profile_hovered);
    assert!(!state.done_hovered);

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(done_x + 2),
            f64::from(footer_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert!(!state.default_profile_hovered);
    assert!(!state.performance_profile_hovered);
    assert!(state.done_hovered);
}

#[test]
fn debug_options_screen_projects_not_allowed_tooltip_on_entry_name_hover() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    assert!(input.handle_debug_options_screen_text_input("biome"));
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let content_x = debug_options_content_x(surface);
    let name_x = content_x + 2;
    let entry_y = DEBUG_OPTIONS_HEADER_HEIGHT + DEBUG_OPTIONS_ROW_HEIGHT + 2;

    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(name_x),
            f64::from(entry_y)
        )),
        surface,
    ));
    let reduced = input
        .debug_options_screen_hud_state(surface, true)
        .expect("screen should project under reduced debug info");
    assert_eq!(
        reduced
            .tooltip
            .as_ref()
            .map(|tooltip| tooltip.text.as_str()),
        Some("Not visible when debug info is reduced")
    );
    assert_eq!(
        reduced
            .tooltip
            .as_ref()
            .map(|tooltip| (tooltip.x, tooltip.y)),
        Some((name_x, entry_y))
    );

    let full = input
        .debug_options_screen_hud_state(surface, false)
        .expect("screen should project without reduced debug info");
    assert_eq!(full.tooltip, None);

    let buttons_start = content_x + DEBUG_OPTIONS_ROW_WIDTH - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(buttons_start),
            f64::from(entry_y)
        )),
        surface,
    ));
    let over_button = input
        .debug_options_screen_hud_state(surface, true)
        .expect("screen should project with button hover");
    assert_eq!(over_button.tooltip, None);
}

#[test]
fn debug_options_screen_scrollbar_drag_updates_scroll_row() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    let surface = winit::dpi::PhysicalSize::new(420, 240);
    let scrollbar_x = debug_options_scrollbar_x(surface) + 1;
    let start_y = DEBUG_OPTIONS_HEADER_HEIGHT + 29;

    assert_eq!(
        debug_options_scrollbar_drag_scroll_row(0, 47, surface, start_y, start_y + 3),
        1
    );
    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Pressed,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(start_y)
        )),
        surface,
        false,
    ));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(start_y + 3)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.scroll_row, 1);

    let below_list_y = DEBUG_OPTIONS_HEADER_HEIGHT + debug_options_list_height(surface) + 1;
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(below_list_y)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.scroll_row, 40);

    assert!(input.handle_debug_options_screen_mouse_input(
        winit::event::MouseButton::Left,
        ElementState::Released,
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(below_list_y)
        )),
        surface,
        false,
    ));
    assert!(input.handle_debug_options_screen_cursor_moved(
        Some(winit::dpi::PhysicalPosition::new(
            f64::from(scrollbar_x),
            f64::from(DEBUG_OPTIONS_HEADER_HEIGHT)
        )),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.scroll_row, 40);
}

#[test]
fn debug_options_screen_consumes_keys_scrolls_and_allows_f3_global_keymap() {
    let mut input = ClientInputState::new(true);
    input.open_debug_options_screen();
    let surface = winit::dpi::PhysicalSize::new(420, 161);

    assert!(input.handle_debug_options_screen_mouse_wheel(
        winit::event::MouseScrollDelta::LineDelta(0.0, -3.0),
        surface,
    ));
    let state = input
        .debug_options_screen_hud_state(surface, false)
        .unwrap();
    assert_eq!(state.visible_rows, 3);
    assert_eq!(state.scroll_row, 3);

    assert!(!input
        .handle_debug_options_screen_key(PhysicalKey::Code(KeyCode::F3), ElementState::Pressed));
    assert!(input.debug_options_screen_is_open());
    assert!(input.handle_debug_options_screen_key(
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed
    ));
    assert!(!input.debug_options_screen_is_open());
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
fn releasing_active_input_preserves_f3_escape_debug_action_state() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    let mut counters = NetCounters::default();

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

    release_active_input(&mut input, &mut world, &mut counters, &None);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn debug_pause_screen_consumes_gameplay_keys_and_escape_closes() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    input.open_debug_pause_screen_without_menu();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::KeyW),
        ElementState::Pressed,
    );

    assert!(input.debug_pause_screen_is_open());
    assert!(!input.forward);
    assert!(!input.pressed_keys.contains(&KeyCode::KeyW));

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!input.debug_pause_screen_is_open());
    assert!(!input.pressed_keys.contains(&KeyCode::Escape));
}

#[test]
fn escape_opens_pause_screen_with_menu() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &None,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert_eq!(
        input.debug_pause_screen(),
        Some(DebugPauseScreenState {
            show_pause_menu: true,
            cursor_position: None
        })
    );
    assert!(!input.pressed_keys.contains(&KeyCode::Escape));
}

#[test]
fn debug_pause_screen_return_to_game_button_closes_menu_screen() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(input.handle_debug_pause_screen_cursor_moved(Some(PhysicalPosition::new(68.0, 78.0))));
    assert_eq!(
        input.debug_pause_screen(),
        Some(DebugPauseScreenState {
            show_pause_menu: true,
            cursor_position: Some((68, 78))
        })
    );
    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(68.0, 78.0)),
        surface,
    ));

    assert!(!input.debug_pause_screen_is_open());
}

#[test]
fn debug_pause_screen_advancements_button_opens_advancements_screen() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    world.apply_update_advancements(UpdateAdvancements {
        reset: true,
        added: vec![input_displayed_advancement("minecraft:story/root", None)],
        removed: Vec::new(),
        progress: Vec::new(),
        show_advancements: false,
    });
    input.open_debug_pause_screen_with_menu();

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(68.0, 102.0)),
        surface,
    ));

    assert!(!input.debug_pause_screen_is_open());
    assert!(world.advancements_screen_is_open());
    assert_eq!(
        world.selected_advancements_tab(),
        Some("minecraft:story/root")
    );
    assert_eq!(counters.advancements_seen_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SeenAdvancements(SeenAdvancements::OpenedTab {
            tab: "minecraft:story/root".to_string()
        })
    );
    assert!(rx.try_recv().is_err());
}

#[test]
fn debug_pause_screen_stats_button_opens_stats_screen_and_requests_stats() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(170.0, 102.0)),
        surface,
    ));

    assert!(!input.debug_pause_screen_is_open());
    assert!(world.stats_screen_is_open());
    assert_eq!(counters.request_stats_commands_queued, 1);
    assert_eq!(rx.try_recv().unwrap(), NetCommand::RequestStats);
    assert!(rx.try_recv().is_err());
}

#[test]
fn debug_pause_screen_send_feedback_button_records_link_request() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(68.0, 126.0)),
        surface,
    ));

    assert!(input.debug_pause_screen_is_open());
    assert_eq!(
        input.take_pause_screen_link_requests(),
        vec![PauseScreenLinkRequest::SendFeedback {
            url: "https://aka.ms/javafeedback?ref=game"
        }]
    );
}

#[test]
fn debug_pause_screen_report_bugs_button_records_link_request_when_enabled() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(pause_screen_report_bugs_button_enabled());
    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(170.0, 126.0)),
        surface,
    ));

    assert!(input.debug_pause_screen_is_open());
    assert_eq!(
        input.take_pause_screen_link_requests(),
        vec![PauseScreenLinkRequest::ReportBugs {
            url: "https://aka.ms/snapshotbugs?ref=game"
        }]
    );
}

#[test]
fn debug_pause_screen_disconnect_button_records_disconnect_request() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    input.open_debug_pause_screen_with_menu();

    assert!(input.handle_debug_pause_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(68.0, 198.0)),
        surface,
    ));

    assert!(input.debug_pause_screen_is_open());
    assert!(input.pause_screen_disconnect_requested());
    assert!(input.take_pause_screen_link_requests().is_empty());
}

#[test]
fn debug_pause_screen_still_allows_global_f3_overlay_toggle() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    input.open_debug_pause_screen_without_menu();

    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
    );
    handle_key_input_without_world(
        &mut input,
        &mut counters,
        &None,
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
    );

    assert!(input.debug_pause_screen_is_open());
    assert!(input.debug_overlay_visible());
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
fn f3_f4_game_mode_switcher_does_not_open_over_existing_screen() {
    #[derive(Debug, Clone, Copy)]
    enum ExistingScreen {
        Chat,
        LocalInventory,
        Book,
        Dialog,
        Advancements,
        SignEditor,
    }

    for existing_screen in [
        ExistingScreen::Chat,
        ExistingScreen::LocalInventory,
        ExistingScreen::Book,
        ExistingScreen::Dialog,
        ExistingScreen::Advancements,
        ExistingScreen::SignEditor,
    ] {
        let commands = None;
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_debug_player(false);
        grant_debug_recreate_nbt_permission(&mut world);

        match existing_screen {
            ExistingScreen::Chat => handle_key_input(
                &mut input,
                &mut counters,
                &mut world,
                &commands,
                PhysicalKey::Code(KeyCode::KeyT),
                ElementState::Pressed,
            ),
            ExistingScreen::LocalInventory => {
                assert!(world.open_local_inventory());
            }
            ExistingScreen::Book => open_test_book_screen(&mut world, vec!["First"]),
            ExistingScreen::Dialog => world.apply_show_dialog(ProtocolShowDialog {
                dialog: DialogHolder::Reference { registry_id: 11 },
            }),
            ExistingScreen::Advancements => {
                assert!(world.open_advancements_screen());
            }
            ExistingScreen::SignEditor => world.apply_open_sign_editor(OpenSignEditor {
                pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
                is_front_text: true,
            }),
        }

        assert!(input.handle_debug_overlay_key(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Pressed,
            Some(&mut world),
            None
        ));
        assert!(
            !input.handle_debug_overlay_key(
                PhysicalKey::Code(KeyCode::F4),
                ElementState::Pressed,
                Some(&mut world),
                None
            ),
            "F3+F4 should not be a debug action over {existing_screen:?}"
        );
        assert_eq!(input.debug_game_mode_switcher_selected(), None);
        assert!(world.client_chat().messages.is_empty());
        assert!(input.handle_debug_overlay_key(
            PhysicalKey::Code(KeyCode::F3),
            ElementState::Released,
            Some(&mut world),
            None
        ));
        assert!(
            input.debug_overlay_visible(),
            "blocked F4 should leave F3 release as an ordinary overlay toggle over {existing_screen:?}"
        );
    }
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
fn stats_screen_done_button_closes_stats_screen() {
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    let surface = PhysicalSize::new(320, 240);
    assert!(world.open_stats_screen());

    assert!(
        input.handle_stats_screen_cursor_moved(&world, Some(PhysicalPosition::new(70.0, 223.0)))
    );
    assert_eq!(input.stats_screen_cursor_position(), Some((70, 223)));
    assert!(input.handle_stats_screen_mouse_input(
        &mut counters,
        &mut world,
        &None,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(70.0, 223.0)),
        surface,
    ));

    assert!(!world.stats_screen_is_open());
    assert_eq!(input.stats_screen_cursor_position(), None);
}

#[test]
fn escape_key_closes_stats_screen_without_requesting_stats_again() {
    let (tx, mut rx) = mpsc::channel(1);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = WorldStore::new();
    assert!(world.open_stats_screen());

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );

    assert!(!world.stats_screen_is_open());
    assert_eq!(counters.request_stats_commands_queued, 0);
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
