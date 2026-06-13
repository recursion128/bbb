use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};

use bbb_protocol::{
    codec::{Decoder, ProtocolError},
    packets::{
        AddEntity as ProtocolAddEntity, AttributeSnapshot as ProtocolAttributeSnapshot,
        BlockDestruction as ProtocolBlockDestruction, BlockEntityData as ProtocolBlockEntityData,
        BlockEvent as ProtocolBlockEvent, BlockUpdate as ProtocolBlockUpdate,
        ChunksBiomes as ProtocolChunksBiomes, CommonPlayerSpawnInfo as ProtocolSpawnInfo,
        ContainerClose as ProtocolContainerClose,
        ContainerSetContent as ProtocolContainerSetContent,
        ContainerSetData as ProtocolContainerSetData, ContainerSetSlot as ProtocolContainerSetSlot,
        EntityAnimation as ProtocolEntityAnimation, EntityDataValue as ProtocolEntityDataValue,
        EntityDataValueKind, EntityEvent as ProtocolEntityEvent, EntityMove as ProtocolEntityMove,
        EntityPositionSync as ProtocolEntityPositionSync,
        EquipmentSlotUpdate as ProtocolEquipmentSlotUpdate, HurtAnimation as ProtocolHurtAnimation,
        ItemStackSummary as ProtocolItemStackSummary, LevelChunkWithLight,
        LevelEvent as ProtocolLevelEvent, LightUpdate as ProtocolLightUpdate,
        MoveVehicle as ProtocolMoveVehicle, OpenScreen as ProtocolOpenScreen,
        PlayLogin as ProtocolPlayLogin, RemoveEntities as ProtocolRemoveEntities,
        Respawn as ProtocolRespawn, RotateHead as ProtocolRotateHead,
        SectionBlocksUpdate as ProtocolSectionBlocksUpdate, SetCursorItem as ProtocolSetCursorItem,
        SetEntityData as ProtocolSetEntityData, SetEntityLink as ProtocolSetEntityLink,
        SetEntityMotion as ProtocolSetEntityMotion, SetEquipment as ProtocolSetEquipment,
        SetPassengers as ProtocolSetPassengers, SetPlayerInventory as ProtocolSetPlayerInventory,
        TakeItemEntity as ProtocolTakeItemEntity, TeleportEntity as ProtocolTeleportEntity,
        UpdateAttributes as ProtocolUpdateAttributes, Vec3d as ProtocolVec3d,
        PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_DELTA_Z,
        PLAYER_RELATIVE_ROTATE_DELTA, PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y,
        PLAYER_RELATIVE_Y_ROT, PLAYER_RELATIVE_Z,
    },
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const MAX_CHUNK_SECTION_BUFFER: usize = 2 * 1024 * 1024;
const LIGHT_ARRAY_BYTES: usize = 2048;
const VANILLA_BLOCK_STATES_JSON: &str = include_str!("../data/block_states_26_1.json");
const VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID: i32 = 49;
const VANILLA_ENTITY_TYPE_ITEM_ID: i32 = 71;
const VANILLA_ITEM_ENTITY_STACK_DATA_ID: u8 = 8;
const MOVE_VEHICLE_SNAP_EPSILON_SQUARED: f64 = 1e-10;

static VANILLA_BLOCK_STATES: OnceLock<Arc<Vec<Option<BlockStateInfo>>>> = OnceLock::new();

#[derive(Debug, Error)]
pub enum WorldDecodeError {
    #[error(transparent)]
    Protocol(#[from] ProtocolError),
    #[error("invalid paletted container bits_per_entry {0}")]
    InvalidPalettedBits(u8),
    #[error("chunk section buffer has {actual} bytes, max is {max}")]
    ChunkSectionBufferTooLarge { actual: usize, max: usize },
    #[error("byte array has {actual} bytes, max is {max}")]
    ByteArrayTooLarge { actual: usize, max: usize },
    #[error("biome update has {remaining} trailing bytes")]
    TrailingBiomeData { remaining: usize },
    #[error("block entity data has {remaining} trailing bytes")]
    TrailingBlockEntityData { remaining: usize },
    #[error("negative NBT array length {0}")]
    NegativeNbtArrayLength(i32),
    #[error("invalid NBT tag id {0}")]
    InvalidNbtTag(u8),
}

pub type Result<T> = std::result::Result<T, WorldDecodeError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldDimension {
    pub min_y: i32,
    pub height: i32,
}

impl Default for WorldDimension {
    fn default() -> Self {
        Self {
            min_y: -64,
            height: 384,
        }
    }
}

impl WorldDimension {
    pub fn min_section_y(self) -> i32 {
        self.min_y.div_euclid(16)
    }

    pub fn contains_y(self, y: i32) -> bool {
        y >= self.min_y && y < self.min_y + self.height
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldLevelInfo {
    pub dimension: String,
    pub dimension_type_id: i32,
    pub dimension_type_name: Option<String>,
    pub sea_level: i32,
    pub is_debug: bool,
    pub is_flat: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkState {
    Missing,
    Received,
    Decoded,
    NeighborsReady,
    MeshPending,
    MeshBuilding,
    MeshReady,
    GpuUploading,
    GpuResidentHidden,
    Visible,
    Retiring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkColumn {
    pub pos: ChunkPos,
    pub state: ChunkState,
    pub heightmaps: Vec<HeightmapData>,
    pub sections: Vec<ChunkSection>,
    pub block_entities: Vec<BlockEntityRecord>,
    pub light: LightData,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityState {
    pub id: i32,
    pub uuid: Uuid,
    pub entity_type_id: i32,
    pub data: i32,
    pub position: EntityVec3,
    pub position_base: EntityVec3,
    pub delta_movement: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub y_head_rot: f32,
    pub on_ground: Option<bool>,
    pub data_values: Vec<ProtocolEntityDataValue>,
    pub equipment: Vec<ProtocolEquipmentSlotUpdate>,
    pub attributes: Vec<ProtocolAttributeSnapshot>,
    pub vehicle_id: Option<i32>,
    pub passengers: Vec<i32>,
    pub leash_holder_id: Option<i32>,
    pub last_animation_action: Option<u8>,
    pub last_event_id: Option<i8>,
    pub last_hurt_yaw: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockDestructionProgress {
    pub id: i32,
    pub pos: BlockPos,
    pub progress: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEventRecord {
    pub pos: BlockPos,
    pub b0: u8,
    pub b1: u8,
    pub block_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelEventRecord {
    pub event_type: i32,
    pub pos: BlockPos,
    pub data: i32,
    pub global: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VehicleMoveReport {
    pub vehicle_id: i32,
    pub position: EntityVec3,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
    pub snapped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySlot {
    pub slot: i32,
    pub item: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerSlot {
    pub slot: i16,
    pub item: ProtocolItemStackSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerDataValue {
    pub id: i16,
    pub value: i16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerState {
    pub container_id: i32,
    pub menu_type_id: Option<i32>,
    pub title: Option<String>,
    pub state_id: i32,
    pub slots: Vec<ContainerSlot>,
    pub data_values: Vec<ContainerDataValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryState {
    pub player_slots: Vec<InventorySlot>,
    pub cursor_item: ProtocolItemStackSummary,
    pub open_container: Option<ContainerState>,
}

impl Default for InventoryState {
    fn default() -> Self {
        Self {
            player_slots: Vec::new(),
            cursor_item: ProtocolItemStackSummary::empty(),
            open_container: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeightmapData {
    pub kind_id: i32,
    pub data: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkSection {
    pub non_empty_block_count: i16,
    pub fluid_count: i16,
    pub block_states: PalettedContainerData,
    pub biomes: PalettedContainerData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteDomain {
    BlockStates,
    Biomes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteKind {
    SingleValue,
    Local,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PalettedContainerData {
    pub domain: PaletteDomain,
    pub bits_per_entry: u8,
    pub palette_kind: PaletteKind,
    pub palette_global_ids: Vec<i32>,
    pub packed_data: Vec<i64>,
    pub entry_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaletteValue {
    pub global_id: i32,
    pub palette_index: Option<usize>,
}

impl PalettedContainerData {
    pub fn value_at(&self, index: usize) -> Option<PaletteValue> {
        if index >= self.entry_count {
            return None;
        }

        match self.palette_kind {
            PaletteKind::SingleValue => Some(PaletteValue {
                global_id: self.palette_global_ids.first().copied()?,
                palette_index: Some(0),
            }),
            PaletteKind::Local => {
                let palette_index =
                    read_packed_value(&self.packed_data, self.bits_per_entry, index)?;
                let palette_index = usize::try_from(palette_index).ok()?;
                Some(PaletteValue {
                    global_id: *self.palette_global_ids.get(palette_index)?,
                    palette_index: Some(palette_index),
                })
            }
            PaletteKind::Global => Some(PaletteValue {
                global_id: read_packed_value(&self.packed_data, self.bits_per_entry, index)? as i32,
                palette_index: None,
            }),
        }
    }

    pub fn set_value_at(&mut self, index: usize, global_id: i32) -> bool {
        if index >= self.entry_count || global_id < 0 {
            return false;
        }

        match self.palette_kind {
            PaletteKind::SingleValue => {
                if self.palette_global_ids.first().copied() == Some(global_id) {
                    return true;
                }
                self.upgrade_to_global_with(index, global_id)
            }
            PaletteKind::Local => {
                if let Some(palette_index) = self
                    .palette_global_ids
                    .iter()
                    .position(|id| *id == global_id)
                {
                    set_packed_value(
                        &mut self.packed_data,
                        self.bits_per_entry,
                        index,
                        palette_index as u64,
                    )
                } else {
                    self.upgrade_to_global_with(index, global_id)
                }
            }
            PaletteKind::Global => set_packed_value(
                &mut self.packed_data,
                self.bits_per_entry,
                index,
                global_id as u64,
            ),
        }
    }

    fn upgrade_to_global_with(&mut self, index: usize, global_id: i32) -> bool {
        let mut values = Vec::with_capacity(self.entry_count);
        let mut max_value = global_id as u64;
        for entry_index in 0..self.entry_count {
            let Some(value) = self.value_at(entry_index) else {
                return false;
            };
            let Ok(global_id) = u64::try_from(value.global_id) else {
                return false;
            };
            max_value = max_value.max(global_id);
            values.push(global_id);
        }

        values[index] = global_id as u64;
        self.bits_per_entry = bits_needed(max_value);
        self.palette_kind = PaletteKind::Global;
        self.palette_global_ids.clear();
        self.packed_data = pack_values_to_longs(&values, self.bits_per_entry as usize);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockEntityRecord {
    pub local_x: u8,
    pub y: i16,
    pub local_z: u8,
    pub type_id: i32,
    pub nbt: Option<NbtPayloadSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NbtPayloadSummary {
    pub root_type: u8,
    pub byte_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LightData {
    pub sky_y_mask: Vec<i64>,
    pub block_y_mask: Vec<i64>,
    pub empty_sky_y_mask: Vec<i64>,
    pub empty_block_y_mask: Vec<i64>,
    pub sky_updates: Vec<Vec<u8>>,
    pub block_updates: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySet {
    pub registries: Vec<RegistryPacket>,
    #[serde(skip)]
    pub block_states: BlockStateRegistry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPacket {
    pub name: String,
    pub raw_payload_len: usize,
}

#[derive(Debug, Clone)]
pub struct BlockStateRegistry {
    states: Arc<Vec<Option<BlockStateInfo>>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockStateInfo {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct BlockStateReport {
    version: String,
    states: Vec<BlockStateInfo>,
}

impl RegistrySet {
    pub fn vanilla_26_1() -> Self {
        Self {
            registries: Vec::new(),
            block_states: BlockStateRegistry::vanilla_26_1(),
        }
    }

    pub fn block_state(&self, id: i32) -> Option<&BlockStateInfo> {
        self.block_states.by_id(id)
    }

    pub fn block_state_count(&self) -> usize {
        self.block_states.len()
    }
}

impl BlockStateRegistry {
    pub fn vanilla_26_1() -> Self {
        let states = VANILLA_BLOCK_STATES
            .get_or_init(|| Arc::new(load_vanilla_block_states()))
            .clone();
        Self { states }
    }

    pub fn by_id(&self, id: i32) -> Option<&BlockStateInfo> {
        let id = usize::try_from(id).ok()?;
        self.states.get(id)?.as_ref()
    }

    pub fn len(&self) -> usize {
        self.states.iter().filter(|state| state.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for BlockStateRegistry {
    fn default() -> Self {
        Self::vanilla_26_1()
    }
}

fn load_vanilla_block_states() -> Vec<Option<BlockStateInfo>> {
    let report: BlockStateReport = serde_json::from_str(VANILLA_BLOCK_STATES_JSON)
        .expect("embedded vanilla 26.1 block state registry is valid JSON");
    assert_eq!(
        report.version, "26.1",
        "embedded block state registry version must match protocol target"
    );

    let max_id = report
        .states
        .iter()
        .map(|state| state.id)
        .max()
        .expect("embedded block state registry is not empty");
    let mut states = vec![None; usize::try_from(max_id).expect("block state id is positive") + 1];
    for state in report.states {
        let index = usize::try_from(state.id).expect("block state id is positive");
        assert!(
            states[index].is_none(),
            "duplicate block state id {}",
            state.id
        );
        states[index] = Some(state);
    }
    states
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldCounters {
    pub registries_seen: usize,
    pub play_logins_received: usize,
    pub respawns_received: usize,
    pub chunks_received: usize,
    pub chunks_decoded: usize,
    pub sections_decoded: usize,
    pub block_entities_seen: usize,
    pub block_entity_updates_received: usize,
    pub block_entity_updates_applied: usize,
    pub light_arrays_seen: usize,
    pub light_updates_received: usize,
    pub light_updates_applied: usize,
    pub biome_updates_received: usize,
    pub biome_updates_applied: usize,
    pub block_updates_received: usize,
    pub block_updates_applied: usize,
    #[serde(default)]
    pub block_destructions_received: usize,
    #[serde(default)]
    pub block_destructions_tracked: usize,
    #[serde(default)]
    pub block_destructions_removed: usize,
    #[serde(default)]
    pub block_events_received: usize,
    #[serde(default)]
    pub block_events_tracked: usize,
    #[serde(default)]
    pub level_events_received: usize,
    #[serde(default)]
    pub level_events_tracked: usize,
    pub chunk_forgets_received: usize,
    pub chunks_forgotten: usize,
    pub inventory_slot_updates_received: usize,
    pub inventory_slots_tracked: usize,
    pub cursor_item_updates_received: usize,
    pub container_open_updates_received: usize,
    pub container_content_updates_received: usize,
    pub container_slot_updates_received: usize,
    pub container_data_updates_received: usize,
    pub container_close_updates_received: usize,
    pub entities_tracked: usize,
    pub entities_received: usize,
    pub entity_position_syncs_received: usize,
    pub entity_position_syncs_applied: usize,
    pub entity_moves_received: usize,
    pub entity_moves_applied: usize,
    pub entity_teleports_received: usize,
    pub entity_teleports_applied: usize,
    pub entity_animation_updates_received: usize,
    pub entity_animation_updates_applied: usize,
    pub entity_events_received: usize,
    pub entity_events_applied: usize,
    pub entity_hurt_animations_received: usize,
    pub entity_hurt_animations_applied: usize,
    pub entity_data_updates_received: usize,
    pub entity_data_values_received: usize,
    pub entity_data_updates_applied: usize,
    pub entity_equipment_updates_received: usize,
    pub entity_equipment_slots_received: usize,
    pub entity_equipment_updates_applied: usize,
    pub entity_attribute_updates_received: usize,
    pub entity_attributes_received: usize,
    pub entity_attribute_updates_applied: usize,
    pub entity_passenger_updates_received: usize,
    pub entity_passenger_ids_received: usize,
    pub entity_passenger_updates_applied: usize,
    #[serde(default)]
    pub vehicle_moves_received: usize,
    #[serde(default)]
    pub vehicle_moves_applied: usize,
    #[serde(default)]
    pub vehicle_moves_acked: usize,
    #[serde(default)]
    pub vehicle_moves_snapped: usize,
    pub entity_link_updates_received: usize,
    pub entity_link_updates_applied: usize,
    pub entity_motion_updates_received: usize,
    pub entity_motion_updates_applied: usize,
    pub entity_head_rotations_received: usize,
    pub entity_head_rotations_applied: usize,
    pub take_item_entities_received: usize,
    pub take_item_entities_applied: usize,
    pub item_entity_stack_shrinks: usize,
    pub take_item_entities_removed: usize,
    pub entity_removes_received: usize,
    pub entities_removed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockProbe {
    pub pos: BlockPos,
    pub chunk: ChunkPos,
    pub local_x: u8,
    pub local_y: u8,
    pub local_z: u8,
    pub section_y: i32,
    pub section_index: usize,
    pub block_state_id: i32,
    pub block_name: Option<String>,
    pub block_properties: BTreeMap<String, String>,
    pub material: TerrainMaterialClass,
    pub block_palette_kind: PaletteKind,
    pub block_palette_index: Option<usize>,
    pub biome_id: Option<i32>,
    pub biome_palette_kind: PaletteKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainMaterialClass {
    Empty,
    Opaque,
    Cutout,
    Fluid,
    Translucent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainBlockCell {
    pub block_state_id: i32,
    pub block_name: Option<String>,
    pub block_properties: BTreeMap<String, String>,
    pub biome_id: Option<i32>,
    pub material: TerrainMaterialClass,
    pub light: TerrainLight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainLight {
    pub sky: u8,
    pub block: u8,
}

impl TerrainLight {
    pub const FULL_BRIGHT: Self = Self { sky: 15, block: 0 };
    pub const DARK: Self = Self { sky: 0, block: 0 };

    fn clamp(self) -> Self {
        Self {
            sky: self.sky.min(15),
            block: self.block.min(15),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainChunkSnapshot {
    pub pos: ChunkPos,
    pub min_y: i32,
    pub height: usize,
    pub cells: Vec<TerrainBlockCell>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainChunkSummary {
    pub pos: Option<ChunkPos>,
    pub height: usize,
    pub total_blocks: usize,
    pub empty_blocks: usize,
    pub opaque_blocks: usize,
    pub cutout_blocks: usize,
    pub fluid_blocks: usize,
    pub translucent_blocks: usize,
}

impl TerrainChunkSnapshot {
    pub fn summary(&self) -> TerrainChunkSummary {
        let mut summary = TerrainChunkSummary {
            pos: Some(self.pos),
            height: self.height,
            total_blocks: self.cells.len(),
            ..TerrainChunkSummary::default()
        };
        for cell in &self.cells {
            match cell.material {
                TerrainMaterialClass::Empty => summary.empty_blocks += 1,
                TerrainMaterialClass::Opaque => summary.opaque_blocks += 1,
                TerrainMaterialClass::Cutout => summary.cutout_blocks += 1,
                TerrainMaterialClass::Fluid => summary.fluid_blocks += 1,
                TerrainMaterialClass::Translucent => summary.translucent_blocks += 1,
            }
        }
        summary
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldStore {
    dimension: WorldDimension,
    level: Option<WorldLevelInfo>,
    registries: RegistrySet,
    chunks: Vec<ChunkColumn>,
    #[serde(default)]
    block_destructions: Vec<BlockDestructionProgress>,
    #[serde(default)]
    block_events: Vec<BlockEventRecord>,
    #[serde(default)]
    level_events: Vec<LevelEventRecord>,
    entities: Vec<EntityState>,
    #[serde(default)]
    local_player_id: Option<i32>,
    #[serde(default)]
    local_player_vehicle_id: Option<i32>,
    inventory: InventoryState,
    counters: WorldCounters,
}

impl WorldStore {
    pub fn new() -> Self {
        Self {
            registries: RegistrySet::vanilla_26_1(),
            ..Self::default()
        }
    }

    pub fn with_dimension(dimension: WorldDimension) -> Self {
        Self {
            dimension,
            registries: RegistrySet::vanilla_26_1(),
            ..Self::default()
        }
    }

    pub fn record_registry(&mut self, name: impl Into<String>, raw_payload_len: usize) {
        self.registries.registries.push(RegistryPacket {
            name: name.into(),
            raw_payload_len,
        });
        self.counters.registries_seen = self.registries.registries.len();
    }

    pub fn apply_login(&mut self, login: &ProtocolPlayLogin) {
        self.counters.play_logins_received += 1;
        if let Some(local_player_id) = self.local_player_id {
            self.clear_local_player_mount(local_player_id);
        } else {
            self.local_player_vehicle_id = None;
        }
        self.local_player_id = Some(login.player_id);
        self.apply_spawn_info(&login.common_spawn_info);
    }

    pub fn apply_respawn(&mut self, respawn: &ProtocolRespawn) {
        self.counters.respawns_received += 1;
        self.apply_spawn_info(&respawn.common_spawn_info);
    }

    fn apply_spawn_info(&mut self, spawn_info: &ProtocolSpawnInfo) {
        let profile = dimension_profile(spawn_info.dimension_type_id, &spawn_info.dimension);
        let dimension_key_changed = self
            .level
            .as_ref()
            .is_some_and(|level| level.dimension != spawn_info.dimension);
        if self.dimension != profile.dimension || dimension_key_changed {
            self.chunks.clear();
            self.block_destructions.clear();
            self.block_events.clear();
            self.level_events.clear();
            self.entities.clear();
            self.counters.block_destructions_tracked = 0;
            self.counters.block_events_tracked = 0;
            self.counters.level_events_tracked = 0;
            self.update_entity_count();
        }
        self.dimension = profile.dimension;
        self.level = Some(WorldLevelInfo {
            dimension: spawn_info.dimension.clone(),
            dimension_type_id: spawn_info.dimension_type_id,
            dimension_type_name: profile.name.map(str::to_string),
            sea_level: spawn_info.sea_level,
            is_debug: spawn_info.is_debug,
            is_flat: spawn_info.is_flat,
        });
    }

    pub fn insert_level_chunk_with_light(
        &mut self,
        packet: LevelChunkWithLight,
    ) -> Result<ChunkPos> {
        let pos = ChunkPos {
            x: packet.x,
            z: packet.z,
        };
        let column = decode_level_chunk_with_light(pos, &packet.raw_after_position)?;
        self.insert_decoded_chunk(column);
        Ok(pos)
    }

    pub fn insert_decoded_chunk(&mut self, column: ChunkColumn) {
        let pos = column.pos;
        self.counters.chunks_received += 1;
        self.counters.chunks_decoded += 1;
        self.counters.sections_decoded += column.sections.len();
        self.counters.block_entities_seen += column.block_entities.len();
        self.counters.light_arrays_seen +=
            column.light.sky_updates.len() + column.light.block_updates.len();
        if let Some(existing) = self.chunks.iter_mut().find(|chunk| chunk.pos == pos) {
            *existing = column;
        } else {
            self.chunks.push(column);
        }
    }

    pub fn apply_block_update(&mut self, update: ProtocolBlockUpdate) -> bool {
        self.counters.block_updates_received += 1;
        let applied =
            self.set_block_state_id(protocol_block_pos(update.pos), update.block_state_id);
        if applied {
            self.counters.block_updates_applied += 1;
        }
        applied
    }

    pub fn apply_block_destruction(&mut self, update: ProtocolBlockDestruction) -> bool {
        self.counters.block_destructions_received += 1;
        if update.progress < 10 {
            let progress = BlockDestructionProgress {
                id: update.id,
                pos: protocol_block_pos(update.pos),
                progress: update.progress,
            };
            if let Some(existing) = self
                .block_destructions
                .iter_mut()
                .find(|existing| existing.id == update.id)
            {
                *existing = progress;
            } else {
                self.block_destructions.push(progress);
            }
            self.counters.block_destructions_tracked = self.block_destructions.len();
            return true;
        }

        let before = self.block_destructions.len();
        self.block_destructions
            .retain(|progress| progress.id != update.id);
        let removed = before - self.block_destructions.len();
        self.counters.block_destructions_removed += removed;
        self.counters.block_destructions_tracked = self.block_destructions.len();
        removed > 0
    }

    pub fn apply_block_event(&mut self, event: ProtocolBlockEvent) {
        self.counters.block_events_received += 1;
        self.block_events.push(BlockEventRecord {
            pos: protocol_block_pos(event.pos),
            b0: event.b0,
            b1: event.b1,
            block_id: event.block_id,
        });
        self.counters.block_events_tracked = self.block_events.len();
    }

    pub fn apply_section_blocks_update(&mut self, update: ProtocolSectionBlocksUpdate) -> usize {
        self.counters.block_updates_received += update.updates.len();
        let mut applied = 0;
        for block_update in update.updates {
            if self.set_block_state_id(
                protocol_block_pos(block_update.pos),
                block_update.block_state_id,
            ) {
                applied += 1;
            }
        }
        self.counters.block_updates_applied += applied;
        applied
    }

    pub fn apply_block_entity_data(&mut self, packet: ProtocolBlockEntityData) -> Result<bool> {
        self.counters.block_entity_updates_received += 1;
        let pos = protocol_block_pos(packet.pos);
        let y = i16::try_from(pos.y).map_err(|_| {
            ProtocolError::InvalidData(format!("block entity y {} is out of i16 range", pos.y))
        })?;
        let mut decoder = Decoder::new(&packet.raw_nbt);
        let nbt = skip_nbt_any(&mut decoder)?;
        if !decoder.is_empty() {
            return Err(WorldDecodeError::TrailingBlockEntityData {
                remaining: decoder.remaining_len(),
            });
        }

        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let record = BlockEntityRecord {
            local_x: pos.x.rem_euclid(16) as u8,
            y,
            local_z: pos.z.rem_euclid(16) as u8,
            type_id: packet.block_entity_type_id,
            nbt,
        };
        let Some(chunk) = self.chunks.iter_mut().find(|chunk| chunk.pos == chunk_pos) else {
            return Ok(false);
        };
        if let Some(existing) = chunk.block_entities.iter_mut().find(|entity| {
            entity.local_x == record.local_x
                && entity.y == record.y
                && entity.local_z == record.local_z
        }) {
            *existing = record;
        } else {
            chunk.block_entities.push(record);
        }
        self.counters.block_entity_updates_applied += 1;
        Ok(true)
    }

    pub fn apply_light_update(&mut self, update: ProtocolLightUpdate) -> Result<bool> {
        self.counters.light_updates_received += 1;
        let mut decoder = Decoder::new(&update.raw_light_data);
        let update_light = decode_light_data(&mut decoder)?;
        let pos = ChunkPos {
            x: update.chunk_x,
            z: update.chunk_z,
        };
        let Some(chunk) = self.chunks.iter_mut().find(|chunk| chunk.pos == pos) else {
            return Ok(false);
        };

        merge_light_data(&mut chunk.light, update_light);
        self.counters.light_updates_applied += 1;
        Ok(true)
    }

    pub fn apply_biome_update(&mut self, update: ProtocolChunksBiomes) -> Result<usize> {
        self.counters.biome_updates_received += update.chunks.len();
        let mut replacements = Vec::new();
        for chunk_update in update.chunks {
            let pos = ChunkPos {
                x: chunk_update.pos.x,
                z: chunk_update.pos.z,
            };
            let Some(chunk_index) = self.chunks.iter().position(|chunk| chunk.pos == pos) else {
                continue;
            };
            let section_count = self.chunks[chunk_index].sections.len();
            let biomes = decode_biome_sections(&chunk_update.raw_biomes, section_count)?;
            replacements.push((chunk_index, biomes));
        }

        let applied = replacements.len();
        for (chunk_index, biomes) in replacements {
            if let Some(chunk) = self.chunks.get_mut(chunk_index) {
                for (section, biomes) in chunk.sections.iter_mut().zip(biomes) {
                    section.biomes = biomes;
                }
            }
        }
        self.counters.biome_updates_applied += applied;
        Ok(applied)
    }

    pub fn apply_level_event(&mut self, event: ProtocolLevelEvent) {
        self.counters.level_events_received += 1;
        self.level_events.push(LevelEventRecord {
            event_type: event.event_type,
            pos: protocol_block_pos(event.pos),
            data: event.data,
            global: event.global,
        });
        self.counters.level_events_tracked = self.level_events.len();
    }

    pub fn apply_set_player_inventory(&mut self, packet: ProtocolSetPlayerInventory) {
        self.counters.inventory_slot_updates_received += 1;
        set_inventory_slot(
            &mut self.inventory.player_slots,
            InventorySlot {
                slot: packet.slot,
                item: packet.item,
            },
        );
        self.update_inventory_slot_count();
    }

    pub fn apply_set_cursor_item(&mut self, packet: ProtocolSetCursorItem) {
        self.counters.cursor_item_updates_received += 1;
        self.inventory.cursor_item = packet.item;
    }

    pub fn apply_open_screen(&mut self, packet: ProtocolOpenScreen) {
        self.counters.container_open_updates_received += 1;
        let existing = self
            .inventory
            .open_container
            .take()
            .filter(|container| container.container_id == packet.container_id)
            .unwrap_or_else(|| ContainerState {
                container_id: packet.container_id,
                ..ContainerState::default()
            });
        self.inventory.open_container = Some(ContainerState {
            container_id: packet.container_id,
            menu_type_id: Some(packet.menu_type_id),
            title: Some(packet.title),
            state_id: existing.state_id,
            slots: existing.slots,
            data_values: existing.data_values,
        });
    }

    pub fn apply_container_set_content(&mut self, packet: ProtocolContainerSetContent) {
        self.counters.container_content_updates_received += 1;
        self.inventory.cursor_item = packet.carried_item;
        let existing = self
            .inventory
            .open_container
            .take()
            .filter(|container| container.container_id == packet.container_id);
        self.inventory.open_container = Some(ContainerState {
            container_id: packet.container_id,
            menu_type_id: existing
                .as_ref()
                .and_then(|container| container.menu_type_id),
            title: existing
                .as_ref()
                .and_then(|container| container.title.clone()),
            state_id: packet.state_id,
            slots: packet
                .items
                .into_iter()
                .enumerate()
                .map(|(slot, item)| ContainerSlot {
                    slot: slot as i16,
                    item,
                })
                .collect(),
            data_values: existing
                .map(|container| container.data_values)
                .unwrap_or_default(),
        });
    }

    pub fn apply_container_set_slot(&mut self, packet: ProtocolContainerSetSlot) {
        self.counters.container_slot_updates_received += 1;
        let container = self.ensure_container(packet.container_id);
        container.state_id = packet.state_id;
        set_container_slot(
            &mut container.slots,
            ContainerSlot {
                slot: packet.slot,
                item: packet.item,
            },
        );
    }

    pub fn apply_container_set_data(&mut self, packet: ProtocolContainerSetData) {
        self.counters.container_data_updates_received += 1;
        let container = self.ensure_container(packet.container_id);
        if let Some(existing) = container
            .data_values
            .iter_mut()
            .find(|value| value.id == packet.id)
        {
            *existing = ContainerDataValue {
                id: packet.id,
                value: packet.value,
            };
        } else {
            container.data_values.push(ContainerDataValue {
                id: packet.id,
                value: packet.value,
            });
        }
        container.data_values.sort_by_key(|value| value.id);
    }

    pub fn apply_container_close(&mut self, packet: ProtocolContainerClose) -> bool {
        self.counters.container_close_updates_received += 1;
        if self
            .inventory
            .open_container
            .as_ref()
            .is_some_and(|container| container.container_id == packet.container_id)
        {
            self.inventory.open_container = None;
            true
        } else {
            false
        }
    }

    pub fn apply_add_entity(&mut self, packet: ProtocolAddEntity) {
        self.counters.entities_received += 1;
        let entity = EntityState {
            id: packet.id,
            uuid: packet.uuid,
            entity_type_id: packet.entity_type_id,
            data: packet.data,
            position: entity_vec3(packet.position),
            position_base: entity_vec3(packet.position),
            delta_movement: entity_vec3(packet.delta_movement),
            y_rot: packet.y_rot,
            x_rot: packet.x_rot,
            y_head_rot: packet.y_head_rot,
            on_ground: None,
            data_values: Vec::new(),
            equipment: Vec::new(),
            attributes: Vec::new(),
            vehicle_id: None,
            passengers: Vec::new(),
            leash_holder_id: None,
            last_animation_action: None,
            last_event_id: None,
            last_hurt_yaw: None,
        };

        if let Some(existing) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        {
            *existing = entity;
        } else {
            self.entities.push(entity);
        }
        self.update_entity_count();
    }

    pub fn apply_entity_animation(&mut self, packet: ProtocolEntityAnimation) -> bool {
        self.counters.entity_animation_updates_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.last_animation_action = Some(packet.action);
        self.counters.entity_animation_updates_applied += 1;
        true
    }

    pub fn apply_entity_event(&mut self, packet: ProtocolEntityEvent) -> bool {
        self.counters.entity_events_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.entity_id)
        else {
            return false;
        };

        entity.last_event_id = Some(packet.event_id);
        self.counters.entity_events_applied += 1;
        true
    }

    pub fn apply_hurt_animation(&mut self, packet: ProtocolHurtAnimation) -> bool {
        self.counters.entity_hurt_animations_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.last_hurt_yaw = Some(packet.yaw);
        self.counters.entity_hurt_animations_applied += 1;
        true
    }

    pub fn apply_entity_position_sync(&mut self, packet: ProtocolEntityPositionSync) -> bool {
        self.counters.entity_position_syncs_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.position = entity_vec3(packet.position);
        entity.position_base = entity_vec3(packet.position);
        entity.delta_movement = entity_vec3(packet.delta_movement);
        entity.y_rot = packet.y_rot;
        entity.x_rot = packet.x_rot;
        entity.on_ground = Some(packet.on_ground);
        self.counters.entity_position_syncs_applied += 1;
        true
    }

    pub fn apply_entity_move(&mut self, packet: ProtocolEntityMove) -> bool {
        self.counters.entity_moves_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        if packet.delta_x != 0 || packet.delta_y != 0 || packet.delta_z != 0 {
            let position = decode_entity_delta_position(
                entity.position_base,
                packet.delta_x,
                packet.delta_y,
                packet.delta_z,
            );
            entity.position = position;
            entity.position_base = position;
        }
        if let Some(y_rot) = packet.y_rot {
            entity.y_rot = y_rot;
        }
        if let Some(x_rot) = packet.x_rot {
            entity.x_rot = x_rot;
        }
        entity.on_ground = Some(packet.on_ground);
        self.counters.entity_moves_applied += 1;
        true
    }

    pub fn apply_teleport_entity(&mut self, packet: ProtocolTeleportEntity) -> bool {
        self.counters.entity_teleports_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        let absolute = entity_absolute_move_rotation(
            entity.position,
            entity.delta_movement,
            entity.y_rot,
            entity.x_rot,
            packet.position,
            packet.delta_movement,
            packet.y_rot,
            packet.x_rot,
            packet.relatives_mask,
        );
        entity.position = absolute.position;
        entity.delta_movement = absolute.delta_movement;
        entity.y_rot = absolute.y_rot;
        entity.x_rot = absolute.x_rot;
        entity.on_ground = Some(packet.on_ground);
        self.counters.entity_teleports_applied += 1;
        true
    }

    pub fn apply_set_entity_data(&mut self, packet: ProtocolSetEntityData) -> bool {
        self.counters.entity_data_updates_received += 1;
        self.counters.entity_data_values_received += packet.values.len();
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        for value in packet.values {
            if let Some(existing) = entity
                .data_values
                .iter_mut()
                .find(|existing| existing.data_id == value.data_id)
            {
                *existing = value;
            } else {
                entity.data_values.push(value);
            }
        }
        entity.data_values.sort_by_key(|value| value.data_id);
        self.counters.entity_data_updates_applied += 1;
        true
    }

    pub fn apply_set_equipment(&mut self, packet: ProtocolSetEquipment) -> bool {
        self.counters.entity_equipment_updates_received += 1;
        self.counters.entity_equipment_slots_received += packet.slots.len();
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.entity_id)
        else {
            return false;
        };

        for update in packet.slots {
            if let Some(existing) = entity
                .equipment
                .iter_mut()
                .find(|existing| existing.slot == update.slot)
            {
                *existing = update;
            } else {
                entity.equipment.push(update);
            }
        }
        entity.equipment.sort_by_key(|update| update.slot.ordinal());
        self.counters.entity_equipment_updates_applied += 1;
        true
    }

    pub fn apply_update_attributes(&mut self, packet: ProtocolUpdateAttributes) -> bool {
        self.counters.entity_attribute_updates_received += 1;
        self.counters.entity_attributes_received += packet.attributes.len();
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.entity_id)
        else {
            return false;
        };

        for attribute in packet.attributes {
            if let Some(existing) = entity
                .attributes
                .iter_mut()
                .find(|existing| existing.attribute_id == attribute.attribute_id)
            {
                *existing = attribute;
            } else {
                entity.attributes.push(attribute);
            }
        }
        entity
            .attributes
            .sort_by_key(|attribute| attribute.attribute_id);
        self.counters.entity_attribute_updates_applied += 1;
        true
    }

    pub fn apply_set_passengers(&mut self, packet: ProtocolSetPassengers) -> bool {
        self.counters.entity_passenger_updates_received += 1;
        self.counters.entity_passenger_ids_received += packet.passenger_ids.len();
        let local_player_id = self.local_player_id;
        let local_player_was_on_packet_vehicle =
            self.local_player_vehicle_id == Some(packet.vehicle_id);
        let Some(vehicle_index) = self
            .entities
            .iter()
            .position(|entity| entity.id == packet.vehicle_id)
        else {
            return false;
        };

        for entity in &mut self.entities {
            if entity.vehicle_id == Some(packet.vehicle_id) {
                entity.vehicle_id = None;
            }
        }
        self.entities[vehicle_index].passengers.clear();

        let mut mounted = Vec::new();
        let mut local_player_mounted_here = false;
        for passenger_id in packet.passenger_ids {
            if passenger_id == packet.vehicle_id || mounted.contains(&passenger_id) {
                continue;
            }
            let is_local_player = local_player_id == Some(passenger_id);
            if is_local_player {
                if let Some(old_vehicle_id) = self.local_player_vehicle_id {
                    if old_vehicle_id != packet.vehicle_id {
                        self.remove_passenger_from_vehicle(old_vehicle_id, passenger_id);
                    }
                }
                self.local_player_vehicle_id = Some(packet.vehicle_id);
                local_player_mounted_here = true;
            }
            let passenger_index = self
                .entities
                .iter()
                .position(|entity| entity.id == passenger_id);
            let Some(passenger_index) = passenger_index else {
                if is_local_player {
                    mounted.push(passenger_id);
                }
                continue;
            };
            if let Some(old_vehicle_id) = self.entities[passenger_index].vehicle_id {
                if let Some(old_vehicle) = self
                    .entities
                    .iter_mut()
                    .find(|entity| entity.id == old_vehicle_id)
                {
                    old_vehicle
                        .passengers
                        .retain(|existing| *existing != passenger_id);
                }
            }
            self.entities[passenger_index].vehicle_id = Some(packet.vehicle_id);
            mounted.push(passenger_id);
        }

        if local_player_was_on_packet_vehicle && !local_player_mounted_here {
            self.local_player_vehicle_id = None;
        }
        self.entities[vehicle_index].passengers = mounted;
        self.counters.entity_passenger_updates_applied += 1;
        true
    }

    pub fn apply_move_vehicle(&mut self, packet: ProtocolMoveVehicle) -> Option<VehicleMoveReport> {
        self.counters.vehicle_moves_received += 1;
        let root_vehicle_id = self.local_player_root_vehicle_id()?;
        let root_vehicle_index = self
            .entities
            .iter()
            .position(|entity| entity.id == root_vehicle_id)?;
        let packet_position = entity_vec3(packet.position);
        let snapped =
            entity_distance_squared(self.entities[root_vehicle_index].position, packet_position)
                > MOVE_VEHICLE_SNAP_EPSILON_SQUARED;

        if snapped {
            let vehicle = &mut self.entities[root_vehicle_index];
            vehicle.position = packet_position;
            vehicle.position_base = packet_position;
            vehicle.y_rot = packet.y_rot;
            vehicle.x_rot = packet.x_rot;
            self.counters.vehicle_moves_snapped += 1;
        }

        self.counters.vehicle_moves_applied += 1;
        self.counters.vehicle_moves_acked += 1;
        let vehicle = &self.entities[root_vehicle_index];
        Some(VehicleMoveReport {
            vehicle_id: vehicle.id,
            position: vehicle.position,
            y_rot: vehicle.y_rot,
            x_rot: vehicle.x_rot,
            on_ground: vehicle.on_ground.unwrap_or(false),
            snapped,
        })
    }

    pub fn apply_set_entity_link(&mut self, packet: ProtocolSetEntityLink) -> bool {
        self.counters.entity_link_updates_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.source_id)
        else {
            return false;
        };

        entity.leash_holder_id = if packet.dest_id == 0 {
            None
        } else {
            Some(packet.dest_id)
        };
        self.counters.entity_link_updates_applied += 1;
        true
    }

    pub fn apply_set_entity_motion(&mut self, packet: ProtocolSetEntityMotion) -> bool {
        self.counters.entity_motion_updates_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.delta_movement = entity_vec3(packet.delta_movement);
        self.counters.entity_motion_updates_applied += 1;
        true
    }

    pub fn apply_rotate_head(&mut self, packet: ProtocolRotateHead) -> bool {
        self.counters.entity_head_rotations_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.y_head_rot = packet.y_head_rot;
        self.counters.entity_head_rotations_applied += 1;
        true
    }

    pub fn apply_take_item_entity(&mut self, packet: ProtocolTakeItemEntity) -> bool {
        self.counters.take_item_entities_received += 1;
        let Some(entity_index) = self
            .entities
            .iter()
            .position(|entity| entity.id == packet.item_id)
        else {
            return false;
        };

        self.counters.take_item_entities_applied += 1;
        let entity_type_id = self.entities[entity_index].entity_type_id;
        if entity_type_id == VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID {
            return true;
        }

        if entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID {
            if let Some(stack) = item_entity_stack_mut(&mut self.entities[entity_index]) {
                if stack.count > 0 && packet.amount > 0 {
                    stack.count = stack.count.saturating_sub(packet.amount).max(0);
                    self.counters.item_entity_stack_shrinks += 1;
                }
                if stack.count > 0 {
                    return true;
                }
            }
        }

        let removed = self.remove_entities_by_ids(&[packet.item_id]);
        self.counters.take_item_entities_removed += removed;
        true
    }

    pub fn apply_remove_entities(&mut self, packet: ProtocolRemoveEntities) -> usize {
        self.counters.entity_removes_received += packet.entity_ids.len();
        self.remove_entities_by_ids(&packet.entity_ids)
    }

    fn remove_entities_by_ids(&mut self, removed_ids: &[i32]) -> usize {
        let before = self.entities.len();
        self.entities
            .retain(|entity| !removed_ids.contains(&entity.id));
        let removed = before - self.entities.len();
        if self
            .local_player_vehicle_id
            .is_some_and(|vehicle_id| removed_ids.contains(&vehicle_id))
        {
            self.local_player_vehicle_id = None;
        }
        for entity in &mut self.entities {
            if entity
                .vehicle_id
                .is_some_and(|vehicle_id| removed_ids.contains(&vehicle_id))
            {
                entity.vehicle_id = None;
            }
            if entity
                .leash_holder_id
                .is_some_and(|holder_id| removed_ids.contains(&holder_id))
            {
                entity.leash_holder_id = None;
            }
            entity
                .passengers
                .retain(|passenger_id| !removed_ids.contains(passenger_id));
        }
        self.counters.entities_removed += removed;
        self.update_entity_count();
        removed
    }

    pub fn forget_chunk(&mut self, pos: ChunkPos) -> bool {
        self.counters.chunk_forgets_received += 1;
        let Some(index) = self.chunks.iter().position(|chunk| chunk.pos == pos) else {
            return false;
        };
        self.chunks.remove(index);
        self.counters.chunks_forgotten += 1;
        true
    }

    pub fn probe_chunk(&self, pos: ChunkPos) -> Option<&ChunkColumn> {
        self.chunks.iter().find(|chunk| chunk.pos == pos)
    }

    pub fn probe_entity(&self, id: i32) -> Option<&EntityState> {
        self.entities.iter().find(|entity| entity.id == id)
    }

    pub fn probe_block(&self, pos: BlockPos) -> Option<BlockProbe> {
        if !self.dimension.contains_y(pos.y) {
            return None;
        }

        let chunk = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let local_x = pos.x.rem_euclid(16) as u8;
        let local_y = pos.y.rem_euclid(16) as u8;
        let local_z = pos.z.rem_euclid(16) as u8;
        let section_y = pos.y.div_euclid(16);
        let section_index = usize::try_from(section_y - self.dimension.min_section_y()).ok()?;
        let section = self.probe_chunk(chunk)?.sections.get(section_index)?;

        let block_index = section_block_index(local_x, local_y, local_z);
        let block_value = section.block_states.value_at(block_index)?;
        let block_state = self.registries.block_state(block_value.global_id);
        let block_name = block_state.map(|state| state.name.clone());
        let biome_index = section_biome_index(local_x / 4, local_y / 4, local_z / 4);
        let biome_value = section.biomes.value_at(biome_index);

        Some(BlockProbe {
            pos,
            chunk,
            local_x,
            local_y,
            local_z,
            section_y,
            section_index,
            block_state_id: block_value.global_id,
            material: classify_terrain_material(block_name.as_deref()),
            block_name,
            block_properties: block_state
                .map(|state| state.properties.clone())
                .unwrap_or_default(),
            block_palette_kind: section.block_states.palette_kind,
            block_palette_index: block_value.palette_index,
            biome_id: biome_value.map(|value| value.global_id),
            biome_palette_kind: section.biomes.palette_kind,
        })
    }

    pub fn extract_terrain_chunk(&self, pos: ChunkPos) -> Option<TerrainChunkSnapshot> {
        let chunk = self.probe_chunk(pos)?;

        let height = usize::try_from(self.dimension.height).ok()?;
        let mut cells = Vec::with_capacity(16 * height * 16);
        for y_offset in 0..height {
            let y = self.dimension.min_y + y_offset as i32;
            let section_y = y.div_euclid(16);
            let section_index = usize::try_from(section_y - self.dimension.min_section_y()).ok()?;
            let section = chunk.sections.get(section_index)?;
            let local_y = y.rem_euclid(16) as u8;
            for z in 0..16 {
                for x in 0..16 {
                    let block_index = section_block_index(x as u8, local_y, z as u8);
                    let block_value = section.block_states.value_at(block_index)?;
                    let block_state = self.registries.block_state(block_value.global_id);
                    let block_name = block_state.map(|state| state.name.clone());
                    let block_properties = block_state
                        .map(|state| state.properties.clone())
                        .unwrap_or_default();
                    let biome_index = section_biome_index(x as u8 / 4, local_y / 4, z as u8 / 4);
                    let biome_id = section
                        .biomes
                        .value_at(biome_index)
                        .map(|value| value.global_id);
                    cells.push(TerrainBlockCell {
                        block_state_id: block_value.global_id,
                        biome_id,
                        material: classify_terrain_material(block_name.as_deref()),
                        block_name,
                        block_properties,
                        light: sample_terrain_light(&chunk.light, self.dimension, x, y, z),
                    });
                }
            }
        }

        Some(TerrainChunkSnapshot {
            pos,
            min_y: self.dimension.min_y,
            height,
            cells,
        })
    }

    pub fn extract_terrain_chunks(&self) -> Vec<TerrainChunkSnapshot> {
        self.chunks
            .iter()
            .filter_map(|chunk| self.extract_terrain_chunk(chunk.pos))
            .collect()
    }

    pub fn chunk_positions(&self) -> Vec<ChunkPos> {
        self.chunks.iter().map(|chunk| chunk.pos).collect()
    }

    pub fn block_destructions(&self) -> &[BlockDestructionProgress] {
        &self.block_destructions
    }

    pub fn block_destruction(&self, id: i32) -> Option<&BlockDestructionProgress> {
        self.block_destructions
            .iter()
            .find(|progress| progress.id == id)
    }

    pub fn block_events(&self) -> &[BlockEventRecord] {
        &self.block_events
    }

    pub fn level_events(&self) -> &[LevelEventRecord] {
        &self.level_events
    }

    pub fn counters(&self) -> WorldCounters {
        self.counters.clone()
    }

    pub fn local_player_id(&self) -> Option<i32> {
        self.local_player_id
    }

    pub fn local_player_vehicle_id(&self) -> Option<i32> {
        self.local_player_vehicle_id
    }

    pub fn local_player_root_vehicle_id(&self) -> Option<i32> {
        self.resolve_root_vehicle_id(self.local_player_vehicle_id?)
    }

    pub fn inventory(&self) -> &InventoryState {
        &self.inventory
    }

    pub fn dimension(&self) -> WorldDimension {
        self.dimension
    }

    pub fn level_info(&self) -> Option<&WorldLevelInfo> {
        self.level.as_ref()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn registries(&self) -> &RegistrySet {
        &self.registries
    }

    fn ensure_container(&mut self, container_id: i32) -> &mut ContainerState {
        if self
            .inventory
            .open_container
            .as_ref()
            .is_none_or(|container| container.container_id != container_id)
        {
            self.inventory.open_container = Some(ContainerState {
                container_id,
                ..ContainerState::default()
            });
        }
        self.inventory
            .open_container
            .as_mut()
            .expect("container was initialized")
    }

    fn update_inventory_slot_count(&mut self) {
        self.counters.inventory_slots_tracked = self.inventory.player_slots.len();
    }

    fn update_entity_count(&mut self) {
        self.counters.entities_tracked = self.entities.len();
    }

    fn clear_local_player_mount(&mut self, local_player_id: i32) {
        self.local_player_vehicle_id = None;
        for entity in &mut self.entities {
            if entity.id == local_player_id {
                entity.vehicle_id = None;
            }
            entity
                .passengers
                .retain(|passenger_id| *passenger_id != local_player_id);
        }
    }

    fn remove_passenger_from_vehicle(&mut self, vehicle_id: i32, passenger_id: i32) {
        if let Some(vehicle) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == vehicle_id)
        {
            vehicle
                .passengers
                .retain(|existing| *existing != passenger_id);
        }
    }

    fn resolve_root_vehicle_id(&self, vehicle_id: i32) -> Option<i32> {
        let mut root_vehicle_id = vehicle_id;
        for _ in 0..self.entities.len() {
            let vehicle = self.probe_entity(root_vehicle_id)?;
            let Some(parent_vehicle_id) = vehicle.vehicle_id else {
                return Some(root_vehicle_id);
            };
            root_vehicle_id = parent_vehicle_id;
        }
        None
    }

    fn set_block_state_id(&mut self, pos: BlockPos, block_state_id: i32) -> bool {
        if block_state_id < 0 || !self.dimension.contains_y(pos.y) {
            return false;
        }

        let chunk_pos = ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        };
        let local_x = pos.x.rem_euclid(16) as u8;
        let local_y = pos.y.rem_euclid(16) as u8;
        let local_z = pos.z.rem_euclid(16) as u8;
        let section_y = pos.y.div_euclid(16);
        let Some(section_index) = usize::try_from(section_y - self.dimension.min_section_y()).ok()
        else {
            return false;
        };
        let block_index = section_block_index(local_x, local_y, local_z);
        let old_block_state_id = self
            .probe_chunk(chunk_pos)
            .and_then(|chunk| chunk.sections.get(section_index))
            .and_then(|section| section.block_states.value_at(block_index))
            .map(|value| value.global_id);
        let Some(old_block_state_id) = old_block_state_id else {
            return false;
        };

        let old_non_empty = !is_empty_block_state_id(&self.registries, old_block_state_id);
        let new_non_empty = !is_empty_block_state_id(&self.registries, block_state_id);
        let old_fluid = is_fluid_block_state_id(&self.registries, old_block_state_id);
        let new_fluid = is_fluid_block_state_id(&self.registries, block_state_id);

        let Some(section) = self
            .chunks
            .iter_mut()
            .find(|chunk| chunk.pos == chunk_pos)
            .and_then(|chunk| chunk.sections.get_mut(section_index))
        else {
            return false;
        };
        if !section
            .block_states
            .set_value_at(block_index, block_state_id)
        {
            return false;
        }

        apply_counted_delta(
            &mut section.non_empty_block_count,
            old_non_empty,
            new_non_empty,
        );
        apply_counted_delta(&mut section.fluid_count, old_fluid, new_fluid);
        true
    }
}

impl Default for RegistrySet {
    fn default() -> Self {
        Self::vanilla_26_1()
    }
}

pub fn decode_level_chunk_with_light(pos: ChunkPos, payload: &[u8]) -> Result<ChunkColumn> {
    let mut decoder = Decoder::new(payload);
    let heightmaps = decode_heightmaps(&mut decoder)?;

    let section_buffer_len = decoder.read_len()?;
    if section_buffer_len > MAX_CHUNK_SECTION_BUFFER {
        return Err(WorldDecodeError::ChunkSectionBufferTooLarge {
            actual: section_buffer_len,
            max: MAX_CHUNK_SECTION_BUFFER,
        });
    }
    let section_buffer = decoder.read_exact(section_buffer_len, "chunk section buffer")?;
    let sections = decode_sections(section_buffer)?;
    let block_entities = decode_block_entities(&mut decoder)?;
    let light = decode_light_data(&mut decoder)?;

    Ok(ChunkColumn {
        pos,
        state: ChunkState::Decoded,
        heightmaps,
        sections,
        block_entities,
        light,
    })
}

fn decode_heightmaps(decoder: &mut Decoder<'_>) -> Result<Vec<HeightmapData>> {
    let count = decoder.read_len()?;
    let mut heightmaps = Vec::with_capacity(count);
    for _ in 0..count {
        let kind_id = decoder.read_var_i32()?;
        let data = read_long_array(decoder)?;
        heightmaps.push(HeightmapData { kind_id, data });
    }
    Ok(heightmaps)
}

fn decode_sections(bytes: &[u8]) -> Result<Vec<ChunkSection>> {
    let mut decoder = Decoder::new(bytes);
    let mut sections = Vec::new();
    while !decoder.is_empty() {
        sections.push(decode_section(&mut decoder)?);
    }
    Ok(sections)
}

fn decode_section(decoder: &mut Decoder<'_>) -> Result<ChunkSection> {
    let non_empty_block_count = decoder.read_i16()?;
    let fluid_count = decoder.read_i16()?;
    let block_states = decode_paletted_container(decoder, PaletteDomain::BlockStates)?;
    let biomes = decode_paletted_container(decoder, PaletteDomain::Biomes)?;
    Ok(ChunkSection {
        non_empty_block_count,
        fluid_count,
        block_states,
        biomes,
    })
}

fn decode_biome_sections(
    bytes: &[u8],
    expected_sections: usize,
) -> Result<Vec<PalettedContainerData>> {
    if bytes.len() > MAX_CHUNK_SECTION_BUFFER {
        return Err(WorldDecodeError::ByteArrayTooLarge {
            actual: bytes.len(),
            max: MAX_CHUNK_SECTION_BUFFER,
        });
    }

    let mut decoder = Decoder::new(bytes);
    let mut biomes = Vec::with_capacity(expected_sections);
    for _ in 0..expected_sections {
        biomes.push(decode_paletted_container(
            &mut decoder,
            PaletteDomain::Biomes,
        )?);
    }
    if !decoder.is_empty() {
        return Err(WorldDecodeError::TrailingBiomeData {
            remaining: decoder.remaining_len(),
        });
    }
    Ok(biomes)
}

fn decode_paletted_container(
    decoder: &mut Decoder<'_>,
    domain: PaletteDomain,
) -> Result<PalettedContainerData> {
    let bits_per_entry = decoder.read_u8()?;
    if bits_per_entry > 64 {
        return Err(WorldDecodeError::InvalidPalettedBits(bits_per_entry));
    }
    let entry_count = match domain {
        PaletteDomain::BlockStates => 16 * 16 * 16,
        PaletteDomain::Biomes => 4 * 4 * 4,
    };
    let palette_kind = palette_kind(domain, bits_per_entry);
    let palette_global_ids = match palette_kind {
        PaletteKind::SingleValue => vec![decoder.read_var_i32()?],
        PaletteKind::Local => read_var_i32_array(decoder)?,
        PaletteKind::Global => Vec::new(),
    };
    let packed_data_len = packed_long_len(entry_count, bits_per_entry as usize);
    let packed_data = read_fixed_long_array(decoder, packed_data_len)?;

    Ok(PalettedContainerData {
        domain,
        bits_per_entry,
        palette_kind,
        palette_global_ids,
        packed_data,
        entry_count,
    })
}

fn palette_kind(domain: PaletteDomain, bits_per_entry: u8) -> PaletteKind {
    match (domain, bits_per_entry) {
        (_, 0) => PaletteKind::SingleValue,
        (PaletteDomain::BlockStates, 1..=8) => PaletteKind::Local,
        (PaletteDomain::Biomes, 1..=3) => PaletteKind::Local,
        _ => PaletteKind::Global,
    }
}

fn packed_long_len(entry_count: usize, bits_per_entry: usize) -> usize {
    if bits_per_entry == 0 {
        0
    } else {
        let values_per_long = 64 / bits_per_entry;
        entry_count.div_ceil(values_per_long)
    }
}

fn read_packed_value(packed_data: &[i64], bits_per_entry: u8, index: usize) -> Option<u64> {
    if bits_per_entry == 0 || bits_per_entry > 64 {
        return None;
    }

    let bits = bits_per_entry as usize;
    let values_per_long = 64 / bits;
    if values_per_long == 0 {
        return None;
    }

    let cell_index = index / values_per_long;
    let bit_index = (index - cell_index * values_per_long) * bits;
    let cell = *packed_data.get(cell_index)? as u64;
    let mask = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    Some((cell >> bit_index) & mask)
}

fn set_packed_value(packed_data: &mut [i64], bits_per_entry: u8, index: usize, value: u64) -> bool {
    if bits_per_entry == 0 || bits_per_entry > 64 {
        return false;
    }

    let bits = bits_per_entry as usize;
    let values_per_long = 64 / bits;
    if values_per_long == 0 {
        return false;
    }

    let cell_index = index / values_per_long;
    let bit_index = (index - cell_index * values_per_long) * bits;
    let Some(cell) = packed_data.get_mut(cell_index) else {
        return false;
    };
    let mask = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    if value & !mask != 0 {
        return false;
    }

    let raw = *cell as u64;
    *cell = ((raw & !(mask << bit_index)) | (value << bit_index)) as i64;
    true
}

fn pack_values_to_longs(values: &[u64], bits_per_entry: usize) -> Vec<i64> {
    if bits_per_entry == 0 {
        return Vec::new();
    }

    let values_per_long = 64 / bits_per_entry;
    if values_per_long == 0 {
        return Vec::new();
    }

    let mut packed = vec![0u64; values.len().div_ceil(values_per_long)];
    let mask = if bits_per_entry == 64 {
        u64::MAX
    } else {
        (1u64 << bits_per_entry) - 1
    };
    for (index, value) in values.iter().copied().enumerate() {
        let cell_index = index / values_per_long;
        let bit_index = (index - cell_index * values_per_long) * bits_per_entry;
        packed[cell_index] |= (value & mask) << bit_index;
    }

    packed.into_iter().map(|value| value as i64).collect()
}

fn bits_needed(max_value: u64) -> u8 {
    (u64::BITS - max_value.leading_zeros()).max(1) as u8
}

fn section_block_index(x: u8, y: u8, z: u8) -> usize {
    ((y as usize) << 8) | ((z as usize) << 4) | x as usize
}

fn section_biome_index(x: u8, y: u8, z: u8) -> usize {
    ((y as usize) << 4) | ((z as usize) << 2) | x as usize
}

fn classify_terrain_material(block_name: Option<&str>) -> TerrainMaterialClass {
    let Some(name) = block_name else {
        return TerrainMaterialClass::Opaque;
    };
    match name {
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air" => {
            TerrainMaterialClass::Empty
        }
        "minecraft:water" | "minecraft:lava" => TerrainMaterialClass::Fluid,
        name if is_cutout_block_name(name) => TerrainMaterialClass::Cutout,
        name if is_translucent_block_name(name) => TerrainMaterialClass::Translucent,
        _ => TerrainMaterialClass::Opaque,
    }
}

fn is_cutout_block_name(name: &str) -> bool {
    name.contains("sapling")
        || name.contains("leaves")
        || name == "minecraft:short_grass"
        || name == "minecraft:tall_grass"
        || name == "minecraft:grass"
        || name.contains("fern")
        || name.contains("flower")
        || name.contains("mushroom")
        || name.contains("roots")
        || name.contains("vine")
        || name.contains("kelp")
        || name.contains("seagrass")
}

fn is_translucent_block_name(name: &str) -> bool {
    name.contains("glass")
        || name.contains("ice")
        || name.contains("slime")
        || name.contains("honey")
}

fn protocol_block_pos(pos: bbb_protocol::packets::BlockPos) -> BlockPos {
    BlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn set_inventory_slot(slots: &mut Vec<InventorySlot>, update: InventorySlot) {
    if let Some(existing) = slots.iter_mut().find(|slot| slot.slot == update.slot) {
        *existing = update;
    } else {
        slots.push(update);
    }
    slots.sort_by_key(|slot| slot.slot);
}

fn set_container_slot(slots: &mut Vec<ContainerSlot>, update: ContainerSlot) {
    if let Some(existing) = slots.iter_mut().find(|slot| slot.slot == update.slot) {
        *existing = update;
    } else {
        slots.push(update);
    }
    slots.sort_by_key(|slot| slot.slot);
}

fn entity_vec3(vec: ProtocolVec3d) -> EntityVec3 {
    EntityVec3 {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}

fn entity_distance_squared(a: EntityVec3, b: EntityVec3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

fn item_entity_stack_mut(entity: &mut EntityState) -> Option<&mut ProtocolItemStackSummary> {
    entity.data_values.iter_mut().find_map(|value| {
        if value.data_id == VANILLA_ITEM_ENTITY_STACK_DATA_ID {
            if let EntityDataValueKind::ItemStack(stack) = &mut value.value {
                return Some(stack);
            }
        }
        None
    })
}

#[derive(Debug, Clone, Copy)]
struct EntityMoveRotation {
    position: EntityVec3,
    delta_movement: EntityVec3,
    y_rot: f32,
    x_rot: f32,
}

fn decode_entity_delta_position(base: EntityVec3, xa: i16, ya: i16, za: i16) -> EntityVec3 {
    if xa == 0 && ya == 0 && za == 0 {
        return base;
    }

    EntityVec3 {
        x: decode_entity_delta_axis(base.x, xa),
        y: decode_entity_delta_axis(base.y, ya),
        z: decode_entity_delta_axis(base.z, za),
    }
}

fn decode_entity_delta_axis(base: f64, delta: i16) -> f64 {
    if delta == 0 {
        base
    } else {
        java_round_to_i64(base * 4096.0).saturating_add(i64::from(delta)) as f64 / 4096.0
    }
}

fn java_round_to_i64(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

fn entity_absolute_move_rotation(
    current_position: EntityVec3,
    current_delta_movement: EntityVec3,
    current_y_rot: f32,
    current_x_rot: f32,
    change_position: ProtocolVec3d,
    change_delta_movement: ProtocolVec3d,
    change_y_rot: f32,
    change_x_rot: f32,
    relatives_mask: i32,
) -> EntityMoveRotation {
    let position = EntityVec3 {
        x: absolute_or_relative_f64(
            current_position.x,
            change_position.x,
            relatives_mask,
            PLAYER_RELATIVE_X,
        ),
        y: absolute_or_relative_f64(
            current_position.y,
            change_position.y,
            relatives_mask,
            PLAYER_RELATIVE_Y,
        ),
        z: absolute_or_relative_f64(
            current_position.z,
            change_position.z,
            relatives_mask,
            PLAYER_RELATIVE_Z,
        ),
    };
    let y_rot = absolute_or_relative_f32(
        current_y_rot,
        change_y_rot,
        relatives_mask,
        PLAYER_RELATIVE_Y_ROT,
    );
    let x_rot = absolute_or_relative_f32(
        current_x_rot,
        change_x_rot,
        relatives_mask,
        PLAYER_RELATIVE_X_ROT,
    )
    .clamp(-90.0, 90.0);

    let rotated_delta = if relatives_mask & PLAYER_RELATIVE_ROTATE_DELTA != 0 {
        rotate_entity_delta(
            current_delta_movement,
            current_y_rot - y_rot,
            current_x_rot - x_rot,
        )
    } else {
        current_delta_movement
    };
    let delta_movement = EntityVec3 {
        x: absolute_or_relative_f64(
            rotated_delta.x,
            change_delta_movement.x,
            relatives_mask,
            PLAYER_RELATIVE_DELTA_X,
        ),
        y: absolute_or_relative_f64(
            rotated_delta.y,
            change_delta_movement.y,
            relatives_mask,
            PLAYER_RELATIVE_DELTA_Y,
        ),
        z: absolute_or_relative_f64(
            rotated_delta.z,
            change_delta_movement.z,
            relatives_mask,
            PLAYER_RELATIVE_DELTA_Z,
        ),
    };

    EntityMoveRotation {
        position,
        delta_movement,
        y_rot,
        x_rot,
    }
}

fn absolute_or_relative_f64(current: f64, change: f64, mask: i32, relative_bit: i32) -> f64 {
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

fn rotate_entity_delta(delta: EntityVec3, y_rot_degrees: f32, x_rot_degrees: f32) -> EntityVec3 {
    let x_rad = f64::from(x_rot_degrees).to_radians();
    let y_rad = f64::from(y_rot_degrees).to_radians();
    let cos_x = x_rad.cos();
    let sin_x = x_rad.sin();
    let after_x = EntityVec3 {
        x: delta.x,
        y: delta.y * cos_x + delta.z * sin_x,
        z: delta.z * cos_x - delta.y * sin_x,
    };
    let cos_y = y_rad.cos();
    let sin_y = y_rad.sin();
    EntityVec3 {
        x: after_x.x * cos_y + after_x.z * sin_y,
        y: after_x.y,
        z: after_x.z * cos_y - after_x.x * sin_y,
    }
}

fn is_empty_block_state_id(registries: &RegistrySet, block_state_id: i32) -> bool {
    matches!(
        registries
            .block_state(block_state_id)
            .map(|state| state.name.as_str()),
        Some("minecraft:air" | "minecraft:cave_air" | "minecraft:void_air")
    )
}

fn is_fluid_block_state_id(registries: &RegistrySet, block_state_id: i32) -> bool {
    matches!(
        registries
            .block_state(block_state_id)
            .map(|state| state.name.as_str()),
        Some("minecraft:water" | "minecraft:lava")
    )
}

fn apply_counted_delta(count: &mut i16, old_counted: bool, new_counted: bool) {
    match (old_counted, new_counted) {
        (true, false) => *count = count.saturating_sub(1),
        (false, true) => *count = count.saturating_add(1),
        _ => {}
    }
}

struct DimensionProfile {
    dimension: WorldDimension,
    name: Option<&'static str>,
}

fn dimension_profile(dimension_type_id: i32, dimension: &str) -> DimensionProfile {
    match (dimension_type_id, dimension) {
        (0, _) | (_, "minecraft:overworld") => DimensionProfile {
            dimension: WorldDimension {
                min_y: -64,
                height: 384,
            },
            name: Some("minecraft:overworld"),
        },
        (1, _) | (_, "minecraft:the_nether") => DimensionProfile {
            dimension: WorldDimension {
                min_y: 0,
                height: 256,
            },
            name: Some("minecraft:the_nether"),
        },
        (2, _) | (_, "minecraft:the_end") => DimensionProfile {
            dimension: WorldDimension {
                min_y: 0,
                height: 256,
            },
            name: Some("minecraft:the_end"),
        },
        (3, _) | (_, "minecraft:overworld_caves") => DimensionProfile {
            dimension: WorldDimension {
                min_y: -64,
                height: 384,
            },
            name: Some("minecraft:overworld_caves"),
        },
        _ => DimensionProfile {
            dimension: WorldDimension::default(),
            name: None,
        },
    }
}

fn decode_block_entities(decoder: &mut Decoder<'_>) -> Result<Vec<BlockEntityRecord>> {
    let count = decoder.read_len()?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let packed_xz = decoder.read_u8()?;
        let y = decoder.read_i16()?;
        let type_id = decoder.read_var_i32()?;
        let nbt = skip_nbt_any(decoder)?;
        out.push(BlockEntityRecord {
            local_x: packed_xz >> 4,
            y,
            local_z: packed_xz & 0x0f,
            type_id,
            nbt,
        });
    }
    Ok(out)
}

fn decode_light_data(decoder: &mut Decoder<'_>) -> Result<LightData> {
    Ok(LightData {
        sky_y_mask: read_long_array(decoder)?,
        block_y_mask: read_long_array(decoder)?,
        empty_sky_y_mask: read_long_array(decoder)?,
        empty_block_y_mask: read_long_array(decoder)?,
        sky_updates: read_byte_array_list(decoder, LIGHT_ARRAY_BYTES)?,
        block_updates: read_byte_array_list(decoder, LIGHT_ARRAY_BYTES)?,
    })
}

fn merge_light_data(target: &mut LightData, update: LightData) {
    merge_light_layer(
        &mut target.sky_y_mask,
        &mut target.empty_sky_y_mask,
        &mut target.sky_updates,
        &update.sky_y_mask,
        &update.empty_sky_y_mask,
        &update.sky_updates,
    );
    merge_light_layer(
        &mut target.block_y_mask,
        &mut target.empty_block_y_mask,
        &mut target.block_updates,
        &update.block_y_mask,
        &update.empty_block_y_mask,
        &update.block_updates,
    );
}

fn merge_light_layer(
    mask: &mut Vec<i64>,
    empty_mask: &mut Vec<i64>,
    updates: &mut Vec<Vec<u8>>,
    update_mask: &[i64],
    update_empty_mask: &[i64],
    update_arrays: &[Vec<u8>],
) {
    for (section_index, update_array) in set_bit_indices(update_mask).into_iter().zip(update_arrays)
    {
        set_light_layer_data(
            mask,
            empty_mask,
            updates,
            section_index,
            update_array.clone(),
        );
    }
    for section_index in set_bit_indices(update_empty_mask) {
        set_light_layer_empty(mask, empty_mask, updates, section_index);
    }
}

fn set_light_layer_data(
    mask: &mut Vec<i64>,
    empty_mask: &mut Vec<i64>,
    updates: &mut Vec<Vec<u8>>,
    section_index: usize,
    update: Vec<u8>,
) {
    clear_bit(empty_mask, section_index);
    if let Some(rank) = bitset_rank(mask, section_index) {
        if let Some(existing) = updates.get_mut(rank) {
            *existing = update;
        }
        return;
    }

    let insert_index = bitset_rank_before(mask, section_index);
    set_bit(mask, section_index);
    updates.insert(insert_index.min(updates.len()), update);
}

fn set_light_layer_empty(
    mask: &mut Vec<i64>,
    empty_mask: &mut Vec<i64>,
    updates: &mut Vec<Vec<u8>>,
    section_index: usize,
) {
    if let Some(rank) = bitset_rank(mask, section_index) {
        if rank < updates.len() {
            updates.remove(rank);
        }
        clear_bit(mask, section_index);
    }
    set_bit(empty_mask, section_index);
}

fn sample_terrain_light(
    light: &LightData,
    dimension: WorldDimension,
    local_x: usize,
    y: i32,
    local_z: usize,
) -> TerrainLight {
    let section_y = y.div_euclid(16);
    let light_section_index = section_y - (dimension.min_section_y() - 1);
    let Ok(light_section_index) = usize::try_from(light_section_index) else {
        return TerrainLight::FULL_BRIGHT;
    };
    let local_y = y.rem_euclid(16) as usize;
    let nibble_index = section_block_index(local_x as u8, local_y as u8, local_z as u8);
    TerrainLight {
        sky: sample_light_layer(
            &light.sky_y_mask,
            &light.empty_sky_y_mask,
            &light.sky_updates,
            light_section_index,
            nibble_index,
            15,
        ),
        block: sample_light_layer(
            &light.block_y_mask,
            &light.empty_block_y_mask,
            &light.block_updates,
            light_section_index,
            nibble_index,
            0,
        ),
    }
    .clamp()
}

fn sample_light_layer(
    mask: &[i64],
    empty_mask: &[i64],
    updates: &[Vec<u8>],
    section_index: usize,
    nibble_index: usize,
    fallback: u8,
) -> u8 {
    if bitset_get(empty_mask, section_index) {
        return 0;
    }
    if !bitset_get(mask, section_index) {
        return fallback;
    }
    let Some(update_index) = bitset_rank(mask, section_index) else {
        return fallback;
    };
    let Some(layer) = updates.get(update_index) else {
        return fallback;
    };
    read_light_nibble(layer, nibble_index).unwrap_or(fallback)
}

fn bitset_get(words: &[i64], bit: usize) -> bool {
    words
        .get(bit / 64)
        .map(|word| ((*word as u64) & (1u64 << (bit % 64))) != 0)
        .unwrap_or(false)
}

fn set_bit(words: &mut Vec<i64>, bit: usize) {
    let word_index = bit / 64;
    if words.len() <= word_index {
        words.resize(word_index + 1, 0);
    }
    let raw = words[word_index] as u64 | (1u64 << (bit % 64));
    words[word_index] = raw as i64;
}

fn clear_bit(words: &mut [i64], bit: usize) {
    if let Some(word) = words.get_mut(bit / 64) {
        let raw = *word as u64 & !(1u64 << (bit % 64));
        *word = raw as i64;
    }
}

fn bitset_rank(words: &[i64], bit: usize) -> Option<usize> {
    if !bitset_get(words, bit) {
        return None;
    }
    Some(bitset_rank_before(words, bit))
}

fn bitset_rank_before(words: &[i64], bit: usize) -> usize {
    let full_words = bit / 64;
    let mut rank = 0usize;
    for word in &words[..full_words.min(words.len())] {
        rank += (*word as u64).count_ones() as usize;
    }
    let within = bit % 64;
    let mask = if within == 0 { 0 } else { (1u64 << within) - 1 };
    rank += words
        .get(full_words)
        .map(|word| ((*word as u64) & mask).count_ones() as usize)
        .unwrap_or(0);
    rank
}

fn set_bit_indices(words: &[i64]) -> Vec<usize> {
    let mut out = Vec::new();
    for (word_index, word) in words.iter().enumerate() {
        let mut bits = *word as u64;
        while bits != 0 {
            let bit = bits.trailing_zeros() as usize;
            out.push(word_index * 64 + bit);
            bits &= bits - 1;
        }
    }
    out
}

fn read_light_nibble(layer: &[u8], nibble_index: usize) -> Option<u8> {
    let byte = *layer.get(nibble_index / 2)?;
    let shift = (nibble_index % 2) * 4;
    Some((byte >> shift) & 0x0f)
}

fn read_var_i32_array(decoder: &mut Decoder<'_>) -> Result<Vec<i32>> {
    let count = decoder.read_len()?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(decoder.read_var_i32()?);
    }
    Ok(out)
}

fn read_long_array(decoder: &mut Decoder<'_>) -> Result<Vec<i64>> {
    let count = decoder.read_len()?;
    read_fixed_long_array(decoder, count)
}

fn read_fixed_long_array(decoder: &mut Decoder<'_>, count: usize) -> Result<Vec<i64>> {
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(decoder.read_i64()?);
    }
    Ok(out)
}

fn read_byte_array_list(decoder: &mut Decoder<'_>, max_size: usize) -> Result<Vec<Vec<u8>>> {
    let count = decoder.read_len()?;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let len = decoder.read_len()?;
        if len > max_size {
            return Err(WorldDecodeError::ByteArrayTooLarge {
                actual: len,
                max: max_size,
            });
        }
        out.push(decoder.read_exact(len, "byte array")?.to_vec());
    }
    Ok(out)
}

fn skip_nbt_any(decoder: &mut Decoder<'_>) -> Result<Option<NbtPayloadSummary>> {
    let start = decoder.position();
    let root_type = decoder.read_u8()?;
    if root_type == 0 {
        return Ok(None);
    }
    skip_nbt_payload(decoder, root_type)?;
    Ok(Some(NbtPayloadSummary {
        root_type,
        byte_len: decoder.position() - start,
    }))
}

fn skip_nbt_payload(decoder: &mut Decoder<'_>, tag_id: u8) -> Result<()> {
    match tag_id {
        0 => Ok(()),
        1 => {
            decoder.read_exact(1, "nbt byte")?;
            Ok(())
        }
        2 => {
            decoder.read_exact(2, "nbt short")?;
            Ok(())
        }
        3 | 5 => {
            decoder.read_exact(4, "nbt int/float")?;
            Ok(())
        }
        4 | 6 => {
            decoder.read_exact(8, "nbt long/double")?;
            Ok(())
        }
        7 => {
            let len = read_nbt_len(decoder)?;
            decoder.read_exact(len, "nbt byte array")?;
            Ok(())
        }
        8 => {
            let len = decoder.read_u16()? as usize;
            decoder.read_exact(len, "nbt string")?;
            Ok(())
        }
        9 => {
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder)?;
            for _ in 0..len {
                skip_nbt_payload(decoder, element_type)?;
            }
            Ok(())
        }
        10 => {
            loop {
                let nested_type = decoder.read_u8()?;
                if nested_type == 0 {
                    break;
                }
                let name_len = decoder.read_u16()? as usize;
                decoder.read_exact(name_len, "nbt compound name")?;
                skip_nbt_payload(decoder, nested_type)?;
            }
            Ok(())
        }
        11 => {
            let len = read_nbt_len(decoder)?;
            decoder.read_exact(len * 4, "nbt int array")?;
            Ok(())
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            decoder.read_exact(len * 8, "nbt long array")?;
            Ok(())
        }
        other => Err(WorldDecodeError::InvalidNbtTag(other)),
    }
}

fn read_nbt_len(decoder: &mut Decoder<'_>) -> Result<usize> {
    let len = decoder.read_i32()?;
    if len < 0 {
        return Err(WorldDecodeError::NegativeNbtArrayLength(len));
    }
    Ok(len as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::codec::Encoder;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
        AttributeSnapshot as ProtocolAttributeSnapshot,
        BlockDestruction as ProtocolBlockDestruction, BlockEntityData as ProtocolBlockEntityData,
        BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos,
        BlockUpdate as ProtocolBlockUpdate, ChunkBiomeData as ProtocolChunkBiomeData,
        ChunkPos as ProtocolChunkPos, ChunksBiomes as ProtocolChunksBiomes,
        CommonPlayerSpawnInfo as ProtocolSpawnInfo, ContainerClose as ProtocolContainerClose,
        ContainerSetContent as ProtocolContainerSetContent,
        ContainerSetData as ProtocolContainerSetData, ContainerSetSlot as ProtocolContainerSetSlot,
        EntityAnimation as ProtocolEntityAnimation, EntityDataValue as ProtocolEntityDataValue,
        EntityDataValueKind, EntityEvent as ProtocolEntityEvent, EntityMove as ProtocolEntityMove,
        EntityPositionSync as ProtocolEntityPositionSync, EquipmentSlot, EquipmentSlotUpdate,
        HurtAnimation as ProtocolHurtAnimation, ItemStackSummary, LevelEvent as ProtocolLevelEvent,
        MoveVehicle as ProtocolMoveVehicle, OpenScreen as ProtocolOpenScreen,
        PlayLogin as ProtocolPlayLogin, RemoveEntities as ProtocolRemoveEntities,
        Respawn as ProtocolRespawn, RotateHead as ProtocolRotateHead,
        SectionBlocksUpdate as ProtocolSectionBlocksUpdate, SetCursorItem as ProtocolSetCursorItem,
        SetEntityData as ProtocolSetEntityData, SetEntityLink as ProtocolSetEntityLink,
        SetEntityMotion as ProtocolSetEntityMotion, SetEquipment as ProtocolSetEquipment,
        SetPassengers as ProtocolSetPassengers, SetPlayerInventory as ProtocolSetPlayerInventory,
        TakeItemEntity as ProtocolTakeItemEntity, TeleportEntity as ProtocolTeleportEntity,
        UpdateAttributes as ProtocolUpdateAttributes, Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    #[test]
    fn loads_vanilla_block_state_registry() {
        let registries = RegistrySet::vanilla_26_1();
        assert_eq!(registries.block_state_count(), 29873);
        assert_eq!(registries.block_state(0).unwrap().name, "minecraft:air");
        let grass = registries.block_state(9).unwrap();
        assert_eq!(grass.name, "minecraft:grass_block");
        assert_eq!(grass.properties.get("snowy").unwrap(), "false");
    }

    #[test]
    fn decodes_level_chunk_with_light_structure() {
        let packet = synthetic_level_chunk_packet();
        let mut store = WorldStore::new();
        let pos = store.insert_level_chunk_with_light(packet).unwrap();
        let chunk = store.probe_chunk(pos).unwrap();

        assert_eq!(pos, ChunkPos { x: 1, z: -2 });
        assert_eq!(chunk.state, ChunkState::Decoded);
        assert_eq!(chunk.heightmaps.len(), 1);
        assert_eq!(chunk.heightmaps[0].kind_id, 1);
        assert_eq!(chunk.sections.len(), 1);
        assert_eq!(
            chunk.sections[0].block_states.palette_kind,
            PaletteKind::SingleValue
        );
        assert_eq!(chunk.sections[0].block_states.palette_global_ids, vec![0]);
        assert_eq!(chunk.sections[0].biomes.entry_count, 64);
        assert_eq!(chunk.block_entities.len(), 1);
        assert_eq!(chunk.block_entities[0].local_x, 10);
        assert_eq!(chunk.block_entities[0].local_z, 11);
        assert!(chunk.block_entities[0].nbt.is_none());
        assert_eq!(chunk.light.sky_updates, vec![vec![1, 2]]);
        assert_eq!(store.counters().chunks_decoded, 1);
        assert_eq!(store.counters().sections_decoded, 1);
    }

    #[test]
    fn play_login_updates_world_dimension_and_level_info() {
        let mut store = WorldStore::new();
        store
            .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
            .unwrap();

        store.apply_login(&ProtocolPlayLogin {
            player_id: 42,
            hardcore: false,
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
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 12345,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 32,
            },
            enforces_secure_chat: true,
        });

        assert_eq!(
            store.dimension(),
            WorldDimension {
                min_y: 0,
                height: 256,
            }
        );
        assert_eq!(store.chunk_count(), 0);
        assert_eq!(store.counters().play_logins_received, 1);
        let level = store.level_info().unwrap();
        assert_eq!(level.dimension, "minecraft:the_nether");
        assert_eq!(level.dimension_type_id, 1);
        assert_eq!(
            level.dimension_type_name.as_deref(),
            Some("minecraft:the_nether")
        );
        assert_eq!(level.sea_level, 32);
    }

    #[test]
    fn respawn_updates_dimension_and_clears_old_chunks() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 256,
        });
        store
            .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
            .unwrap();
        store.apply_add_entity(protocol_add_entity(123));

        store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 12345,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 32,
            },
            data_to_keep: 3,
        });
        assert_eq!(store.chunk_count(), 1);
        assert_eq!(store.entity_count(), 1);

        store.apply_respawn(&ProtocolRespawn {
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 2,
                dimension: "minecraft:the_end".to_string(),
                seed: 98765,
                game_type: 1,
                previous_game_type: 1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            data_to_keep: 3,
        });

        assert_eq!(
            store.dimension(),
            WorldDimension {
                min_y: 0,
                height: 256,
            }
        );
        assert_eq!(store.chunk_count(), 0);
        assert_eq!(store.entity_count(), 0);
        assert_eq!(store.counters().entities_tracked, 0);
        assert_eq!(store.counters().respawns_received, 2);
        let level = store.level_info().unwrap();
        assert_eq!(level.dimension, "minecraft:the_end");
        assert_eq!(level.dimension_type_id, 2);
        assert_eq!(
            level.dimension_type_name.as_deref(),
            Some("minecraft:the_end")
        );
    }

    #[test]
    fn tracks_player_inventory_and_container_state() {
        let mut store = WorldStore::new();

        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 36,
            item: item_stack(42, 1),
        });
        store.apply_set_player_inventory(ProtocolSetPlayerInventory {
            slot: 36,
            item: item_stack(43, 2),
        });
        store.apply_set_cursor_item(ProtocolSetCursorItem {
            item: item_stack(99, 1),
        });

        assert_eq!(
            store.inventory().player_slots,
            vec![InventorySlot {
                slot: 36,
                item: item_stack(43, 2),
            }]
        );
        assert_eq!(store.inventory().cursor_item, item_stack(99, 1));

        store.apply_open_screen(ProtocolOpenScreen {
            container_id: 7,
            menu_type_id: 2,
            title: "Chest".to_string(),
        });
        store.apply_container_set_content(ProtocolContainerSetContent {
            container_id: 7,
            state_id: 12,
            items: vec![ItemStackSummary::empty(), item_stack(42, 64)],
            carried_item: ItemStackSummary::empty(),
        });
        store.apply_container_set_slot(ProtocolContainerSetSlot {
            container_id: 7,
            state_id: 13,
            slot: 1,
            item: item_stack(44, 3),
        });
        store.apply_container_set_data(ProtocolContainerSetData {
            container_id: 7,
            id: 2,
            value: 9,
        });
        store.apply_container_set_data(ProtocolContainerSetData {
            container_id: 7,
            id: 2,
            value: 10,
        });

        let container = store.inventory().open_container.as_ref().unwrap();
        assert_eq!(container.container_id, 7);
        assert_eq!(container.menu_type_id, Some(2));
        assert_eq!(container.title.as_deref(), Some("Chest"));
        assert_eq!(container.state_id, 13);
        assert_eq!(
            container.slots,
            vec![
                ContainerSlot {
                    slot: 0,
                    item: ItemStackSummary::empty(),
                },
                ContainerSlot {
                    slot: 1,
                    item: item_stack(44, 3),
                },
            ]
        );
        assert_eq!(
            container.data_values,
            vec![ContainerDataValue { id: 2, value: 10 }]
        );
        assert_eq!(store.inventory().cursor_item, ItemStackSummary::empty());

        assert!(store.apply_container_close(ProtocolContainerClose { container_id: 7 }));
        assert!(store.inventory().open_container.is_none());
        assert!(!store.apply_container_close(ProtocolContainerClose { container_id: 99 }));

        assert_eq!(store.counters().inventory_slot_updates_received, 2);
        assert_eq!(store.counters().inventory_slots_tracked, 1);
        assert_eq!(store.counters().cursor_item_updates_received, 1);
        assert_eq!(store.counters().container_open_updates_received, 1);
        assert_eq!(store.counters().container_content_updates_received, 1);
        assert_eq!(store.counters().container_slot_updates_received, 1);
        assert_eq!(store.counters().container_data_updates_received, 2);
        assert_eq!(store.counters().container_close_updates_received, 2);
    }

    #[test]
    fn tracks_entity_lifecycle_and_absolute_state_updates() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(123));

        let entity = store.probe_entity(123).unwrap();
        assert_eq!(entity.entity_type_id, 7);
        assert_eq!(
            entity.position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            }
        );
        assert_eq!(store.entity_count(), 1);
        assert_eq!(store.counters().entities_received, 1);
        assert_eq!(store.counters().entities_tracked, 1);

        assert!(
            store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id: 123,
                position: ProtocolVec3d {
                    x: 2.0,
                    y: 65.0,
                    z: -3.0,
                },
                delta_movement: ProtocolVec3d {
                    x: 0.0,
                    y: 0.25,
                    z: 0.0,
                },
                y_rot: 180.0,
                x_rot: 30.0,
                on_ground: true,
            })
        );
        assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
            id: 123,
            delta_movement: ProtocolVec3d {
                x: 0.1,
                y: 0.0,
                z: -0.1,
            },
        }));
        assert!(store.apply_rotate_head(ProtocolRotateHead {
            id: 123,
            y_head_rot: 90.0,
        }));

        let entity = store.probe_entity(123).unwrap();
        assert_eq!(
            entity.position,
            EntityVec3 {
                x: 2.0,
                y: 65.0,
                z: -3.0,
            }
        );
        assert_eq!(
            entity.delta_movement,
            EntityVec3 {
                x: 0.1,
                y: 0.0,
                z: -0.1,
            }
        );
        assert_eq!(entity.y_rot, 180.0);
        assert_eq!(entity.x_rot, 30.0);
        assert_eq!(entity.y_head_rot, 90.0);
        assert_eq!(entity.on_ground, Some(true));

        assert!(store.apply_entity_move(ProtocolEntityMove {
            id: 123,
            delta_x: 4096,
            delta_y: 0,
            delta_z: -2048,
            y_rot: Some(-90.0),
            x_rot: Some(45.0),
            on_ground: false,
        }));
        let entity = store.probe_entity(123).unwrap();
        assert_eq!(
            entity.position,
            EntityVec3 {
                x: 3.0,
                y: 65.0,
                z: -3.5,
            }
        );
        assert_eq!(entity.position_base, entity.position);
        assert_eq!(entity.y_rot, -90.0);
        assert_eq!(entity.x_rot, 45.0);
        assert_eq!(entity.on_ground, Some(false));

        assert!(store.apply_entity_move(ProtocolEntityMove {
            id: 123,
            delta_x: 0,
            delta_y: 0,
            delta_z: 0,
            y_rot: Some(30.0),
            x_rot: Some(-15.0),
            on_ground: true,
        }));
        let entity = store.probe_entity(123).unwrap();
        assert_eq!(
            entity.position,
            EntityVec3 {
                x: 3.0,
                y: 65.0,
                z: -3.5,
            }
        );
        assert_eq!(entity.y_rot, 30.0);
        assert_eq!(entity.x_rot, -15.0);
        assert_eq!(entity.on_ground, Some(true));

        assert!(store.apply_teleport_entity(ProtocolTeleportEntity {
            id: 123,
            position: ProtocolVec3d {
                x: 0.5,
                y: 70.0,
                z: -4.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.2,
                z: 0.0,
            },
            y_rot: 10.0,
            x_rot: -120.0,
            relatives_mask: PLAYER_RELATIVE_X | PLAYER_RELATIVE_DELTA_Y,
            on_ground: true,
        }));
        let entity = store.probe_entity(123).unwrap();
        assert_eq!(
            entity.position,
            EntityVec3 {
                x: 3.5,
                y: 70.0,
                z: -4.0,
            }
        );
        assert_eq!(
            entity.delta_movement,
            EntityVec3 {
                x: 0.0,
                y: 0.2,
                z: 0.0,
            }
        );
        assert_eq!(entity.y_rot, 10.0);
        assert_eq!(entity.x_rot, -90.0);
        assert_eq!(entity.on_ground, Some(true));

        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 123,
            values: vec![
                ProtocolEntityDataValue {
                    data_id: 0,
                    serializer_id: 0,
                    value: EntityDataValueKind::Byte(0x20),
                },
                ProtocolEntityDataValue {
                    data_id: 2,
                    serializer_id: 1,
                    value: EntityDataValueKind::Int(300),
                },
            ],
        }));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 123,
            values: vec![ProtocolEntityDataValue {
                data_id: 2,
                serializer_id: 1,
                value: EntityDataValueKind::Int(301),
            }],
        }));
        let entity = store.probe_entity(123).unwrap();
        assert_eq!(
            entity.data_values,
            vec![
                ProtocolEntityDataValue {
                    data_id: 0,
                    serializer_id: 0,
                    value: EntityDataValueKind::Byte(0x20),
                },
                ProtocolEntityDataValue {
                    data_id: 2,
                    serializer_id: 1,
                    value: EntityDataValueKind::Int(301),
                },
            ]
        );

        assert!(store.apply_set_equipment(ProtocolSetEquipment {
            entity_id: 123,
            slots: vec![
                EquipmentSlotUpdate {
                    slot: EquipmentSlot::Head,
                    item: ItemStackSummary {
                        item_id: Some(42),
                        count: 1,
                        component_patch: Default::default(),
                    },
                },
                EquipmentSlotUpdate {
                    slot: EquipmentSlot::MainHand,
                    item: ItemStackSummary::empty(),
                },
            ],
        }));
        assert!(store.apply_set_equipment(ProtocolSetEquipment {
            entity_id: 123,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Head,
                item: ItemStackSummary {
                    item_id: Some(51),
                    count: 2,
                    component_patch: Default::default(),
                },
            }],
        }));
        assert!(!store.apply_set_equipment(ProtocolSetEquipment {
            entity_id: 999,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::OffHand,
                item: ItemStackSummary::empty(),
            }],
        }));
        let entity = store.probe_entity(123).unwrap();
        assert_eq!(
            entity.equipment,
            vec![
                EquipmentSlotUpdate {
                    slot: EquipmentSlot::MainHand,
                    item: ItemStackSummary::empty(),
                },
                EquipmentSlotUpdate {
                    slot: EquipmentSlot::Head,
                    item: ItemStackSummary {
                        item_id: Some(51),
                        count: 2,
                        component_patch: Default::default(),
                    },
                },
            ]
        );

        assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![
                ProtocolAttributeSnapshot {
                    attribute_id: 21,
                    base: 20.0,
                    modifiers: vec![ProtocolAttributeModifier {
                        id: "minecraft:health_bonus".to_string(),
                        amount: 4.0,
                        operation_id: 0,
                    }],
                },
                ProtocolAttributeSnapshot {
                    attribute_id: 26,
                    base: 0.7,
                    modifiers: Vec::new(),
                },
            ],
        }));
        assert!(store.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 123,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: 26,
                base: 0.9,
                modifiers: vec![ProtocolAttributeModifier {
                    id: "minecraft:speed_bonus".to_string(),
                    amount: 0.2,
                    operation_id: 2,
                }],
            }],
        }));
        assert!(!store.apply_update_attributes(ProtocolUpdateAttributes {
            entity_id: 999,
            attributes: vec![ProtocolAttributeSnapshot {
                attribute_id: 21,
                base: 20.0,
                modifiers: Vec::new(),
            }],
        }));
        let entity = store.probe_entity(123).unwrap();
        assert_eq!(
            entity.attributes,
            vec![
                ProtocolAttributeSnapshot {
                    attribute_id: 21,
                    base: 20.0,
                    modifiers: vec![ProtocolAttributeModifier {
                        id: "minecraft:health_bonus".to_string(),
                        amount: 4.0,
                        operation_id: 0,
                    }],
                },
                ProtocolAttributeSnapshot {
                    attribute_id: 26,
                    base: 0.9,
                    modifiers: vec![ProtocolAttributeModifier {
                        id: "minecraft:speed_bonus".to_string(),
                        amount: 0.2,
                        operation_id: 2,
                    }],
                },
            ]
        );

        assert!(
            !store.apply_entity_position_sync(ProtocolEntityPositionSync {
                id: 999,
                position: ProtocolVec3d::default(),
                delta_movement: ProtocolVec3d::default(),
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: false,
            })
        );
        assert_eq!(store.counters().entity_position_syncs_received, 2);
        assert_eq!(store.counters().entity_position_syncs_applied, 1);
        assert_eq!(store.counters().entity_moves_received, 2);
        assert_eq!(store.counters().entity_moves_applied, 2);
        assert_eq!(store.counters().entity_teleports_received, 1);
        assert_eq!(store.counters().entity_teleports_applied, 1);
        assert_eq!(store.counters().entity_data_updates_received, 2);
        assert_eq!(store.counters().entity_data_values_received, 3);
        assert_eq!(store.counters().entity_data_updates_applied, 2);
        assert_eq!(store.counters().entity_equipment_updates_received, 3);
        assert_eq!(store.counters().entity_equipment_slots_received, 4);
        assert_eq!(store.counters().entity_equipment_updates_applied, 2);
        assert_eq!(store.counters().entity_attribute_updates_received, 3);
        assert_eq!(store.counters().entity_attributes_received, 4);
        assert_eq!(store.counters().entity_attribute_updates_applied, 2);
        assert_eq!(store.counters().entity_motion_updates_applied, 1);
        assert_eq!(store.counters().entity_head_rotations_applied, 1);

        assert_eq!(
            store.apply_remove_entities(ProtocolRemoveEntities {
                entity_ids: vec![123, 456],
            }),
            1
        );
        assert!(store.probe_entity(123).is_none());
        assert_eq!(store.entity_count(), 0);
        assert_eq!(store.counters().entity_removes_received, 2);
        assert_eq!(store.counters().entities_removed, 1);
        assert_eq!(store.counters().entities_tracked, 0);
    }

    #[test]
    fn take_item_entity_shrinks_item_stacks_and_removes_entities() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity_with_type(
            10,
            VANILLA_ENTITY_TYPE_ITEM_ID,
        ));
        store.apply_add_entity(protocol_add_entity_with_type(
            20,
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        ));
        store.apply_add_entity(protocol_add_entity_with_type(30, 7));

        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 10,
            values: vec![item_stack_entity_data(item_stack(42, 5))],
        }));

        assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
            item_id: 10,
            player_id: 99,
            amount: 2,
        }));
        let item_entity = store.probe_entity(10).unwrap();
        assert_eq!(
            item_entity.data_values,
            vec![item_stack_entity_data(item_stack(42, 3))]
        );

        assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
            item_id: 10,
            player_id: 99,
            amount: 3,
        }));
        assert!(store.probe_entity(10).is_none());

        assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
            item_id: 20,
            player_id: 99,
            amount: 1,
        }));
        assert!(store.probe_entity(20).is_some());

        assert!(store.apply_take_item_entity(ProtocolTakeItemEntity {
            item_id: 30,
            player_id: 99,
            amount: 1,
        }));
        assert!(store.probe_entity(30).is_none());
        assert!(!store.apply_take_item_entity(ProtocolTakeItemEntity {
            item_id: 999,
            player_id: 99,
            amount: 1,
        }));

        assert_eq!(store.entity_count(), 1);
        assert_eq!(store.counters().take_item_entities_received, 5);
        assert_eq!(store.counters().take_item_entities_applied, 4);
        assert_eq!(store.counters().item_entity_stack_shrinks, 2);
        assert_eq!(store.counters().take_item_entities_removed, 2);
        assert_eq!(store.counters().entities_removed, 2);
        assert_eq!(store.counters().entities_tracked, 1);
    }

    #[test]
    fn tracks_entity_transient_events() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(123));

        assert!(store.apply_entity_animation(ProtocolEntityAnimation { id: 123, action: 3 }));
        assert!(store.apply_entity_event(ProtocolEntityEvent {
            entity_id: 123,
            event_id: 35,
        }));
        assert!(store.apply_hurt_animation(ProtocolHurtAnimation { id: 123, yaw: 45.5 }));

        let entity = store.probe_entity(123).unwrap();
        assert_eq!(entity.last_animation_action, Some(3));
        assert_eq!(entity.last_event_id, Some(35));
        assert_eq!(entity.last_hurt_yaw, Some(45.5));

        assert!(!store.apply_entity_animation(ProtocolEntityAnimation { id: 999, action: 4 }));
        assert!(!store.apply_entity_event(ProtocolEntityEvent {
            entity_id: 999,
            event_id: 21,
        }));
        assert!(!store.apply_hurt_animation(ProtocolHurtAnimation { id: 999, yaw: 90.0 }));

        assert_eq!(store.counters().entity_animation_updates_received, 2);
        assert_eq!(store.counters().entity_animation_updates_applied, 1);
        assert_eq!(store.counters().entity_events_received, 2);
        assert_eq!(store.counters().entity_events_applied, 1);
        assert_eq!(store.counters().entity_hurt_animations_received, 2);
        assert_eq!(store.counters().entity_hurt_animations_applied, 1);
    }

    #[test]
    fn tracks_entity_passenger_updates() {
        let mut store = WorldStore::new();
        for id in [10, 20, 21, 30] {
            store.apply_add_entity(protocol_add_entity(id));
        }

        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![20, 21, 999, 20],
        }));
        assert_eq!(store.probe_entity(10).unwrap().passengers, vec![20, 21]);
        assert_eq!(store.probe_entity(20).unwrap().vehicle_id, Some(10));
        assert_eq!(store.probe_entity(21).unwrap().vehicle_id, Some(10));

        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 30,
            passenger_ids: vec![20],
        }));
        assert_eq!(store.probe_entity(10).unwrap().passengers, vec![21]);
        assert_eq!(store.probe_entity(20).unwrap().vehicle_id, Some(30));
        assert_eq!(store.probe_entity(30).unwrap().passengers, vec![20]);

        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 10,
            passenger_ids: Vec::new(),
        }));
        assert!(store.probe_entity(10).unwrap().passengers.is_empty());
        assert_eq!(store.probe_entity(21).unwrap().vehicle_id, None);

        assert!(!store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 999,
            passenger_ids: vec![21],
        }));
        assert_eq!(
            store.apply_remove_entities(ProtocolRemoveEntities {
                entity_ids: vec![30],
            }),
            1
        );
        assert_eq!(store.probe_entity(20).unwrap().vehicle_id, None);
        assert!(store.probe_entity(30).is_none());

        assert_eq!(store.counters().entity_passenger_updates_received, 4);
        assert_eq!(store.counters().entity_passenger_ids_received, 6);
        assert_eq!(store.counters().entity_passenger_updates_applied, 3);
    }

    #[test]
    fn tracks_local_player_passenger_without_entity() {
        let mut store = WorldStore::new();
        store.apply_login(&protocol_play_login(99));
        for id in [10, 20, 30] {
            store.apply_add_entity(protocol_add_entity(id));
        }

        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99, 20],
        }));
        assert_eq!(store.local_player_id(), Some(99));
        assert_eq!(store.local_player_vehicle_id(), Some(10));
        assert!(store.probe_entity(99).is_none());
        assert_eq!(store.probe_entity(10).unwrap().passengers, vec![99, 20]);
        assert_eq!(store.probe_entity(20).unwrap().vehicle_id, Some(10));

        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 30,
            passenger_ids: vec![99],
        }));
        assert_eq!(store.local_player_vehicle_id(), Some(30));
        assert_eq!(store.probe_entity(10).unwrap().passengers, vec![20]);
        assert_eq!(store.probe_entity(30).unwrap().passengers, vec![99]);

        assert!(!store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 999,
            passenger_ids: Vec::new(),
        }));
        assert_eq!(store.local_player_vehicle_id(), Some(30));
        assert_eq!(store.probe_entity(30).unwrap().passengers, vec![99]);

        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 30,
            passenger_ids: Vec::new(),
        }));
        assert_eq!(store.local_player_vehicle_id(), None);
        assert!(store.probe_entity(30).unwrap().passengers.is_empty());

        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        }));
        store.apply_login(&protocol_play_login(100));
        assert_eq!(store.local_player_id(), Some(100));
        assert_eq!(store.local_player_vehicle_id(), None);
        assert_eq!(
            store.probe_entity(10).unwrap().passengers,
            Vec::<i32>::new()
        );
    }

    #[test]
    fn move_vehicle_snaps_root_vehicle_and_returns_ack() {
        let mut store = WorldStore::new();
        store.apply_login(&protocol_play_login(99));
        store.apply_add_entity(protocol_add_entity(10));
        store.apply_add_entity(protocol_add_entity(20));
        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 20,
            passenger_ids: vec![99],
        }));
        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![20],
        }));

        let report = store
            .apply_move_vehicle(ProtocolMoveVehicle {
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 66.0,
                    z: -7.0,
                },
                y_rot: 45.0,
                x_rot: -5.0,
            })
            .unwrap();

        assert_eq!(store.local_player_vehicle_id(), Some(20));
        assert_eq!(store.local_player_root_vehicle_id(), Some(10));
        assert_eq!(
            report,
            VehicleMoveReport {
                vehicle_id: 10,
                position: EntityVec3 {
                    x: 5.0,
                    y: 66.0,
                    z: -7.0,
                },
                y_rot: 45.0,
                x_rot: -5.0,
                on_ground: false,
                snapped: true,
            }
        );
        let root = store.probe_entity(10).unwrap();
        assert_eq!(root.position, report.position);
        assert_eq!(root.position_base, report.position);
        assert_eq!(root.y_rot, 45.0);
        assert_eq!(root.x_rot, -5.0);
        assert_eq!(
            store.probe_entity(20).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            }
        );
        assert_eq!(store.counters().vehicle_moves_received, 1);
        assert_eq!(store.counters().vehicle_moves_applied, 1);
        assert_eq!(store.counters().vehicle_moves_acked, 1);
        assert_eq!(store.counters().vehicle_moves_snapped, 1);
    }

    #[test]
    fn move_vehicle_without_mount_is_noop() {
        let mut store = WorldStore::new();
        store.apply_login(&protocol_play_login(99));
        store.apply_add_entity(protocol_add_entity(10));

        assert_eq!(
            store.apply_move_vehicle(ProtocolMoveVehicle {
                position: ProtocolVec3d {
                    x: 5.0,
                    y: 66.0,
                    z: -7.0,
                },
                y_rot: 45.0,
                x_rot: -5.0,
            }),
            None
        );

        let entity = store.probe_entity(10).unwrap();
        assert_eq!(
            entity.position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            }
        );
        assert_eq!(store.counters().vehicle_moves_received, 1);
        assert_eq!(store.counters().vehicle_moves_applied, 0);
        assert_eq!(store.counters().vehicle_moves_acked, 0);
        assert_eq!(store.counters().vehicle_moves_snapped, 0);
    }

    #[test]
    fn move_vehicle_small_delta_acks_without_snap() {
        let mut store = WorldStore::new();
        store.apply_login(&protocol_play_login(99));
        store.apply_add_entity(protocol_add_entity(10));
        assert!(store.apply_set_passengers(ProtocolSetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        }));

        let report = store
            .apply_move_vehicle(ProtocolMoveVehicle {
                position: ProtocolVec3d {
                    x: 1.000001,
                    y: 64.0,
                    z: -2.0,
                },
                y_rot: 80.0,
                x_rot: 35.0,
            })
            .unwrap();

        assert_eq!(
            report,
            VehicleMoveReport {
                vehicle_id: 10,
                position: EntityVec3 {
                    x: 1.0,
                    y: 64.0,
                    z: -2.0,
                },
                y_rot: 20.0,
                x_rot: -10.0,
                on_ground: false,
                snapped: false,
            }
        );
        let entity = store.probe_entity(10).unwrap();
        assert_eq!(entity.position, report.position);
        assert_eq!(entity.y_rot, 20.0);
        assert_eq!(entity.x_rot, -10.0);
        assert_eq!(store.counters().vehicle_moves_received, 1);
        assert_eq!(store.counters().vehicle_moves_applied, 1);
        assert_eq!(store.counters().vehicle_moves_acked, 1);
        assert_eq!(store.counters().vehicle_moves_snapped, 0);
    }

    #[test]
    fn tracks_entity_link_updates() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(10));
        store.apply_add_entity(protocol_add_entity(20));

        assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
            source_id: 10,
            dest_id: 20,
        }));
        assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, Some(20));

        assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
            source_id: 10,
            dest_id: 999,
        }));
        assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, Some(999));

        assert!(!store.apply_set_entity_link(ProtocolSetEntityLink {
            source_id: 999,
            dest_id: 20,
        }));

        assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
            source_id: 10,
            dest_id: 0,
        }));
        assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, None);

        assert!(store.apply_set_entity_link(ProtocolSetEntityLink {
            source_id: 10,
            dest_id: 20,
        }));
        assert_eq!(
            store.apply_remove_entities(ProtocolRemoveEntities {
                entity_ids: vec![20],
            }),
            1
        );
        assert_eq!(store.probe_entity(10).unwrap().leash_holder_id, None);

        assert_eq!(store.counters().entity_link_updates_received, 5);
        assert_eq!(store.counters().entity_link_updates_applied, 4);
    }

    #[test]
    fn samples_terrain_light_from_packet_layers() {
        let dimension = WorldDimension {
            min_y: 0,
            height: 16,
        };
        let index = section_block_index(2, 1, 3);
        let mut sky = vec![0; LIGHT_ARRAY_BYTES];
        let mut block = vec![0; LIGHT_ARRAY_BYTES];
        set_light_nibble(&mut sky, index, 12);
        set_light_nibble(&mut block, index, 7);
        let light = LightData {
            sky_y_mask: vec![0b10],
            block_y_mask: vec![0b10],
            empty_sky_y_mask: Vec::new(),
            empty_block_y_mask: Vec::new(),
            sky_updates: vec![sky],
            block_updates: vec![block],
        };

        assert_eq!(
            sample_terrain_light(&light, dimension, 2, 1, 3),
            TerrainLight { sky: 12, block: 7 }
        );
    }

    #[test]
    fn terrain_light_empty_masks_override_fallback() {
        let dimension = WorldDimension {
            min_y: 0,
            height: 16,
        };
        let light = LightData {
            sky_y_mask: Vec::new(),
            block_y_mask: Vec::new(),
            empty_sky_y_mask: vec![0b10],
            empty_block_y_mask: vec![0b10],
            sky_updates: Vec::new(),
            block_updates: Vec::new(),
        };

        assert_eq!(
            sample_terrain_light(&light, dimension, 2, 1, 3),
            TerrainLight::DARK
        );
    }

    #[test]
    fn applies_light_update_to_existing_chunk_sections() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store
            .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
            .unwrap();
        let index = section_block_index(2, 1, 3);
        let mut sky = vec![0; LIGHT_ARRAY_BYTES];
        let mut block = vec![0; LIGHT_ARRAY_BYTES];
        set_light_nibble(&mut sky, index, 4);
        set_light_nibble(&mut block, index, 13);

        let applied = store
            .apply_light_update(ProtocolLightUpdate {
                chunk_x: 2,
                chunk_z: -3,
                raw_light_data: light_update_payload(
                    &[0b10],
                    &[0b10],
                    &[],
                    &[],
                    &[&sky],
                    &[&block],
                ),
            })
            .unwrap();

        assert!(applied);
        assert_eq!(store.counters().light_updates_received, 1);
        assert_eq!(store.counters().light_updates_applied, 1);
        let terrain = store
            .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap();
        assert_eq!(
            terrain.cells[terrain_cell_index(2, 1, 3, 16)].light,
            TerrainLight { sky: 4, block: 13 }
        );

        let applied = store
            .apply_light_update(ProtocolLightUpdate {
                chunk_x: 2,
                chunk_z: -3,
                raw_light_data: light_update_payload(&[], &[], &[], &[0b10], &[], &[]),
            })
            .unwrap();

        assert!(applied);
        let terrain = store
            .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap();
        assert_eq!(
            terrain.cells[terrain_cell_index(2, 1, 3, 16)].light,
            TerrainLight { sky: 4, block: 0 }
        );
    }

    #[test]
    fn applies_biome_update_to_existing_chunk_sections() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store
            .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
            .unwrap();

        let applied = store
            .apply_biome_update(ProtocolChunksBiomes {
                chunks: vec![ProtocolChunkBiomeData {
                    pos: ProtocolChunkPos { x: 2, z: -3 },
                    raw_biomes: single_biome_payload(7),
                }],
            })
            .unwrap();

        assert_eq!(applied, 1);
        assert_eq!(store.counters().biome_updates_received, 1);
        assert_eq!(store.counters().biome_updates_applied, 1);
        assert_eq!(
            store
                .probe_block(BlockPos {
                    x: 34,
                    y: 1,
                    z: -45,
                })
                .unwrap()
                .biome_id,
            Some(7)
        );
        let terrain = store
            .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap();
        assert_eq!(
            terrain.cells[terrain_cell_index(2, 1, 3, 16)].biome_id,
            Some(7)
        );
    }

    #[test]
    fn biome_update_for_missing_chunk_is_counted_but_not_applied() {
        let mut store = WorldStore::new();

        let applied = store
            .apply_biome_update(ProtocolChunksBiomes {
                chunks: vec![ProtocolChunkBiomeData {
                    pos: ProtocolChunkPos { x: 2, z: -3 },
                    raw_biomes: Vec::new(),
                }],
            })
            .unwrap();

        assert_eq!(applied, 0);
        assert_eq!(store.counters().biome_updates_received, 1);
        assert_eq!(store.counters().biome_updates_applied, 0);
    }

    #[test]
    fn probes_block_state_from_local_palette() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store
            .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
            .unwrap();

        let probe = store
            .probe_block(BlockPos {
                x: 34,
                y: 1,
                z: -45,
            })
            .unwrap();

        assert_eq!(probe.chunk, ChunkPos { x: 2, z: -3 });
        assert_eq!(probe.local_x, 2);
        assert_eq!(probe.local_y, 1);
        assert_eq!(probe.local_z, 3);
        assert_eq!(probe.section_y, 0);
        assert_eq!(probe.section_index, 0);
        assert_eq!(probe.block_state_id, 9);
        assert_eq!(probe.block_name.as_deref(), Some("minecraft:grass_block"));
        assert_eq!(probe.material, TerrainMaterialClass::Opaque);
        assert_eq!(probe.block_properties.get("snowy").unwrap(), "false");
        assert_eq!(probe.block_palette_kind, PaletteKind::Local);
        assert_eq!(probe.block_palette_index, Some(2));
        assert_eq!(probe.biome_id, Some(4));
        assert_eq!(probe.biome_palette_kind, PaletteKind::SingleValue);

        assert!(store
            .probe_block(BlockPos {
                x: 34,
                y: 16,
                z: -45,
            })
            .is_none());
        assert!(store.probe_block(BlockPos { x: 0, y: 1, z: 0 }).is_none());
    }

    #[test]
    fn extracts_terrain_chunk_summary() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store
            .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
            .unwrap();

        let terrain = store
            .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap();
        let summary = terrain.summary();
        assert_eq!(summary.total_blocks, 4096);
        assert_eq!(summary.opaque_blocks, 4096);
        assert_eq!(summary.empty_blocks, 0);
        assert_eq!(summary.cutout_blocks, 0);
        assert_eq!(
            terrain.cells[terrain_cell_index(2, 1, 3, 16)].biome_id,
            Some(4)
        );
    }

    #[test]
    fn applies_single_block_update_and_reuploads_palette() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store
            .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
            .unwrap();

        let applied = store.apply_block_update(ProtocolBlockUpdate {
            pos: ProtocolBlockPos {
                x: 34,
                y: 1,
                z: -45,
            },
            block_state_id: 0,
        });

        assert!(applied);
        assert_eq!(store.counters().block_updates_received, 1);
        assert_eq!(store.counters().block_updates_applied, 1);

        let probe = store
            .probe_block(BlockPos {
                x: 34,
                y: 1,
                z: -45,
            })
            .unwrap();
        assert_eq!(probe.block_state_id, 0);
        assert_eq!(probe.block_name.as_deref(), Some("minecraft:air"));
        assert_eq!(probe.material, TerrainMaterialClass::Empty);
        assert_eq!(probe.block_palette_kind, PaletteKind::Global);
        assert_eq!(probe.block_palette_index, None);

        let chunk = store.probe_chunk(ChunkPos { x: 2, z: -3 }).unwrap();
        assert_eq!(chunk.sections[0].non_empty_block_count, 4095);
        let summary = store
            .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .summary();
        assert_eq!(summary.empty_blocks, 1);
        assert_eq!(summary.opaque_blocks, 4095);
    }

    #[test]
    fn tracks_block_destruction_progress_by_id() {
        let mut store = WorldStore::new();

        assert!(store.apply_block_destruction(ProtocolBlockDestruction {
            id: 7,
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            progress: 3,
        }));
        assert_eq!(
            store.block_destruction(7),
            Some(&BlockDestructionProgress {
                id: 7,
                pos: BlockPos {
                    x: 12,
                    y: 64,
                    z: -5,
                },
                progress: 3,
            })
        );
        assert_eq!(store.counters().block_destructions_received, 1);
        assert_eq!(store.counters().block_destructions_tracked, 1);
        assert_eq!(store.counters().block_destructions_removed, 0);

        assert!(store.apply_block_destruction(ProtocolBlockDestruction {
            id: 7,
            pos: ProtocolBlockPos {
                x: 13,
                y: 65,
                z: -6,
            },
            progress: 9,
        }));
        assert_eq!(store.block_destructions().len(), 1);
        assert_eq!(
            store.block_destruction(7),
            Some(&BlockDestructionProgress {
                id: 7,
                pos: BlockPos {
                    x: 13,
                    y: 65,
                    z: -6,
                },
                progress: 9,
            })
        );

        assert!(store.apply_block_destruction(ProtocolBlockDestruction {
            id: 7,
            pos: ProtocolBlockPos {
                x: 13,
                y: 65,
                z: -6,
            },
            progress: 10,
        }));
        assert!(store.block_destructions().is_empty());
        assert_eq!(store.counters().block_destructions_received, 3);
        assert_eq!(store.counters().block_destructions_tracked, 0);
        assert_eq!(store.counters().block_destructions_removed, 1);

        assert!(!store.apply_block_destruction(ProtocolBlockDestruction {
            id: 99,
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            progress: 255,
        }));
        assert_eq!(store.counters().block_destructions_received, 4);
        assert_eq!(store.counters().block_destructions_removed, 1);
    }

    #[test]
    fn tracks_transient_block_and_level_events() {
        let mut store = WorldStore::new();

        store.apply_block_event(ProtocolBlockEvent {
            pos: ProtocolBlockPos {
                x: 12,
                y: 64,
                z: -5,
            },
            b0: 1,
            b1: 5,
            block_id: 123,
        });
        store.apply_level_event(ProtocolLevelEvent {
            event_type: 2001,
            pos: ProtocolBlockPos {
                x: 13,
                y: 65,
                z: -6,
            },
            data: 9,
            global: true,
        });

        assert_eq!(
            store.block_events(),
            &[BlockEventRecord {
                pos: BlockPos {
                    x: 12,
                    y: 64,
                    z: -5,
                },
                b0: 1,
                b1: 5,
                block_id: 123,
            }]
        );
        assert_eq!(
            store.level_events(),
            &[LevelEventRecord {
                event_type: 2001,
                pos: BlockPos {
                    x: 13,
                    y: 65,
                    z: -6,
                },
                data: 9,
                global: true,
            }]
        );
        assert_eq!(store.counters().block_events_received, 1);
        assert_eq!(store.counters().block_events_tracked, 1);
        assert_eq!(store.counters().level_events_received, 1);
        assert_eq!(store.counters().level_events_tracked, 1);

        store.apply_login(&ProtocolPlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:the_nether".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: ProtocolSpawnInfo {
                dimension_type_id: 1,
                dimension: "minecraft:the_nether".to_string(),
                seed: 12345,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 32,
            },
            enforces_secure_chat: true,
        });

        assert!(store.block_events().is_empty());
        assert!(store.level_events().is_empty());
        assert_eq!(store.counters().block_events_tracked, 0);
        assert_eq!(store.counters().level_events_tracked, 0);
    }

    #[test]
    fn applies_section_blocks_update() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store
            .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
            .unwrap();

        let applied = store.apply_section_blocks_update(ProtocolSectionBlocksUpdate {
            section_x: 2,
            section_y: 0,
            section_z: -3,
            updates: vec![
                ProtocolBlockUpdate {
                    pos: ProtocolBlockPos {
                        x: 34,
                        y: 1,
                        z: -45,
                    },
                    block_state_id: 0,
                },
                ProtocolBlockUpdate {
                    pos: ProtocolBlockPos {
                        x: 35,
                        y: 1,
                        z: -45,
                    },
                    block_state_id: 0,
                },
            ],
        });

        assert_eq!(applied, 2);
        assert_eq!(store.counters().block_updates_received, 2);
        assert_eq!(store.counters().block_updates_applied, 2);

        let summary = store
            .extract_terrain_chunk(ChunkPos { x: 2, z: -3 })
            .unwrap()
            .summary();
        assert_eq!(summary.empty_blocks, 2);
        assert_eq!(summary.opaque_blocks, 4094);
        assert_eq!(
            store
                .probe_chunk(ChunkPos { x: 2, z: -3 })
                .unwrap()
                .sections[0]
                .non_empty_block_count,
            4094
        );
    }

    #[test]
    fn applies_block_entity_data_update() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store
            .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
            .unwrap();

        let raw_nbt = nbt_compound_with_string("id", "minecraft:chest");
        let applied = store
            .apply_block_entity_data(ProtocolBlockEntityData {
                pos: ProtocolBlockPos {
                    x: 33,
                    y: 7,
                    z: -46,
                },
                block_entity_type_id: 9,
                raw_nbt: raw_nbt.clone(),
            })
            .unwrap();

        assert!(applied);
        assert_eq!(store.counters().block_entity_updates_received, 1);
        assert_eq!(store.counters().block_entity_updates_applied, 1);

        let chunk = store.probe_chunk(ChunkPos { x: 2, z: -3 }).unwrap();
        assert_eq!(chunk.block_entities.len(), 1);
        assert_eq!(
            chunk.block_entities[0],
            BlockEntityRecord {
                local_x: 1,
                y: 7,
                local_z: 2,
                type_id: 9,
                nbt: Some(NbtPayloadSummary {
                    root_type: 10,
                    byte_len: raw_nbt.len(),
                }),
            }
        );

        let replacement_nbt = nbt_compound_with_string("id", "minecraft:furnace");
        assert!(store
            .apply_block_entity_data(ProtocolBlockEntityData {
                pos: ProtocolBlockPos {
                    x: 33,
                    y: 7,
                    z: -46,
                },
                block_entity_type_id: 11,
                raw_nbt: replacement_nbt,
            })
            .unwrap());
        let chunk = store.probe_chunk(ChunkPos { x: 2, z: -3 }).unwrap();
        assert_eq!(chunk.block_entities.len(), 1);
        assert_eq!(chunk.block_entities[0].type_id, 11);

        let missing_chunk_applied = store
            .apply_block_entity_data(ProtocolBlockEntityData {
                pos: ProtocolBlockPos {
                    x: 800,
                    y: 7,
                    z: -46,
                },
                block_entity_type_id: 9,
                raw_nbt: vec![0],
            })
            .unwrap();
        assert!(!missing_chunk_applied);
        assert_eq!(store.counters().block_entity_updates_received, 3);
        assert_eq!(store.counters().block_entity_updates_applied, 2);
    }

    #[test]
    fn forgets_loaded_chunk() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store
            .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
            .unwrap();

        assert!(store.forget_chunk(ChunkPos { x: 2, z: -3 }));
        assert_eq!(store.counters().chunk_forgets_received, 1);
        assert_eq!(store.counters().chunks_forgotten, 1);
        assert_eq!(store.chunk_count(), 0);
        assert!(store.probe_chunk(ChunkPos { x: 2, z: -3 }).is_none());
        assert!(store
            .probe_block(BlockPos {
                x: 34,
                y: 1,
                z: -45,
            })
            .is_none());
        assert!(store.extract_terrain_chunks().is_empty());
    }

    #[test]
    fn forget_missing_chunk_is_counted_but_not_applied() {
        let mut store = WorldStore::new();

        assert!(!store.forget_chunk(ChunkPos { x: 2, z: -3 }));
        assert_eq!(store.counters().chunk_forgets_received, 1);
        assert_eq!(store.counters().chunks_forgotten, 0);
        assert_eq!(store.chunk_count(), 0);
    }

    #[test]
    fn extracts_all_terrain_chunks() {
        let mut store = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        store
            .insert_level_chunk_with_light(synthetic_local_palette_chunk_packet())
            .unwrap();

        assert_eq!(store.chunk_positions(), vec![ChunkPos { x: 2, z: -3 }]);
        let chunks = store.extract_terrain_chunks();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].pos, ChunkPos { x: 2, z: -3 });
        assert_eq!(chunks[0].summary().opaque_blocks, 4096);
    }

    #[test]
    fn classifies_basic_terrain_materials() {
        assert_eq!(
            classify_terrain_material(Some("minecraft:air")),
            TerrainMaterialClass::Empty
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:grass_block")),
            TerrainMaterialClass::Opaque
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:short_grass")),
            TerrainMaterialClass::Cutout
        );
        assert_eq!(
            classify_terrain_material(Some("minecraft:water")),
            TerrainMaterialClass::Fluid
        );
    }

    fn protocol_play_login(player_id: i32) -> ProtocolPlayLogin {
        ProtocolPlayLogin {
            player_id,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: ProtocolSpawnInfo {
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
        }
    }

    fn protocol_add_entity(id: i32) -> ProtocolAddEntity {
        protocol_add_entity_with_type(id, 7)
    }

    fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: -10.0,
            y_rot: 20.0,
            y_head_rot: 30.0,
            data: 99,
        }
    }

    fn item_stack_entity_data(item: ItemStackSummary) -> ProtocolEntityDataValue {
        ProtocolEntityDataValue {
            data_id: VANILLA_ITEM_ENTITY_STACK_DATA_ID,
            serializer_id: 7,
            value: EntityDataValueKind::ItemStack(item),
        }
    }

    fn item_stack(item_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: Default::default(),
        }
    }

    fn synthetic_level_chunk_packet() -> LevelChunkWithLight {
        let mut payload = Encoder::new();

        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_var_i32(1);
        payload.write_i64(42);

        let mut sections = Encoder::new();
        sections.write_i16(0);
        sections.write_i16(0);
        sections.write_u8(0);
        sections.write_var_i32(0);
        sections.write_u8(0);
        sections.write_var_i32(0);
        let sections = sections.into_inner();
        payload.write_var_i32(sections.len() as i32);
        payload.write_bytes(&sections);

        payload.write_var_i32(1);
        payload.write_u8(0xab);
        payload.write_i16(64);
        payload.write_var_i32(7);
        payload.write_u8(0);

        write_long_array(&mut payload, &[0b10]);
        write_long_array(&mut payload, &[0b100]);
        write_long_array(&mut payload, &[]);
        write_long_array(&mut payload, &[]);
        write_byte_arrays(&mut payload, &[&[1, 2]]);
        write_byte_arrays(&mut payload, &[&[3, 4]]);

        LevelChunkWithLight {
            x: 1,
            z: -2,
            raw_after_position: payload.into_inner(),
        }
    }

    fn synthetic_local_palette_chunk_packet() -> LevelChunkWithLight {
        let mut payload = Encoder::new();

        payload.write_var_i32(0);

        let mut sections = Encoder::new();
        sections.write_i16(4096);
        sections.write_i16(0);
        write_local_block_palette(&mut sections);
        sections.write_u8(0);
        sections.write_var_i32(4);
        let sections = sections.into_inner();
        payload.write_var_i32(sections.len() as i32);
        payload.write_bytes(&sections);

        payload.write_var_i32(0);
        write_long_array(&mut payload, &[]);
        write_long_array(&mut payload, &[]);
        write_long_array(&mut payload, &[]);
        write_long_array(&mut payload, &[]);
        write_byte_arrays(&mut payload, &[]);
        write_byte_arrays(&mut payload, &[]);

        LevelChunkWithLight {
            x: 2,
            z: -3,
            raw_after_position: payload.into_inner(),
        }
    }

    fn write_local_block_palette(out: &mut Encoder) {
        let target_index = section_block_index(2, 1, 3);
        let mut values = vec![0u64; 4096];
        values[target_index] = 2;

        out.write_u8(2);
        out.write_var_i32(3);
        out.write_var_i32(5);
        out.write_var_i32(7);
        out.write_var_i32(9);
        for value in pack_fixed_values(&values, 2) {
            out.write_i64(value as i64);
        }
    }

    fn single_biome_payload(biome_id: i32) -> Vec<u8> {
        let mut payload = Encoder::new();
        payload.write_u8(0);
        payload.write_var_i32(biome_id);
        payload.into_inner()
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

    fn pack_fixed_values(values: &[u64], bits_per_entry: usize) -> Vec<u64> {
        let values_per_long = 64 / bits_per_entry;
        let mut packed = vec![0; values.len().div_ceil(values_per_long)];
        let mask = (1u64 << bits_per_entry) - 1;
        for (index, value) in values.iter().copied().enumerate() {
            let cell_index = index / values_per_long;
            let bit_index = (index - cell_index * values_per_long) * bits_per_entry;
            packed[cell_index] |= (value & mask) << bit_index;
        }
        packed
    }

    fn set_light_nibble(layer: &mut [u8], nibble_index: usize, value: u8) {
        let byte = layer.get_mut(nibble_index / 2).unwrap();
        let shift = (nibble_index % 2) * 4;
        *byte = (*byte & !(0x0f << shift)) | ((value & 0x0f) << shift);
    }

    fn terrain_cell_index(x: usize, y: usize, z: usize, height: usize) -> usize {
        assert!(x < 16);
        assert!(y < height);
        assert!(z < 16);
        ((y * 16) + z) * 16 + x
    }

    fn light_update_payload(
        sky_y_mask: &[i64],
        block_y_mask: &[i64],
        empty_sky_y_mask: &[i64],
        empty_block_y_mask: &[i64],
        sky_updates: &[&[u8]],
        block_updates: &[&[u8]],
    ) -> Vec<u8> {
        let mut payload = Encoder::new();
        write_long_array(&mut payload, sky_y_mask);
        write_long_array(&mut payload, block_y_mask);
        write_long_array(&mut payload, empty_sky_y_mask);
        write_long_array(&mut payload, empty_block_y_mask);
        write_byte_arrays(&mut payload, sky_updates);
        write_byte_arrays(&mut payload, block_updates);
        payload.into_inner()
    }

    fn write_long_array(out: &mut Encoder, values: &[i64]) {
        out.write_var_i32(values.len() as i32);
        for value in values {
            out.write_i64(*value);
        }
    }

    fn write_byte_arrays(out: &mut Encoder, values: &[&[u8]]) {
        out.write_var_i32(values.len() as i32);
        for value in values {
            out.write_var_i32(value.len() as i32);
            out.write_bytes(value);
        }
    }
}
