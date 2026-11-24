mod chat_runtime;
mod command_editors;
use super::debug_recreate::{
    PendingDebugRecreateServerQuery, PendingDebugRecreateServerQueryTarget,
};
use super::FallbackTextMeasurer as FixedCommandSuggestionTextMeasurer;
use super::*;
use bbb_client_net::{OutboundChatCommand, PlayerMoveCommand};
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_world::vanilla_entity_metadata_entity_accessors;
use bbb_client_world::{
    BlockEntityRecord, BlockPos, BlockStateRegistry, ChatMessageKind, ChunkColumn, ChunkPos,
    ChunkSection, ChunkState, CommandBlockEntityDataState, JigsawBlockEntityDataState, LightData,
    LocalPlayerPoseState, PaletteDomain, PaletteKind, PalettedContainerData,
    SignBlockEntityTextState, StructureBlockEntityDataState, StructureBlockSizeState,
    TestBlockEntityDataState, TestInstanceBlockEntityDataState, TestInstanceBlockRunStatus,
    TestInstanceBlockSizeState, WorldStore,
};
use bbb_protocol::block_entity_types::{
    VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
    VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID, VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
    VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
};
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
};
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID,
    VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_CREEPER_ID,
    VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
    VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
    VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::equipment_slots::VANILLA_PLAYER_INVENTORY_CHEST_SLOT;
use bbb_protocol::item_types::VANILLA_ITEM_SPYGLASS_ID;
use bbb_protocol::menu_types::{
    VANILLA_MENU_TYPE_ANVIL_ID, VANILLA_MENU_TYPE_CRAFTING_ID, VANILLA_MENU_TYPE_FURNACE_ID,
    VANILLA_MENU_TYPE_GENERIC_9X1_ID, VANILLA_MENU_TYPE_GENERIC_9X3_ID,
    VANILLA_MENU_TYPE_MERCHANT_ID,
};
use bbb_protocol::packets::AwardStats;
use bbb_protocol::packets::BlockEntityData;
use bbb_protocol::packets::{
    AddEntity, AdvancementDisplaySummary, AdvancementFrameType, AdvancementIconSummary,
    AdvancementSummary, BlockEntityTagQuery, BlockPos as ProtocolBlockPos,
    BlockUpdate as ProtocolBlockUpdate, CommandArgumentParser, CommandNode, CommandNodeType,
    CommandSuggestion, CommandSuggestionRequest, CommandSuggestions, Commands,
    CommonPlayerSpawnInfo, ContainerClick, ContainerCloseRequest, ContainerInput,
    ContainerSetContent as ProtocolContainerSetContent, CustomChatCompletions,
    CustomChatCompletionsAction, DataComponentPatchSummary, DeleteChat, DialogHolder,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
    EntityEvent as ProtocolEntityEvent, EntityTagQuery, EquipmentSlot, EquipmentSlotUpdate,
    FilterMask, FilterMaskKind, GameEvent as ProtocolGameEvent, GameProfile, HashedComponentPatch,
    HashedItemStack, HashedStack, ItemStackSummary as ProtocolItemStackSummary, JigsawGenerate,
    JigsawJoint, LastSeenMessagesUpdate, MessageSignature, OpenBook,
    OpenScreen as ProtocolOpenScreen, OpenSignEditor, PackedMessageSignature, PaddleBoat,
    PlayLogin, PlayerAbilities, PlayerAbilitiesCommand, PlayerAction, PlayerChat, PlayerCommand,
    PlayerHealth, PlayerInfoAction, PlayerInfoEntry, PlayerInfoUpdate,
    RemoveEntities as ProtocolRemoveEntities, RenameItem, SeenAdvancements, SelectBundleItem,
    SetCommandMinecart, SetCursorItem as ProtocolSetCursorItem,
    SetEntityData as ProtocolSetEntityData, SetEquipment, SetJigsawBlock, SetPassengers,
    SetPlayerInventory as ProtocolSetPlayerInventory, SetStructureBlock, SetTestBlock,
    ShowDialog as ProtocolShowDialog, SignUpdate, SignedMessageBody, StatUpdate,
    StructureBlockMirror, StructureBlockMode, StructureBlockRotation, StructureBlockUpdateKind,
    TagQuery, TestBlockMode, TestInstanceBlockAction, TestInstanceBlockActionKind,
    TestInstanceBlockRotation, TestInstanceBlockStatus, UpdateAdvancements, Vec3d as ProtocolVec3d,
    Vec3i, WrittenBookContentSummary, VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
    VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
};
use bbb_protocol::packets::{ChatTypeBound, ChatTypeHolder};
use bbb_protocol::recipe_wire_types::{
    VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID, VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
    VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID, VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
};
use bbb_protocol::stat_types::{
    VANILLA_STAT_TYPE_ENTITY_KILLED_ID, VANILLA_STAT_TYPE_ITEM_CRAFTED_ID,
};
use bbb_protocol::{
    RichTextComponent, MC_BUILD_TIME, MC_DATA_PACK_FORMAT, MC_DATA_VERSION, MC_DATA_VERSION_SERIES,
    MC_RESOURCE_PACK_FORMAT, MC_STABLE, MC_VERSION, PROTOCOL_VERSION,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
mod block_editors;
mod debug_options_search;
mod debug_recreate;
mod non_fishy;
mod pause_if_inactive;
mod screen_replacement;
mod sign_clipboard;
mod stats_screen;

const TEST_SADDLE_ITEM_ID: i32 = 8_903;
const TEST_GLIDER_ITEM_ID: i32 = 8_904;

#[cfg(target_os = "macos")]
const TEXT_EDIT_SHORTCUT_KEY: KeyCode = KeyCode::SuperLeft;
#[cfg(not(target_os = "macos"))]
const TEXT_EDIT_SHORTCUT_KEY: KeyCode = KeyCode::ControlLeft;

fn set_text_edit_shortcut_down(input: &mut ClientInputState, pressed: bool) {
    input.set_key_down(TEXT_EDIT_SHORTCUT_KEY, pressed);
    input.set_control_key(TEXT_EDIT_SHORTCUT_KEY, pressed);
}

fn entity_metadata_accessors() -> bbb_client_world::VanillaEntityMetadataAccessors {
    let accessors = vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert_eq!(accessors.shared_flags_default, 0);
    assert!(!accessors.custom_name_visible_default);
    assert!(!accessors.silent_default);
    assert!(!accessors.no_gravity_default);
    assert_eq!(accessors.ticks_frozen_default, 0);
    accessors
}

fn entity_pose_data(pose_id: i32) -> ProtocolEntityDataValue {
    let accessors = entity_metadata_accessors();
    ProtocolEntityDataValue {
        data_id: accessors.pose_data_id,
        serializer_id:
            bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
        value: EntityDataValueKind::Pose(pose_id),
    }
}

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

fn finish_test_level_load(world: &mut WorldStore) {
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id:
            bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
        param: 0.0,
    });
    assert!(world.complete_client_level_load(false));
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
    finish_test_level_load(&mut world);
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

fn grant_gamemaster_block_permission(world: &mut WorldStore) {
    grant_debug_recreate_nbt_permission(world);
    set_player_abilities(world, false, true);
    assert!(world.local_player_can_use_gamemaster_blocks());
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
    finish_test_level_load(&mut world);
    world
}

fn world_with_reduced_debug_info(player_id: i32) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_login(&PlayLogin {
        player_id,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: true,
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
    finish_test_level_load(&mut world);
    world
}

fn set_local_player_sprint_test_pose(world: &mut WorldStore) {
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        },
        on_ground: true,
        ..LocalPlayerPoseState::default()
    });
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

fn world_with_local_boat(player_id: i32) -> WorldStore {
    world_with_local_vehicle(player_id, 10, VANILLA_ENTITY_TYPE_OAK_BOAT_ID)
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
    if entity_type_id == VANILLA_ENTITY_TYPE_HORSE_ID {
        crate::item_registry_test_fixture::install_equippable_item_default(
            &mut world,
            TEST_SADDLE_ITEM_ID,
            "saddle",
        );
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

fn sign_text_side(lines: [&str; 4]) -> bbb_client_world::SignTextSideState {
    bbb_client_world::SignTextSideState {
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
        filtered_lines: None,
        color: bbb_client_world::SignTextDyeColor::Black,
        has_glowing_text: false,
    }
}

fn sign_text_side_with_filtered(
    lines: [&str; 4],
    filtered: [&str; 4],
) -> bbb_client_world::SignTextSideState {
    let mut side = sign_text_side(lines);
    side.filtered_lines = Some(sign_text_side(filtered).lines);
    side
}

fn world_with_sign_text(pos: BlockPos, front: [&str; 4], back: [&str; 4]) -> WorldStore {
    let mut world = WorldStore::new();
    insert_sign_text(&mut world, pos, front, back);
    world
}

fn insert_sign_text(world: &mut WorldStore, pos: BlockPos, front: [&str; 4], back: [&str; 4]) {
    insert_sign_text_state(world, pos, sign_text_side(front), sign_text_side(back));
}

fn insert_sign_text_state(
    world: &mut WorldStore,
    pos: BlockPos,
    front: bbb_client_world::SignTextSideState,
    back: bbb_client_world::SignTextSideState,
) {
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: Vec::new(),
        block_entities: std::sync::Arc::new(vec![BlockEntityRecord::empty(
            pos.x.rem_euclid(16) as u8,
            i16::try_from(pos.y).unwrap(),
            pos.z.rem_euclid(16) as u8,
            7,
        )
        .with_raw_nbt(vec![0])
        .with_payload(bbb_client_world::BlockEntityPayload::SignText(Box::new(
            SignBlockEntityTextState {
                front,
                back,
                is_waxed: false,
            },
        )))]),
        light: LightData::default(),
    });
}

fn insert_empty_chunk_for_block(world: &mut WorldStore, pos: BlockPos) {
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|_| std::sync::Arc::new(empty_chunk_section()))
            .collect(),
        block_entities: std::sync::Arc::default(),
        light: LightData::default(),
    });
}

fn world_with_command_block(pos: BlockPos, command: &str) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_gamemaster_block_permission(&mut world);
    let block_state_id = bbb_client_world::vanilla_block_state_id(
        "minecraft:command_block",
        &BTreeMap::from([
            ("conditional".to_string(), "false".to_string()),
            ("facing".to_string(), "north".to_string()),
        ]),
    )
    .expect("vanilla command block state");
    let section_index = usize::try_from((pos.y + 64).div_euclid(16))
        .expect("test command block lies in the overworld build height");
    let record = BlockEntityRecord::empty(
        pos.x.rem_euclid(16) as u8,
        i16::try_from(pos.y).unwrap(),
        pos.z.rem_euclid(16) as u8,
        VANILLA_BLOCK_ENTITY_TYPE_COMMAND_BLOCK_ID,
    )
    .with_raw_nbt(vec![0])
    .with_payload(bbb_client_world::BlockEntityPayload::CommandBlock(
        Box::new(CommandBlockEntityDataState {
            command: command.into(),
            ..CommandBlockEntityDataState::default()
        }),
    ));
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|index| {
                std::sync::Arc::new(ChunkSection {
                    non_empty_block_count: if index == section_index { 4096 } else { 0 },
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        if index == section_index {
                            block_state_id
                        } else {
                            0
                        },
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
            })
            .collect(),
        block_entities: std::sync::Arc::new(vec![record]),
        light: LightData::default(),
    });
    world
}

fn world_with_command_minecart(entity_id: i32, command: &str, output: &str) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_gamemaster_block_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: entity_id,
        uuid: Uuid::from_u128(u128::try_from(entity_id).unwrap()),
        entity_type_id: VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
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
    apply_command_minecart_metadata(&mut world, entity_id, command, output);
    world
}

fn apply_command_minecart_metadata(
    world: &mut WorldStore,
    entity_id: i32,
    command: &str,
    output: &str,
) {
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: entity_id,
        values: vec![
            ProtocolEntityDataValue {
                data_id: bbb_client_world::VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_STRING_ID,
                value: EntityDataValueKind::String(command.into()),
            },
            ProtocolEntityDataValue {
                data_id: bbb_client_world::VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_COMPONENT_ID,
                value: EntityDataValueKind::Component(RichTextComponent::plain(output)),
            },
        ],
    }));
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
        unsigned_content_styled: None,
        unsigned_content_component: None,
        filter_mask: FilterMask {
            kind: FilterMaskKind::PassThrough,
            mask_words: Vec::new(),
        },
        chat_type: ChatTypeBound {
            chat_type: ChatTypeHolder::Registry { id: 0 },
            name: "Alice".to_string(),
            name_styled: Vec::new(),
            name_component: None,
            target_name: None,
            target_name_styled: None,
            target_name_component: None,
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
                flags: 22,
                children: Vec::new(),
                redirect: None,
                name: Some("message".to_string()),
                parser: Some(CommandArgumentParser {
                    type_id: VANILLA_COMMAND_ARGUMENT_TYPE_MESSAGE_ID,
                    name: "minecraft:message".to_string(),
                    properties: Vec::new(),
                }),
                suggestions: Some("minecraft:ask_server".to_string()),
                executable: true,
                restricted: false,
            },
        ],
    }
}

fn ask_server_command_tree() -> Commands {
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
                node_type: CommandNodeType::Argument,
                flags: 18,
                children: Vec::new(),
                redirect: None,
                name: Some("value".to_string()),
                parser: Some(CommandArgumentParser {
                    type_id: VANILLA_COMMAND_ARGUMENT_TYPE_BRIGADIER_STRING_ID,
                    name: "brigadier:string".to_string(),
                    properties: vec![2],
                }),
                suggestions: Some("minecraft:ask_server".to_string()),
                executable: true,
                restricted: false,
            },
        ],
    }
}

fn local_literal_command_tree(literals: &[&str]) -> Commands {
    let children = (1..=literals.len())
        .map(|index| i32::try_from(index).expect("fixture node index"))
        .collect();
    let mut nodes = vec![CommandNode {
        node_type: CommandNodeType::Root,
        flags: 0,
        children,
        redirect: None,
        name: None,
        parser: None,
        suggestions: None,
        executable: false,
        restricted: false,
    }];
    nodes.extend(literals.iter().map(|literal| CommandNode {
        node_type: CommandNodeType::Literal,
        flags: 5,
        children: Vec::new(),
        redirect: None,
        name: Some((*literal).to_string()),
        parser: None,
        suggestions: None,
        executable: true,
        restricted: false,
    }));
    Commands {
        root_index: 0,
        nodes,
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

fn equip_test_glider(world: &mut WorldStore, damage: i32) {
    world.set_default_item_component_codec_values(BTreeMap::from([(
        TEST_GLIDER_ITEM_ID,
        BTreeMap::from([
            (
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                serde_json::json!(1),
            ),
            (
                VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
                serde_json::json!(432),
            ),
            (VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, serde_json::json!(0)),
            (VANILLA_DATA_COMPONENT_GLIDER_TYPE_ID, serde_json::json!({})),
            (
                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                serde_json::json!({"slot": "chest"}),
            ),
        ]),
    )]));
    world.rebuild_item_component_defaults().unwrap();
    let mut item = ProtocolItemStackSummary {
        item_id: Some(TEST_GLIDER_ITEM_ID),
        count: 1,
        component_patch: Default::default(),
    };
    if damage != 0 {
        item.component_patch.added_type_ids = vec![VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID];
        item.component_patch.damage = Some(damage);
    }
    world.apply_set_player_inventory(ProtocolSetPlayerInventory {
        slot: VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
        item,
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
        entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
        position: ProtocolVec3d::default(),
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    assert!(world.apply_set_entity_data(ProtocolSetEntityData {
        id: player_id,
        values: vec![entity_pose_data(VANILLA_ENTITY_POSE_SLEEPING_ID)],
    }));
    world
}

fn open_test_stats_screen(input: &mut ClientInputState) {
    input.set_screen_for_test(Screen::Stats(StatsScreen::default()));
}

fn open_test_advancements_screen(input: &mut ClientInputState) {
    input.set_screen_for_test(Screen::Advancements(AdvancementsScreen::default()));
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
        title: bbb_protocol::RichTextComponent::plain(id),
        description: bbb_protocol::RichTextComponent::plain(""),
        icon: AdvancementIconSummary {
            item_id: 1,
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
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

fn test_item_stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: Default::default(),
    }
}

fn open_test_book_screen(input: &mut ClientInputState, world: &mut WorldStore, pages: Vec<&str>) {
    let mut stack = test_item_stack(42, 1);
    let pages: Vec<String> = pages.into_iter().map(str::to_string).collect();
    let page_filters = vec![None; pages.len()];
    stack.component_patch.written_book = Some(WrittenBookContentSummary {
        title: "Guide".into(),
        title_filter: None,
        author: "Alex".into(),
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
    input.apply_screen_requests(world, &mut NetCounters::default(), &None);
    assert!(input.book_screen().is_some());
}

fn test_bundle_stack(item_id: i32, count: i32, contents_count: usize) -> ProtocolItemStackSummary {
    let mut stack = test_item_stack(item_id, count);
    stack
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID);
    stack.component_patch.bundle_contents_items = (0..contents_count)
        .map(|_| bbb_protocol::packets::ItemStackTemplateSummary {
            item_id: bbb_protocol::item_types::VANILLA_ITEM_STONE_ID,
            count: 1,
            component_patch: Default::default(),
        })
        .collect();
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
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X1_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
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
        menu_type_id: VANILLA_MENU_TYPE_CRAFTING_ID,
        title: bbb_protocol::RichTextComponent::plain("Crafting"),
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
        menu_type_id: VANILLA_MENU_TYPE_FURNACE_ID,
        title: bbb_protocol::RichTextComponent::plain("Furnace"),
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
                crafting: Some(Box::new(
                    bbb_protocol::packets::CraftingRecipeDisplaySummary::Shapeless {
                        ingredients: Vec::new(),
                        result: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
                            raw_payload: Vec::new(),
                            item_stack: Some(test_item_stack(result_item_id, 1)),
                            tag: None,
                        },
                        crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                            raw_payload: Vec::new(),
                            item_stack: None,
                            tag: None,
                        },
                    },
                )),
                furnace: None,
                stonecutter: None,
                smithing: None,
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
                furnace: Some(Box::new(
                    bbb_protocol::packets::FurnaceRecipeDisplaySummary {
                        ingredient: recipe_book_slot_display_item(1),
                        fuel: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ANY_FUEL_ID,
                            raw_payload: vec![1],
                            item_stack: None,
                            tag: None,
                        },
                        result: recipe_book_slot_display_stack(result_item_id, 1),
                        crafting_station: bbb_protocol::packets::SlotDisplaySummary {
                            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_EMPTY_ID,
                            raw_payload: vec![0],
                            item_stack: None,
                            tag: None,
                        },
                        duration: 200,
                        experience_bits: 0.0_f32.to_bits(),
                    },
                )),
                stonecutter: None,
                smithing: None,
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
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID);
    payload.write_var_i32(item_id);
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
        raw_payload: payload.into_inner(),
        item_stack: Some(test_item_stack(item_id, 1)),
        tag: None,
    }
}

fn recipe_book_slot_display_stack(
    item_id: i32,
    count: i32,
) -> bbb_protocol::packets::SlotDisplaySummary {
    let mut payload = bbb_protocol::codec::Encoder::new();
    payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID);
    payload.write_var_i32(item_id);
    payload.write_var_i32(count);
    payload.write_var_i32(0);
    payload.write_var_i32(0);
    bbb_protocol::packets::SlotDisplaySummary {
        display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_STACK_ID,
        raw_payload: payload.into_inner(),
        item_stack: Some(test_item_stack(item_id, count)),
        tag: None,
    }
}

fn open_container_slot_bundle_selection(world: &WorldStore, slot: i16) -> Option<i32> {
    world
        .inventory()
        .container_slot(world.inventory().open_container.as_ref()?, slot)
        .map(|state| state.local_selected_bundle_item_index)
}

fn test_open_container_slot_item(world: &WorldStore, slot: i16) -> ProtocolItemStackSummary {
    world
        .inventory()
        .open_container
        .as_ref()
        .and_then(|container| world.inventory().container_slot(container, slot))
        .map(|state| state.item.clone())
        .unwrap_or_else(ProtocolItemStackSummary::empty)
}

fn anvil_container_world(
    container_id: i32,
    state_id: i32,
    input_item: Option<ProtocolItemStackSummary>,
) -> WorldStore {
    let mut world = WorldStore::new();
    world.apply_open_screen(ProtocolOpenScreen {
        container_id,
        menu_type_id: VANILLA_MENU_TYPE_ANVIL_ID,
        title: bbb_protocol::RichTextComponent::plain("Anvil"),
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
    crate::item_registry_test_fixture::write_item_registry_fixture(root, &["test_combo"]);
}

fn input_assets_dir(root: &Path) -> PathBuf {
    root.join("sources")
        .join(bbb_client_assets::MC_VERSION)
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
    std::env::temp_dir().join(format!("bbb-client-input-{label}-{nanos}"))
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

fn press_key(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
) {
    handle_key_input(
        input,
        counters,
        world,
        net_commands,
        PhysicalKey::Code(code),
        ElementState::Pressed,
    );
}

fn press_key_with_clipboard(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    world: &mut WorldStore,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    code: KeyCode,
    clipboard: &mut MockDebugClipboard,
) {
    crate::input::key_events::dispatch_key_input(
        input,
        counters,
        world,
        net_commands,
        None,
        PhysicalKey::Code(code),
        ElementState::Pressed,
        PhysicalSize::new(1280, 720),
        &FixedCommandSuggestionTextMeasurer,
        Some(clipboard),
        None,
        &mut true,
        &mut crate::simulation::effects::PlatformEffects::default(),
    );
}

mod chat_and_suggestions;
mod container;
mod debug_keys_and_charts;
mod debug_recreate_and_tag_queries;
mod debug_screens_and_gamemode;
mod hotbar_clipboard_and_ime;
mod movement_and_focus;
mod recipe_book_and_advancements;
mod sign_and_containers;

use debug_recreate_and_tag_queries::{nbt_compound, nbt_string};
