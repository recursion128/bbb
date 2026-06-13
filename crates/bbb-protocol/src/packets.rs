use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{decode_component_summary, decode_component_summary_from_decoder},
    ids, PROTOCOL_VERSION,
};

pub mod chunks;
pub mod client_state;
pub mod command_suggestions;
pub mod entities;
pub mod inventory;
pub mod movement;
pub mod player_info;
pub mod scoreboard;
pub mod server_presentation;
pub mod world_border;
pub use chunks::*;
pub use client_state::*;
pub use command_suggestions::*;
pub use entities::*;
pub use inventory::*;
pub use movement::*;
pub use player_info::*;
pub use scoreboard::*;
pub use server_presentation::*;
pub use world_border::*;

const MAX_AWARD_STATS: usize = 8192;
const MAX_SERVER_ICON_BYTES: usize = 2 * 1024 * 1024;
const PLAYER_INPUT_FORWARD: u8 = 1;
const PLAYER_INPUT_BACKWARD: u8 = 2;
const PLAYER_INPUT_LEFT: u8 = 4;
const PLAYER_INPUT_RIGHT: u8 = 8;
const PLAYER_INPUT_JUMP: u8 = 16;
const PLAYER_INPUT_SHIFT: u8 = 32;
const PLAYER_INPUT_SPRINT: u8 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ClientIntent {
    Status = 1,
    Login = 2,
    Transfer = 3,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameProfile {
    pub uuid: Uuid,
    pub name: String,
    pub properties: Vec<GameProfileProperty>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginClientbound {
    Disconnect { raw_json: String },
    EncryptionRequest,
    LoginFinished { profile: GameProfile },
    SetCompression { threshold: i32 },
    CustomQuery { transaction_id: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigurationClientbound {
    Finish,
    KeepAlive {
        id: i64,
    },
    Ping {
        id: i32,
    },
    RegistryData {
        registry: String,
        raw_payload_len: usize,
    },
    SelectKnownPacks {
        offered: usize,
    },
    Unknown {
        packet_id: i32,
        len: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlayClientbound {
    BundleDelimiter,
    AddEntity(AddEntity),
    EntityAnimation(EntityAnimation),
    AwardStats(AwardStats),
    BlockChangedAck(BlockChangedAck),
    BlockDestruction(BlockDestruction),
    BlockEntityData(BlockEntityData),
    BlockEvent(BlockEvent),
    BlockUpdate(BlockUpdate),
    BossEvent(BossEvent),
    ChangeDifficulty(ChangeDifficulty),
    ChunkBatchStart,
    ChunkBatchFinished { batch_size: i32 },
    ChunksBiomes(ChunksBiomes),
    CommandSuggestions(CommandSuggestions),
    ContainerClose(ContainerClose),
    ContainerSetContent(ContainerSetContent),
    ContainerSetData(ContainerSetData),
    ContainerSetSlot(ContainerSetSlot),
    Cooldown(Cooldown),
    DamageEvent(DamageEvent),
    Disconnect(Disconnect),
    EntityEvent(EntityEvent),
    EntityPositionSync(EntityPositionSync),
    ForgetLevelChunk(ForgetLevelChunk),
    GameEvent(GameEvent),
    HurtAnimation(HurtAnimation),
    InitializeBorder(InitializeBorder),
    KeepAlive { id: i64 },
    LevelEvent(LevelEvent),
    Ping { id: i32 },
    Login(PlayLogin),
    MoveEntity(EntityMove),
    MoveVehicle(MoveVehicle),
    OpenScreen(OpenScreen),
    PlayerPosition(PlayerPositionUpdate),
    PlayerAbilities(PlayerAbilities),
    PlayerInfoRemove(PlayerInfoRemove),
    PlayerInfoUpdate(PlayerInfoUpdate),
    PlayerRotation(PlayerRotationUpdate),
    RemoveEntities(RemoveEntities),
    RemoveMobEffect(RemoveMobEffect),
    ResetScore(ResetScore),
    ResourcePackPop(ResourcePackPop),
    ResourcePackPush(ResourcePackPush),
    Respawn(Respawn),
    RotateHead(RotateHead),
    ServerData(ServerData),
    SetActionBarText(SetActionBarText),
    SetBorderCenter(SetBorderCenter),
    SetBorderLerpSize(SetBorderLerpSize),
    SetBorderSize(SetBorderSize),
    SetBorderWarningDelay(SetBorderWarningDelay),
    SetBorderWarningDistance(SetBorderWarningDistance),
    SetCamera(SetCamera),
    SetCursorItem(SetCursorItem),
    SetDefaultSpawnPosition(SetDefaultSpawnPosition),
    SetEntityData(SetEntityData),
    SetEntityLink(SetEntityLink),
    SetEntityMotion(SetEntityMotion),
    SetEquipment(SetEquipment),
    SetExperience(PlayerExperience),
    SetHealth(PlayerHealth),
    SetHeldSlot(SetHeldSlot),
    SetPassengers(SetPassengers),
    SetPlayerInventory(SetPlayerInventory),
    SectionBlocksUpdate(SectionBlocksUpdate),
    SetChunkCacheCenter(SetChunkCacheCenter),
    SetChunkCacheRadius(SetChunkCacheRadius),
    SetSimulationDistance(SetSimulationDistance),
    SetDisplayObjective(SetDisplayObjective),
    SetObjective(SetObjective),
    SetPlayerTeam(SetPlayerTeam),
    SetScore(SetScore),
    SetSubtitleText(SetSubtitleText),
    StartConfiguration,
    SetTime(PlayTime),
    SetTitleText(SetTitleText),
    SetTitlesAnimation(SetTitlesAnimation),
    SystemChat(SystemChat),
    TabList(TabList),
    TakeItemEntity(TakeItemEntity),
    TeleportEntity(TeleportEntity),
    TickingState(TickingState),
    TickingStep(TickingStep),
    UpdateAttributes(UpdateAttributes),
    UpdateMobEffect(UpdateMobEffect),
    LevelChunkWithLight(LevelChunkWithLight),
    LightUpdate(LightUpdate),
    Unknown { packet_id: i32, len: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AwardStats {
    pub stats: Vec<StatUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatUpdate {
    pub stat_type_id: i32,
    pub value_id: i32,
    pub amount: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disconnect {
    pub reason: String,
    pub raw_reason: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayLogin {
    pub player_id: i32,
    pub hardcore: bool,
    pub levels: Vec<String>,
    pub max_players: i32,
    pub chunk_radius: i32,
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub show_death_screen: bool,
    pub do_limited_crafting: bool,
    pub common_spawn_info: CommonPlayerSpawnInfo,
    pub enforces_secure_chat: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonPlayerSpawnInfo {
    pub dimension_type_id: i32,
    pub dimension: String,
    pub seed: i64,
    pub game_type: i8,
    pub previous_game_type: i8,
    pub is_debug: bool,
    pub is_flat: bool,
    pub last_death_location: Option<GlobalPos>,
    pub portal_cooldown: i32,
    pub sea_level: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalPos {
    pub dimension: String,
    pub pos: BlockPos,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Respawn {
    pub common_spawn_info: CommonPlayerSpawnInfo,
    pub data_to_keep: i8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub shift: bool,
    pub sprint: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCommand {
    pub entity_id: i32,
    pub action: PlayerCommandAction,
    pub data: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAction {
    pub action: PlayerActionKind,
    pub pos: BlockPos,
    pub direction: Direction,
    pub sequence: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BlockHitResult {
    pub pos: BlockPos,
    pub direction: Direction,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_z: f32,
    pub inside: bool,
    pub world_border_hit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UseItemOn {
    pub hand: InteractionHand,
    pub hit: BlockHitResult,
    pub sequence: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct UseItem {
    pub hand: InteractionHand,
    pub sequence: i32,
    pub y_rot: f32,
    pub x_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PickItemFromBlock {
    pub pos: BlockPos,
    pub include_data: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerActionKind {
    StartDestroyBlock,
    AbortDestroyBlock,
    StopDestroyBlock,
    DropAllItems,
    DropItem,
    ReleaseUseItem,
    SwapItemWithOffhand,
    Stab,
}

impl PlayerActionKind {
    fn ordinal(self) -> i32 {
        match self {
            Self::StartDestroyBlock => 0,
            Self::AbortDestroyBlock => 1,
            Self::StopDestroyBlock => 2,
            Self::DropAllItems => 3,
            Self::DropItem => 4,
            Self::ReleaseUseItem => 5,
            Self::SwapItemWithOffhand => 6,
            Self::Stab => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Direction {
    pub fn id(self) -> u8 {
        match self {
            Self::Down => 0,
            Self::Up => 1,
            Self::North => 2,
            Self::South => 3,
            Self::West => 4,
            Self::East => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerCommandAction {
    StopSleeping,
    StartSprinting,
    StopSprinting,
    StartRidingJump,
    StopRidingJump,
    OpenInventory,
    StartFallFlying,
}

impl PlayerCommandAction {
    fn ordinal(self) -> i32 {
        match self {
            Self::StopSleeping => 0,
            Self::StartSprinting => 1,
            Self::StopSprinting => 2,
            Self::StartRidingJump => 3,
            Self::StopRidingJump => 4,
            Self::OpenInventory => 5,
            Self::StartFallFlying => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionHand {
    MainHand,
    OffHand,
}

impl InteractionHand {
    fn id(self) -> i32 {
        match self {
            Self::MainHand => 0,
            Self::OffHand => 1,
        }
    }
}

pub fn encode_handshake(host: &str, port: u16, intent: ClientIntent) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(PROTOCOL_VERSION);
    out.write_string(host);
    out.write_u16(port);
    out.write_var_i32(intent as i32);
    (
        ids::handshake::SERVERBOUND_CLIENT_INTENTION,
        out.into_inner(),
    )
}

pub fn encode_status_request() -> (i32, Vec<u8>) {
    (ids::status::SERVERBOUND_STATUS_REQUEST, Vec::new())
}

pub fn encode_ping_request(time: i64) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(time);
    (ids::status::SERVERBOUND_PING_REQUEST, out.into_inner())
}

pub fn decode_status_response(payload: &[u8]) -> Result<String> {
    Decoder::new(payload).read_string(32767)
}

pub fn decode_pong_response(payload: &[u8]) -> Result<i64> {
    Decoder::new(payload).read_i64()
}

pub fn encode_login_hello(username: &str, profile_id: Uuid) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_string(username);
    out.write_uuid(profile_id);
    (ids::login::SERVERBOUND_HELLO, out.into_inner())
}

pub fn encode_login_acknowledged() -> (i32, Vec<u8>) {
    (ids::login::SERVERBOUND_LOGIN_ACKNOWLEDGED, Vec::new())
}

pub fn decode_login_clientbound(packet_id: i32, payload: &[u8]) -> Result<LoginClientbound> {
    match packet_id {
        ids::login::CLIENTBOUND_LOGIN_DISCONNECT => Ok(LoginClientbound::Disconnect {
            raw_json: Decoder::new(payload).read_string(262144)?,
        }),
        ids::login::CLIENTBOUND_HELLO => Ok(LoginClientbound::EncryptionRequest),
        ids::login::CLIENTBOUND_LOGIN_FINISHED => {
            let mut decoder = Decoder::new(payload);
            Ok(LoginClientbound::LoginFinished {
                profile: decode_game_profile(&mut decoder)?,
            })
        }
        ids::login::CLIENTBOUND_LOGIN_COMPRESSION => Ok(LoginClientbound::SetCompression {
            threshold: Decoder::new(payload).read_var_i32()?,
        }),
        ids::login::CLIENTBOUND_CUSTOM_QUERY => Ok(LoginClientbound::CustomQuery {
            transaction_id: Decoder::new(payload).read_var_i32()?,
        }),
        id => Err(ProtocolError::UnknownPacket { state: "login", id }),
    }
}

pub fn encode_client_information_default() -> (i32, Vec<u8>) {
    (
        ids::configuration::SERVERBOUND_CLIENT_INFORMATION,
        encode_client_information_payload_default(),
    )
}

fn encode_client_information_payload_default() -> Vec<u8> {
    let mut out = Encoder::new();
    out.write_string("en_us");
    out.write_i8(10);
    out.write_var_i32(0);
    out.write_bool(true);
    out.write_u8(0x7f);
    out.write_var_i32(1);
    out.write_bool(false);
    out.write_bool(false);
    out.write_var_i32(0);
    out.into_inner()
}

pub fn encode_configuration_finish() -> (i32, Vec<u8>) {
    (
        ids::configuration::SERVERBOUND_FINISH_CONFIGURATION,
        Vec::new(),
    )
}

pub fn encode_configuration_keep_alive(id: i64) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(id);
    (ids::configuration::SERVERBOUND_KEEP_ALIVE, out.into_inner())
}

pub fn encode_configuration_pong(id: i32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i32(id);
    (ids::configuration::SERVERBOUND_PONG, out.into_inner())
}

pub fn encode_select_known_packs_empty() -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(0);
    (
        ids::configuration::SERVERBOUND_SELECT_KNOWN_PACKS,
        out.into_inner(),
    )
}

pub fn decode_configuration_clientbound(
    packet_id: i32,
    payload: &[u8],
) -> Result<ConfigurationClientbound> {
    match packet_id {
        ids::configuration::CLIENTBOUND_FINISH_CONFIGURATION => {
            Ok(ConfigurationClientbound::Finish)
        }
        ids::configuration::CLIENTBOUND_KEEP_ALIVE => Ok(ConfigurationClientbound::KeepAlive {
            id: Decoder::new(payload).read_i64()?,
        }),
        ids::configuration::CLIENTBOUND_PING => Ok(ConfigurationClientbound::Ping {
            id: Decoder::new(payload).read_i32()?,
        }),
        ids::configuration::CLIENTBOUND_REGISTRY_DATA => {
            let mut decoder = Decoder::new(payload);
            let registry = decoder.read_string(32767)?;
            Ok(ConfigurationClientbound::RegistryData {
                registry,
                raw_payload_len: payload.len(),
            })
        }
        ids::configuration::CLIENTBOUND_SELECT_KNOWN_PACKS => {
            let mut decoder = Decoder::new(payload);
            let offered = decoder.read_len()?;
            Ok(ConfigurationClientbound::SelectKnownPacks { offered })
        }
        id => Ok(ConfigurationClientbound::Unknown {
            packet_id: id,
            len: payload.len(),
        }),
    }
}

pub fn encode_play_keep_alive(id: i64) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(id);
    (ids::play::SERVERBOUND_KEEP_ALIVE, out.into_inner())
}

pub fn encode_play_pong(id: i32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i32(id);
    (ids::play::SERVERBOUND_PONG, out.into_inner())
}

pub fn encode_play_client_information_default() -> (i32, Vec<u8>) {
    (
        ids::play::SERVERBOUND_CLIENT_INFORMATION,
        encode_client_information_payload_default(),
    )
}

pub fn encode_play_move_vehicle(
    x: f64,
    y: f64,
    z: f64,
    y_rot: f32,
    x_rot: f32,
    on_ground: bool,
) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_f64(x);
    out.write_f64(y);
    out.write_f64(z);
    out.write_f32(y_rot);
    out.write_f32(x_rot);
    out.write_bool(on_ground);
    (ids::play::SERVERBOUND_MOVE_VEHICLE, out.into_inner())
}

pub fn encode_play_player_loaded() -> (i32, Vec<u8>) {
    (ids::play::SERVERBOUND_PLAYER_LOADED, Vec::new())
}

pub fn encode_play_client_tick_end() -> (i32, Vec<u8>) {
    (ids::play::SERVERBOUND_CLIENT_TICK_END, Vec::new())
}

pub fn encode_play_player_input(input: PlayerInput) -> (i32, Vec<u8>) {
    let mut flags = 0u8;
    if input.forward {
        flags |= PLAYER_INPUT_FORWARD;
    }
    if input.backward {
        flags |= PLAYER_INPUT_BACKWARD;
    }
    if input.left {
        flags |= PLAYER_INPUT_LEFT;
    }
    if input.right {
        flags |= PLAYER_INPUT_RIGHT;
    }
    if input.jump {
        flags |= PLAYER_INPUT_JUMP;
    }
    if input.shift {
        flags |= PLAYER_INPUT_SHIFT;
    }
    if input.sprint {
        flags |= PLAYER_INPUT_SPRINT;
    }

    let mut out = Encoder::new();
    out.write_u8(flags);
    (ids::play::SERVERBOUND_PLAYER_INPUT, out.into_inner())
}

pub fn encode_play_player_command(command: PlayerCommand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(command.entity_id);
    out.write_var_i32(command.action.ordinal());
    out.write_var_i32(command.data);
    (ids::play::SERVERBOUND_PLAYER_COMMAND, out.into_inner())
}

pub fn encode_play_player_action(action: PlayerAction) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(action.action.ordinal());
    out.write_i64(chunks::encode_block_pos(action.pos));
    out.write_u8(action.direction.id());
    out.write_var_i32(action.sequence);
    (ids::play::SERVERBOUND_PLAYER_ACTION, out.into_inner())
}

pub fn encode_play_swing(hand: InteractionHand) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(hand.id());
    (ids::play::SERVERBOUND_SWING, out.into_inner())
}

pub fn encode_play_use_item_on(packet: UseItemOn) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.hand.id());
    chunks::encode_block_hit_result(&mut out, packet.hit);
    out.write_var_i32(packet.sequence);
    (ids::play::SERVERBOUND_USE_ITEM_ON, out.into_inner())
}

pub fn encode_play_use_item(packet: UseItem) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(packet.hand.id());
    out.write_var_i32(packet.sequence);
    out.write_f32(packet.y_rot);
    out.write_f32(packet.x_rot);
    (ids::play::SERVERBOUND_USE_ITEM, out.into_inner())
}

pub fn encode_play_pick_item_from_block(packet: PickItemFromBlock) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i64(chunks::encode_block_pos(packet.pos));
    out.write_bool(packet.include_data);
    (
        ids::play::SERVERBOUND_PICK_ITEM_FROM_BLOCK,
        out.into_inner(),
    )
}

pub fn encode_play_set_carried_item(slot: i16) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_i16(slot);
    (ids::play::SERVERBOUND_SET_CARRIED_ITEM, out.into_inner())
}

pub fn encode_play_chunk_batch_received(desired_chunks_per_tick: f32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_f32(desired_chunks_per_tick);
    (
        ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED,
        out.into_inner(),
    )
}

pub fn encode_play_perform_respawn() -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(0);
    (ids::play::SERVERBOUND_CLIENT_COMMAND, out.into_inner())
}

pub fn encode_play_configuration_acknowledged() -> (i32, Vec<u8>) {
    (
        ids::play::SERVERBOUND_CONFIGURATION_ACKNOWLEDGED,
        Vec::new(),
    )
}

pub fn decode_play_clientbound(packet_id: i32, payload: &[u8]) -> Result<PlayClientbound> {
    match packet_id {
        ids::play::CLIENTBOUND_BUNDLE_DELIMITER => Ok(PlayClientbound::BundleDelimiter),
        ids::play::CLIENTBOUND_ADD_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::AddEntity(entities::decode_add_entity(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_ANIMATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::EntityAnimation(
                entities::decode_entity_animation(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_AWARD_STATS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::AwardStats(decode_award_stats(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockChangedAck(
                chunks::decode_block_changed_ack(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_BLOCK_DESTRUCTION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockDestruction(
                chunks::decode_block_destruction(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockEntityData(
                chunks::decode_block_entity_data(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_BLOCK_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockEvent(chunks::decode_block_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_BLOCK_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockUpdate(chunks::decode_block_update(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_BOSS_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BossEvent(client_state::decode_boss_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_CHANGE_DIFFICULTY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ChangeDifficulty(
                client_state::decode_change_difficulty(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CHUNK_BATCH_START => Ok(PlayClientbound::ChunkBatchStart),
        ids::play::CLIENTBOUND_CHUNK_BATCH_FINISHED => Ok(PlayClientbound::ChunkBatchFinished {
            batch_size: Decoder::new(payload).read_var_i32()?,
        }),
        ids::play::CLIENTBOUND_CHUNKS_BIOMES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ChunksBiomes(chunks::decode_chunks_biomes(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::CommandSuggestions(
                command_suggestions::decode_command_suggestions(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CONTAINER_CLOSE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerClose(
                inventory::decode_container_close(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CONTAINER_SET_CONTENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerSetContent(
                inventory::decode_container_set_content(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CONTAINER_SET_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerSetData(
                inventory::decode_container_set_data(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CONTAINER_SET_SLOT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerSetSlot(
                inventory::decode_container_set_slot(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_COOLDOWN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Cooldown(client_state::decode_cooldown(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_DAMAGE_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::DamageEvent(
                client_state::decode_damage_event(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_DISCONNECT => Ok(PlayClientbound::Disconnect(Disconnect {
            reason: decode_component_summary(payload)?,
            raw_reason: payload.to_vec(),
        })),
        ids::play::CLIENTBOUND_ENTITY_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::EntityEvent(entities::decode_entity_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_ENTITY_POSITION_SYNC => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::EntityPositionSync(
                entities::decode_entity_position_sync(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_FORGET_LEVEL_CHUNK => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ForgetLevelChunk(
                chunks::decode_forget_level_chunk(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_GAME_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::GameEvent(client_state::decode_game_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_HURT_ANIMATION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::HurtAnimation(
                entities::decode_hurt_animation(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_INITIALIZE_BORDER => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::InitializeBorder(InitializeBorder {
                new_center_x: decoder.read_f64()?,
                new_center_z: decoder.read_f64()?,
                old_size: decoder.read_f64()?,
                new_size: decoder.read_f64()?,
                lerp_time: decoder.read_var_i64()?,
                new_absolute_max_size: decoder.read_var_i32()?,
                warning_blocks: decoder.read_var_i32()?,
                warning_time: decoder.read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_KEEP_ALIVE => Ok(PlayClientbound::KeepAlive {
            id: Decoder::new(payload).read_i64()?,
        }),
        ids::play::CLIENTBOUND_PING => Ok(PlayClientbound::Ping {
            id: Decoder::new(payload).read_i32()?,
        }),
        ids::play::CLIENTBOUND_LOGIN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Login(decode_play_login(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_MOVE_ENTITY_POS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MoveEntity(entities::decode_move_entity(
                &mut decoder,
                true,
                false,
            )?))
        }
        ids::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MoveEntity(entities::decode_move_entity(
                &mut decoder,
                true,
                true,
            )?))
        }
        ids::play::CLIENTBOUND_MOVE_ENTITY_ROT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MoveEntity(entities::decode_move_entity(
                &mut decoder,
                false,
                true,
            )?))
        }
        ids::play::CLIENTBOUND_MOVE_VEHICLE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MoveVehicle(entities::decode_move_vehicle(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_OPEN_SCREEN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::OpenScreen(inventory::decode_open_screen(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_PLAYER_ABILITIES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerAbilities(
                client_state::decode_player_abilities(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLAYER_INFO_REMOVE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerInfoRemove(
                player_info::decode_player_info_remove(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerInfoUpdate(
                player_info::decode_player_info_update(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLAYER_POSITION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerPosition(decode_player_position(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_PLAYER_ROTATION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerRotation(PlayerRotationUpdate {
                y_rot: decoder.read_f32()?,
                relative_y: decoder.read_bool()?,
                x_rot: decoder.read_f32()?,
                relative_x: decoder.read_bool()?,
            }))
        }
        ids::play::CLIENTBOUND_REMOVE_ENTITIES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::RemoveEntities(
                entities::decode_remove_entities(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_REMOVE_MOB_EFFECT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::RemoveMobEffect(
                client_state::decode_remove_mob_effect(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_RESET_SCORE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ResetScore(scoreboard::decode_reset_score(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_RESOURCE_PACK_POP => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ResourcePackPop(
                server_presentation::decode_resource_pack_pop(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_RESOURCE_PACK_PUSH => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ResourcePackPush(
                server_presentation::decode_resource_pack_push(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_RESPAWN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Respawn(decode_respawn(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_ROTATE_HEAD => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::RotateHead(entities::decode_rotate_head(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SERVER_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ServerData(
                server_presentation::decode_server_data(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_ACTION_BAR_TEXT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetActionBarText(
                client_state::decode_set_action_bar_text(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_BORDER_CENTER => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetBorderCenter(SetBorderCenter {
                new_center_x: decoder.read_f64()?,
                new_center_z: decoder.read_f64()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_BORDER_LERP_SIZE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetBorderLerpSize(SetBorderLerpSize {
                old_size: decoder.read_f64()?,
                new_size: decoder.read_f64()?,
                lerp_time: decoder.read_var_i64()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_BORDER_SIZE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetBorderSize(SetBorderSize {
                size: decoder.read_f64()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_BORDER_WARNING_DELAY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetBorderWarningDelay(
                SetBorderWarningDelay {
                    warning_delay: decoder.read_var_i32()?,
                },
            ))
        }
        ids::play::CLIENTBOUND_SET_BORDER_WARNING_DISTANCE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetBorderWarningDistance(
                SetBorderWarningDistance {
                    warning_blocks: decoder.read_var_i32()?,
                },
            ))
        }
        ids::play::CLIENTBOUND_SET_CAMERA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetCamera(client_state::decode_set_camera(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SET_HEALTH => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetHealth(
                client_state::decode_player_health(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_HELD_SLOT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetHeldSlot(
                client_state::decode_set_held_slot(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_OBJECTIVE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetObjective(
                scoreboard::decode_set_objective(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_PASSENGERS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetPassengers(
                entities::decode_set_passengers(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SectionBlocksUpdate(
                chunks::decode_section_blocks_update(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_CENTER => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetChunkCacheCenter(
                chunks::decode_set_chunk_cache_center(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_RADIUS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetChunkCacheRadius(
                chunks::decode_set_chunk_cache_radius(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_CURSOR_ITEM => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetCursorItem(
                inventory::decode_set_cursor_item(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetDefaultSpawnPosition(
                client_state::decode_default_spawn_position(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_DISPLAY_OBJECTIVE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetDisplayObjective(
                scoreboard::decode_set_display_objective(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_ENTITY_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEntityData(
                entities::decode_set_entity_data(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_ENTITY_LINK => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEntityLink(
                entities::decode_set_entity_link(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_ENTITY_MOTION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEntityMotion(
                entities::decode_set_entity_motion(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_EQUIPMENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEquipment(
                entities::decode_set_equipment(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_EXPERIENCE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetExperience(
                client_state::decode_player_experience(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_PLAYER_INVENTORY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetPlayerInventory(
                inventory::decode_set_player_inventory(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_PLAYER_TEAM => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetPlayerTeam(
                scoreboard::decode_set_player_team(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_SCORE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetScore(scoreboard::decode_set_score(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SET_SIMULATION_DISTANCE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetSimulationDistance(
                client_state::decode_set_simulation_distance(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_SUBTITLE_TEXT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetSubtitleText(
                client_state::decode_set_subtitle_text(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_TIME => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetTime(client_state::decode_play_time(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SET_TITLE_TEXT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetTitleText(
                client_state::decode_set_title_text(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_TITLES_ANIMATION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetTitlesAnimation(
                client_state::decode_set_titles_animation(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_START_CONFIGURATION => Ok(PlayClientbound::StartConfiguration),
        ids::play::CLIENTBOUND_SYSTEM_CHAT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SystemChat(
                client_state::decode_system_chat(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TAB_LIST => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TabList(
                server_presentation::decode_tab_list(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TAKE_ITEM_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TakeItemEntity(
                entities::decode_take_item_entity(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TELEPORT_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TeleportEntity(
                entities::decode_teleport_entity(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TICKING_STATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TickingState(
                client_state::decode_ticking_state(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_TICKING_STEP => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TickingStep(
                client_state::decode_ticking_step(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::UpdateAttributes(
                entities::decode_update_attributes(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_UPDATE_MOB_EFFECT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::UpdateMobEffect(
                client_state::decode_update_mob_effect(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_LEVEL_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::LevelEvent(chunks::decode_level_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::LevelChunkWithLight(
                chunks::decode_level_chunk_with_light(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_LIGHT_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::LightUpdate(chunks::decode_light_update(
                &mut decoder,
            )?))
        }
        id => Ok(PlayClientbound::Unknown {
            packet_id: id,
            len: payload.len(),
        }),
    }
}

fn decode_nullable_string(decoder: &mut Decoder<'_>) -> Result<Option<String>> {
    if decoder.read_bool()? {
        Ok(Some(decoder.read_string(32767)?))
    } else {
        Ok(None)
    }
}

fn decode_optional_uuid(decoder: &mut Decoder<'_>) -> Result<Option<Uuid>> {
    if decoder.read_bool()? {
        Ok(Some(decoder.read_uuid()?))
    } else {
        Ok(None)
    }
}

fn decode_optional_component_summary_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<Option<String>> {
    if decoder.read_bool()? {
        Ok(Some(decode_component_summary_from_decoder(decoder)?))
    } else {
        Ok(None)
    }
}

fn decode_optional_byte_array(
    decoder: &mut Decoder<'_>,
    max_len: usize,
    what: &'static str,
) -> Result<Option<Vec<u8>>> {
    if decoder.read_bool()? {
        Ok(Some(decode_byte_array(decoder, max_len, what)?))
    } else {
        Ok(None)
    }
}

fn decode_optional_trailing_number_format(decoder: &mut Decoder<'_>) -> Result<Option<Vec<u8>>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }

    let len = decoder.remaining_len();
    Ok(Some(
        decoder.read_exact(len, "number format payload")?.to_vec(),
    ))
}

fn decode_award_stats(decoder: &mut Decoder<'_>) -> Result<AwardStats> {
    let count = decoder.read_len()?;
    if count > MAX_AWARD_STATS {
        return Err(ProtocolError::PacketTooLarge(count, MAX_AWARD_STATS));
    }

    let mut stats = Vec::with_capacity(count);
    for _ in 0..count {
        stats.push(StatUpdate {
            stat_type_id: decoder.read_var_i32()?,
            value_id: decoder.read_var_i32()?,
            amount: decoder.read_var_i32()?,
        });
    }
    Ok(AwardStats { stats })
}

fn decode_play_login(decoder: &mut Decoder<'_>) -> Result<PlayLogin> {
    let player_id = decoder.read_i32()?;
    let hardcore = decoder.read_bool()?;
    let level_count = decoder.read_len()?;
    let mut levels = Vec::with_capacity(level_count);
    for _ in 0..level_count {
        levels.push(read_resource_key(decoder)?);
    }
    Ok(PlayLogin {
        player_id,
        hardcore,
        levels,
        max_players: decoder.read_var_i32()?,
        chunk_radius: decoder.read_var_i32()?,
        simulation_distance: decoder.read_var_i32()?,
        reduced_debug_info: decoder.read_bool()?,
        show_death_screen: decoder.read_bool()?,
        do_limited_crafting: decoder.read_bool()?,
        common_spawn_info: decode_common_spawn_info(decoder)?,
        enforces_secure_chat: decoder.read_bool()?,
    })
}

fn decode_common_spawn_info(decoder: &mut Decoder<'_>) -> Result<CommonPlayerSpawnInfo> {
    Ok(CommonPlayerSpawnInfo {
        dimension_type_id: decoder.read_var_i32()?,
        dimension: read_resource_key(decoder)?,
        seed: decoder.read_i64()?,
        game_type: decoder.read_i8()?,
        previous_game_type: decoder.read_i8()?,
        is_debug: decoder.read_bool()?,
        is_flat: decoder.read_bool()?,
        last_death_location: decode_optional_global_pos(decoder)?,
        portal_cooldown: decoder.read_var_i32()?,
        sea_level: decoder.read_var_i32()?,
    })
}

fn decode_respawn(decoder: &mut Decoder<'_>) -> Result<Respawn> {
    Ok(Respawn {
        common_spawn_info: decode_common_spawn_info(decoder)?,
        data_to_keep: decoder.read_i8()?,
    })
}

fn decode_byte_array(
    decoder: &mut Decoder<'_>,
    max_len: usize,
    what: &'static str,
) -> Result<Vec<u8>> {
    let len = decoder.read_len()?;
    if len > max_len {
        return Err(ProtocolError::PacketTooLarge(len, max_len));
    }
    Ok(decoder.read_exact(len, what)?.to_vec())
}

fn decode_optional_global_pos(decoder: &mut Decoder<'_>) -> Result<Option<GlobalPos>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    Ok(Some(GlobalPos {
        dimension: read_resource_key(decoder)?,
        pos: chunks::decode_block_pos(decoder.read_i64()?),
    }))
}

fn read_resource_key(decoder: &mut Decoder<'_>) -> Result<String> {
    decoder.read_string(32767)
}

fn decode_player_position(decoder: &mut Decoder<'_>) -> Result<PlayerPositionUpdate> {
    let id = decoder.read_var_i32()?;
    let position = decode_vec3d(decoder)?;
    let delta_movement = decode_vec3d(decoder)?;
    let y_rot = decoder.read_f32()?;
    let x_rot = decoder.read_f32()?;
    let relatives_mask = decoder.read_i32()?;
    Ok(PlayerPositionUpdate {
        id,
        position,
        delta_movement,
        y_rot,
        x_rot,
        relatives_mask,
    })
}

fn decode_vec3d(decoder: &mut Decoder<'_>) -> Result<Vec3d> {
    Ok(Vec3d {
        x: decoder.read_f64()?,
        y: decoder.read_f64()?,
        z: decoder.read_f64()?,
    })
}

fn decode_optional_vec3d(decoder: &mut Decoder<'_>) -> Result<Option<Vec3d>> {
    if decoder.read_bool()? {
        Ok(Some(decode_vec3d(decoder)?))
    } else {
        Ok(None)
    }
}

fn pack_move_flags(on_ground: bool, horizontal_collision: bool) -> u8 {
    let mut flags = 0;
    if on_ground {
        flags |= 1;
    }
    if horizontal_collision {
        flags |= 2;
    }
    flags
}

fn decode_game_profile(decoder: &mut Decoder<'_>) -> Result<GameProfile> {
    let uuid = decoder.read_uuid()?;
    let name = decoder.read_string(16)?;
    let property_count = decoder.read_len()?;
    let mut properties = Vec::with_capacity(property_count);
    for _ in 0..property_count {
        let name = decoder.read_string(32767)?;
        let value = decoder.read_string(32767)?;
        let signature = if decoder.read_bool()? {
            Some(decoder.read_string(32767)?)
        } else {
            None
        };
        properties.push(GameProfileProperty {
            name,
            value,
            signature,
        });
    }
    Ok(GameProfile {
        uuid,
        name,
        properties,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::offline_player_uuid;

    #[test]
    fn play_clientbound_packet_ids_match_vanilla_26_1_registration_order() {
        let ids = [
            (
                "CLIENTBOUND_BUNDLE_DELIMITER",
                ids::play::CLIENTBOUND_BUNDLE_DELIMITER,
                0,
            ),
            (
                "CLIENTBOUND_ADD_ENTITY",
                ids::play::CLIENTBOUND_ADD_ENTITY,
                1,
            ),
            ("CLIENTBOUND_ANIMATE", ids::play::CLIENTBOUND_ANIMATE, 2),
            (
                "CLIENTBOUND_AWARD_STATS",
                ids::play::CLIENTBOUND_AWARD_STATS,
                3,
            ),
            (
                "CLIENTBOUND_BLOCK_CHANGED_ACK",
                ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK,
                4,
            ),
            (
                "CLIENTBOUND_BLOCK_DESTRUCTION",
                ids::play::CLIENTBOUND_BLOCK_DESTRUCTION,
                5,
            ),
            (
                "CLIENTBOUND_BLOCK_ENTITY_DATA",
                ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA,
                6,
            ),
            (
                "CLIENTBOUND_BLOCK_EVENT",
                ids::play::CLIENTBOUND_BLOCK_EVENT,
                7,
            ),
            (
                "CLIENTBOUND_BLOCK_UPDATE",
                ids::play::CLIENTBOUND_BLOCK_UPDATE,
                8,
            ),
            (
                "CLIENTBOUND_BOSS_EVENT",
                ids::play::CLIENTBOUND_BOSS_EVENT,
                9,
            ),
            (
                "CLIENTBOUND_CHANGE_DIFFICULTY",
                ids::play::CLIENTBOUND_CHANGE_DIFFICULTY,
                10,
            ),
            (
                "CLIENTBOUND_CHUNK_BATCH_FINISHED",
                ids::play::CLIENTBOUND_CHUNK_BATCH_FINISHED,
                11,
            ),
            (
                "CLIENTBOUND_CHUNK_BATCH_START",
                ids::play::CLIENTBOUND_CHUNK_BATCH_START,
                12,
            ),
            (
                "CLIENTBOUND_CHUNKS_BIOMES",
                ids::play::CLIENTBOUND_CHUNKS_BIOMES,
                13,
            ),
            (
                "CLIENTBOUND_CLEAR_TITLES",
                ids::play::CLIENTBOUND_CLEAR_TITLES,
                14,
            ),
            (
                "CLIENTBOUND_COMMAND_SUGGESTIONS",
                ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS,
                15,
            ),
            ("CLIENTBOUND_COMMANDS", ids::play::CLIENTBOUND_COMMANDS, 16),
            (
                "CLIENTBOUND_CONTAINER_CLOSE",
                ids::play::CLIENTBOUND_CONTAINER_CLOSE,
                17,
            ),
            (
                "CLIENTBOUND_CONTAINER_SET_CONTENT",
                ids::play::CLIENTBOUND_CONTAINER_SET_CONTENT,
                18,
            ),
            (
                "CLIENTBOUND_CONTAINER_SET_DATA",
                ids::play::CLIENTBOUND_CONTAINER_SET_DATA,
                19,
            ),
            (
                "CLIENTBOUND_CONTAINER_SET_SLOT",
                ids::play::CLIENTBOUND_CONTAINER_SET_SLOT,
                20,
            ),
            (
                "CLIENTBOUND_COOKIE_REQUEST",
                ids::play::CLIENTBOUND_COOKIE_REQUEST,
                21,
            ),
            ("CLIENTBOUND_COOLDOWN", ids::play::CLIENTBOUND_COOLDOWN, 22),
            (
                "CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS",
                ids::play::CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS,
                23,
            ),
            (
                "CLIENTBOUND_CUSTOM_PAYLOAD",
                ids::play::CLIENTBOUND_CUSTOM_PAYLOAD,
                24,
            ),
            (
                "CLIENTBOUND_DAMAGE_EVENT",
                ids::play::CLIENTBOUND_DAMAGE_EVENT,
                25,
            ),
            (
                "CLIENTBOUND_DEBUG_BLOCK_VALUE",
                ids::play::CLIENTBOUND_DEBUG_BLOCK_VALUE,
                26,
            ),
            (
                "CLIENTBOUND_DEBUG_CHUNK_VALUE",
                ids::play::CLIENTBOUND_DEBUG_CHUNK_VALUE,
                27,
            ),
            (
                "CLIENTBOUND_DEBUG_ENTITY_VALUE",
                ids::play::CLIENTBOUND_DEBUG_ENTITY_VALUE,
                28,
            ),
            (
                "CLIENTBOUND_DEBUG_EVENT",
                ids::play::CLIENTBOUND_DEBUG_EVENT,
                29,
            ),
            (
                "CLIENTBOUND_DEBUG_SAMPLE",
                ids::play::CLIENTBOUND_DEBUG_SAMPLE,
                30,
            ),
            (
                "CLIENTBOUND_DELETE_CHAT",
                ids::play::CLIENTBOUND_DELETE_CHAT,
                31,
            ),
            (
                "CLIENTBOUND_DISCONNECT",
                ids::play::CLIENTBOUND_DISCONNECT,
                32,
            ),
            (
                "CLIENTBOUND_DISGUISED_CHAT",
                ids::play::CLIENTBOUND_DISGUISED_CHAT,
                33,
            ),
            (
                "CLIENTBOUND_ENTITY_EVENT",
                ids::play::CLIENTBOUND_ENTITY_EVENT,
                34,
            ),
            (
                "CLIENTBOUND_ENTITY_POSITION_SYNC",
                ids::play::CLIENTBOUND_ENTITY_POSITION_SYNC,
                35,
            ),
            ("CLIENTBOUND_EXPLODE", ids::play::CLIENTBOUND_EXPLODE, 36),
            (
                "CLIENTBOUND_FORGET_LEVEL_CHUNK",
                ids::play::CLIENTBOUND_FORGET_LEVEL_CHUNK,
                37,
            ),
            (
                "CLIENTBOUND_GAME_EVENT",
                ids::play::CLIENTBOUND_GAME_EVENT,
                38,
            ),
            (
                "CLIENTBOUND_GAME_RULE_VALUES",
                ids::play::CLIENTBOUND_GAME_RULE_VALUES,
                39,
            ),
            (
                "CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS",
                ids::play::CLIENTBOUND_GAME_TEST_HIGHLIGHT_POS,
                40,
            ),
            (
                "CLIENTBOUND_MOUNT_SCREEN_OPEN",
                ids::play::CLIENTBOUND_MOUNT_SCREEN_OPEN,
                41,
            ),
            (
                "CLIENTBOUND_HURT_ANIMATION",
                ids::play::CLIENTBOUND_HURT_ANIMATION,
                42,
            ),
            (
                "CLIENTBOUND_INITIALIZE_BORDER",
                ids::play::CLIENTBOUND_INITIALIZE_BORDER,
                43,
            ),
            (
                "CLIENTBOUND_KEEP_ALIVE",
                ids::play::CLIENTBOUND_KEEP_ALIVE,
                44,
            ),
            (
                "CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT",
                ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT,
                45,
            ),
            (
                "CLIENTBOUND_LEVEL_EVENT",
                ids::play::CLIENTBOUND_LEVEL_EVENT,
                46,
            ),
            (
                "CLIENTBOUND_LEVEL_PARTICLES",
                ids::play::CLIENTBOUND_LEVEL_PARTICLES,
                47,
            ),
            (
                "CLIENTBOUND_LIGHT_UPDATE",
                ids::play::CLIENTBOUND_LIGHT_UPDATE,
                48,
            ),
            ("CLIENTBOUND_LOGIN", ids::play::CLIENTBOUND_LOGIN, 49),
            (
                "CLIENTBOUND_LOW_DISK_SPACE_WARNING",
                ids::play::CLIENTBOUND_LOW_DISK_SPACE_WARNING,
                50,
            ),
            (
                "CLIENTBOUND_MAP_ITEM_DATA",
                ids::play::CLIENTBOUND_MAP_ITEM_DATA,
                51,
            ),
            (
                "CLIENTBOUND_MERCHANT_OFFERS",
                ids::play::CLIENTBOUND_MERCHANT_OFFERS,
                52,
            ),
            (
                "CLIENTBOUND_MOVE_ENTITY_POS",
                ids::play::CLIENTBOUND_MOVE_ENTITY_POS,
                53,
            ),
            (
                "CLIENTBOUND_MOVE_ENTITY_POS_ROT",
                ids::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT,
                54,
            ),
            (
                "CLIENTBOUND_MOVE_MINECART_ALONG_TRACK",
                ids::play::CLIENTBOUND_MOVE_MINECART_ALONG_TRACK,
                55,
            ),
            (
                "CLIENTBOUND_MOVE_ENTITY_ROT",
                ids::play::CLIENTBOUND_MOVE_ENTITY_ROT,
                56,
            ),
            (
                "CLIENTBOUND_MOVE_VEHICLE",
                ids::play::CLIENTBOUND_MOVE_VEHICLE,
                57,
            ),
            (
                "CLIENTBOUND_OPEN_BOOK",
                ids::play::CLIENTBOUND_OPEN_BOOK,
                58,
            ),
            (
                "CLIENTBOUND_OPEN_SCREEN",
                ids::play::CLIENTBOUND_OPEN_SCREEN,
                59,
            ),
            (
                "CLIENTBOUND_OPEN_SIGN_EDITOR",
                ids::play::CLIENTBOUND_OPEN_SIGN_EDITOR,
                60,
            ),
            ("CLIENTBOUND_PING", ids::play::CLIENTBOUND_PING, 61),
            (
                "CLIENTBOUND_PONG_RESPONSE",
                ids::play::CLIENTBOUND_PONG_RESPONSE,
                62,
            ),
            (
                "CLIENTBOUND_PLACE_GHOST_RECIPE",
                ids::play::CLIENTBOUND_PLACE_GHOST_RECIPE,
                63,
            ),
            (
                "CLIENTBOUND_PLAYER_ABILITIES",
                ids::play::CLIENTBOUND_PLAYER_ABILITIES,
                64,
            ),
            (
                "CLIENTBOUND_PLAYER_CHAT",
                ids::play::CLIENTBOUND_PLAYER_CHAT,
                65,
            ),
            (
                "CLIENTBOUND_PLAYER_COMBAT_END",
                ids::play::CLIENTBOUND_PLAYER_COMBAT_END,
                66,
            ),
            (
                "CLIENTBOUND_PLAYER_COMBAT_ENTER",
                ids::play::CLIENTBOUND_PLAYER_COMBAT_ENTER,
                67,
            ),
            (
                "CLIENTBOUND_PLAYER_COMBAT_KILL",
                ids::play::CLIENTBOUND_PLAYER_COMBAT_KILL,
                68,
            ),
            (
                "CLIENTBOUND_PLAYER_INFO_REMOVE",
                ids::play::CLIENTBOUND_PLAYER_INFO_REMOVE,
                69,
            ),
            (
                "CLIENTBOUND_PLAYER_INFO_UPDATE",
                ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE,
                70,
            ),
            (
                "CLIENTBOUND_PLAYER_LOOK_AT",
                ids::play::CLIENTBOUND_PLAYER_LOOK_AT,
                71,
            ),
            (
                "CLIENTBOUND_PLAYER_POSITION",
                ids::play::CLIENTBOUND_PLAYER_POSITION,
                72,
            ),
            (
                "CLIENTBOUND_PLAYER_ROTATION",
                ids::play::CLIENTBOUND_PLAYER_ROTATION,
                73,
            ),
            (
                "CLIENTBOUND_RECIPE_BOOK_ADD",
                ids::play::CLIENTBOUND_RECIPE_BOOK_ADD,
                74,
            ),
            (
                "CLIENTBOUND_RECIPE_BOOK_REMOVE",
                ids::play::CLIENTBOUND_RECIPE_BOOK_REMOVE,
                75,
            ),
            (
                "CLIENTBOUND_RECIPE_BOOK_SETTINGS",
                ids::play::CLIENTBOUND_RECIPE_BOOK_SETTINGS,
                76,
            ),
            (
                "CLIENTBOUND_REMOVE_ENTITIES",
                ids::play::CLIENTBOUND_REMOVE_ENTITIES,
                77,
            ),
            (
                "CLIENTBOUND_REMOVE_MOB_EFFECT",
                ids::play::CLIENTBOUND_REMOVE_MOB_EFFECT,
                78,
            ),
            (
                "CLIENTBOUND_RESET_SCORE",
                ids::play::CLIENTBOUND_RESET_SCORE,
                79,
            ),
            (
                "CLIENTBOUND_RESOURCE_PACK_POP",
                ids::play::CLIENTBOUND_RESOURCE_PACK_POP,
                80,
            ),
            (
                "CLIENTBOUND_RESOURCE_PACK_PUSH",
                ids::play::CLIENTBOUND_RESOURCE_PACK_PUSH,
                81,
            ),
            ("CLIENTBOUND_RESPAWN", ids::play::CLIENTBOUND_RESPAWN, 82),
            (
                "CLIENTBOUND_ROTATE_HEAD",
                ids::play::CLIENTBOUND_ROTATE_HEAD,
                83,
            ),
            (
                "CLIENTBOUND_SECTION_BLOCKS_UPDATE",
                ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE,
                84,
            ),
            (
                "CLIENTBOUND_SELECT_ADVANCEMENTS_TAB",
                ids::play::CLIENTBOUND_SELECT_ADVANCEMENTS_TAB,
                85,
            ),
            (
                "CLIENTBOUND_SERVER_DATA",
                ids::play::CLIENTBOUND_SERVER_DATA,
                86,
            ),
            (
                "CLIENTBOUND_SET_ACTION_BAR_TEXT",
                ids::play::CLIENTBOUND_SET_ACTION_BAR_TEXT,
                87,
            ),
            (
                "CLIENTBOUND_SET_BORDER_CENTER",
                ids::play::CLIENTBOUND_SET_BORDER_CENTER,
                88,
            ),
            (
                "CLIENTBOUND_SET_BORDER_LERP_SIZE",
                ids::play::CLIENTBOUND_SET_BORDER_LERP_SIZE,
                89,
            ),
            (
                "CLIENTBOUND_SET_BORDER_SIZE",
                ids::play::CLIENTBOUND_SET_BORDER_SIZE,
                90,
            ),
            (
                "CLIENTBOUND_SET_BORDER_WARNING_DELAY",
                ids::play::CLIENTBOUND_SET_BORDER_WARNING_DELAY,
                91,
            ),
            (
                "CLIENTBOUND_SET_BORDER_WARNING_DISTANCE",
                ids::play::CLIENTBOUND_SET_BORDER_WARNING_DISTANCE,
                92,
            ),
            (
                "CLIENTBOUND_SET_CAMERA",
                ids::play::CLIENTBOUND_SET_CAMERA,
                93,
            ),
            (
                "CLIENTBOUND_SET_CHUNK_CACHE_CENTER",
                ids::play::CLIENTBOUND_SET_CHUNK_CACHE_CENTER,
                94,
            ),
            (
                "CLIENTBOUND_SET_CHUNK_CACHE_RADIUS",
                ids::play::CLIENTBOUND_SET_CHUNK_CACHE_RADIUS,
                95,
            ),
            (
                "CLIENTBOUND_SET_CURSOR_ITEM",
                ids::play::CLIENTBOUND_SET_CURSOR_ITEM,
                96,
            ),
            (
                "CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION",
                ids::play::CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION,
                97,
            ),
            (
                "CLIENTBOUND_SET_DISPLAY_OBJECTIVE",
                ids::play::CLIENTBOUND_SET_DISPLAY_OBJECTIVE,
                98,
            ),
            (
                "CLIENTBOUND_SET_ENTITY_DATA",
                ids::play::CLIENTBOUND_SET_ENTITY_DATA,
                99,
            ),
            (
                "CLIENTBOUND_SET_ENTITY_LINK",
                ids::play::CLIENTBOUND_SET_ENTITY_LINK,
                100,
            ),
            (
                "CLIENTBOUND_SET_ENTITY_MOTION",
                ids::play::CLIENTBOUND_SET_ENTITY_MOTION,
                101,
            ),
            (
                "CLIENTBOUND_SET_EQUIPMENT",
                ids::play::CLIENTBOUND_SET_EQUIPMENT,
                102,
            ),
            (
                "CLIENTBOUND_SET_EXPERIENCE",
                ids::play::CLIENTBOUND_SET_EXPERIENCE,
                103,
            ),
            (
                "CLIENTBOUND_SET_HEALTH",
                ids::play::CLIENTBOUND_SET_HEALTH,
                104,
            ),
            (
                "CLIENTBOUND_SET_HELD_SLOT",
                ids::play::CLIENTBOUND_SET_HELD_SLOT,
                105,
            ),
            (
                "CLIENTBOUND_SET_OBJECTIVE",
                ids::play::CLIENTBOUND_SET_OBJECTIVE,
                106,
            ),
            (
                "CLIENTBOUND_SET_PASSENGERS",
                ids::play::CLIENTBOUND_SET_PASSENGERS,
                107,
            ),
            (
                "CLIENTBOUND_SET_PLAYER_INVENTORY",
                ids::play::CLIENTBOUND_SET_PLAYER_INVENTORY,
                108,
            ),
            (
                "CLIENTBOUND_SET_PLAYER_TEAM",
                ids::play::CLIENTBOUND_SET_PLAYER_TEAM,
                109,
            ),
            (
                "CLIENTBOUND_SET_SCORE",
                ids::play::CLIENTBOUND_SET_SCORE,
                110,
            ),
            (
                "CLIENTBOUND_SET_SIMULATION_DISTANCE",
                ids::play::CLIENTBOUND_SET_SIMULATION_DISTANCE,
                111,
            ),
            (
                "CLIENTBOUND_SET_SUBTITLE_TEXT",
                ids::play::CLIENTBOUND_SET_SUBTITLE_TEXT,
                112,
            ),
            ("CLIENTBOUND_SET_TIME", ids::play::CLIENTBOUND_SET_TIME, 113),
            (
                "CLIENTBOUND_SET_TITLE_TEXT",
                ids::play::CLIENTBOUND_SET_TITLE_TEXT,
                114,
            ),
            (
                "CLIENTBOUND_SET_TITLES_ANIMATION",
                ids::play::CLIENTBOUND_SET_TITLES_ANIMATION,
                115,
            ),
            (
                "CLIENTBOUND_SOUND_ENTITY",
                ids::play::CLIENTBOUND_SOUND_ENTITY,
                116,
            ),
            ("CLIENTBOUND_SOUND", ids::play::CLIENTBOUND_SOUND, 117),
            (
                "CLIENTBOUND_START_CONFIGURATION",
                ids::play::CLIENTBOUND_START_CONFIGURATION,
                118,
            ),
            (
                "CLIENTBOUND_STOP_SOUND",
                ids::play::CLIENTBOUND_STOP_SOUND,
                119,
            ),
            (
                "CLIENTBOUND_STORE_COOKIE",
                ids::play::CLIENTBOUND_STORE_COOKIE,
                120,
            ),
            (
                "CLIENTBOUND_SYSTEM_CHAT",
                ids::play::CLIENTBOUND_SYSTEM_CHAT,
                121,
            ),
            ("CLIENTBOUND_TAB_LIST", ids::play::CLIENTBOUND_TAB_LIST, 122),
            (
                "CLIENTBOUND_TAG_QUERY",
                ids::play::CLIENTBOUND_TAG_QUERY,
                123,
            ),
            (
                "CLIENTBOUND_TAKE_ITEM_ENTITY",
                ids::play::CLIENTBOUND_TAKE_ITEM_ENTITY,
                124,
            ),
            (
                "CLIENTBOUND_TELEPORT_ENTITY",
                ids::play::CLIENTBOUND_TELEPORT_ENTITY,
                125,
            ),
            (
                "CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS",
                ids::play::CLIENTBOUND_TEST_INSTANCE_BLOCK_STATUS,
                126,
            ),
            (
                "CLIENTBOUND_TICKING_STATE",
                ids::play::CLIENTBOUND_TICKING_STATE,
                127,
            ),
            (
                "CLIENTBOUND_TICKING_STEP",
                ids::play::CLIENTBOUND_TICKING_STEP,
                128,
            ),
            ("CLIENTBOUND_TRANSFER", ids::play::CLIENTBOUND_TRANSFER, 129),
            (
                "CLIENTBOUND_UPDATE_ADVANCEMENTS",
                ids::play::CLIENTBOUND_UPDATE_ADVANCEMENTS,
                130,
            ),
            (
                "CLIENTBOUND_UPDATE_ATTRIBUTES",
                ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES,
                131,
            ),
            (
                "CLIENTBOUND_UPDATE_MOB_EFFECT",
                ids::play::CLIENTBOUND_UPDATE_MOB_EFFECT,
                132,
            ),
            (
                "CLIENTBOUND_UPDATE_RECIPES",
                ids::play::CLIENTBOUND_UPDATE_RECIPES,
                133,
            ),
            (
                "CLIENTBOUND_UPDATE_TAGS",
                ids::play::CLIENTBOUND_UPDATE_TAGS,
                134,
            ),
            (
                "CLIENTBOUND_PROJECTILE_POWER",
                ids::play::CLIENTBOUND_PROJECTILE_POWER,
                135,
            ),
            (
                "CLIENTBOUND_CUSTOM_REPORT_DETAILS",
                ids::play::CLIENTBOUND_CUSTOM_REPORT_DETAILS,
                136,
            ),
            (
                "CLIENTBOUND_SERVER_LINKS",
                ids::play::CLIENTBOUND_SERVER_LINKS,
                137,
            ),
            ("CLIENTBOUND_WAYPOINT", ids::play::CLIENTBOUND_WAYPOINT, 138),
            (
                "CLIENTBOUND_CLEAR_DIALOG",
                ids::play::CLIENTBOUND_CLEAR_DIALOG,
                139,
            ),
            (
                "CLIENTBOUND_SHOW_DIALOG",
                ids::play::CLIENTBOUND_SHOW_DIALOG,
                140,
            ),
        ];

        for (name, actual, expected) in ids {
            assert_eq!(actual, expected, "{name}");
        }
    }

    #[test]
    fn decodes_bundle_delimiter_packet() {
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BUNDLE_DELIMITER, &[]).unwrap();
        assert_eq!(packet, PlayClientbound::BundleDelimiter);
    }

    #[test]
    fn encodes_login_hello() {
        let uuid = offline_player_uuid("bbb-client");
        let (id, payload) = encode_login_hello("bbb-client", uuid);
        assert_eq!(id, ids::login::SERVERBOUND_HELLO);

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(16).unwrap(), "bbb-client");
        assert_eq!(decoder.read_uuid().unwrap(), uuid);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_play_disconnect_component() {
        let mut payload = Vec::new();
        payload.push(8);
        payload.extend_from_slice(&6u16.to_be_bytes());
        payload.extend_from_slice(b"Kicked");

        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_DISCONNECT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::Disconnect(Disconnect {
                reason: "Kicked".to_string(),
                raw_reason: payload,
            })
        );
    }

    #[test]
    fn decodes_play_login_spawn_info() {
        let mut payload = Encoder::new();
        payload.write_i32(42);
        payload.write_bool(true);
        payload.write_var_i32(3);
        payload.write_string("minecraft:overworld");
        payload.write_string("minecraft:the_nether");
        payload.write_string("minecraft:the_end");
        payload.write_var_i32(20);
        payload.write_var_i32(8);
        payload.write_var_i32(6);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_bool(false);
        payload.write_var_i32(1);
        payload.write_string("minecraft:the_nether");
        payload.write_i64(12345);
        payload.write_i8(1);
        payload.write_i8(-1);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_string("minecraft:overworld");
        payload.write_i64(encode_block_pos(1, 64, -2));
        payload.write_var_i32(7);
        payload.write_var_i32(32);
        payload.write_bool(true);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_LOGIN, &payload.into_inner()).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::Login(PlayLogin {
                player_id: 42,
                hardcore: true,
                levels: vec![
                    "minecraft:overworld".to_string(),
                    "minecraft:the_nether".to_string(),
                    "minecraft:the_end".to_string(),
                ],
                max_players: 20,
                chunk_radius: 8,
                simulation_distance: 6,
                reduced_debug_info: false,
                show_death_screen: true,
                do_limited_crafting: false,
                common_spawn_info: CommonPlayerSpawnInfo {
                    dimension_type_id: 1,
                    dimension: "minecraft:the_nether".to_string(),
                    seed: 12345,
                    game_type: 1,
                    previous_game_type: -1,
                    is_debug: false,
                    is_flat: false,
                    last_death_location: Some(GlobalPos {
                        dimension: "minecraft:overworld".to_string(),
                        pos: BlockPos { x: 1, y: 64, z: -2 },
                    }),
                    portal_cooldown: 7,
                    sea_level: 32,
                },
                enforces_secure_chat: true,
            })
        );
    }

    #[test]
    fn decodes_respawn_spawn_info() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_string("minecraft:the_end");
        payload.write_i64(98765);
        payload.write_i8(0);
        payload.write_i8(1);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_bool(false);
        payload.write_var_i32(0);
        payload.write_var_i32(63);
        payload.write_i8(3);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_RESPAWN, &payload.into_inner()).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::Respawn(Respawn {
                common_spawn_info: CommonPlayerSpawnInfo {
                    dimension_type_id: 2,
                    dimension: "minecraft:the_end".to_string(),
                    seed: 98765,
                    game_type: 0,
                    previous_game_type: 1,
                    is_debug: false,
                    is_flat: false,
                    last_death_location: None,
                    portal_cooldown: 0,
                    sea_level: 63,
                },
                data_to_keep: 3,
            })
        );
    }

    #[test]
    fn encodes_perform_respawn() {
        let (id, payload) = encode_play_perform_respawn();
        assert_eq!(id, ids::play::SERVERBOUND_CLIENT_COMMAND);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.is_empty());
    }

    #[test]
    fn encodes_set_carried_item() {
        let (id, payload) = encode_play_set_carried_item(6);
        assert_eq!(id, ids::play::SERVERBOUND_SET_CARRIED_ITEM);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_i16().unwrap(), 6);
        assert!(decoder.is_empty());
    }

    #[test]
    fn encodes_player_input_flags() {
        let (id, payload) = encode_play_player_input(PlayerInput {
            forward: true,
            backward: false,
            left: true,
            right: false,
            jump: true,
            shift: true,
            sprint: false,
        });

        assert_eq!(id, ids::play::SERVERBOUND_PLAYER_INPUT);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_u8().unwrap(), 0b0011_0101);
        assert!(decoder.is_empty());
    }

    #[test]
    fn encodes_player_command_actions() {
        let (id, payload) = encode_play_player_command(PlayerCommand {
            entity_id: 1234,
            action: PlayerCommandAction::StartSprinting,
            data: 0,
        });
        assert_eq!(id, ids::play::SERVERBOUND_PLAYER_COMMAND);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 1234);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.is_empty());

        let (_, payload) = encode_play_player_command(PlayerCommand {
            entity_id: -7,
            action: PlayerCommandAction::StopSprinting,
            data: 0,
        });
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), -7);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.is_empty());
    }

    #[test]
    fn encodes_player_action_packet() {
        let (id, payload) = encode_play_player_action(PlayerAction {
            action: PlayerActionKind::StartDestroyBlock,
            pos: BlockPos {
                x: 34,
                y: -12,
                z: -45,
            },
            direction: Direction::North,
            sequence: 7,
        });

        assert_eq!(id, ids::play::SERVERBOUND_PLAYER_ACTION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(
            chunks::decode_block_pos(decoder.read_i64().unwrap()),
            BlockPos {
                x: 34,
                y: -12,
                z: -45,
            }
        );
        assert_eq!(decoder.read_u8().unwrap(), 2);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert!(decoder.is_empty());
    }

    #[test]
    fn encodes_swing_hand() {
        let (id, payload) = encode_play_swing(InteractionHand::MainHand);
        assert_eq!(id, ids::play::SERVERBOUND_SWING);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.is_empty());

        let (_, payload) = encode_play_swing(InteractionHand::OffHand);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert!(decoder.is_empty());
    }

    #[test]
    fn encodes_use_item_on_packet() {
        let (id, payload) = encode_play_use_item_on(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: BlockPos {
                    x: 34,
                    y: -12,
                    z: -45,
                },
                direction: Direction::Up,
                cursor_x: 0.25,
                cursor_y: 1.0,
                cursor_z: 0.75,
                inside: true,
                world_border_hit: false,
            },
            sequence: 11,
        });

        assert_eq!(id, ids::play::SERVERBOUND_USE_ITEM_ON);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(
            chunks::decode_block_pos(decoder.read_i64().unwrap()),
            BlockPos {
                x: 34,
                y: -12,
                z: -45,
            }
        );
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_f32().unwrap(), 0.25);
        assert_eq!(decoder.read_f32().unwrap(), 1.0);
        assert_eq!(decoder.read_f32().unwrap(), 0.75);
        assert!(decoder.read_bool().unwrap());
        assert!(!decoder.read_bool().unwrap());
        assert_eq!(decoder.read_var_i32().unwrap(), 11);
        assert!(decoder.is_empty());
    }

    #[test]
    fn encodes_use_item_packet() {
        let (id, payload) = encode_play_use_item(UseItem {
            hand: InteractionHand::OffHand,
            sequence: 12,
            y_rot: 180.0,
            x_rot: -30.0,
        });

        assert_eq!(id, ids::play::SERVERBOUND_USE_ITEM);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 12);
        assert_eq!(decoder.read_f32().unwrap(), 180.0);
        assert_eq!(decoder.read_f32().unwrap(), -30.0);
        assert!(decoder.is_empty());
    }

    #[test]
    fn encodes_pick_item_from_block_packet() {
        let (id, payload) = encode_play_pick_item_from_block(PickItemFromBlock {
            pos: BlockPos {
                x: -5,
                y: 70,
                z: 12,
            },
            include_data: true,
        });

        assert_eq!(id, ids::play::SERVERBOUND_PICK_ITEM_FROM_BLOCK);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(
            chunks::decode_block_pos(decoder.read_i64().unwrap()),
            BlockPos {
                x: -5,
                y: 70,
                z: 12,
            }
        );
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_award_stats() {
        assert_eq!(ids::play::CLIENTBOUND_AWARD_STATS, 3);

        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i32(34);
        payload.write_var_i32(12);
        payload.write_var_i32(8);
        payload.write_var_i32(5);
        payload.write_var_i32(1);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_AWARD_STATS, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::AwardStats(AwardStats {
                stats: vec![
                    StatUpdate {
                        stat_type_id: 0,
                        value_id: 34,
                        amount: 12,
                    },
                    StatUpdate {
                        stat_type_id: 8,
                        value_id: 5,
                        amount: 1,
                    },
                ],
            })
        );
    }

    #[test]
    fn decodes_and_encodes_move_vehicle_packets() {
        let mut payload = Encoder::new();
        payload.write_f64(12.5);
        payload.write_f64(65.25);
        payload.write_f64(-8.75);
        payload.write_f32(135.0);
        payload.write_f32(-12.5);
        let payload = payload.into_inner();
        assert_eq!(payload.len(), 32);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_MOVE_VEHICLE, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::MoveVehicle(MoveVehicle {
                position: Vec3d {
                    x: 12.5,
                    y: 65.25,
                    z: -8.75,
                },
                y_rot: 135.0,
                x_rot: -12.5,
            })
        );

        let (id, payload) = encode_play_move_vehicle(12.5, 65.25, -8.75, 135.0, -12.5, true);
        assert_eq!(id, ids::play::SERVERBOUND_MOVE_VEHICLE);
        assert_eq!(id, 34);
        assert_eq!(payload.len(), 33);
        assert_eq!(payload[32], 1);

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_f64().unwrap(), 12.5);
        assert_eq!(decoder.read_f64().unwrap(), 65.25);
        assert_eq!(decoder.read_f64().unwrap(), -8.75);
        assert_eq!(decoder.read_f32().unwrap(), 135.0);
        assert_eq!(decoder.read_f32().unwrap(), -12.5);
        assert!(decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_chunk_batch_and_encodes_client_play_status_packets() {
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_CHUNK_BATCH_START, &[]).unwrap();
        assert_eq!(packet, PlayClientbound::ChunkBatchStart);

        let mut payload = Encoder::new();
        payload.write_var_i32(9);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_CHUNK_BATCH_FINISHED,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ChunkBatchFinished { batch_size: 9 }
        );

        let (id, payload) = encode_play_chunk_batch_received(9.0);
        assert_eq!(id, ids::play::SERVERBOUND_CHUNK_BATCH_RECEIVED);
        assert_eq!(payload.len(), 4);
        assert_eq!(Decoder::new(&payload).read_f32().unwrap(), 9.0);

        let (id, payload) = encode_play_client_tick_end();
        assert_eq!(id, ids::play::SERVERBOUND_CLIENT_TICK_END);
        assert!(payload.is_empty());

        let (id, payload) = encode_play_client_information_default();
        assert_eq!(id, ids::play::SERVERBOUND_CLIENT_INFORMATION);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(16).unwrap(), "en_us");
        assert_eq!(decoder.read_i8().unwrap(), 10);
    }

    fn encode_block_pos(x: i32, y: i32, z: i32) -> i64 {
        (((x as i64) & 0x3ffffff) << 38) | (((z as i64) & 0x3ffffff) << 12) | ((y as i64) & 0xfff)
    }
}
