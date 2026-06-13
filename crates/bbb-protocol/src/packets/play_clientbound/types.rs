use serde::{Deserialize, Serialize};

use super::super::*;

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
    ClearTitles(ClearTitles),
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
