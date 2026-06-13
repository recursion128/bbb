use std::collections::BTreeMap;

use bbb_protocol::codec::ProtocolError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod block_events;
mod chunks;
mod client_hud;
mod command_suggestions;
mod entities;
mod entity_status;
mod inventory;
mod level;
mod player_info;
mod registries;
mod scoreboard;
mod server_presentation;
mod terrain;
mod world_border;

pub use block_events::{BlockDestructionProgress, BlockEventRecord, LevelEventRecord};
pub use chunks::{
    decode_level_chunk_with_light, BlockEntityRecord, ChunkColumn, ChunkSection, ChunkState,
    HeightmapData, LightData, NbtPayloadSummary, PaletteDomain, PaletteKind, PaletteValue,
    PalettedContainerData,
};
pub use client_hud::{BossBarState, ClientHudState, DifficultyState, TabListState};
pub use command_suggestions::{
    CommandSuggestionState, CommandSuggestionsResultState, CommandSuggestionsState,
};
pub use entities::{EntityState, EntityVec3, VehicleMoveReport};
pub use entity_status::{EntityDamageEventState, ItemCooldownState, MobEffectState};
pub use inventory::{
    ContainerDataValue, ContainerSlot, ContainerState, InventorySlot, InventoryState,
};
pub use level::{WorldDimension, WorldLevelInfo};
pub use player_info::{PlayerInfoEntryState, PlayerInfoProfileState, PlayerInfoState};
pub use registries::{BlockStateInfo, BlockStateRegistry, RegistryPacket, RegistrySet};
pub use scoreboard::{
    ScoreboardObjective, ScoreboardScore, ScoreboardState, ScoreboardTeam, ScoreboardTeamParameters,
};
pub use server_presentation::{ResourcePackState, ServerDataState, ServerPresentationState};
pub use terrain::{
    BlockProbe, TerrainBlockCell, TerrainChunkSnapshot, TerrainChunkSummary, TerrainLight,
    TerrainMaterialClass,
};
pub use world_border::WorldBorderState;

#[cfg(test)]
use chunks::sample_terrain_light;

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
    #[serde(default)]
    pub world_border_initializes_received: usize,
    #[serde(default)]
    pub world_border_center_updates_received: usize,
    #[serde(default)]
    pub world_border_lerp_size_updates_received: usize,
    #[serde(default)]
    pub world_border_size_updates_received: usize,
    #[serde(default)]
    pub world_border_warning_delay_updates_received: usize,
    #[serde(default)]
    pub world_border_warning_distance_updates_received: usize,
    #[serde(default)]
    pub reset_score_packets: usize,
    #[serde(default)]
    pub set_display_objective_packets: usize,
    #[serde(default)]
    pub set_objective_packets: usize,
    #[serde(default)]
    pub set_player_team_packets: usize,
    #[serde(default)]
    pub set_score_packets: usize,
    #[serde(default)]
    pub boss_event_packets: usize,
    #[serde(default)]
    pub boss_bars_tracked: usize,
    #[serde(default)]
    pub tab_list_packets: usize,
    #[serde(default)]
    pub change_difficulty_packets: usize,
    #[serde(default)]
    pub player_info_update_packets: usize,
    #[serde(default)]
    pub player_info_remove_packets: usize,
    #[serde(default)]
    pub player_info_entries_tracked: usize,
    #[serde(default)]
    pub listed_players_tracked: usize,
    #[serde(default)]
    pub server_data_packets: usize,
    #[serde(default)]
    pub resource_pack_push_packets: usize,
    #[serde(default)]
    pub resource_pack_pop_packets: usize,
    #[serde(default)]
    pub resource_packs_tracked: usize,
    #[serde(default)]
    pub cooldown_packets: usize,
    #[serde(default)]
    pub cooldowns_tracked: usize,
    #[serde(default)]
    pub update_mob_effect_packets: usize,
    #[serde(default)]
    pub remove_mob_effect_packets: usize,
    #[serde(default)]
    pub active_mob_effects_tracked: usize,
    #[serde(default)]
    pub damage_event_packets: usize,
    #[serde(default)]
    pub damage_events_applied: usize,
    #[serde(default)]
    pub command_suggestion_packets: usize,
    #[serde(default)]
    pub command_suggestion_entries_tracked: usize,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldStore {
    dimension: WorldDimension,
    level: Option<WorldLevelInfo>,
    #[serde(default)]
    world_border: WorldBorderState,
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
    scoreboard: ScoreboardState,
    #[serde(default)]
    client_hud: ClientHudState,
    #[serde(default)]
    player_info: PlayerInfoState,
    #[serde(default)]
    presentation: ServerPresentationState,
    #[serde(default)]
    cooldowns: BTreeMap<String, ItemCooldownState>,
    #[serde(default)]
    command_suggestions: CommandSuggestionsState,
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

    pub fn counters(&self) -> WorldCounters {
        self.counters.clone()
    }
}

pub(crate) fn section_block_index(x: u8, y: u8, z: u8) -> usize {
    ((y as usize) << 8) | ((z as usize) << 4) | x as usize
}

pub(crate) fn section_biome_index(x: u8, y: u8, z: u8) -> usize {
    ((y as usize) << 4) | ((z as usize) << 2) | x as usize
}

pub(crate) fn protocol_block_pos(pos: bbb_protocol::packets::BlockPos) -> BlockPos {
    BlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

#[cfg(test)]
mod tests {
    use super::chunks::LIGHT_ARRAY_BYTES;
    use super::entities::{
        VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
        VANILLA_ITEM_ENTITY_STACK_DATA_ID,
    };
    use super::*;
    use std::collections::BTreeSet;

    use bbb_protocol::codec::Encoder;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, AttributeModifier as ProtocolAttributeModifier,
        AttributeSnapshot as ProtocolAttributeSnapshot,
        BlockDestruction as ProtocolBlockDestruction, BlockEntityData as ProtocolBlockEntityData,
        BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos,
        BlockUpdate as ProtocolBlockUpdate, ChunkBiomeData as ProtocolChunkBiomeData,
        ChunkPos as ProtocolChunkPos, ChunksBiomes as ProtocolChunksBiomes,
        CommonPlayerSpawnInfo as ProtocolSpawnInfo, EntityAnimation as ProtocolEntityAnimation,
        EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
        EntityEvent as ProtocolEntityEvent, EntityMove as ProtocolEntityMove,
        EntityPositionSync as ProtocolEntityPositionSync, EquipmentSlot, EquipmentSlotUpdate,
        GameProfile as ProtocolGameProfile, GameProfileProperty as ProtocolGameProfileProperty,
        GameType as ProtocolGameType, HurtAnimation as ProtocolHurtAnimation, ItemStackSummary,
        LevelChunkWithLight, LevelEvent as ProtocolLevelEvent, LightUpdate as ProtocolLightUpdate,
        MoveVehicle as ProtocolMoveVehicle, PlayLogin as ProtocolPlayLogin,
        PlayerInfoAction as ProtocolPlayerInfoAction,
        PlayerInfoChatSession as ProtocolPlayerInfoChatSession,
        PlayerInfoEntry as ProtocolPlayerInfoEntry, PlayerInfoRemove as ProtocolPlayerInfoRemove,
        PlayerInfoUpdate as ProtocolPlayerInfoUpdate, RemoveEntities as ProtocolRemoveEntities,
        ResourcePackPop as ProtocolResourcePackPop, ResourcePackPush as ProtocolResourcePackPush,
        Respawn as ProtocolRespawn, RotateHead as ProtocolRotateHead,
        SectionBlocksUpdate as ProtocolSectionBlocksUpdate, ServerData as ProtocolServerData,
        SetEntityData as ProtocolSetEntityData, SetEntityLink as ProtocolSetEntityLink,
        SetEntityMotion as ProtocolSetEntityMotion, SetEquipment as ProtocolSetEquipment,
        SetPassengers as ProtocolSetPassengers, TakeItemEntity as ProtocolTakeItemEntity,
        TeleportEntity as ProtocolTeleportEntity, UpdateAttributes as ProtocolUpdateAttributes,
        Vec3d as ProtocolVec3d, PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_X,
    };
    use uuid::Uuid;

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
    fn server_data_stores_motd_icon_and_counter() {
        let mut store = WorldStore::new();

        store.apply_server_data(ProtocolServerData {
            motd: "Welcome to BBB".to_string(),
            icon_bytes: Some(vec![137, 80, 78, 71]),
        });

        let server_data = store.server_data().expect("server data is stored");
        assert_eq!(server_data.motd, "Welcome to BBB");
        assert_eq!(server_data.icon_byte_len(), Some(4));
        assert_eq!(
            server_data.icon_bytes.as_deref(),
            Some(&[137, 80, 78, 71][..])
        );
        assert_eq!(store.counters().server_data_packets, 1);
    }

    #[test]
    fn resource_pack_push_stores_and_upserts_by_id() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0x11111111111111111111111111111111);

        store.apply_resource_pack_push(protocol_resource_pack_push(
            id,
            "https://example.test/first.zip",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            false,
            Some("Use server pack?"),
        ));
        store.apply_resource_pack_push(protocol_resource_pack_push(
            id,
            "https://example.test/second.zip",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            true,
            None,
        ));

        let pack = store.resource_pack(id).expect("pack is tracked");
        assert_eq!(pack.url, "https://example.test/second.zip");
        assert_eq!(pack.hash, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        assert!(pack.required);
        assert_eq!(pack.prompt, None);
        assert_eq!(store.resource_packs().len(), 1);
        let counters = store.counters();
        assert_eq!(counters.resource_pack_push_packets, 2);
        assert_eq!(counters.resource_packs_tracked, 1);
    }

    #[test]
    fn resource_pack_pop_removes_one_pack_by_id() {
        let mut store = WorldStore::new();
        let first = Uuid::from_u128(0x11111111111111111111111111111111);
        let second = Uuid::from_u128(0x22222222222222222222222222222222);

        store.apply_resource_pack_push(protocol_resource_pack_push(
            first,
            "https://example.test/first.zip",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            false,
            None,
        ));
        store.apply_resource_pack_push(protocol_resource_pack_push(
            second,
            "https://example.test/second.zip",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            false,
            None,
        ));

        assert_eq!(
            store.apply_resource_pack_pop(ProtocolResourcePackPop { id: Some(first) }),
            1
        );
        assert!(store.resource_pack(first).is_none());
        assert!(store.resource_pack(second).is_some());
        let counters = store.counters();
        assert_eq!(counters.resource_pack_pop_packets, 1);
        assert_eq!(counters.resource_packs_tracked, 1);
    }

    #[test]
    fn resource_pack_pop_without_id_clears_all_packs() {
        let mut store = WorldStore::new();
        let first = Uuid::from_u128(0x11111111111111111111111111111111);
        let second = Uuid::from_u128(0x22222222222222222222222222222222);

        store.apply_resource_pack_push(protocol_resource_pack_push(
            first,
            "https://example.test/first.zip",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            false,
            None,
        ));
        store.apply_resource_pack_push(protocol_resource_pack_push(
            second,
            "https://example.test/second.zip",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            true,
            Some("Required pack"),
        ));

        assert_eq!(
            store.apply_resource_pack_pop(ProtocolResourcePackPop { id: None }),
            2
        );
        assert!(store.resource_packs().is_empty());
        let counters = store.counters();
        assert_eq!(counters.resource_pack_pop_packets, 1);
        assert_eq!(counters.resource_packs_tracked, 0);
    }

    #[test]
    fn player_info_adds_player_with_profile_and_fields() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let mut entry = protocol_player_info_entry(id);
        entry.profile = Some(protocol_game_profile(id, "Ada"));
        entry.listed = true;
        entry.latency = 42;
        entry.game_mode = ProtocolGameType::Creative;
        entry.display_name = Some("{\"text\":\"Ada Lovelace\"}".to_string());
        entry.show_hat = true;
        entry.list_order = 7;
        entry.chat_session = Some(protocol_player_info_chat_session());

        let applied = store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::InitializeChat,
                ProtocolPlayerInfoAction::UpdateGameMode,
                ProtocolPlayerInfoAction::UpdateListed,
                ProtocolPlayerInfoAction::UpdateLatency,
                ProtocolPlayerInfoAction::UpdateDisplayName,
                ProtocolPlayerInfoAction::UpdateHat,
                ProtocolPlayerInfoAction::UpdateListOrder,
            ],
            entries: vec![entry],
        });

        assert_eq!(applied, 1);
        let info = store.player_info_entry(id).unwrap();
        assert_eq!(info.profile.uuid, id);
        assert_eq!(info.profile.name, "Ada");
        assert_eq!(info.profile.properties.len(), 1);
        assert!(info.listed);
        assert_eq!(info.latency, 42);
        assert_eq!(info.game_mode, "creative");
        assert_eq!(
            info.display_name.as_deref(),
            Some("{\"text\":\"Ada Lovelace\"}")
        );
        assert!(info.show_hat);
        assert_eq!(info.list_order, 7);
        assert!(info.chat_session_present);
        assert_eq!(store.listed_players(), &BTreeSet::from([id]));

        let counters = store.counters();
        assert_eq!(counters.player_info_update_packets, 1);
        assert_eq!(counters.player_info_entries_tracked, 1);
        assert_eq!(counters.listed_players_tracked, 1);
    }

    #[test]
    fn player_info_update_ignores_unknown_uuid() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb);
        let mut entry = protocol_player_info_entry(id);
        entry.listed = true;
        entry.latency = 99;

        let applied = store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::UpdateListed,
                ProtocolPlayerInfoAction::UpdateLatency,
            ],
            entries: vec![entry],
        });

        assert_eq!(applied, 0);
        assert!(store.player_info().entries.is_empty());
        assert!(store.listed_players().is_empty());
        assert_eq!(store.counters().player_info_update_packets, 1);
        assert_eq!(store.counters().player_info_entries_tracked, 0);
        assert_eq!(store.counters().listed_players_tracked, 0);
    }

    #[test]
    fn player_info_remove_clears_entry_and_listed_tracking() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0xcccccccccccccccccccccccccccccccc);
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::UpdateListed,
            ],
            entries: vec![listed_player_info_entry(id, "Grace", true)],
        });

        assert!(store.player_info_entry(id).is_some());
        assert!(store.listed_players().contains(&id));

        let removed = store.apply_player_info_remove(ProtocolPlayerInfoRemove {
            profile_ids: vec![id],
        });

        assert_eq!(removed, 1);
        assert!(store.player_info_entry(id).is_none());
        assert!(store.listed_players().is_empty());
        let counters = store.counters();
        assert_eq!(counters.player_info_remove_packets, 1);
        assert_eq!(counters.player_info_entries_tracked, 0);
        assert_eq!(counters.listed_players_tracked, 0);
    }

    #[test]
    fn player_info_listed_false_removes_from_listed_set() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0xdddddddddddddddddddddddddddddddd);
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::UpdateListed,
            ],
            entries: vec![listed_player_info_entry(id, "Katherine", true)],
        });
        assert!(store.listed_players().contains(&id));

        let mut unlisted = protocol_player_info_entry(id);
        unlisted.listed = false;
        let applied = store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![ProtocolPlayerInfoAction::UpdateListed],
            entries: vec![unlisted],
        });

        assert_eq!(applied, 1);
        assert!(!store.player_info_entry(id).unwrap().listed);
        assert!(store.listed_players().is_empty());
        assert_eq!(store.counters().listed_players_tracked, 0);
    }

    #[test]
    fn player_info_chat_session_present_flag_can_set_and_clear() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee);
        let mut with_chat = listed_player_info_entry(id, "Margaret", false);
        with_chat.chat_session = Some(protocol_player_info_chat_session());
        store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![
                ProtocolPlayerInfoAction::AddPlayer,
                ProtocolPlayerInfoAction::InitializeChat,
            ],
            entries: vec![with_chat],
        });
        assert!(store.player_info_entry(id).unwrap().chat_session_present);

        let mut without_chat = protocol_player_info_entry(id);
        without_chat.chat_session = None;
        let applied = store.apply_player_info_update(ProtocolPlayerInfoUpdate {
            actions: vec![ProtocolPlayerInfoAction::InitializeChat],
            entries: vec![without_chat],
        });

        assert_eq!(applied, 1);
        assert!(!store.player_info_entry(id).unwrap().chat_session_present);
        assert_eq!(store.counters().player_info_update_packets, 2);
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

    fn protocol_resource_pack_push(
        id: Uuid,
        url: &str,
        hash: &str,
        required: bool,
        prompt: Option<&str>,
    ) -> ProtocolResourcePackPush {
        ProtocolResourcePackPush {
            id,
            url: url.to_string(),
            hash: hash.to_string(),
            required,
            prompt: prompt.map(str::to_string),
        }
    }

    fn protocol_game_profile(uuid: Uuid, name: &str) -> ProtocolGameProfile {
        ProtocolGameProfile {
            uuid,
            name: name.to_string(),
            properties: vec![ProtocolGameProfileProperty {
                name: "textures".to_string(),
                value: "skin-payload".to_string(),
                signature: Some("skin-signature".to_string()),
            }],
        }
    }

    fn protocol_player_info_entry(profile_id: Uuid) -> ProtocolPlayerInfoEntry {
        ProtocolPlayerInfoEntry {
            profile_id,
            profile: None,
            listed: false,
            latency: 0,
            game_mode: ProtocolGameType::default(),
            display_name: None,
            show_hat: false,
            list_order: 0,
            chat_session: None,
        }
    }

    fn listed_player_info_entry(
        profile_id: Uuid,
        name: &str,
        listed: bool,
    ) -> ProtocolPlayerInfoEntry {
        let mut entry = protocol_player_info_entry(profile_id);
        entry.profile = Some(protocol_game_profile(profile_id, name));
        entry.listed = listed;
        entry
    }

    fn protocol_player_info_chat_session() -> ProtocolPlayerInfoChatSession {
        ProtocolPlayerInfoChatSession {
            session_id: Uuid::from_u128(0x12345678123456781234567812345678),
            expires_at_epoch_millis: 1_700_000_000_000,
            public_key: vec![1, 2, 3],
            key_signature: vec![4, 5, 6],
        }
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
