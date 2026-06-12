use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{decode_component_summary, decode_component_summary_from_decoder},
    ids, PROTOCOL_VERSION,
};

const MAX_CHUNKS_BIOMES_BUFFER: usize = 2 * 1024 * 1024;
const MAX_CLOCK_UPDATES: usize = 4096;
const MAX_CONTAINER_ITEMS: usize = 1024;
const MAX_ENTITY_ATTRIBUTES: usize = 1024;
const MAX_ENTITY_ID_LIST: usize = 8192;
const MAX_EQUIPMENT_SLOTS: usize = 8;
const MAX_ATTRIBUTE_MODIFIERS: usize = 1024;
const MAX_ITEM_COMPONENT_PATCH_ENTRIES: usize = 1024;
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
    AddEntity(AddEntity),
    BlockChangedAck(BlockChangedAck),
    BlockEntityData(BlockEntityData),
    BlockUpdate(BlockUpdate),
    ChunkBatchStart,
    ChunkBatchFinished { batch_size: i32 },
    ChunksBiomes(ChunksBiomes),
    ContainerClose(ContainerClose),
    ContainerSetContent(ContainerSetContent),
    ContainerSetData(ContainerSetData),
    ContainerSetSlot(ContainerSetSlot),
    Disconnect(Disconnect),
    EntityPositionSync(EntityPositionSync),
    ForgetLevelChunk(ForgetLevelChunk),
    GameEvent(GameEvent),
    KeepAlive { id: i64 },
    Ping { id: i32 },
    Login(PlayLogin),
    MoveEntity(EntityMove),
    OpenScreen(OpenScreen),
    PlayerPosition(PlayerPositionUpdate),
    PlayerAbilities(PlayerAbilities),
    RemoveEntities(RemoveEntities),
    Respawn(Respawn),
    RotateHead(RotateHead),
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
    StartConfiguration,
    SetTime(PlayTime),
    SystemChat(SystemChat),
    TeleportEntity(TeleportEntity),
    UpdateAttributes(UpdateAttributes),
    LevelChunkWithLight(LevelChunkWithLight),
    LightUpdate(LightUpdate),
    Unknown { packet_id: i32, len: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AddEntity {
    pub id: i32,
    pub uuid: Uuid,
    pub entity_type_id: i32,
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub x_rot: f32,
    pub y_rot: f32,
    pub y_head_rot: f32,
    pub data: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityPositionSync {
    pub id: i32,
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityMove {
    pub id: i32,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub y_rot: Option<f32>,
    pub x_rot: Option<f32>,
    pub on_ground: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveEntities {
    pub entity_ids: Vec<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RotateHead {
    pub id: i32,
    pub y_head_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SetEntityMotion {
    pub id: i32,
    pub delta_movement: Vec3d,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetEntityLink {
    pub source_id: i32,
    pub dest_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetEquipment {
    pub entity_id: i32,
    pub slots: Vec<EquipmentSlotUpdate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPassengers {
    pub vehicle_id: i32,
    pub passenger_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentSlotUpdate {
    pub slot: EquipmentSlot,
    pub item: ItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateAttributes {
    pub entity_id: i32,
    pub attributes: Vec<AttributeSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeSnapshot {
    pub attribute_id: i32,
    pub base: f64,
    pub modifiers: Vec<AttributeModifier>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributeModifier {
    pub id: String,
    pub amount: f64,
    pub operation_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EquipmentSlot {
    MainHand,
    OffHand,
    Feet,
    Legs,
    Chest,
    Head,
    Body,
    Saddle,
}

impl EquipmentSlot {
    pub fn ordinal(self) -> u8 {
        match self {
            Self::MainHand => 0,
            Self::OffHand => 1,
            Self::Feet => 2,
            Self::Legs => 3,
            Self::Chest => 4,
            Self::Head => 5,
            Self::Body => 6,
            Self::Saddle => 7,
        }
    }

    fn from_ordinal(value: u8) -> Result<Self> {
        Ok(match value {
            0 => Self::MainHand,
            1 => Self::OffHand,
            2 => Self::Feet,
            3 => Self::Legs,
            4 => Self::Chest,
            5 => Self::Head,
            6 => Self::Body,
            7 => Self::Saddle,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid equipment slot {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStackSummary {
    pub item_id: Option<i32>,
    pub count: i32,
    pub component_patch: DataComponentPatchSummary,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataComponentPatchSummary {
    pub added: usize,
    pub removed_type_ids: Vec<i32>,
}

impl ItemStackSummary {
    pub fn empty() -> Self {
        Self {
            item_id: None,
            count: 0,
            component_patch: DataComponentPatchSummary::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerClose {
    pub container_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSetContent {
    pub container_id: i32,
    pub state_id: i32,
    pub items: Vec<ItemStackSummary>,
    pub carried_item: ItemStackSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSetData {
    pub container_id: i32,
    pub id: i16,
    pub value: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSetSlot {
    pub container_id: i32,
    pub state_id: i32,
    pub slot: i16,
    pub item: ItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenScreen {
    pub container_id: i32,
    pub menu_type_id: i32,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCursorItem {
    pub item: ItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPlayerInventory {
    pub slot: i32,
    pub item: ItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetEntityData {
    pub id: i32,
    pub values: Vec<EntityDataValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityDataValue {
    pub data_id: u8,
    pub serializer_id: i32,
    pub value: EntityDataValueKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EntityDataValueKind {
    Byte(i8),
    Int(i32),
    Long(i64),
    Float(f32),
    String(String),
    Component(String),
    OptionalComponent(Option<String>),
    Boolean(bool),
    Rotations {
        x: f32,
        y: f32,
        z: f32,
    },
    BlockPos(BlockPos),
    OptionalBlockPos(Option<BlockPos>),
    Direction(i32),
    BlockState(i32),
    OptionalBlockState(Option<i32>),
    VillagerData {
        villager_type: i32,
        profession: i32,
        level: i32,
    },
    OptionalUnsignedInt(Option<i32>),
    Pose(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TeleportEntity {
    pub id: i32,
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub relatives_mask: i32,
    pub on_ground: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockUpdate {
    pub pos: BlockPos,
    pub block_state_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEntityData {
    pub pos: BlockPos,
    pub block_entity_type_id: i32,
    pub raw_nbt: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockChangedAck {
    pub sequence: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionBlocksUpdate {
    pub section_x: i32,
    pub section_y: i32,
    pub section_z: i32,
    pub updates: Vec<BlockUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunksBiomes {
    pub chunks: Vec<ChunkBiomeData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkBiomeData {
    pub pos: ChunkPos,
    pub raw_biomes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disconnect {
    pub reason: String,
    pub raw_reason: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgetLevelChunk {
    pub pos: ChunkPos,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GameEvent {
    pub event_id: u8,
    pub param: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayTime {
    pub game_time: i64,
    pub clock_updates: Vec<ClockUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClockUpdate {
    pub clock_id: i32,
    pub total_ticks: i64,
    pub partial_tick: f32,
    pub rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetChunkCacheCenter {
    pub chunk_x: i32,
    pub chunk_z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetChunkCacheRadius {
    pub radius: i32,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerAbilities {
    pub invulnerable: bool,
    pub flying: bool,
    pub can_fly: bool,
    pub instabuild: bool,
    pub flying_speed: f32,
    pub walking_speed: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetDefaultSpawnPosition {
    pub dimension: String,
    pub pos: BlockPos,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSimulationDistance {
    pub distance: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemChat {
    pub content: String,
    pub overlay: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerHealth {
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerExperience {
    pub progress: f32,
    pub level: i32,
    pub total: i32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetHeldSlot {
    pub slot: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec3d {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerPositionUpdate {
    pub id: i32,
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub relatives_mask: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerPositionState {
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
}

impl PlayerPositionUpdate {
    pub fn apply_to_state(self, current: PlayerPositionState) -> PlayerPositionState {
        let mut current_delta = current.delta_movement;
        let position = Vec3d {
            x: absolute_or_relative(
                current.position.x,
                self.position.x,
                self.relatives_mask,
                PLAYER_RELATIVE_X,
            ),
            y: absolute_or_relative(
                current.position.y,
                self.position.y,
                self.relatives_mask,
                PLAYER_RELATIVE_Y,
            ),
            z: absolute_or_relative(
                current.position.z,
                self.position.z,
                self.relatives_mask,
                PLAYER_RELATIVE_Z,
            ),
        };
        let y_rot = absolute_or_relative_f32(
            current.y_rot,
            self.y_rot,
            self.relatives_mask,
            PLAYER_RELATIVE_Y_ROT,
        );
        let x_rot = absolute_or_relative_f32(
            current.x_rot,
            self.x_rot,
            self.relatives_mask,
            PLAYER_RELATIVE_X_ROT,
        )
        .clamp(-90.0, 90.0);
        if self.relatives_mask & PLAYER_RELATIVE_ROTATE_DELTA != 0 {
            current_delta =
                rotate_delta_movement(current_delta, current.y_rot - y_rot, current.x_rot - x_rot);
        }
        let delta_movement = Vec3d {
            x: absolute_or_relative(
                current_delta.x,
                self.delta_movement.x,
                self.relatives_mask,
                PLAYER_RELATIVE_DELTA_X,
            ),
            y: absolute_or_relative(
                current_delta.y,
                self.delta_movement.y,
                self.relatives_mask,
                PLAYER_RELATIVE_DELTA_Y,
            ),
            z: absolute_or_relative(
                current_delta.z,
                self.delta_movement.z,
                self.relatives_mask,
                PLAYER_RELATIVE_DELTA_Z,
            ),
        };

        PlayerPositionState {
            position,
            delta_movement,
            y_rot,
            x_rot,
        }
    }
}

pub const PLAYER_RELATIVE_X: i32 = 1 << 0;
pub const PLAYER_RELATIVE_Y: i32 = 1 << 1;
pub const PLAYER_RELATIVE_Z: i32 = 1 << 2;
pub const PLAYER_RELATIVE_Y_ROT: i32 = 1 << 3;
pub const PLAYER_RELATIVE_X_ROT: i32 = 1 << 4;
pub const PLAYER_RELATIVE_DELTA_X: i32 = 1 << 5;
pub const PLAYER_RELATIVE_DELTA_Y: i32 = 1 << 6;
pub const PLAYER_RELATIVE_DELTA_Z: i32 = 1 << 7;
pub const PLAYER_RELATIVE_ROTATE_DELTA: i32 = 1 << 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelChunkWithLight {
    pub x: i32,
    pub z: i32,
    pub raw_after_position: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightUpdate {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub raw_light_data: Vec<u8>,
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

pub fn encode_play_accept_teleportation(id: i32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(id);
    (
        ids::play::SERVERBOUND_ACCEPT_TELEPORTATION,
        out.into_inner(),
    )
}

pub fn encode_play_move_player_pos_rot(
    x: f64,
    y: f64,
    z: f64,
    y_rot: f32,
    x_rot: f32,
    on_ground: bool,
    horizontal_collision: bool,
) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_f64(x);
    out.write_f64(y);
    out.write_f64(z);
    out.write_f32(y_rot);
    out.write_f32(x_rot);
    out.write_u8(pack_move_flags(on_ground, horizontal_collision));
    (ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT, out.into_inner())
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
    out.write_i64(encode_block_pos(action.pos));
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
    encode_block_hit_result(&mut out, packet.hit);
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
    out.write_i64(encode_block_pos(packet.pos));
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
        ids::play::CLIENTBOUND_ADD_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::AddEntity(decode_add_entity(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK => {
            Ok(PlayClientbound::BlockChangedAck(BlockChangedAck {
                sequence: Decoder::new(payload).read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockEntityData(BlockEntityData {
                pos: decode_block_pos(decoder.read_i64()?),
                block_entity_type_id: decoder.read_var_i32()?,
                raw_nbt: decoder.remaining().to_vec(),
            }))
        }
        ids::play::CLIENTBOUND_BLOCK_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockUpdate(decode_block_update(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_CHUNK_BATCH_START => Ok(PlayClientbound::ChunkBatchStart),
        ids::play::CLIENTBOUND_CHUNK_BATCH_FINISHED => Ok(PlayClientbound::ChunkBatchFinished {
            batch_size: Decoder::new(payload).read_var_i32()?,
        }),
        ids::play::CLIENTBOUND_CHUNKS_BIOMES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ChunksBiomes(decode_chunks_biomes(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_CONTAINER_CLOSE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerClose(ContainerClose {
                container_id: decoder.read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_CONTAINER_SET_CONTENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerSetContent(
                decode_container_set_content(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_CONTAINER_SET_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerSetData(ContainerSetData {
                container_id: decoder.read_var_i32()?,
                id: decoder.read_i16()?,
                value: decoder.read_i16()?,
            }))
        }
        ids::play::CLIENTBOUND_CONTAINER_SET_SLOT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ContainerSetSlot(
                decode_container_set_slot(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_DISCONNECT => Ok(PlayClientbound::Disconnect(Disconnect {
            reason: decode_component_summary(payload)?,
            raw_reason: payload.to_vec(),
        })),
        ids::play::CLIENTBOUND_ENTITY_POSITION_SYNC => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::EntityPositionSync(
                decode_entity_position_sync(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_FORGET_LEVEL_CHUNK => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ForgetLevelChunk(ForgetLevelChunk {
                pos: decode_chunk_pos(decoder.read_i64()?),
            }))
        }
        ids::play::CLIENTBOUND_GAME_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::GameEvent(GameEvent {
                event_id: decoder.read_u8()?,
                param: decoder.read_f32()?,
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
            Ok(PlayClientbound::MoveEntity(decode_move_entity(
                &mut decoder,
                true,
                false,
            )?))
        }
        ids::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MoveEntity(decode_move_entity(
                &mut decoder,
                true,
                true,
            )?))
        }
        ids::play::CLIENTBOUND_MOVE_ENTITY_ROT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MoveEntity(decode_move_entity(
                &mut decoder,
                false,
                true,
            )?))
        }
        ids::play::CLIENTBOUND_OPEN_SCREEN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::OpenScreen(OpenScreen {
                container_id: decoder.read_var_i32()?,
                menu_type_id: decoder.read_var_i32()?,
                title: decode_component_summary_from_decoder(&mut decoder)?,
            }))
        }
        ids::play::CLIENTBOUND_PLAYER_ABILITIES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerAbilities(decode_player_abilities(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_PLAYER_POSITION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerPosition(decode_player_position(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_REMOVE_ENTITIES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::RemoveEntities(decode_remove_entities(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_RESPAWN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Respawn(decode_respawn(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_ROTATE_HEAD => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::RotateHead(RotateHead {
                id: decoder.read_var_i32()?,
                y_head_rot: unpack_degrees(decoder.read_i8()?),
            }))
        }
        ids::play::CLIENTBOUND_SET_HEALTH => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetHealth(PlayerHealth {
                health: decoder.read_f32()?,
                food: decoder.read_var_i32()?,
                saturation: decoder.read_f32()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_HELD_SLOT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetHeldSlot(SetHeldSlot {
                slot: decoder.read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_PASSENGERS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetPassengers(decode_set_passengers(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SectionBlocksUpdate(
                decode_section_blocks_update(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_CENTER => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetChunkCacheCenter(SetChunkCacheCenter {
                chunk_x: decoder.read_var_i32()?,
                chunk_z: decoder.read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_CHUNK_CACHE_RADIUS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetChunkCacheRadius(SetChunkCacheRadius {
                radius: decoder.read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_CURSOR_ITEM => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetCursorItem(SetCursorItem {
                item: decode_item_stack_summary(&mut decoder)?,
            }))
        }
        ids::play::CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetDefaultSpawnPosition(
                decode_default_spawn_position(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_ENTITY_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEntityData(decode_set_entity_data(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SET_ENTITY_LINK => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEntityLink(SetEntityLink {
                source_id: decoder.read_i32()?,
                dest_id: decoder.read_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_ENTITY_MOTION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEntityMotion(SetEntityMotion {
                id: decoder.read_var_i32()?,
                delta_movement: decode_lp_vec3d(&mut decoder)?,
            }))
        }
        ids::play::CLIENTBOUND_SET_EQUIPMENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetEquipment(decode_set_equipment(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SET_EXPERIENCE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetExperience(PlayerExperience {
                progress: decoder.read_f32()?,
                level: decoder.read_var_i32()?,
                total: decoder.read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_SET_PLAYER_INVENTORY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetPlayerInventory(
                decode_set_player_inventory(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_SET_SIMULATION_DISTANCE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetSimulationDistance(
                SetSimulationDistance {
                    distance: decoder.read_var_i32()?,
                },
            ))
        }
        ids::play::CLIENTBOUND_SET_TIME => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetTime(decode_play_time(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_START_CONFIGURATION => Ok(PlayClientbound::StartConfiguration),
        ids::play::CLIENTBOUND_SYSTEM_CHAT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SystemChat(SystemChat {
                content: decode_component_summary_from_decoder(&mut decoder)?,
                overlay: decoder.read_bool()?,
            }))
        }
        ids::play::CLIENTBOUND_TELEPORT_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TeleportEntity(decode_teleport_entity(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::UpdateAttributes(decode_update_attributes(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT => {
            let mut decoder = Decoder::new(payload);
            let x = decoder.read_i32()?;
            let z = decoder.read_i32()?;
            Ok(PlayClientbound::LevelChunkWithLight(LevelChunkWithLight {
                x,
                z,
                raw_after_position: decoder.remaining().to_vec(),
            }))
        }
        ids::play::CLIENTBOUND_LIGHT_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::LightUpdate(LightUpdate {
                chunk_x: decoder.read_var_i32()?,
                chunk_z: decoder.read_var_i32()?,
                raw_light_data: decoder.remaining().to_vec(),
            }))
        }
        id => Ok(PlayClientbound::Unknown {
            packet_id: id,
            len: payload.len(),
        }),
    }
}

fn decode_chunks_biomes(decoder: &mut Decoder<'_>) -> Result<ChunksBiomes> {
    let count = decoder.read_len()?;
    let mut chunks = Vec::with_capacity(count);
    for _ in 0..count {
        let pos = decode_chunk_pos(decoder.read_i64()?);
        let len = decoder.read_len()?;
        if len > MAX_CHUNKS_BIOMES_BUFFER {
            return Err(ProtocolError::PacketTooLarge(len, MAX_CHUNKS_BIOMES_BUFFER));
        }
        chunks.push(ChunkBiomeData {
            pos,
            raw_biomes: decoder.read_exact(len, "chunk biome data")?.to_vec(),
        });
    }
    Ok(ChunksBiomes { chunks })
}

fn decode_container_set_content(decoder: &mut Decoder<'_>) -> Result<ContainerSetContent> {
    let container_id = decoder.read_var_i32()?;
    let state_id = decoder.read_var_i32()?;
    let item_count = decoder.read_len()?;
    if item_count > MAX_CONTAINER_ITEMS {
        return Err(ProtocolError::PacketTooLarge(
            item_count,
            MAX_CONTAINER_ITEMS,
        ));
    }
    let mut items = Vec::with_capacity(item_count);
    for _ in 0..item_count {
        items.push(decode_item_stack_summary(decoder)?);
    }
    let carried_item = decode_item_stack_summary(decoder)?;
    Ok(ContainerSetContent {
        container_id,
        state_id,
        items,
        carried_item,
    })
}

fn decode_container_set_slot(decoder: &mut Decoder<'_>) -> Result<ContainerSetSlot> {
    Ok(ContainerSetSlot {
        container_id: decoder.read_var_i32()?,
        state_id: decoder.read_var_i32()?,
        slot: decoder.read_i16()?,
        item: decode_item_stack_summary(decoder)?,
    })
}

fn decode_set_player_inventory(decoder: &mut Decoder<'_>) -> Result<SetPlayerInventory> {
    Ok(SetPlayerInventory {
        slot: decoder.read_var_i32()?,
        item: decode_item_stack_summary(decoder)?,
    })
}

fn decode_add_entity(decoder: &mut Decoder<'_>) -> Result<AddEntity> {
    Ok(AddEntity {
        id: decoder.read_var_i32()?,
        uuid: decoder.read_uuid()?,
        entity_type_id: decoder.read_var_i32()?,
        position: decode_vec3d(decoder)?,
        delta_movement: decode_lp_vec3d(decoder)?,
        x_rot: unpack_degrees(decoder.read_i8()?),
        y_rot: unpack_degrees(decoder.read_i8()?),
        y_head_rot: unpack_degrees(decoder.read_i8()?),
        data: decoder.read_var_i32()?,
    })
}

fn decode_entity_position_sync(decoder: &mut Decoder<'_>) -> Result<EntityPositionSync> {
    Ok(EntityPositionSync {
        id: decoder.read_var_i32()?,
        position: decode_vec3d(decoder)?,
        delta_movement: decode_vec3d(decoder)?,
        y_rot: decoder.read_f32()?,
        x_rot: decoder.read_f32()?,
        on_ground: decoder.read_bool()?,
    })
}

fn decode_move_entity(
    decoder: &mut Decoder<'_>,
    has_position: bool,
    has_rotation: bool,
) -> Result<EntityMove> {
    let id = decoder.read_var_i32()?;
    let (delta_x, delta_y, delta_z) = if has_position {
        (
            decoder.read_i16()?,
            decoder.read_i16()?,
            decoder.read_i16()?,
        )
    } else {
        (0, 0, 0)
    };
    let (y_rot, x_rot) = if has_rotation {
        (
            Some(unpack_degrees(decoder.read_i8()?)),
            Some(unpack_degrees(decoder.read_i8()?)),
        )
    } else {
        (None, None)
    };

    Ok(EntityMove {
        id,
        delta_x,
        delta_y,
        delta_z,
        y_rot,
        x_rot,
        on_ground: decoder.read_bool()?,
    })
}

fn decode_remove_entities(decoder: &mut Decoder<'_>) -> Result<RemoveEntities> {
    let count = decoder.read_len()?;
    if count > MAX_ENTITY_ID_LIST {
        return Err(ProtocolError::PacketTooLarge(count, MAX_ENTITY_ID_LIST));
    }
    let mut entity_ids = Vec::with_capacity(count);
    for _ in 0..count {
        entity_ids.push(decoder.read_var_i32()?);
    }
    Ok(RemoveEntities { entity_ids })
}

fn decode_set_passengers(decoder: &mut Decoder<'_>) -> Result<SetPassengers> {
    let vehicle_id = decoder.read_var_i32()?;
    let count = decoder.read_len()?;
    if count > MAX_ENTITY_ID_LIST {
        return Err(ProtocolError::PacketTooLarge(count, MAX_ENTITY_ID_LIST));
    }
    let mut passenger_ids = Vec::with_capacity(count);
    for _ in 0..count {
        passenger_ids.push(decoder.read_var_i32()?);
    }
    Ok(SetPassengers {
        vehicle_id,
        passenger_ids,
    })
}

fn decode_teleport_entity(decoder: &mut Decoder<'_>) -> Result<TeleportEntity> {
    Ok(TeleportEntity {
        id: decoder.read_var_i32()?,
        position: decode_vec3d(decoder)?,
        delta_movement: decode_vec3d(decoder)?,
        y_rot: decoder.read_f32()?,
        x_rot: decoder.read_f32()?,
        relatives_mask: decoder.read_i32()?,
        on_ground: decoder.read_bool()?,
    })
}

fn decode_set_equipment(decoder: &mut Decoder<'_>) -> Result<SetEquipment> {
    let entity_id = decoder.read_var_i32()?;
    let mut slots = Vec::new();
    loop {
        if slots.len() >= MAX_EQUIPMENT_SLOTS {
            return Err(ProtocolError::PacketTooLarge(
                slots.len() + 1,
                MAX_EQUIPMENT_SLOTS,
            ));
        }

        let raw_slot = decoder.read_u8()?;
        let should_continue = raw_slot & 0x80 != 0;
        let slot = EquipmentSlot::from_ordinal(raw_slot & 0x7f)?;
        let item = decode_item_stack_summary(decoder)?;
        slots.push(EquipmentSlotUpdate { slot, item });

        if !should_continue {
            break;
        }
    }
    Ok(SetEquipment { entity_id, slots })
}

fn decode_item_stack_summary(decoder: &mut Decoder<'_>) -> Result<ItemStackSummary> {
    let count = decoder.read_var_i32()?;
    if count <= 0 {
        return Ok(ItemStackSummary::empty());
    }

    let item_id = decoder.read_var_i32()?;
    let component_patch = decode_data_component_patch_summary(decoder)?;
    Ok(ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch,
    })
}

fn decode_data_component_patch_summary(
    decoder: &mut Decoder<'_>,
) -> Result<DataComponentPatchSummary> {
    let added = decoder.read_len()?;
    let removed = decoder.read_len()?;
    if added + removed > MAX_ITEM_COMPONENT_PATCH_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            added + removed,
            MAX_ITEM_COMPONENT_PATCH_ENTRIES,
        ));
    }
    if added != 0 {
        return Err(ProtocolError::InvalidData(format!(
            "unsupported item stack component patch with {added} added component(s)"
        )));
    }

    let mut removed_type_ids = Vec::with_capacity(removed);
    for _ in 0..removed {
        removed_type_ids.push(decoder.read_var_i32()?);
    }
    Ok(DataComponentPatchSummary {
        added,
        removed_type_ids,
    })
}

fn decode_set_entity_data(decoder: &mut Decoder<'_>) -> Result<SetEntityData> {
    let id = decoder.read_var_i32()?;
    let mut values = Vec::new();
    loop {
        let data_id = decoder.read_u8()?;
        if data_id == 0xff {
            break;
        }
        let serializer_id = decoder.read_var_i32()?;
        values.push(EntityDataValue {
            data_id,
            serializer_id,
            value: decode_entity_data_value(decoder, serializer_id)?,
        });
    }
    Ok(SetEntityData { id, values })
}

fn decode_entity_data_value(
    decoder: &mut Decoder<'_>,
    serializer_id: i32,
) -> Result<EntityDataValueKind> {
    Ok(match serializer_id {
        0 => EntityDataValueKind::Byte(decoder.read_i8()?),
        1 => EntityDataValueKind::Int(decoder.read_var_i32()?),
        2 => EntityDataValueKind::Long(decoder.read_var_i64()?),
        3 => EntityDataValueKind::Float(decoder.read_f32()?),
        4 => EntityDataValueKind::String(decoder.read_string(32767)?),
        5 => EntityDataValueKind::Component(decode_component_summary_from_decoder(decoder)?),
        6 => EntityDataValueKind::OptionalComponent(if decoder.read_bool()? {
            Some(decode_component_summary_from_decoder(decoder)?)
        } else {
            None
        }),
        8 => EntityDataValueKind::Boolean(decoder.read_bool()?),
        9 => EntityDataValueKind::Rotations {
            x: decoder.read_f32()?,
            y: decoder.read_f32()?,
            z: decoder.read_f32()?,
        },
        10 => EntityDataValueKind::BlockPos(decode_block_pos(decoder.read_i64()?)),
        11 => EntityDataValueKind::OptionalBlockPos(if decoder.read_bool()? {
            Some(decode_block_pos(decoder.read_i64()?))
        } else {
            None
        }),
        12 => EntityDataValueKind::Direction(decoder.read_var_i32()?),
        14 => EntityDataValueKind::BlockState(decoder.read_var_i32()?),
        15 => {
            let id = decoder.read_var_i32()?;
            EntityDataValueKind::OptionalBlockState((id != 0).then_some(id))
        }
        18 => EntityDataValueKind::VillagerData {
            villager_type: decoder.read_var_i32()?,
            profession: decoder.read_var_i32()?,
            level: decoder.read_var_i32()?,
        },
        19 => {
            let value = decoder.read_var_i32()?;
            EntityDataValueKind::OptionalUnsignedInt((value != 0).then_some(value - 1))
        }
        20 => EntityDataValueKind::Pose(decoder.read_var_i32()?),
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "unsupported entity data serializer {other}"
            )))
        }
    })
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

fn decode_play_time(decoder: &mut Decoder<'_>) -> Result<PlayTime> {
    let game_time = decoder.read_i64()?;
    let clock_count = decoder.read_len()?;
    if clock_count > MAX_CLOCK_UPDATES {
        return Err(ProtocolError::PacketTooLarge(
            clock_count,
            MAX_CLOCK_UPDATES,
        ));
    }
    let mut clock_updates = Vec::with_capacity(clock_count);
    for _ in 0..clock_count {
        clock_updates.push(ClockUpdate {
            clock_id: decoder.read_var_i32()?,
            total_ticks: decoder.read_var_i64()?,
            partial_tick: decoder.read_f32()?,
            rate: decoder.read_f32()?,
        });
    }
    Ok(PlayTime {
        game_time,
        clock_updates,
    })
}

fn decode_player_abilities(decoder: &mut Decoder<'_>) -> Result<PlayerAbilities> {
    let flags = decoder.read_u8()?;
    Ok(PlayerAbilities {
        invulnerable: flags & 0x01 != 0,
        flying: flags & 0x02 != 0,
        can_fly: flags & 0x04 != 0,
        instabuild: flags & 0x08 != 0,
        flying_speed: decoder.read_f32()?,
        walking_speed: decoder.read_f32()?,
    })
}

fn decode_default_spawn_position(decoder: &mut Decoder<'_>) -> Result<SetDefaultSpawnPosition> {
    Ok(SetDefaultSpawnPosition {
        dimension: read_resource_key(decoder)?,
        pos: decode_block_pos(decoder.read_i64()?),
        yaw: decoder.read_f32()?,
        pitch: decoder.read_f32()?,
    })
}

fn decode_optional_global_pos(decoder: &mut Decoder<'_>) -> Result<Option<GlobalPos>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }
    Ok(Some(GlobalPos {
        dimension: read_resource_key(decoder)?,
        pos: decode_block_pos(decoder.read_i64()?),
    }))
}

fn read_resource_key(decoder: &mut Decoder<'_>) -> Result<String> {
    decoder.read_string(32767)
}

fn decode_block_update(decoder: &mut Decoder<'_>) -> Result<BlockUpdate> {
    Ok(BlockUpdate {
        pos: decode_block_pos(decoder.read_i64()?),
        block_state_id: decoder.read_var_i32()?,
    })
}

fn decode_section_blocks_update(decoder: &mut Decoder<'_>) -> Result<SectionBlocksUpdate> {
    let (section_x, section_y, section_z) = decode_section_pos(decoder.read_i64()?);
    let count = decoder.read_len()?;
    let mut updates = Vec::with_capacity(count);
    for _ in 0..count {
        let packed_change = decoder.read_var_i64()? as u64;
        let packed_pos = (packed_change & 0x0fff) as u16;
        let block_state_id = (packed_change >> 12) as i32;
        updates.push(BlockUpdate {
            pos: BlockPos {
                x: section_x * 16 + i32::from((packed_pos >> 8) & 0x0f),
                y: section_y * 16 + i32::from(packed_pos & 0x0f),
                z: section_z * 16 + i32::from((packed_pos >> 4) & 0x0f),
            },
            block_state_id,
        });
    }
    Ok(SectionBlocksUpdate {
        section_x,
        section_y,
        section_z,
        updates,
    })
}

fn decode_update_attributes(decoder: &mut Decoder<'_>) -> Result<UpdateAttributes> {
    let entity_id = decoder.read_var_i32()?;
    let attribute_count = decoder.read_len()?;
    if attribute_count > MAX_ENTITY_ATTRIBUTES {
        return Err(ProtocolError::InvalidData(format!(
            "attribute list has {attribute_count} entries, max is {MAX_ENTITY_ATTRIBUTES}"
        )));
    }
    let mut attributes = Vec::with_capacity(attribute_count);
    for _ in 0..attribute_count {
        let attribute_id = decoder.read_var_i32()?;
        let base = decoder.read_f64()?;
        let modifier_count = decoder.read_len()?;
        if modifier_count > MAX_ATTRIBUTE_MODIFIERS {
            return Err(ProtocolError::InvalidData(format!(
                "attribute modifier list has {modifier_count} entries, max is {MAX_ATTRIBUTE_MODIFIERS}"
            )));
        }
        let mut modifiers = Vec::with_capacity(modifier_count);
        for _ in 0..modifier_count {
            modifiers.push(AttributeModifier {
                id: read_resource_key(decoder)?,
                amount: decoder.read_f64()?,
                operation_id: decoder.read_var_i32()?,
            });
        }
        attributes.push(AttributeSnapshot {
            attribute_id,
            base,
            modifiers,
        });
    }
    Ok(UpdateAttributes {
        entity_id,
        attributes,
    })
}

fn decode_block_pos(packed: i64) -> BlockPos {
    BlockPos {
        x: (packed >> 38) as i32,
        y: ((packed << 52) >> 52) as i32,
        z: ((packed << 26) >> 38) as i32,
    }
}

fn encode_block_pos(pos: BlockPos) -> i64 {
    (((pos.x as i64) & 0x3ffffff) << 38)
        | (((pos.z as i64) & 0x3ffffff) << 12)
        | ((pos.y as i64) & 0xfff)
}

fn encode_block_hit_result(out: &mut Encoder, hit: BlockHitResult) {
    out.write_i64(encode_block_pos(hit.pos));
    out.write_var_i32(i32::from(hit.direction.id()));
    out.write_f32(hit.cursor_x);
    out.write_f32(hit.cursor_y);
    out.write_f32(hit.cursor_z);
    out.write_bool(hit.inside);
    out.write_bool(hit.world_border_hit);
}

fn decode_section_pos(packed: i64) -> (i32, i32, i32) {
    (
        (packed >> 42) as i32,
        ((packed << 44) >> 44) as i32,
        ((packed << 22) >> 42) as i32,
    )
}

fn decode_chunk_pos(packed: i64) -> ChunkPos {
    ChunkPos {
        x: packed as i32,
        z: (packed >> 32) as i32,
    }
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

fn decode_lp_vec3d(decoder: &mut Decoder<'_>) -> Result<Vec3d> {
    let lowest = decoder.read_u8()?;
    if lowest == 0 {
        return Ok(Vec3d::default());
    }
    let middle = decoder.read_u8()? as u64;
    let highest = u32::from_be_bytes(
        decoder
            .read_exact(4, "lp vec3 highest")?
            .try_into()
            .expect("fixed length"),
    ) as u64;
    let buffer = (highest << 16) | (middle << 8) | u64::from(lowest);
    let mut scale = u64::from(lowest & 0x03);
    if lowest & 0x04 != 0 {
        scale |= u64::from(decoder.read_var_i32()? as u32) << 2;
    }
    let scale = scale as f64;
    Ok(Vec3d {
        x: unpack_lp_vec_component(buffer >> 3) * scale,
        y: unpack_lp_vec_component(buffer >> 18) * scale,
        z: unpack_lp_vec_component(buffer >> 33) * scale,
    })
}

fn unpack_lp_vec_component(value: u64) -> f64 {
    ((value & 0x7fff).min(32766) as f64) * 2.0 / 32766.0 - 1.0
}

fn unpack_degrees(value: i8) -> f32 {
    f32::from(value) * 360.0 / 256.0
}

fn absolute_or_relative(current: f64, change: f64, mask: i32, relative_bit: i32) -> f64 {
    if mask & relative_bit != 0 {
        current + change
    } else {
        change
    }
}

fn absolute_or_relative_f32(current: f32, change: f32, mask: i32, relative_bit: i32) -> f32 {
    if mask & relative_bit != 0 {
        current + change
    } else {
        change
    }
}

fn rotate_delta_movement(delta: Vec3d, y_rot_degrees: f32, x_rot_degrees: f32) -> Vec3d {
    let x_rad = f64::from(x_rot_degrees).to_radians();
    let y_rad = f64::from(y_rot_degrees).to_radians();
    let cos_x = x_rad.cos();
    let sin_x = x_rad.sin();
    let after_x = Vec3d {
        x: delta.x,
        y: delta.y * cos_x + delta.z * sin_x,
        z: delta.z * cos_x - delta.y * sin_x,
    };
    let cos_y = y_rad.cos();
    let sin_y = y_rad.sin();
    Vec3d {
        x: after_x.x * cos_y + after_x.z * sin_y,
        y: after_x.y,
        z: after_x.z * cos_y - after_x.x * sin_y,
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
    fn decodes_level_chunk_envelope() {
        let mut payload = Encoder::new();
        payload.write_i32(12);
        payload.write_i32(-4);
        payload.write_bytes(&[1, 2, 3]);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_LEVEL_CHUNK_WITH_LIGHT,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::LevelChunkWithLight(LevelChunkWithLight {
                x: 12,
                z: -4,
                raw_after_position: vec![1, 2, 3],
            })
        );
    }

    #[test]
    fn decodes_light_update_envelope() {
        let mut payload = Encoder::new();
        payload.write_var_i32(12);
        payload.write_var_i32(-4);
        payload.write_bytes(&[9, 8, 7]);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_LIGHT_UPDATE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::LightUpdate(LightUpdate {
                chunk_x: 12,
                chunk_z: -4,
                raw_light_data: vec![9, 8, 7],
            })
        );
    }

    #[test]
    fn decodes_chunks_biomes_update() {
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_i64(encode_chunk_pos(12, -4));
        payload.write_var_i32(3);
        payload.write_bytes(&[1, 2, 3]);
        payload.write_i64(encode_chunk_pos(-8, 5));
        payload.write_var_i32(2);
        payload.write_bytes(&[4, 5]);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_CHUNKS_BIOMES, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ChunksBiomes(ChunksBiomes {
                chunks: vec![
                    ChunkBiomeData {
                        pos: ChunkPos { x: 12, z: -4 },
                        raw_biomes: vec![1, 2, 3],
                    },
                    ChunkBiomeData {
                        pos: ChunkPos { x: -8, z: 5 },
                        raw_biomes: vec![4, 5],
                    },
                ],
            })
        );
    }

    #[test]
    fn decodes_entity_lifecycle_packets() {
        let uuid = Uuid::from_u128(0x12345678123456781234567812345678);
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_uuid(uuid);
        payload.write_var_i32(7);
        payload.write_f64(1.0);
        payload.write_f64(64.0);
        payload.write_f64(-2.0);
        payload.write_bytes(&lp_vec3_axis_x());
        payload.write_i8(-64);
        payload.write_i8(64);
        payload.write_i8(32);
        payload.write_var_i32(99);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_ADD_ENTITY, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::AddEntity(AddEntity {
                id: 123,
                uuid,
                entity_type_id: 7,
                position: Vec3d {
                    x: 1.0,
                    y: 64.0,
                    z: -2.0,
                },
                delta_movement: Vec3d {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                x_rot: -90.0,
                y_rot: 90.0,
                y_head_rot: 45.0,
                data: 99,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_f64(2.0);
        payload.write_f64(65.0);
        payload.write_f64(-3.0);
        payload.write_f64(0.0);
        payload.write_f64(0.25);
        payload.write_f64(0.0);
        payload.write_f32(180.0);
        payload.write_f32(30.0);
        payload.write_bool(true);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_ENTITY_POSITION_SYNC,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::EntityPositionSync(EntityPositionSync {
                id: 123,
                position: Vec3d {
                    x: 2.0,
                    y: 65.0,
                    z: -3.0,
                },
                delta_movement: Vec3d {
                    x: 0.0,
                    y: 0.25,
                    z: 0.0,
                },
                y_rot: 180.0,
                x_rot: 30.0,
                on_ground: true,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_i16(4096);
        payload.write_i16(0);
        payload.write_i16(-2048);
        payload.write_i8(-64);
        payload.write_i8(64);
        payload.write_bool(false);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_MOVE_ENTITY_POS_ROT,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::MoveEntity(EntityMove {
                id: 123,
                delta_x: 4096,
                delta_y: 0,
                delta_z: -2048,
                y_rot: Some(-90.0),
                x_rot: Some(90.0),
                on_ground: false,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_bytes(&[0]);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_ENTITY_MOTION,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetEntityMotion(SetEntityMotion {
                id: 123,
                delta_movement: Vec3d::default(),
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_f64(0.5);
        payload.write_f64(1.0);
        payload.write_f64(-0.5);
        payload.write_f64(0.0);
        payload.write_f64(0.1);
        payload.write_f64(0.0);
        payload.write_f32(15.0);
        payload.write_f32(-5.0);
        payload.write_i32(PLAYER_RELATIVE_X | PLAYER_RELATIVE_DELTA_Y);
        payload.write_bool(true);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_TELEPORT_ENTITY,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::TeleportEntity(TeleportEntity {
                id: 123,
                position: Vec3d {
                    x: 0.5,
                    y: 1.0,
                    z: -0.5,
                },
                delta_movement: Vec3d {
                    x: 0.0,
                    y: 0.1,
                    z: 0.0,
                },
                y_rot: 15.0,
                x_rot: -5.0,
                relatives_mask: PLAYER_RELATIVE_X | PLAYER_RELATIVE_DELTA_Y,
                on_ground: true,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_i8(64);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_ROTATE_HEAD, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::RotateHead(RotateHead {
                id: 123,
                y_head_rot: 90.0,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_var_i32(123);
        payload.write_var_i32(456);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_REMOVE_ENTITIES,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::RemoveEntities(RemoveEntities {
                entity_ids: vec![123, 456],
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_u8(0);
        payload.write_var_i32(0);
        payload.write_i8(0x20);
        payload.write_u8(2);
        payload.write_var_i32(1);
        payload.write_var_i32(300);
        payload.write_u8(3);
        payload.write_var_i32(5);
        payload.write_bytes(&nbt_string_root("Name"));
        payload.write_u8(4);
        payload.write_var_i32(6);
        payload.write_bool(false);
        payload.write_u8(5);
        payload.write_var_i32(9);
        payload.write_f32(1.0);
        payload.write_f32(2.0);
        payload.write_f32(3.0);
        payload.write_u8(0xff);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_ENTITY_DATA,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetEntityData(SetEntityData {
                id: 123,
                values: vec![
                    EntityDataValue {
                        data_id: 0,
                        serializer_id: 0,
                        value: EntityDataValueKind::Byte(0x20),
                    },
                    EntityDataValue {
                        data_id: 2,
                        serializer_id: 1,
                        value: EntityDataValueKind::Int(300),
                    },
                    EntityDataValue {
                        data_id: 3,
                        serializer_id: 5,
                        value: EntityDataValueKind::Component("Name".to_string()),
                    },
                    EntityDataValue {
                        data_id: 4,
                        serializer_id: 6,
                        value: EntityDataValueKind::OptionalComponent(None),
                    },
                    EntityDataValue {
                        data_id: 5,
                        serializer_id: 9,
                        value: EntityDataValueKind::Rotations {
                            x: 1.0,
                            y: 2.0,
                            z: 3.0,
                        },
                    },
                ],
            })
        );
    }

    #[test]
    fn decodes_set_equipment_slots_and_item_stacks() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_u8(EquipmentSlot::MainHand.ordinal() | 0x80);
        payload.write_var_i32(0);
        payload.write_u8(EquipmentSlot::Head.ordinal());
        payload.write_var_i32(1);
        payload.write_var_i32(42);
        payload.write_var_i32(0);
        payload.write_var_i32(0);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_EQUIPMENT, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetEquipment(SetEquipment {
                entity_id: 123,
                slots: vec![
                    EquipmentSlotUpdate {
                        slot: EquipmentSlot::MainHand,
                        item: ItemStackSummary::empty(),
                    },
                    EquipmentSlotUpdate {
                        slot: EquipmentSlot::Head,
                        item: ItemStackSummary {
                            item_id: Some(42),
                            count: 1,
                            component_patch: DataComponentPatchSummary::default(),
                        },
                    },
                ],
            })
        );
    }

    #[test]
    fn rejects_set_equipment_item_stack_with_added_component_patch() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_u8(EquipmentSlot::MainHand.ordinal());
        payload.write_var_i32(1);
        payload.write_var_i32(42);
        payload.write_var_i32(1);
        payload.write_var_i32(0);

        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_EQUIPMENT, &payload.into_inner())
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported item stack component patch"));
    }

    #[test]
    fn decodes_update_attributes_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_var_i32(2);
        payload.write_var_i32(21);
        payload.write_f64(20.0);
        payload.write_var_i32(1);
        payload.write_string("minecraft:health_bonus");
        payload.write_f64(4.0);
        payload.write_var_i32(0);
        payload.write_var_i32(26);
        payload.write_f64(0.7);
        payload.write_var_i32(0);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::UpdateAttributes(UpdateAttributes {
                entity_id: 123,
                attributes: vec![
                    AttributeSnapshot {
                        attribute_id: 21,
                        base: 20.0,
                        modifiers: vec![AttributeModifier {
                            id: "minecraft:health_bonus".to_string(),
                            amount: 4.0,
                            operation_id: 0,
                        }],
                    },
                    AttributeSnapshot {
                        attribute_id: 26,
                        base: 0.7,
                        modifiers: Vec::new(),
                    },
                ],
            })
        );
    }

    #[test]
    fn decodes_set_passengers_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(2);
        payload.write_var_i32(123);
        payload.write_var_i32(456);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_PASSENGERS, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPassengers(SetPassengers {
                vehicle_id: 7,
                passenger_ids: vec![123, 456],
            })
        );
    }

    #[test]
    fn decodes_set_entity_link_packet() {
        let mut payload = Encoder::new();
        payload.write_i32(123);
        payload.write_i32(456);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_ENTITY_LINK,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetEntityLink(SetEntityLink {
                source_id: 123,
                dest_id: 456,
            })
        );
    }

    #[test]
    fn decodes_container_and_inventory_item_updates() {
        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_CLOSE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ContainerClose(ContainerClose { container_id: 7 })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_i16(3);
        payload.write_i16(42);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_SET_DATA,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ContainerSetData(ContainerSetData {
                container_id: 7,
                id: 3,
                value: 42,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(2);
        payload.write_bytes(&nbt_string_root("Chest"));
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_OPEN_SCREEN, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::OpenScreen(OpenScreen {
                container_id: 7,
                menu_type_id: 2,
                title: "Chest".to_string(),
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(12);
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i32(64);
        payload.write_var_i32(42);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_SET_CONTENT,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ContainerSetContent(ContainerSetContent {
                container_id: 7,
                state_id: 12,
                items: vec![
                    ItemStackSummary::empty(),
                    ItemStackSummary {
                        item_id: Some(42),
                        count: 64,
                        component_patch: DataComponentPatchSummary::default(),
                    },
                ],
                carried_item: ItemStackSummary::empty(),
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(7);
        payload.write_var_i32(13);
        payload.write_i16(5);
        payload.write_var_i32(1);
        payload.write_var_i32(99);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_CONTAINER_SET_SLOT,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ContainerSetSlot(ContainerSetSlot {
                container_id: 7,
                state_id: 13,
                slot: 5,
                item: ItemStackSummary {
                    item_id: Some(99),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(0);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_CURSOR_ITEM,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetCursorItem(SetCursorItem {
                item: ItemStackSummary::empty(),
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(36);
        payload.write_var_i32(1);
        payload.write_var_i32(42);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_PLAYER_INVENTORY,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPlayerInventory(SetPlayerInventory {
                slot: 36,
                item: ItemStackSummary {
                    item_id: Some(42),
                    count: 1,
                    component_patch: DataComponentPatchSummary::default(),
                },
            })
        );
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
    fn decodes_health_and_encodes_perform_respawn() {
        let mut payload = Encoder::new();
        payload.write_f32(0.0);
        payload.write_var_i32(17);
        payload.write_f32(1.5);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_HEALTH, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetHealth(PlayerHealth {
                health: 0.0,
                food: 17,
                saturation: 1.5,
            })
        );

        let (id, payload) = encode_play_perform_respawn();
        assert_eq!(id, ids::play::SERVERBOUND_CLIENT_COMMAND);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_player_experience() {
        let mut payload = Encoder::new();
        payload.write_f32(0.625);
        payload.write_var_i32(12);
        payload.write_var_i32(345);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_EXPERIENCE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetExperience(PlayerExperience {
                progress: 0.625,
                level: 12,
                total: 345,
            })
        );
    }

    #[test]
    fn decodes_held_slot_and_encodes_set_carried_item() {
        let mut payload = Encoder::new();
        payload.write_var_i32(6);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_HELD_SLOT, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetHeldSlot(SetHeldSlot { slot: 6 })
        );

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
            decode_block_pos(decoder.read_i64().unwrap()),
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
            decode_block_pos(decoder.read_i64().unwrap()),
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
            decode_block_pos(decoder.read_i64().unwrap()),
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
    fn decodes_game_event_and_set_time() {
        let mut payload = Encoder::new();
        payload.write_u8(7);
        payload.write_f32(0.75);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_GAME_EVENT, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::GameEvent(GameEvent {
                event_id: 7,
                param: 0.75,
            })
        );

        let mut payload = Encoder::new();
        payload.write_i64(12345);
        payload.write_var_i32(2);
        payload.write_var_i32(0);
        payload.write_var_i64(6000);
        payload.write_f32(0.25);
        payload.write_f32(1.0);
        payload.write_var_i32(1);
        payload.write_var_i64(18000);
        payload.write_f32(0.5);
        payload.write_f32(0.0);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_TIME, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetTime(PlayTime {
                game_time: 12345,
                clock_updates: vec![
                    ClockUpdate {
                        clock_id: 0,
                        total_ticks: 6000,
                        partial_tick: 0.25,
                        rate: 1.0,
                    },
                    ClockUpdate {
                        clock_id: 1,
                        total_ticks: 18000,
                        partial_tick: 0.5,
                        rate: 0.0,
                    },
                ],
            })
        );
    }

    #[test]
    fn decodes_single_and_section_block_updates() {
        let mut payload = Encoder::new();
        payload.write_var_i32(17);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BlockChangedAck(BlockChangedAck { sequence: 17 })
        );

        let mut payload = Encoder::new();
        payload.write_i64(encode_block_pos(34, -12, -45));
        payload.write_var_i32(9);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_UPDATE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BlockUpdate(BlockUpdate {
                pos: BlockPos {
                    x: 34,
                    y: -12,
                    z: -45,
                },
                block_state_id: 9,
            })
        );

        let mut payload = Encoder::new();
        payload.write_i64(encode_section_pos(2, -1, -3));
        payload.write_var_i32(2);
        payload.write_var_i64((9 << 12) | section_relative_pos(2, 1, 3));
        payload.write_var_i64((0 << 12) | section_relative_pos(15, 15, 15));
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SECTION_BLOCKS_UPDATE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SectionBlocksUpdate(SectionBlocksUpdate {
                section_x: 2,
                section_y: -1,
                section_z: -3,
                updates: vec![
                    BlockUpdate {
                        pos: BlockPos {
                            x: 34,
                            y: -15,
                            z: -45,
                        },
                        block_state_id: 9,
                    },
                    BlockUpdate {
                        pos: BlockPos {
                            x: 47,
                            y: -1,
                            z: -33,
                        },
                        block_state_id: 0,
                    },
                ],
            })
        );
    }

    #[test]
    fn decodes_block_entity_data_update() {
        let raw_nbt = nbt_compound_with_string("id", "minecraft:chest");
        let mut payload = Encoder::new();
        payload.write_i64(encode_block_pos(34, 64, -45));
        payload.write_var_i32(5);
        payload.write_bytes(&raw_nbt);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_BLOCK_ENTITY_DATA,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BlockEntityData(BlockEntityData {
                pos: BlockPos {
                    x: 34,
                    y: 64,
                    z: -45,
                },
                block_entity_type_id: 5,
                raw_nbt,
            })
        );
    }

    #[test]
    fn decodes_forget_level_chunk() {
        let mut payload = Encoder::new();
        payload.write_i64(encode_chunk_pos(12, -4));

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_FORGET_LEVEL_CHUNK,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ForgetLevelChunk(ForgetLevelChunk {
                pos: ChunkPos { x: 12, z: -4 },
            })
        );
    }

    #[test]
    fn decodes_chunk_cache_center_and_radius() {
        let mut payload = Encoder::new();
        payload.write_var_i32(12);
        payload.write_var_i32(-4);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_CHUNK_CACHE_CENTER,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetChunkCacheCenter(SetChunkCacheCenter {
                chunk_x: 12,
                chunk_z: -4,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(10);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_CHUNK_CACHE_RADIUS,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetChunkCacheRadius(SetChunkCacheRadius { radius: 10 })
        );
    }

    #[test]
    fn decodes_player_abilities_spawn_distance_and_system_chat() {
        let mut payload = Encoder::new();
        payload.write_u8(0b0000_1101);
        payload.write_f32(0.05);
        payload.write_f32(0.1);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_PLAYER_ABILITIES,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::PlayerAbilities(PlayerAbilities {
                invulnerable: true,
                flying: false,
                can_fly: true,
                instabuild: true,
                flying_speed: 0.05,
                walking_speed: 0.1,
            })
        );

        let mut payload = Encoder::new();
        payload.write_string("minecraft:overworld");
        payload.write_i64(encode_block_pos(-5, 70, 12));
        payload.write_f32(90.0);
        payload.write_f32(-10.0);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_DEFAULT_SPAWN_POSITION,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetDefaultSpawnPosition(SetDefaultSpawnPosition {
                dimension: "minecraft:overworld".to_string(),
                pos: BlockPos {
                    x: -5,
                    y: 70,
                    z: 12,
                },
                yaw: 90.0,
                pitch: -10.0,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(12);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_SIMULATION_DISTANCE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetSimulationDistance(SetSimulationDistance { distance: 12 })
        );

        let mut payload = nbt_string_root("Server restarting");
        payload.push(1);
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_SYSTEM_CHAT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SystemChat(SystemChat {
                content: "Server restarting".to_string(),
                overlay: true,
            })
        );
    }

    #[test]
    fn decodes_player_position_and_encodes_ack_pair() {
        let mut payload = Encoder::new();
        payload.write_var_i32(77);
        payload.write_f64(1.0);
        payload.write_f64(64.0);
        payload.write_f64(-2.0);
        payload.write_f64(0.0);
        payload.write_f64(0.1);
        payload.write_f64(0.0);
        payload.write_f32(180.0);
        payload.write_f32(15.0);
        payload.write_i32(0);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_PLAYER_POSITION,
            &payload.into_inner(),
        )
        .unwrap();
        let PlayClientbound::PlayerPosition(update) = packet else {
            panic!("wrong packet");
        };
        assert_eq!(update.id, 77);
        assert_eq!(update.position.y, 64.0);

        let (id, ack) = encode_play_accept_teleportation(update.id);
        assert_eq!(id, ids::play::SERVERBOUND_ACCEPT_TELEPORTATION);
        assert_eq!(Decoder::new(&ack).read_var_i32().unwrap(), 77);

        let (id, pos) = encode_play_move_player_pos_rot(
            update.position.x,
            update.position.y,
            update.position.z,
            update.y_rot,
            update.x_rot,
            false,
            false,
        );
        assert_eq!(id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
        assert_eq!(pos.len(), 33);
    }

    #[test]
    fn player_position_update_applies_relative_state() {
        let current = PlayerPositionState {
            position: Vec3d {
                x: 10.0,
                y: 64.0,
                z: -5.0,
            },
            delta_movement: Vec3d {
                x: 0.125,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 90.0,
            x_rot: 15.0,
        };
        let change = PlayerPositionUpdate {
            id: 2,
            position: Vec3d {
                x: 1.5,
                y: -2.0,
                z: 7.0,
            },
            delta_movement: Vec3d {
                x: 0.25,
                y: 0.5,
                z: 0.75,
            },
            y_rot: 20.0,
            x_rot: -120.0,
            relatives_mask: PLAYER_RELATIVE_X
                | PLAYER_RELATIVE_Y_ROT
                | PLAYER_RELATIVE_X_ROT
                | PLAYER_RELATIVE_DELTA_X,
        };

        let state = change.apply_to_state(current);

        assert_eq!(
            state.position,
            Vec3d {
                x: 11.5,
                y: -2.0,
                z: 7.0,
            }
        );
        assert_eq!(
            state.delta_movement,
            Vec3d {
                x: 0.375,
                y: 0.5,
                z: 0.75,
            }
        );
        assert_eq!(state.y_rot, 110.0);
        assert_eq!(state.x_rot, -90.0);
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

    fn encode_section_pos(x: i32, y: i32, z: i32) -> i64 {
        (((x as i64) & 0x3fffff) << 42) | (((z as i64) & 0x3fffff) << 20) | ((y as i64) & 0xfffff)
    }

    fn encode_chunk_pos(x: i32, z: i32) -> i64 {
        (((x as u32) as u64) | (((z as u32) as u64) << 32)) as i64
    }

    fn nbt_string_root(text: &str) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
        payload.extend_from_slice(text.as_bytes());
        payload
    }

    fn nbt_compound_with_string(name: &str, value: &str) -> Vec<u8> {
        let mut payload = vec![10, 8];
        payload.extend_from_slice(&(name.len() as u16).to_be_bytes());
        payload.extend_from_slice(name.as_bytes());
        payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
        payload.extend_from_slice(value.as_bytes());
        payload.push(0);
        payload
    }

    fn section_relative_pos(x: i64, y: i64, z: i64) -> i64 {
        (x << 8) | (z << 4) | y
    }

    fn lp_vec3_axis_x() -> [u8; 6] {
        let buffer = 1u64 | (32766u64 << 3) | (16383u64 << 18) | (16383u64 << 33);
        [
            buffer as u8,
            (buffer >> 8) as u8,
            ((buffer >> 16) >> 24) as u8,
            ((buffer >> 16) >> 16) as u8,
            ((buffer >> 16) >> 8) as u8,
            (buffer >> 16) as u8,
        ]
    }
}
