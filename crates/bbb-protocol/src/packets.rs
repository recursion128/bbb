use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{decode_component_summary, decode_component_summary_from_decoder},
    ids, PROTOCOL_VERSION,
};

pub mod movement;
pub use movement::*;

const MAX_CHUNKS_BIOMES_BUFFER: usize = 2 * 1024 * 1024;
const MAX_CLOCK_UPDATES: usize = 4096;
const MAX_CONTAINER_ITEMS: usize = 1024;
const MAX_ENTITY_ATTRIBUTES: usize = 1024;
const MAX_ENTITY_ID_LIST: usize = 8192;
const MAX_EQUIPMENT_SLOTS: usize = 8;
const MAX_ATTRIBUTE_MODIFIERS: usize = 1024;
const MAX_AWARD_STATS: usize = 8192;
const MAX_COMMAND_SUGGESTIONS: usize = 8192;
const MAX_ITEM_COMPONENT_PATCH_ENTRIES: usize = 1024;
const MAX_PLAYER_TEAM_PLAYERS: usize = 8192;
const MAX_PLAYER_INFO_ENTRIES: usize = 8192;
const MAX_GAME_PROFILE_PROPERTIES: usize = 1024;
const MAX_PROFILE_PUBLIC_KEY_BYTES: usize = 512;
const MAX_PROFILE_PUBLIC_KEY_SIGNATURE_BYTES: usize = 4096;
const MAX_SERVER_ICON_BYTES: usize = 2 * 1024 * 1024;
const PLAYER_INPUT_FORWARD: u8 = 1;
const PLAYER_INPUT_BACKWARD: u8 = 2;
const PLAYER_INPUT_LEFT: u8 = 4;
const PLAYER_INPUT_RIGHT: u8 = 8;
const PLAYER_INPUT_JUMP: u8 = 16;
const PLAYER_INPUT_SHIFT: u8 = 32;
const PLAYER_INPUT_SPRINT: u8 = 64;
const BOSS_EVENT_FLAG_DARKEN_SCREEN: u8 = 1;
const BOSS_EVENT_FLAG_PLAY_MUSIC: u8 = 2;
const BOSS_EVENT_FLAG_CREATE_WORLD_FOG: u8 = 4;
const MOB_EFFECT_FLAG_AMBIENT: u8 = 1;
const MOB_EFFECT_FLAG_VISIBLE: u8 = 2;
const MOB_EFFECT_FLAG_SHOW_ICON: u8 = 4;
const MOB_EFFECT_FLAG_BLEND: u8 = 8;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityAnimation {
    pub id: i32,
    pub action: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityEvent {
    pub entity_id: i32,
    pub event_id: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HurtAnimation {
    pub id: i32,
    pub yaw: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct InitializeBorder {
    pub new_center_x: f64,
    pub new_center_z: f64,
    pub old_size: f64,
    pub new_size: f64,
    pub lerp_time: i64,
    pub new_absolute_max_size: i32,
    pub warning_blocks: i32,
    pub warning_time: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SetBorderCenter {
    pub new_center_x: f64,
    pub new_center_z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SetBorderLerpSize {
    pub old_size: f64,
    pub new_size: f64,
    pub lerp_time: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SetBorderSize {
    pub size: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetBorderWarningDelay {
    pub warning_delay: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetBorderWarningDistance {
    pub warning_blocks: i32,
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MoveVehicle {
    pub position: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveEntities {
    pub entity_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestions {
    pub id: i32,
    pub start: i32,
    pub length: i32,
    pub suggestions: Vec<CommandSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestion {
    pub text: String,
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestionRequest {
    pub id: i32,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cooldown {
    pub cooldown_group: String,
    pub duration: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DamageEvent {
    pub entity_id: i32,
    pub source_type_id: i32,
    pub source_cause_id: i32,
    pub source_direct_id: i32,
    pub source_position: Option<Vec3d>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateMobEffect {
    pub entity_id: i32,
    pub effect_id: i32,
    pub amplifier: i32,
    pub duration_ticks: i32,
    pub flags: MobEffectFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoveMobEffect {
    pub entity_id: i32,
    pub effect_id: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobEffectFlags {
    pub raw: u8,
    pub ambient: bool,
    pub visible: bool,
    pub show_icon: bool,
    pub blend: bool,
}

impl MobEffectFlags {
    fn from_bits(raw: u8) -> Self {
        Self {
            raw,
            ambient: raw & MOB_EFFECT_FLAG_AMBIENT != 0,
            visible: raw & MOB_EFFECT_FLAG_VISIBLE != 0,
            show_icon: raw & MOB_EFFECT_FLAG_SHOW_ICON != 0,
            blend: raw & MOB_EFFECT_FLAG_BLEND != 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TakeItemEntity {
    pub item_id: i32,
    pub player_id: i32,
    pub amount: i32,
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
    ItemStack(ItemStackSummary),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDestruction {
    pub id: i32,
    pub pos: BlockPos,
    pub progress: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEvent {
    pub pos: BlockPos,
    pub b0: u8,
    pub b1: u8,
    pub block_id: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BossEvent {
    pub id: Uuid,
    pub operation: BossEventOperation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BossEventOperation {
    Add {
        name: String,
        progress: f32,
        color: BossBarColor,
        overlay: BossBarOverlay,
        flags: BossEventFlags,
    },
    Remove,
    UpdateProgress {
        progress: f32,
    },
    UpdateName {
        name: String,
    },
    UpdateStyle {
        color: BossBarColor,
        overlay: BossBarOverlay,
    },
    UpdateProperties {
        flags: BossEventFlags,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BossBarColor {
    Pink,
    Blue,
    Red,
    Green,
    Yellow,
    Purple,
    White,
}

impl BossBarColor {
    fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::Pink,
            1 => Self::Blue,
            2 => Self::Red,
            3 => Self::Green,
            4 => Self::Yellow,
            5 => Self::Purple,
            6 => Self::White,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid boss bar color ordinal {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BossBarOverlay {
    Progress,
    Notched6,
    Notched10,
    Notched12,
    Notched20,
}

impl BossBarOverlay {
    fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::Progress,
            1 => Self::Notched6,
            2 => Self::Notched10,
            3 => Self::Notched12,
            4 => Self::Notched20,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid boss bar overlay ordinal {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BossEventFlags {
    pub darken_screen: bool,
    pub play_music: bool,
    pub create_world_fog: bool,
}

impl BossEventFlags {
    fn from_bits(bits: u8) -> Self {
        Self {
            darken_screen: bits & BOSS_EVENT_FLAG_DARKEN_SCREEN != 0,
            play_music: bits & BOSS_EVENT_FLAG_PLAY_MUSIC != 0,
            create_world_fog: bits & BOSS_EVENT_FLAG_CREATE_WORLD_FOG != 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeDifficulty {
    pub difficulty: Difficulty,
    pub locked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    fn from_id(id: i32) -> Self {
        match id.rem_euclid(4) {
            0 => Self::Peaceful,
            1 => Self::Easy,
            2 => Self::Normal,
            3 => Self::Hard,
            _ => unreachable!("rem_euclid(4) is always in 0..4"),
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelEvent {
    pub event_type: i32,
    pub pos: BlockPos,
    pub data: i32,
    pub global: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoRemove {
    pub profile_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoUpdate {
    pub actions: Vec<PlayerInfoAction>,
    pub entries: Vec<PlayerInfoEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerInfoAction {
    AddPlayer,
    InitializeChat,
    UpdateGameMode,
    UpdateListed,
    UpdateLatency,
    UpdateDisplayName,
    UpdateListOrder,
    UpdateHat,
}

impl PlayerInfoAction {
    pub fn ordinal(self) -> u8 {
        match self {
            Self::AddPlayer => 0,
            Self::InitializeChat => 1,
            Self::UpdateGameMode => 2,
            Self::UpdateListed => 3,
            Self::UpdateLatency => 4,
            Self::UpdateDisplayName => 5,
            Self::UpdateListOrder => 6,
            Self::UpdateHat => 7,
        }
    }

    fn from_ordinal(ordinal: u8) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::AddPlayer,
            1 => Self::InitializeChat,
            2 => Self::UpdateGameMode,
            3 => Self::UpdateListed,
            4 => Self::UpdateLatency,
            5 => Self::UpdateDisplayName,
            6 => Self::UpdateListOrder,
            7 => Self::UpdateHat,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid player info action ordinal {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoEntry {
    pub profile_id: Uuid,
    pub profile: Option<GameProfile>,
    pub listed: bool,
    pub latency: i32,
    pub game_mode: GameType,
    pub display_name: Option<String>,
    pub show_hat: bool,
    pub list_order: i32,
    pub chat_session: Option<PlayerInfoChatSession>,
}

impl PlayerInfoEntry {
    fn new(profile_id: Uuid) -> Self {
        Self {
            profile_id,
            profile: None,
            listed: false,
            latency: 0,
            game_mode: GameType::default(),
            display_name: None,
            show_hat: false,
            list_order: 0,
            chat_session: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerInfoChatSession {
    pub session_id: Uuid,
    pub expires_at_epoch_millis: i64,
    pub public_key: Vec<u8>,
    pub key_signature: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameType {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl Default for GameType {
    fn default() -> Self {
        Self::Survival
    }
}

impl GameType {
    pub fn id(self) -> i32 {
        match self {
            Self::Survival => 0,
            Self::Creative => 1,
            Self::Adventure => 2,
            Self::Spectator => 3,
        }
    }

    fn from_id(id: i32) -> Self {
        match id {
            0 => Self::Survival,
            1 => Self::Creative,
            2 => Self::Adventure,
            3 => Self::Spectator,
            _ => Self::Survival,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetDefaultSpawnPosition {
    pub dimension: String,
    pub pos: BlockPos,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetScore {
    pub owner: String,
    pub objective_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetDisplayObjective {
    pub slot: ScoreboardDisplaySlot,
    pub objective_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreboardDisplaySlot {
    List,
    Sidebar,
    BelowName,
    TeamBlack,
    TeamDarkBlue,
    TeamDarkGreen,
    TeamDarkAqua,
    TeamDarkRed,
    TeamDarkPurple,
    TeamGold,
    TeamGray,
    TeamDarkGray,
    TeamBlue,
    TeamGreen,
    TeamAqua,
    TeamRed,
    TeamLightPurple,
    TeamYellow,
    TeamWhite,
}

impl ScoreboardDisplaySlot {
    fn from_id(id: i32) -> Self {
        match id {
            0 => Self::List,
            1 => Self::Sidebar,
            2 => Self::BelowName,
            3 => Self::TeamBlack,
            4 => Self::TeamDarkBlue,
            5 => Self::TeamDarkGreen,
            6 => Self::TeamDarkAqua,
            7 => Self::TeamDarkRed,
            8 => Self::TeamDarkPurple,
            9 => Self::TeamGold,
            10 => Self::TeamGray,
            11 => Self::TeamDarkGray,
            12 => Self::TeamBlue,
            13 => Self::TeamGreen,
            14 => Self::TeamAqua,
            15 => Self::TeamRed,
            16 => Self::TeamLightPurple,
            17 => Self::TeamYellow,
            18 => Self::TeamWhite,
            _ => Self::List,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetObjective {
    pub objective_name: String,
    pub method: SetObjectiveMethod,
    pub parameters: Option<SetObjectiveParameters>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetObjectiveParameters {
    pub display_name: String,
    pub render_type: ObjectiveRenderType,
    pub number_format: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SetObjectiveMethod {
    Add,
    Remove,
    Change,
}

impl SetObjectiveMethod {
    fn from_id(id: i8) -> Result<Self> {
        Ok(match id {
            0 => Self::Add,
            1 => Self::Remove,
            2 => Self::Change,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid objective method {other}"
                )))
            }
        })
    }

    fn has_parameters(self) -> bool {
        matches!(self, Self::Add | Self::Change)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectiveRenderType {
    Integer,
    Hearts,
}

impl ObjectiveRenderType {
    fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::Integer,
            1 => Self::Hearts,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid objective render type {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetScore {
    pub owner: String,
    pub objective_name: String,
    pub score: i32,
    pub display: Option<String>,
    pub number_format: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPlayerTeam {
    pub name: String,
    pub method: PlayerTeamMethod,
    pub parameters: Option<PlayerTeamParameters>,
    pub players: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTeamParameters {
    pub display_name: String,
    pub options: u8,
    pub nametag_visibility: TeamVisibility,
    pub collision_rule: TeamCollisionRule,
    pub color: ChatFormatting,
    pub player_prefix: String,
    pub player_suffix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerTeamMethod {
    Add,
    Remove,
    Change,
    Join,
    Leave,
}

impl PlayerTeamMethod {
    fn from_id(id: i8) -> Result<Self> {
        Ok(match id {
            0 => Self::Add,
            1 => Self::Remove,
            2 => Self::Change,
            3 => Self::Join,
            4 => Self::Leave,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid player team method {other}"
                )))
            }
        })
    }

    fn has_parameters(self) -> bool {
        matches!(self, Self::Add | Self::Change)
    }

    fn has_player_list(self) -> bool {
        matches!(self, Self::Add | Self::Join | Self::Leave)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamVisibility {
    Always,
    Never,
    HideForOtherTeams,
    HideForOwnTeam,
}

impl TeamVisibility {
    fn from_id(id: i32) -> Self {
        match id {
            0 => Self::Always,
            1 => Self::Never,
            2 => Self::HideForOtherTeams,
            3 => Self::HideForOwnTeam,
            _ => Self::Always,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamCollisionRule {
    Always,
    Never,
    PushOtherTeams,
    PushOwnTeam,
}

impl TeamCollisionRule {
    fn from_id(id: i32) -> Self {
        match id {
            0 => Self::Always,
            1 => Self::Never,
            2 => Self::PushOtherTeams,
            3 => Self::PushOwnTeam,
            _ => Self::Always,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatFormatting {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    Obfuscated,
    Bold,
    Strikethrough,
    Underline,
    Italic,
    Reset,
}

impl ChatFormatting {
    fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::Black,
            1 => Self::DarkBlue,
            2 => Self::DarkGreen,
            3 => Self::DarkAqua,
            4 => Self::DarkRed,
            5 => Self::DarkPurple,
            6 => Self::Gold,
            7 => Self::Gray,
            8 => Self::DarkGray,
            9 => Self::Blue,
            10 => Self::Green,
            11 => Self::Aqua,
            12 => Self::Red,
            13 => Self::LightPurple,
            14 => Self::Yellow,
            15 => Self::White,
            16 => Self::Obfuscated,
            17 => Self::Bold,
            18 => Self::Strikethrough,
            19 => Self::Underline,
            20 => Self::Italic,
            21 => Self::Reset,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid chat formatting ordinal {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSimulationDistance {
    pub distance: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetActionBarText {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetTitleText {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetSubtitleText {
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetTitlesAnimation {
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TickingState {
    pub tick_rate: f32,
    pub frozen: bool,
}

impl TickingState {
    pub fn clamped_tick_rate(&self) -> f32 {
        self.tick_rate.max(1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TickingStep {
    pub tick_steps: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetCamera {
    pub camera_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemChat {
    pub content: String,
    pub overlay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabList {
    pub header: Option<String>,
    pub footer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourcePackPush {
    pub id: Uuid,
    pub url: String,
    pub hash: String,
    pub required: bool,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourcePackPop {
    pub id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerData {
    pub motd: String,
    pub icon_bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourcePackResponseAction {
    SuccessfullyLoaded,
    Declined,
    FailedDownload,
    Accepted,
    Downloaded,
    InvalidUrl,
    FailedReload,
    Discarded,
}

impl ResourcePackResponseAction {
    pub fn ordinal(self) -> i32 {
        match self {
            Self::SuccessfullyLoaded => 0,
            Self::Declined => 1,
            Self::FailedDownload => 2,
            Self::Accepted => 3,
            Self::Downloaded => 4,
            Self::InvalidUrl => 5,
            Self::FailedReload => 6,
            Self::Discarded => 7,
        }
    }
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

pub fn encode_configuration_resource_pack_response(
    id: Uuid,
    action: ResourcePackResponseAction,
) -> (i32, Vec<u8>) {
    (
        ids::configuration::SERVERBOUND_RESOURCE_PACK,
        encode_resource_pack_response_payload(id, action),
    )
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

pub fn encode_play_command_suggestion_request(request: CommandSuggestionRequest) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(request.id);
    out.write_string(&request.command);
    (ids::play::SERVERBOUND_COMMAND_SUGGESTION, out.into_inner())
}

pub fn encode_play_resource_pack_response(
    id: Uuid,
    action: ResourcePackResponseAction,
) -> (i32, Vec<u8>) {
    (
        ids::play::SERVERBOUND_RESOURCE_PACK,
        encode_resource_pack_response_payload(id, action),
    )
}

fn encode_resource_pack_response_payload(id: Uuid, action: ResourcePackResponseAction) -> Vec<u8> {
    let mut out = Encoder::new();
    out.write_uuid(id);
    out.write_var_i32(action.ordinal());
    out.into_inner()
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
        ids::play::CLIENTBOUND_BUNDLE_DELIMITER => Ok(PlayClientbound::BundleDelimiter),
        ids::play::CLIENTBOUND_ADD_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::AddEntity(decode_add_entity(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_ANIMATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::EntityAnimation(EntityAnimation {
                id: decoder.read_var_i32()?,
                action: decoder.read_u8()?,
            }))
        }
        ids::play::CLIENTBOUND_AWARD_STATS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::AwardStats(decode_award_stats(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK => {
            Ok(PlayClientbound::BlockChangedAck(BlockChangedAck {
                sequence: Decoder::new(payload).read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_BLOCK_DESTRUCTION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockDestruction(BlockDestruction {
                id: decoder.read_var_i32()?,
                pos: decode_block_pos(decoder.read_i64()?),
                progress: decoder.read_u8()?,
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
        ids::play::CLIENTBOUND_BLOCK_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockEvent(BlockEvent {
                pos: decode_block_pos(decoder.read_i64()?),
                b0: decoder.read_u8()?,
                b1: decoder.read_u8()?,
                block_id: decoder.read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_BLOCK_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BlockUpdate(decode_block_update(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_BOSS_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::BossEvent(decode_boss_event(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_CHANGE_DIFFICULTY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ChangeDifficulty(decode_change_difficulty(
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
        ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::CommandSuggestions(
                decode_command_suggestions(&mut decoder)?,
            ))
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
        ids::play::CLIENTBOUND_COOLDOWN => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::Cooldown(decode_cooldown(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_DAMAGE_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::DamageEvent(decode_damage_event(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_DISCONNECT => Ok(PlayClientbound::Disconnect(Disconnect {
            reason: decode_component_summary(payload)?,
            raw_reason: payload.to_vec(),
        })),
        ids::play::CLIENTBOUND_ENTITY_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::EntityEvent(EntityEvent {
                entity_id: decoder.read_i32()?,
                event_id: decoder.read_i8()?,
            }))
        }
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
        ids::play::CLIENTBOUND_HURT_ANIMATION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::HurtAnimation(HurtAnimation {
                id: decoder.read_var_i32()?,
                yaw: decoder.read_f32()?,
            }))
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
        ids::play::CLIENTBOUND_MOVE_VEHICLE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::MoveVehicle(MoveVehicle {
                position: decode_vec3d(&mut decoder)?,
                y_rot: decoder.read_f32()?,
                x_rot: decoder.read_f32()?,
            }))
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
        ids::play::CLIENTBOUND_PLAYER_INFO_REMOVE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerInfoRemove(
                decode_player_info_remove(&mut decoder)?,
            ))
        }
        ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::PlayerInfoUpdate(
                decode_player_info_update(&mut decoder)?,
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
            Ok(PlayClientbound::RemoveEntities(decode_remove_entities(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_REMOVE_MOB_EFFECT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::RemoveMobEffect(decode_remove_mob_effect(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_RESET_SCORE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ResetScore(decode_reset_score(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_RESOURCE_PACK_POP => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ResourcePackPop(decode_resource_pack_pop(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_RESOURCE_PACK_PUSH => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ResourcePackPush(
                decode_resource_pack_push(&mut decoder)?,
            ))
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
        ids::play::CLIENTBOUND_SERVER_DATA => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::ServerData(decode_server_data(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SET_ACTION_BAR_TEXT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetActionBarText(SetActionBarText {
                content: decode_component_summary_from_decoder(&mut decoder)?,
            }))
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
            Ok(PlayClientbound::SetCamera(SetCamera {
                camera_id: decoder.read_var_i32()?,
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
        ids::play::CLIENTBOUND_SET_OBJECTIVE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetObjective(decode_set_objective(
                &mut decoder,
            )?))
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
        ids::play::CLIENTBOUND_SET_DISPLAY_OBJECTIVE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetDisplayObjective(
                decode_set_display_objective(&mut decoder)?,
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
        ids::play::CLIENTBOUND_SET_PLAYER_TEAM => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetPlayerTeam(decode_set_player_team(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_SET_SCORE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetScore(decode_set_score(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_SET_SIMULATION_DISTANCE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetSimulationDistance(
                SetSimulationDistance {
                    distance: decoder.read_var_i32()?,
                },
            ))
        }
        ids::play::CLIENTBOUND_SET_SUBTITLE_TEXT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetSubtitleText(SetSubtitleText {
                content: decode_component_summary_from_decoder(&mut decoder)?,
            }))
        }
        ids::play::CLIENTBOUND_SET_TIME => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetTime(decode_play_time(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_SET_TITLE_TEXT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetTitleText(SetTitleText {
                content: decode_component_summary_from_decoder(&mut decoder)?,
            }))
        }
        ids::play::CLIENTBOUND_SET_TITLES_ANIMATION => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SetTitlesAnimation(SetTitlesAnimation {
                fade_in: decoder.read_i32()?,
                stay: decoder.read_i32()?,
                fade_out: decoder.read_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_START_CONFIGURATION => Ok(PlayClientbound::StartConfiguration),
        ids::play::CLIENTBOUND_SYSTEM_CHAT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::SystemChat(SystemChat {
                content: decode_component_summary_from_decoder(&mut decoder)?,
                overlay: decoder.read_bool()?,
            }))
        }
        ids::play::CLIENTBOUND_TAB_LIST => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TabList(decode_tab_list(&mut decoder)?))
        }
        ids::play::CLIENTBOUND_TAKE_ITEM_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TakeItemEntity(TakeItemEntity {
                item_id: decoder.read_var_i32()?,
                player_id: decoder.read_var_i32()?,
                amount: decoder.read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_TELEPORT_ENTITY => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TeleportEntity(decode_teleport_entity(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_TICKING_STATE => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TickingState(TickingState {
                tick_rate: decoder.read_f32()?,
                frozen: decoder.read_bool()?,
            }))
        }
        ids::play::CLIENTBOUND_TICKING_STEP => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::TickingStep(TickingStep {
                tick_steps: decoder.read_var_i32()?,
            }))
        }
        ids::play::CLIENTBOUND_UPDATE_ATTRIBUTES => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::UpdateAttributes(decode_update_attributes(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_UPDATE_MOB_EFFECT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::UpdateMobEffect(decode_update_mob_effect(
                &mut decoder,
            )?))
        }
        ids::play::CLIENTBOUND_LEVEL_EVENT => {
            let mut decoder = Decoder::new(payload);
            Ok(PlayClientbound::LevelEvent(LevelEvent {
                event_type: decoder.read_i32()?,
                pos: decode_block_pos(decoder.read_i64()?),
                data: decoder.read_i32()?,
                global: decoder.read_bool()?,
            }))
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

fn decode_reset_score(decoder: &mut Decoder<'_>) -> Result<ResetScore> {
    Ok(ResetScore {
        owner: decoder.read_string(32767)?,
        objective_name: decode_nullable_string(decoder)?,
    })
}

fn decode_resource_pack_pop(decoder: &mut Decoder<'_>) -> Result<ResourcePackPop> {
    Ok(ResourcePackPop {
        id: decode_optional_uuid(decoder)?,
    })
}

fn decode_resource_pack_push(decoder: &mut Decoder<'_>) -> Result<ResourcePackPush> {
    Ok(ResourcePackPush {
        id: decoder.read_uuid()?,
        url: decoder.read_string(32767)?,
        hash: decoder.read_string(40)?,
        required: decoder.read_bool()?,
        prompt: decode_optional_component_summary_from_decoder(decoder)?,
    })
}

fn decode_server_data(decoder: &mut Decoder<'_>) -> Result<ServerData> {
    Ok(ServerData {
        motd: decode_component_summary_from_decoder(decoder)?,
        icon_bytes: decode_optional_byte_array(decoder, MAX_SERVER_ICON_BYTES, "server icon")?,
    })
}

fn decode_boss_event(decoder: &mut Decoder<'_>) -> Result<BossEvent> {
    let id = decoder.read_uuid()?;
    let operation = match decoder.read_var_i32()? {
        0 => BossEventOperation::Add {
            name: decode_component_summary_from_decoder(decoder)?,
            progress: decoder.read_f32()?,
            color: BossBarColor::from_ordinal(decoder.read_var_i32()?)?,
            overlay: BossBarOverlay::from_ordinal(decoder.read_var_i32()?)?,
            flags: BossEventFlags::from_bits(decoder.read_u8()?),
        },
        1 => BossEventOperation::Remove,
        2 => BossEventOperation::UpdateProgress {
            progress: decoder.read_f32()?,
        },
        3 => BossEventOperation::UpdateName {
            name: decode_component_summary_from_decoder(decoder)?,
        },
        4 => BossEventOperation::UpdateStyle {
            color: BossBarColor::from_ordinal(decoder.read_var_i32()?)?,
            overlay: BossBarOverlay::from_ordinal(decoder.read_var_i32()?)?,
        },
        5 => BossEventOperation::UpdateProperties {
            flags: BossEventFlags::from_bits(decoder.read_u8()?),
        },
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "invalid boss event operation ordinal {other}"
            )))
        }
    };

    Ok(BossEvent { id, operation })
}

fn decode_change_difficulty(decoder: &mut Decoder<'_>) -> Result<ChangeDifficulty> {
    Ok(ChangeDifficulty {
        difficulty: Difficulty::from_id(decoder.read_var_i32()?),
        locked: decoder.read_bool()?,
    })
}

fn decode_tab_list(decoder: &mut Decoder<'_>) -> Result<TabList> {
    Ok(TabList {
        header: decode_tab_list_component(decoder)?,
        footer: decode_tab_list_component(decoder)?,
    })
}

fn decode_tab_list_component(decoder: &mut Decoder<'_>) -> Result<Option<String>> {
    let before = decoder.remaining();
    let summary = decode_component_summary_from_decoder(decoder)?;
    let consumed_len = before.len().saturating_sub(decoder.remaining_len());
    let consumed = &before[..consumed_len];

    if is_empty_string_component_nbt(consumed) {
        Ok(None)
    } else {
        Ok(Some(summary))
    }
}

fn is_empty_string_component_nbt(payload: &[u8]) -> bool {
    payload == [8, 0, 0]
}

fn decode_set_display_objective(decoder: &mut Decoder<'_>) -> Result<SetDisplayObjective> {
    let slot = ScoreboardDisplaySlot::from_id(decoder.read_var_i32()?);
    let objective_name = decoder.read_string(32767)?;
    Ok(SetDisplayObjective {
        slot,
        objective_name: (!objective_name.is_empty()).then_some(objective_name),
    })
}

fn decode_set_objective(decoder: &mut Decoder<'_>) -> Result<SetObjective> {
    let objective_name = decoder.read_string(32767)?;
    let method = SetObjectiveMethod::from_id(decoder.read_i8()?)?;
    let parameters = if method.has_parameters() {
        Some(SetObjectiveParameters {
            display_name: decode_component_summary_from_decoder(decoder)?,
            render_type: ObjectiveRenderType::from_ordinal(decoder.read_var_i32()?)?,
            number_format: decode_optional_trailing_number_format(decoder)?,
        })
    } else {
        None
    };

    Ok(SetObjective {
        objective_name,
        method,
        parameters,
    })
}

fn decode_set_score(decoder: &mut Decoder<'_>) -> Result<SetScore> {
    Ok(SetScore {
        owner: decoder.read_string(32767)?,
        objective_name: decoder.read_string(32767)?,
        score: decoder.read_var_i32()?,
        display: decode_optional_component_summary_from_decoder(decoder)?,
        number_format: decode_optional_trailing_number_format(decoder)?,
    })
}

fn decode_set_player_team(decoder: &mut Decoder<'_>) -> Result<SetPlayerTeam> {
    let name = decoder.read_string(32767)?;
    let method = PlayerTeamMethod::from_id(decoder.read_i8()?)?;
    let parameters = if method.has_parameters() {
        Some(decode_player_team_parameters(decoder)?)
    } else {
        None
    };
    let players = if method.has_player_list() {
        decode_player_team_player_list(decoder)?
    } else {
        Vec::new()
    };

    Ok(SetPlayerTeam {
        name,
        method,
        parameters,
        players,
    })
}

fn decode_player_team_parameters(decoder: &mut Decoder<'_>) -> Result<PlayerTeamParameters> {
    Ok(PlayerTeamParameters {
        display_name: decode_component_summary_from_decoder(decoder)?,
        options: decoder.read_u8()?,
        nametag_visibility: TeamVisibility::from_id(decoder.read_var_i32()?),
        collision_rule: TeamCollisionRule::from_id(decoder.read_var_i32()?),
        color: ChatFormatting::from_ordinal(decoder.read_var_i32()?)?,
        player_prefix: decode_component_summary_from_decoder(decoder)?,
        player_suffix: decode_component_summary_from_decoder(decoder)?,
    })
}

fn decode_player_team_player_list(decoder: &mut Decoder<'_>) -> Result<Vec<String>> {
    let count = decoder.read_len()?;
    if count > MAX_PLAYER_TEAM_PLAYERS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_PLAYER_TEAM_PLAYERS,
        ));
    }

    let mut players = Vec::with_capacity(count);
    for _ in 0..count {
        players.push(decoder.read_string(32767)?);
    }
    Ok(players)
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

fn decode_command_suggestions(decoder: &mut Decoder<'_>) -> Result<CommandSuggestions> {
    let id = decoder.read_var_i32()?;
    let start = decoder.read_var_i32()?;
    let length = decoder.read_var_i32()?;
    let count = decoder.read_len()?;
    if count > MAX_COMMAND_SUGGESTIONS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_COMMAND_SUGGESTIONS,
        ));
    }

    let mut suggestions = Vec::with_capacity(count);
    for _ in 0..count {
        suggestions.push(CommandSuggestion {
            text: decoder.read_string(32767)?,
            tooltip: decode_optional_component_summary_from_decoder(decoder)?,
        });
    }

    Ok(CommandSuggestions {
        id,
        start,
        length,
        suggestions,
    })
}

fn decode_cooldown(decoder: &mut Decoder<'_>) -> Result<Cooldown> {
    Ok(Cooldown {
        cooldown_group: read_resource_key(decoder)?,
        duration: decoder.read_var_i32()?,
    })
}

fn decode_damage_event(decoder: &mut Decoder<'_>) -> Result<DamageEvent> {
    Ok(DamageEvent {
        entity_id: decoder.read_var_i32()?,
        source_type_id: decoder.read_var_i32()?,
        source_cause_id: decoder.read_var_i32()? - 1,
        source_direct_id: decoder.read_var_i32()? - 1,
        source_position: decode_optional_vec3d(decoder)?,
    })
}

fn decode_update_mob_effect(decoder: &mut Decoder<'_>) -> Result<UpdateMobEffect> {
    Ok(UpdateMobEffect {
        entity_id: decoder.read_var_i32()?,
        effect_id: decoder.read_var_i32()?,
        amplifier: decoder.read_var_i32()?,
        duration_ticks: decoder.read_var_i32()?,
        flags: MobEffectFlags::from_bits(decoder.read_u8()?),
    })
}

fn decode_remove_mob_effect(decoder: &mut Decoder<'_>) -> Result<RemoveMobEffect> {
    Ok(RemoveMobEffect {
        entity_id: decoder.read_var_i32()?,
        effect_id: decoder.read_var_i32()?,
    })
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
        7 => EntityDataValueKind::ItemStack(decode_item_stack_summary(decoder)?),
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

fn decode_player_info_remove(decoder: &mut Decoder<'_>) -> Result<PlayerInfoRemove> {
    let count = decoder.read_len()?;
    if count > MAX_PLAYER_INFO_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_PLAYER_INFO_ENTRIES,
        ));
    }

    let mut profile_ids = Vec::with_capacity(count);
    for _ in 0..count {
        profile_ids.push(decoder.read_uuid()?);
    }
    Ok(PlayerInfoRemove { profile_ids })
}

fn decode_player_info_update(decoder: &mut Decoder<'_>) -> Result<PlayerInfoUpdate> {
    let actions = decode_player_info_actions(decoder)?;
    let count = decoder.read_len()?;
    if count > MAX_PLAYER_INFO_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_PLAYER_INFO_ENTRIES,
        ));
    }

    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let profile_id = decoder.read_uuid()?;
        let mut entry = PlayerInfoEntry::new(profile_id);

        for action in &actions {
            match action {
                PlayerInfoAction::AddPlayer => {
                    let name = decoder.read_string(16)?;
                    let properties = decode_game_profile_properties(decoder)?;
                    entry.profile = Some(GameProfile {
                        uuid: profile_id,
                        name,
                        properties,
                    });
                }
                PlayerInfoAction::InitializeChat => {
                    entry.chat_session = decode_optional_player_info_chat_session(decoder)?;
                }
                PlayerInfoAction::UpdateGameMode => {
                    entry.game_mode = GameType::from_id(decoder.read_var_i32()?);
                }
                PlayerInfoAction::UpdateListed => {
                    entry.listed = decoder.read_bool()?;
                }
                PlayerInfoAction::UpdateLatency => {
                    entry.latency = decoder.read_var_i32()?;
                }
                PlayerInfoAction::UpdateDisplayName => {
                    entry.display_name = decode_optional_component_summary_from_decoder(decoder)?;
                }
                PlayerInfoAction::UpdateListOrder => {
                    entry.list_order = decoder.read_var_i32()?;
                }
                PlayerInfoAction::UpdateHat => {
                    entry.show_hat = decoder.read_bool()?;
                }
            }
        }

        entries.push(entry);
    }

    Ok(PlayerInfoUpdate { actions, entries })
}

fn decode_player_info_actions(decoder: &mut Decoder<'_>) -> Result<Vec<PlayerInfoAction>> {
    let bits = decoder.read_u8()?;
    let mut actions = Vec::new();
    for ordinal in 0..8 {
        if bits & (1 << ordinal) != 0 {
            actions.push(PlayerInfoAction::from_ordinal(ordinal)?);
        }
    }
    Ok(actions)
}

fn decode_game_profile_properties(decoder: &mut Decoder<'_>) -> Result<Vec<GameProfileProperty>> {
    let count = decoder.read_len()?;
    if count > MAX_GAME_PROFILE_PROPERTIES {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_GAME_PROFILE_PROPERTIES,
        ));
    }

    let mut properties = Vec::with_capacity(count);
    for _ in 0..count {
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
    Ok(properties)
}

fn decode_optional_player_info_chat_session(
    decoder: &mut Decoder<'_>,
) -> Result<Option<PlayerInfoChatSession>> {
    if !decoder.read_bool()? {
        return Ok(None);
    }

    Ok(Some(PlayerInfoChatSession {
        session_id: decoder.read_uuid()?,
        expires_at_epoch_millis: decoder.read_i64()?,
        public_key: decode_byte_array(decoder, MAX_PROFILE_PUBLIC_KEY_BYTES, "profile public key")?,
        key_signature: decode_byte_array(
            decoder,
            MAX_PROFILE_PUBLIC_KEY_SIGNATURE_BYTES,
            "profile public key signature",
        )?,
    }))
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

fn decode_optional_vec3d(decoder: &mut Decoder<'_>) -> Result<Option<Vec3d>> {
    if decoder.read_bool()? {
        Ok(Some(decode_vec3d(decoder)?))
    } else {
        Ok(None)
    }
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
    fn decodes_boss_event_operations() {
        let id = Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);

        let payload = boss_event_payload(id, 0, |payload| {
            payload.write_bytes(&nbt_string_root("Raid"));
            payload.write_f32(0.75);
            payload.write_var_i32(5);
            payload.write_var_i32(3);
            payload.write_u8(BOSS_EVENT_FLAG_DARKEN_SCREEN | BOSS_EVENT_FLAG_CREATE_WORLD_FOG);
        });
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::Add {
                    name: "Raid".to_string(),
                    progress: 0.75,
                    color: BossBarColor::Purple,
                    overlay: BossBarOverlay::Notched12,
                    flags: BossEventFlags {
                        darken_screen: true,
                        play_music: false,
                        create_world_fog: true,
                    },
                },
            })
        );

        let payload = boss_event_payload(id, 1, |_| {});
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::Remove,
            })
        );

        let payload = boss_event_payload(id, 2, |payload| {
            payload.write_f32(0.25);
        });
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::UpdateProgress { progress: 0.25 },
            })
        );

        let payload = boss_event_payload(id, 3, |payload| {
            payload.write_bytes(&nbt_string_root("Dragon"));
        });
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::UpdateName {
                    name: "Dragon".to_string(),
                },
            })
        );

        let payload = boss_event_payload(id, 4, |payload| {
            payload.write_var_i32(6);
            payload.write_var_i32(4);
        });
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::UpdateStyle {
                    color: BossBarColor::White,
                    overlay: BossBarOverlay::Notched20,
                },
            })
        );

        let payload = boss_event_payload(id, 5, |payload| {
            payload.write_u8(BOSS_EVENT_FLAG_PLAY_MUSIC);
        });
        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_BOSS_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BossEvent(BossEvent {
                id,
                operation: BossEventOperation::UpdateProperties {
                    flags: BossEventFlags {
                        darken_screen: false,
                        play_music: true,
                        create_world_fog: false,
                    },
                },
            })
        );
    }

    #[test]
    fn decodes_change_difficulty_with_wrapped_ids() {
        let payload = change_difficulty_payload(2, true);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_CHANGE_DIFFICULTY, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ChangeDifficulty(ChangeDifficulty {
                difficulty: Difficulty::Normal,
                locked: true,
            })
        );

        let payload = change_difficulty_payload(5, false);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_CHANGE_DIFFICULTY, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ChangeDifficulty(ChangeDifficulty {
                difficulty: Difficulty::Easy,
                locked: false,
            })
        );

        let payload = change_difficulty_payload(-1, false);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_CHANGE_DIFFICULTY, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ChangeDifficulty(ChangeDifficulty {
                difficulty: Difficulty::Hard,
                locked: false,
            })
        );
    }

    #[test]
    fn decodes_tab_list_header_footer() {
        let mut payload = Encoder::new();
        payload.write_bytes(&nbt_string_root("Online players"));
        payload.write_bytes(&nbt_string_root(""));
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_TAB_LIST, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::TabList(TabList {
                header: Some("Online players".to_string()),
                footer: None,
            })
        );

        let mut payload = Encoder::new();
        payload.write_bytes(&nbt_string_root(""));
        payload.write_bytes(&nbt_string_root("Welcome"));
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_TAB_LIST, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::TabList(TabList {
                header: None,
                footer: Some("Welcome".to_string()),
            })
        );
    }

    #[test]
    fn decodes_resource_pack_push_packet() {
        let pack_id = Uuid::from_u128(0x11111111_2222_3333_4444_555555555555);
        let mut payload = Encoder::new();
        payload.write_uuid(pack_id);
        payload.write_string("https://example.invalid/server-pack.zip");
        payload.write_string("0123456789abcdef0123456789abcdef01234567");
        payload.write_bool(true);
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Install this pack"));

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_RESOURCE_PACK_PUSH,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ResourcePackPush(ResourcePackPush {
                id: pack_id,
                url: "https://example.invalid/server-pack.zip".to_string(),
                hash: "0123456789abcdef0123456789abcdef01234567".to_string(),
                required: true,
                prompt: Some("Install this pack".to_string()),
            })
        );
    }

    #[test]
    fn decodes_resource_pack_pop_packet() {
        let pack_id = Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);
        let mut payload = Encoder::new();
        payload.write_bool(true);
        payload.write_uuid(pack_id);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_RESOURCE_PACK_POP,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ResourcePackPop(ResourcePackPop { id: Some(pack_id) })
        );
    }

    #[test]
    fn decodes_server_data_packet_with_icon() {
        let icon_bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let mut payload = Encoder::new();
        payload.write_bytes(&nbt_string_root("A native test server"));
        payload.write_bool(true);
        payload.write_var_i32(icon_bytes.len() as i32);
        payload.write_bytes(&icon_bytes);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SERVER_DATA, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ServerData(ServerData {
                motd: "A native test server".to_string(),
                icon_bytes: Some(icon_bytes),
            })
        );
    }

    #[test]
    fn encodes_resource_pack_response_packets() {
        let pack_id = Uuid::from_u128(0x22222222_3333_4444_5555_666666666666);
        let expected_ordinals = [
            (ResourcePackResponseAction::SuccessfullyLoaded, 0),
            (ResourcePackResponseAction::Declined, 1),
            (ResourcePackResponseAction::FailedDownload, 2),
            (ResourcePackResponseAction::Accepted, 3),
            (ResourcePackResponseAction::Downloaded, 4),
            (ResourcePackResponseAction::InvalidUrl, 5),
            (ResourcePackResponseAction::FailedReload, 6),
            (ResourcePackResponseAction::Discarded, 7),
        ];
        for (action, expected) in expected_ordinals {
            assert_eq!(action.ordinal(), expected);
        }

        let (id, payload) = encode_configuration_resource_pack_response(
            pack_id,
            ResourcePackResponseAction::Accepted,
        );
        assert_eq!(id, ids::configuration::SERVERBOUND_RESOURCE_PACK);
        assert_resource_pack_response_payload(
            &payload,
            pack_id,
            ResourcePackResponseAction::Accepted,
        );

        let (id, payload) =
            encode_play_resource_pack_response(pack_id, ResourcePackResponseAction::FailedReload);
        assert_eq!(id, ids::play::SERVERBOUND_RESOURCE_PACK);
        assert_eq!(id, 49);
        assert_resource_pack_response_payload(
            &payload,
            pack_id,
            ResourcePackResponseAction::FailedReload,
        );
    }

    #[test]
    fn decodes_initialize_border_packet() {
        let mut payload = Encoder::new();
        payload.write_f64(12.5);
        payload.write_f64(-30.25);
        payload.write_f64(60000000.0);
        payload.write_f64(300.0);
        payload.write_var_i64(1200);
        payload.write_var_i32(29999984);
        payload.write_var_i32(5);
        payload.write_var_i32(15);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_INITIALIZE_BORDER,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::InitializeBorder(InitializeBorder {
                new_center_x: 12.5,
                new_center_z: -30.25,
                old_size: 60000000.0,
                new_size: 300.0,
                lerp_time: 1200,
                new_absolute_max_size: 29999984,
                warning_blocks: 5,
                warning_time: 15,
            })
        );
    }

    #[test]
    fn decodes_set_border_center_packet() {
        let mut payload = Encoder::new();
        payload.write_f64(-7.75);
        payload.write_f64(42.125);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_BORDER_CENTER,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderCenter(SetBorderCenter {
                new_center_x: -7.75,
                new_center_z: 42.125,
            })
        );
    }

    #[test]
    fn decodes_set_border_lerp_size_packet() {
        let mut payload = Encoder::new();
        payload.write_f64(512.0);
        payload.write_f64(128.0);
        payload.write_var_i64(6000);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_BORDER_LERP_SIZE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderLerpSize(SetBorderLerpSize {
                old_size: 512.0,
                new_size: 128.0,
                lerp_time: 6000,
            })
        );
    }

    #[test]
    fn decodes_set_border_size_packet() {
        let mut payload = Encoder::new();
        payload.write_f64(2048.0);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_BORDER_SIZE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderSize(SetBorderSize { size: 2048.0 })
        );
    }

    #[test]
    fn decodes_set_border_warning_delay_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(30);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_BORDER_WARNING_DELAY,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderWarningDelay(SetBorderWarningDelay { warning_delay: 30 })
        );
    }

    #[test]
    fn decodes_set_border_warning_distance_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(8);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_BORDER_WARNING_DISTANCE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetBorderWarningDistance(SetBorderWarningDistance {
                warning_blocks: 8
            })
        );
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
        payload.write_var_i32(321);
        payload.write_var_i32(654);
        payload.write_var_i32(3);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_TAKE_ITEM_ENTITY,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::TakeItemEntity(TakeItemEntity {
                item_id: 321,
                player_id: 654,
                amount: 3,
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
        payload.write_u8(8);
        payload.write_var_i32(7);
        payload.write_var_i32(3);
        payload.write_var_i32(42);
        payload.write_var_i32(0);
        payload.write_var_i32(0);
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
                    EntityDataValue {
                        data_id: 8,
                        serializer_id: 7,
                        value: EntityDataValueKind::ItemStack(ItemStackSummary {
                            item_id: Some(42),
                            count: 3,
                            component_patch: Default::default(),
                        }),
                    },
                ],
            })
        );
    }

    #[test]
    fn decodes_command_suggestions_packet_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(17);
        payload.write_var_i32(1);
        payload.write_var_i32(5);
        payload.write_var_i32(2);
        payload.write_string("give");
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Run give"));
        payload.write_string("gamemode");
        payload.write_bool(false);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::CommandSuggestions(CommandSuggestions {
                id: 17,
                start: 1,
                length: 5,
                suggestions: vec![
                    CommandSuggestion {
                        text: "give".to_string(),
                        tooltip: Some("Run give".to_string()),
                    },
                    CommandSuggestion {
                        text: "gamemode".to_string(),
                        tooltip: None,
                    },
                ],
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 17);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 5);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert_eq!(decoder.read_string(32767).unwrap(), "give");
        assert!(decoder.read_bool().unwrap());
        assert_eq!(
            decode_component_summary_from_decoder(&mut decoder).unwrap(),
            "Run give"
        );
        assert_eq!(decoder.read_string(32767).unwrap(), "gamemode");
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_cooldown_packet_wire_order() {
        let mut payload = Encoder::new();
        payload.write_string("minecraft:ender_pearl");
        payload.write_var_i32(40);
        let payload = payload.into_inner();

        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_COOLDOWN, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::Cooldown(Cooldown {
                cooldown_group: "minecraft:ender_pearl".to_string(),
                duration: 40,
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:ender_pearl");
        assert_eq!(decoder.read_var_i32().unwrap(), 40);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_damage_event_without_source_position_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_var_i32(7);
        payload.write_var_i32(0);
        payload.write_var_i32(35);
        payload.write_bool(false);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_DAMAGE_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::DamageEvent(DamageEvent {
                entity_id: 123,
                source_type_id: 7,
                source_cause_id: -1,
                source_direct_id: 34,
                source_position: None,
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
        assert_eq!(decoder.read_var_i32().unwrap(), 7);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert_eq!(decoder.read_var_i32().unwrap(), 35);
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_damage_event_with_source_position_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(456);
        payload.write_var_i32(12);
        payload.write_var_i32(79);
        payload.write_var_i32(0);
        payload.write_bool(true);
        payload.write_f64(1.25);
        payload.write_f64(-2.5);
        payload.write_f64(64.0);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_DAMAGE_EVENT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::DamageEvent(DamageEvent {
                entity_id: 456,
                source_type_id: 12,
                source_cause_id: 78,
                source_direct_id: -1,
                source_position: Some(Vec3d {
                    x: 1.25,
                    y: -2.5,
                    z: 64.0,
                }),
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 456);
        assert_eq!(decoder.read_var_i32().unwrap(), 12);
        assert_eq!(decoder.read_var_i32().unwrap(), 79);
        assert_eq!(decoder.read_var_i32().unwrap(), 0);
        assert!(decoder.read_bool().unwrap());
        assert_eq!(decoder.read_f64().unwrap(), 1.25);
        assert_eq!(decoder.read_f64().unwrap(), -2.5);
        assert_eq!(decoder.read_f64().unwrap(), 64.0);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_update_mob_effect_packet_wire_order_and_flags() {
        let flags = MOB_EFFECT_FLAG_AMBIENT | MOB_EFFECT_FLAG_SHOW_ICON | MOB_EFFECT_FLAG_BLEND;
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_var_i32(5);
        payload.write_var_i32(2);
        payload.write_var_i32(600);
        payload.write_u8(flags);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_UPDATE_MOB_EFFECT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::UpdateMobEffect(UpdateMobEffect {
                entity_id: 123,
                effect_id: 5,
                amplifier: 2,
                duration_ticks: 600,
                flags: MobEffectFlags {
                    raw: flags,
                    ambient: true,
                    visible: false,
                    show_icon: true,
                    blend: true,
                },
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
        assert_eq!(decoder.read_var_i32().unwrap(), 5);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert_eq!(decoder.read_var_i32().unwrap(), 600);
        assert_eq!(decoder.read_u8().unwrap(), flags);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_remove_mob_effect_packet_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_var_i32(5);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_REMOVE_MOB_EFFECT, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::RemoveMobEffect(RemoveMobEffect {
                entity_id: 123,
                effect_id: 5,
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 123);
        assert_eq!(decoder.read_var_i32().unwrap(), 5);
        assert!(decoder.is_empty());
    }

    #[test]
    fn decodes_entity_transient_event_packets() {
        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_u8(3);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_ANIMATE, &payload.into_inner()).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::EntityAnimation(EntityAnimation { id: 123, action: 3 })
        );

        let mut payload = Encoder::new();
        payload.write_i32(123);
        payload.write_i8(35);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_ENTITY_EVENT, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 123,
                event_id: 35,
            })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(123);
        payload.write_f32(45.5);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_HURT_ANIMATION, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::HurtAnimation(HurtAnimation { id: 123, yaw: 45.5 })
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
    fn decodes_player_info_remove_uuid_list() {
        let first = Uuid::from_u128(0x11111111_2222_3333_4444_555555555555);
        let second = Uuid::from_u128(0xaaaaaaaa_bbbb_cccc_dddd_eeeeeeeeeeee);
        let mut payload = Encoder::new();
        payload.write_var_i32(2);
        payload.write_uuid(first);
        payload.write_uuid(second);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_PLAYER_INFO_REMOVE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::PlayerInfoRemove(PlayerInfoRemove {
                profile_ids: vec![first, second],
            })
        );
    }

    #[test]
    fn decodes_player_info_update_actions_and_signed_property() {
        let profile_id = Uuid::from_u128(0x12345678_1234_5678_90ab_cdef12345678);
        let actions = vec![
            PlayerInfoAction::AddPlayer,
            PlayerInfoAction::UpdateGameMode,
            PlayerInfoAction::UpdateListed,
            PlayerInfoAction::UpdateLatency,
            PlayerInfoAction::UpdateDisplayName,
            PlayerInfoAction::UpdateListOrder,
            PlayerInfoAction::UpdateHat,
        ];

        let mut payload = Encoder::new();
        payload.write_u8(player_info_actions_bits(&actions));
        payload.write_var_i32(1);
        payload.write_uuid(profile_id);
        payload.write_string("Steve");
        payload.write_var_i32(1);
        payload.write_string("textures");
        payload.write_string("texture-value");
        payload.write_bool(true);
        payload.write_string("texture-signature");
        payload.write_var_i32(GameType::Adventure.id());
        payload.write_bool(true);
        payload.write_var_i32(47);
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Captain Steve"));
        payload.write_var_i32(12);
        payload.write_bool(true);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::PlayerInfoUpdate(PlayerInfoUpdate {
                actions,
                entries: vec![PlayerInfoEntry {
                    profile_id,
                    profile: Some(GameProfile {
                        uuid: profile_id,
                        name: "Steve".to_string(),
                        properties: vec![GameProfileProperty {
                            name: "textures".to_string(),
                            value: "texture-value".to_string(),
                            signature: Some("texture-signature".to_string()),
                        }],
                    }),
                    listed: true,
                    latency: 47,
                    game_mode: GameType::Adventure,
                    display_name: Some("Captain Steve".to_string()),
                    show_hat: true,
                    list_order: 12,
                    chat_session: None,
                }],
            })
        );
    }

    #[test]
    fn decodes_player_info_update_chat_session_null() {
        let profile_id = Uuid::from_u128(0x22222222_3333_4444_5555_666666666666);
        let actions = vec![PlayerInfoAction::InitializeChat];
        let mut payload = Encoder::new();
        payload.write_u8(player_info_actions_bits(&actions));
        payload.write_var_i32(1);
        payload.write_uuid(profile_id);
        payload.write_bool(false);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_PLAYER_INFO_UPDATE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::PlayerInfoUpdate(PlayerInfoUpdate {
                actions,
                entries: vec![PlayerInfoEntry::new(profile_id)],
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
    fn encodes_command_suggestion_request_packet() {
        let (id, payload) = encode_play_command_suggestion_request(CommandSuggestionRequest {
            id: 33,
            command: "/give @p minecraft:stone".to_string(),
        });

        assert_eq!(id, ids::play::SERVERBOUND_COMMAND_SUGGESTION);
        assert_eq!(id, 15);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 33);
        assert_eq!(
            decoder.read_string(32500).unwrap(),
            "/give @p minecraft:stone"
        );
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
        payload.write_i32(2001);
        payload.write_i64(encode_block_pos(34, 64, -45));
        payload.write_i32(9);
        payload.write_bool(true);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_LEVEL_EVENT, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::LevelEvent(LevelEvent {
                event_type: 2001,
                pos: BlockPos {
                    x: 34,
                    y: 64,
                    z: -45,
                },
                data: 9,
                global: true,
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
        assert_eq!(ids::play::CLIENTBOUND_AWARD_STATS, 3);
        assert_eq!(ids::play::CLIENTBOUND_BLOCK_CHANGED_ACK, 4);

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
        payload.write_var_i32(1234);
        payload.write_i64(encode_block_pos(34, -12, -45));
        payload.write_u8(7);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_BLOCK_DESTRUCTION,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BlockDestruction(BlockDestruction {
                id: 1234,
                pos: BlockPos {
                    x: 34,
                    y: -12,
                    z: -45,
                },
                progress: 7,
            })
        );

        let mut payload = Encoder::new();
        payload.write_i64(encode_block_pos(35, 64, -46));
        payload.write_u8(1);
        payload.write_u8(5);
        payload.write_var_i32(123);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_BLOCK_EVENT, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::BlockEvent(BlockEvent {
                pos: BlockPos {
                    x: 35,
                    y: 64,
                    z: -46,
                },
                b0: 1,
                b1: 5,
                block_id: 123,
            })
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
    fn decodes_scoreboard_display_objective_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_string("");

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_DISPLAY_OBJECTIVE,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetDisplayObjective(SetDisplayObjective {
                slot: ScoreboardDisplaySlot::Sidebar,
                objective_name: None,
            })
        );
    }

    #[test]
    fn decodes_scoreboard_set_objective_add_and_remove_packets() {
        let mut payload = Encoder::new();
        payload.write_string("kills");
        payload.write_i8(0);
        payload.write_bytes(&nbt_string_root("Kills"));
        payload.write_var_i32(1);
        payload.write_bool(true);
        payload.write_var_i32(0);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_OBJECTIVE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetObjective(SetObjective {
                objective_name: "kills".to_string(),
                method: SetObjectiveMethod::Add,
                parameters: Some(SetObjectiveParameters {
                    display_name: "Kills".to_string(),
                    render_type: ObjectiveRenderType::Hearts,
                    number_format: Some(vec![0]),
                }),
            })
        );

        let mut payload = Encoder::new();
        payload.write_string("kills");
        payload.write_i8(1);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_OBJECTIVE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetObjective(SetObjective {
                objective_name: "kills".to_string(),
                method: SetObjectiveMethod::Remove,
                parameters: None,
            })
        );
    }

    #[test]
    fn decodes_scoreboard_set_score_with_optional_display_and_number_format() {
        let mut payload = Encoder::new();
        payload.write_string("Steve");
        payload.write_string("kills");
        payload.write_var_i32(42);
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Forty two"));
        payload.write_bool(true);
        payload.write_var_i32(0);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_SCORE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetScore(SetScore {
                owner: "Steve".to_string(),
                objective_name: "kills".to_string(),
                score: 42,
                display: Some("Forty two".to_string()),
                number_format: Some(vec![0]),
            })
        );
    }

    #[test]
    fn decodes_scoreboard_reset_score_null_and_objective_packets() {
        let mut payload = Encoder::new();
        payload.write_string("Steve");
        payload.write_bool(false);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_RESET_SCORE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ResetScore(ResetScore {
                owner: "Steve".to_string(),
                objective_name: None,
            })
        );

        let mut payload = Encoder::new();
        payload.write_string("Alex");
        payload.write_bool(true);
        payload.write_string("kills");

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_RESET_SCORE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ResetScore(ResetScore {
                owner: "Alex".to_string(),
                objective_name: Some("kills".to_string()),
            })
        );
    }

    #[test]
    fn decodes_scoreboard_player_team_add_join_and_leave_packets() {
        let mut payload = Encoder::new();
        payload.write_string("red");
        payload.write_i8(0);
        payload.write_bytes(&nbt_string_root("Red Team"));
        payload.write_u8(0b11);
        payload.write_var_i32(2);
        payload.write_var_i32(3);
        payload.write_var_i32(12);
        payload.write_bytes(&nbt_string_root("[R]"));
        payload.write_bytes(&nbt_string_root("!"));
        payload.write_var_i32(2);
        payload.write_string("Steve");
        payload.write_string("Alex");

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_PLAYER_TEAM,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "red".to_string(),
                method: PlayerTeamMethod::Add,
                parameters: Some(PlayerTeamParameters {
                    display_name: "Red Team".to_string(),
                    options: 0b11,
                    nametag_visibility: TeamVisibility::HideForOtherTeams,
                    collision_rule: TeamCollisionRule::PushOwnTeam,
                    color: ChatFormatting::Red,
                    player_prefix: "[R]".to_string(),
                    player_suffix: "!".to_string(),
                }),
                players: vec!["Steve".to_string(), "Alex".to_string()],
            })
        );

        let mut payload = Encoder::new();
        payload.write_string("red");
        payload.write_i8(3);
        payload.write_var_i32(1);
        payload.write_string("Sam");

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_PLAYER_TEAM,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "red".to_string(),
                method: PlayerTeamMethod::Join,
                parameters: None,
                players: vec!["Sam".to_string()],
            })
        );

        let mut payload = Encoder::new();
        payload.write_string("red");
        payload.write_i8(4);
        payload.write_var_i32(1);
        payload.write_string("Sam");

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_PLAYER_TEAM,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "red".to_string(),
                method: PlayerTeamMethod::Leave,
                parameters: None,
                players: vec!["Sam".to_string()],
            })
        );
    }

    #[test]
    fn decodes_title_camera_and_ticking_packets() {
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_ACTION_BAR_TEXT,
            &nbt_string_root("Action"),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetActionBarText(SetActionBarText {
                content: "Action".to_string(),
            })
        );

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_TITLE_TEXT,
            &nbt_string_root("Title"),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetTitleText(SetTitleText {
                content: "Title".to_string(),
            })
        );

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_SUBTITLE_TEXT,
            &nbt_string_root("Subtitle"),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetSubtitleText(SetSubtitleText {
                content: "Subtitle".to_string(),
            })
        );

        let mut payload = Encoder::new();
        payload.write_i32(10);
        payload.write_i32(70);
        payload.write_i32(-5);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_TITLES_ANIMATION,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetTitlesAnimation(SetTitlesAnimation {
                fade_in: 10,
                stay: 70,
                fade_out: -5,
            })
        );

        let mut payload = Encoder::new();
        payload.write_f32(0.25);
        payload.write_bool(true);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_TICKING_STATE, &payload.into_inner())
                .unwrap();
        let PlayClientbound::TickingState(ticking_state) = packet else {
            panic!("wrong packet");
        };
        assert_eq!(
            ticking_state,
            TickingState {
                tick_rate: 0.25,
                frozen: true,
            }
        );
        assert_eq!(ticking_state.clamped_tick_rate(), 1.0);
        assert_eq!(
            TickingState {
                tick_rate: 2.5,
                frozen: false,
            }
            .clamped_tick_rate(),
            2.5
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(40);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_TICKING_STEP, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::TickingStep(TickingStep { tick_steps: 40 })
        );

        let mut payload = Encoder::new();
        payload.write_var_i32(12345);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_CAMERA, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetCamera(SetCamera { camera_id: 12345 })
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

    fn boss_event_payload(
        id: Uuid,
        operation: i32,
        write_body: impl FnOnce(&mut Encoder),
    ) -> Vec<u8> {
        let mut payload = Encoder::new();
        payload.write_uuid(id);
        payload.write_var_i32(operation);
        write_body(&mut payload);
        payload.into_inner()
    }

    fn change_difficulty_payload(id: i32, locked: bool) -> Vec<u8> {
        let mut payload = Encoder::new();
        payload.write_var_i32(id);
        payload.write_bool(locked);
        payload.into_inner()
    }

    fn player_info_actions_bits(actions: &[PlayerInfoAction]) -> u8 {
        actions
            .iter()
            .fold(0, |bits, action| bits | (1u8 << action.ordinal()))
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

    fn assert_resource_pack_response_payload(
        payload: &[u8],
        expected_id: Uuid,
        expected_action: ResourcePackResponseAction,
    ) {
        let mut decoder = Decoder::new(payload);
        assert_eq!(decoder.read_uuid().unwrap(), expected_id);
        assert_eq!(decoder.read_var_i32().unwrap(), expected_action.ordinal());
        assert!(decoder.is_empty());
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
