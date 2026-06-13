use std::{net::ToSocketAddrs, time::Duration};

use anyhow::{anyhow, Result};
use bbb_protocol::{
    codec::offline_player_uuid,
    packets::{
        self, AddEntity, BlockChangedAck, BlockDestruction, BlockEntityData, BlockEvent,
        BlockUpdate, BossEvent, ChangeDifficulty, ChunksBiomes, ClearTitles,
        CommandSuggestionRequest, CommandSuggestions, ContainerClose, ContainerSetContent,
        ContainerSetData, ContainerSetSlot, Cooldown, CustomReportDetails, DamageEvent,
        EntityAnimation, EntityEvent, EntityMove, EntityPositionSync, ForgetLevelChunk, GameEvent,
        HurtAnimation, InitializeBorder, InteractionHand, LevelChunkWithLight, LevelEvent,
        LightUpdate, MoveVehicle, OpenScreen, PickItemFromBlock, PlayLogin, PlayTime,
        PlayerAbilities, PlayerAction, PlayerCommand, PlayerExperience, PlayerHealth,
        PlayerInfoRemove, PlayerInfoUpdate, PlayerInput, PlayerPositionState, PlayerPositionUpdate,
        PlayerRotationUpdate, RemoveEntities, RemoveMobEffect, ResetScore, ResourcePackPop,
        ResourcePackPush, Respawn, RotateHead, SectionBlocksUpdate, ServerData, ServerLinks,
        SetActionBarText, SetBorderCenter, SetBorderLerpSize, SetBorderSize, SetBorderWarningDelay,
        SetBorderWarningDistance, SetCamera, SetChunkCacheCenter, SetChunkCacheRadius,
        SetCursorItem, SetDefaultSpawnPosition, SetDisplayObjective, SetEntityData, SetEntityLink,
        SetEntityMotion, SetEquipment, SetHeldSlot, SetObjective, SetPassengers,
        SetPlayerInventory, SetPlayerTeam, SetScore, SetSimulationDistance, SetSubtitleText,
        SetTitleText, SetTitlesAnimation, SystemChat, TabList, TakeItemEntity, TeleportEntity,
        TickingState, TickingStep, Transfer, UpdateAttributes, UpdateMobEffect, UseItem, UseItemOn,
        Vec3d,
    },
};
use bbb_world::{BlockProbe, ChunkColumn, ChunkPos, ChunkState, WorldCounters, WorldStore};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionOptions {
    pub address: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub profile_id: Uuid,
    pub timeout: Duration,
}

impl ConnectionOptions {
    pub fn offline(address: impl Into<String>, username: impl Into<String>) -> Result<Self> {
        let address = address.into();
        let username = username.into();
        let (host, port) = split_host_port(&address)?;
        let profile_id = offline_player_uuid(&username);
        Ok(Self {
            address,
            host,
            port,
            username,
            profile_id,
            timeout: Duration::from_secs(20),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusPing {
    pub json: String,
    pub latency_ms: u128,
}

#[derive(Debug, Clone)]
pub enum NetEvent {
    Connected,
    Disconnected {
        reason: Option<String>,
    },
    StateChanged {
        state: ConnectionState,
    },
    CompressionSet {
        threshold: i32,
    },
    CookieRequest {
        key: String,
        response_payload_present: bool,
    },
    StoreCookie {
        key: String,
        payload_len: usize,
        stored_cookie_count: usize,
    },
    CustomReportDetails(CustomReportDetails),
    ServerLinks(ServerLinks),
    PacketSeen {
        state: ConnectionState,
        packet_id: i32,
        len: usize,
    },
    ContainerClose(ContainerClose),
    ContainerSetContent(ContainerSetContent),
    ContainerSetData(ContainerSetData),
    ContainerSetSlot(ContainerSetSlot),
    OpenScreen(OpenScreen),
    SetCursorItem(SetCursorItem),
    SetPlayerInventory(SetPlayerInventory),
    BlockDestruction(BlockDestruction),
    AddEntity(AddEntity),
    EntityAnimation(EntityAnimation),
    EntityEvent(EntityEvent),
    HurtAnimation(HurtAnimation),
    MoveEntity(EntityMove),
    MoveVehicle(MoveVehicle),
    EntityPositionSync(EntityPositionSync),
    RemoveEntities(RemoveEntities),
    RotateHead(RotateHead),
    SetEntityData(SetEntityData),
    SetEntityLink(SetEntityLink),
    SetEntityMotion(SetEntityMotion),
    SetEquipment(SetEquipment),
    TakeItemEntity(TakeItemEntity),
    SetPassengers(SetPassengers),
    UpdateAttributes(UpdateAttributes),
    TeleportEntity(TeleportEntity),
    RegistryData {
        registry: String,
        raw_payload_len: usize,
    },
    Login(PlayLogin),
    Respawn(Respawn),
    PlayerPosition(PlayerPositionUpdate),
    PlayerRotation(PlayerRotationUpdate),
    PlayerInfoUpdate(PlayerInfoUpdate),
    PlayerInfoRemove(PlayerInfoRemove),
    ServerData(ServerData),
    ResourcePackPush(ResourcePackPush),
    ResourcePackPop(ResourcePackPop),
    Cooldown(Cooldown),
    DamageEvent(DamageEvent),
    UpdateMobEffect(UpdateMobEffect),
    RemoveMobEffect(RemoveMobEffect),
    PlayerAbilities(PlayerAbilities),
    PlayerHealth(PlayerHealth),
    PlayerExperience(PlayerExperience),
    HeldSlot(SetHeldSlot),
    SetDefaultSpawnPosition(SetDefaultSpawnPosition),
    SetSimulationDistance(SetSimulationDistance),
    SystemChat(SystemChat),
    SetActionBarText(SetActionBarText),
    SetTitleText(SetTitleText),
    SetSubtitleText(SetSubtitleText),
    ClearTitles(ClearTitles),
    SetTitlesAnimation(SetTitlesAnimation),
    TickingState(TickingState),
    TickingStep(TickingStep),
    Transfer(Transfer),
    SetCamera(SetCamera),
    InitializeBorder(InitializeBorder),
    SetBorderCenter(SetBorderCenter),
    SetBorderLerpSize(SetBorderLerpSize),
    SetBorderSize(SetBorderSize),
    SetBorderWarningDelay(SetBorderWarningDelay),
    SetBorderWarningDistance(SetBorderWarningDistance),
    ResetScore(ResetScore),
    SetDisplayObjective(SetDisplayObjective),
    SetObjective(SetObjective),
    SetPlayerTeam(SetPlayerTeam),
    SetScore(SetScore),
    CommandSuggestions(CommandSuggestions),
    BossEvent(BossEvent),
    ChangeDifficulty(ChangeDifficulty),
    TabList(TabList),
    GameEvent(GameEvent),
    SetTime(PlayTime),
    BlockEntityData(BlockEntityData),
    BlockEvent(BlockEvent),
    LevelEvent(LevelEvent),
    BlockUpdate(BlockUpdate),
    SectionBlocksUpdate(SectionBlocksUpdate),
    SetChunkCacheCenter(SetChunkCacheCenter),
    SetChunkCacheRadius(SetChunkCacheRadius),
    ForgetLevelChunk(ForgetLevelChunk),
    LevelChunkWithLight(LevelChunkWithLight),
    LightUpdate(LightUpdate),
    ChunksBiomes(ChunksBiomes),
    BlockChangedAck(BlockChangedAck),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerMoveCommand {
    pub state: PlayerPositionState,
    pub on_ground: bool,
    pub horizontal_collision: bool,
}

impl PlayerMoveCommand {
    pub(crate) fn encode_packet(self) -> (i32, Vec<u8>) {
        packets::encode_play_move_player_pos_rot(
            self.state.position.x,
            self.state.position.y,
            self.state.position.z,
            self.state.y_rot,
            self.state.x_rot,
            self.on_ground,
            self.horizontal_collision,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleMoveCommand {
    pub position: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
}

impl VehicleMoveCommand {
    pub(crate) fn encode_packet(self) -> (i32, Vec<u8>) {
        packets::encode_play_move_vehicle(
            self.position.x,
            self.position.y,
            self.position.z,
            self.y_rot,
            self.x_rot,
            self.on_ground,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetCommand {
    MovePlayer(PlayerMoveCommand),
    MoveVehicle(VehicleMoveCommand),
    PlayerAction(PlayerAction),
    PlayerCommand(PlayerCommand),
    PlayerInput(PlayerInput),
    SetHeldSlot(u8),
    Swing(InteractionHand),
    UseItemOn(UseItemOn),
    UseItem(UseItem),
    PickItemFromBlock(PickItemFromBlock),
    CommandSuggestionRequest(CommandSuggestionRequest),
    Disconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeReport {
    pub reached_state: ConnectionState,
    pub compression_threshold: Option<i32>,
    pub packets_seen: usize,
    pub registries_seen: usize,
    pub first_chunk: Option<ChunkPos>,
    pub first_chunk_summary: Option<ChunkProbeSummary>,
    pub first_chunk_center_block: Option<BlockProbe>,
    pub world_counters: WorldCounters,
    #[serde(skip)]
    pub world: WorldStore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkProbeSummary {
    pub pos: ChunkPos,
    pub state: ChunkState,
    pub heightmaps: usize,
    pub sections: usize,
    pub block_entities: usize,
    pub sky_light_arrays: usize,
    pub block_light_arrays: usize,
}

impl ChunkProbeSummary {
    pub(crate) fn from_column(column: &ChunkColumn) -> Self {
        Self {
            pos: column.pos,
            state: column.state,
            heightmaps: column.heightmaps.len(),
            sections: column.sections.len(),
            block_entities: column.block_entities.len(),
            sky_light_arrays: column.light.sky_updates.len(),
            block_light_arrays: column.light.block_updates.len(),
        }
    }
}

pub(crate) fn split_host_port(address: &str) -> Result<(String, u16)> {
    if let Some((host, port)) = address.rsplit_once(':') {
        let port = port.parse::<u16>()?;
        return Ok((host.to_string(), port));
    }

    let mut addrs = (address, 25565).to_socket_addrs()?;
    let first = addrs
        .next()
        .ok_or_else(|| anyhow!("could not resolve {address}:25565"))?;
    Ok((address.to_string(), first.port()))
}
