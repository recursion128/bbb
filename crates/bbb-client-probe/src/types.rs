use std::{collections::BTreeMap, sync::Arc, time::Duration};

use bbb_client_assets::KnownPacksManager;
use bbb_client_net::{ConnectionOptions, ConnectionState};
use bbb_client_world::{
    BlockPos, BlockProbe, ChunkColumn, ChunkPos, ChunkState, LocalPlayerAbilitiesState,
    WorldCounters, WorldStore,
};
use serde::{Deserialize, Serialize};

pub const ENTITY_RUNTIME_PROBE_USERNAME: &str = "server-entity";

#[derive(Debug, Clone)]
pub struct ProbeOptions {
    pub connection: ConnectionOptions,
    pub probe_after_first_chunk_packets: usize,
    pub probe_interactions: bool,
    pub probe_operator: bool,
    pub probe_dimension_transitions: bool,
    pub probe_entity_runtime: bool,
    pub probe_container_menu: bool,
    pub probe_hold: Option<Duration>,
    pub known_packs_manager: Option<Arc<KnownPacksManager>>,
}

impl ProbeOptions {
    pub fn from_connection(connection: ConnectionOptions) -> Self {
        Self {
            connection,
            probe_after_first_chunk_packets: 0,
            probe_interactions: false,
            probe_operator: false,
            probe_dimension_transitions: false,
            probe_entity_runtime: false,
            probe_container_menu: false,
            probe_hold: None,
            known_packs_manager: None,
        }
    }
}

/// What a session that was asked to stay connected actually did.
///
/// A load driver needs sessions that overlap in time, and the only honest way
/// to say a session was still there is for the session itself to report that it
/// pumped the connection until the requested moment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HoldReport {
    pub requested_seconds: f64,
    /// Monotonic time from opening the connection to decoding its first terrain column.
    #[serde(default)]
    pub join_to_first_terrain_micros: Option<u128>,
    /// Clientbound packets handled while holding. Zero is legitimate on an idle
    /// server, so it is reported rather than required.
    pub packets: usize,
    /// The session reached the end of the hold. False means the server ended it
    /// early, which is what a capacity or timeout regression looks like.
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeReport {
    pub reached_state: ConnectionState,
    pub compression_threshold: Option<i32>,
    pub packets_seen: usize,
    #[serde(default)]
    pub unsupported_packets: usize,
    #[serde(default)]
    pub last_unsupported_packet_state: Option<String>,
    #[serde(default)]
    pub last_unsupported_packet_id: Option<i32>,
    #[serde(default)]
    pub last_unsupported_packet_len: Option<usize>,
    pub registries_seen: usize,
    #[serde(default)]
    pub registry_entries_seen: usize,
    #[serde(default)]
    pub registry_entries_with_data: usize,
    #[serde(default)]
    pub registry_entry_stubs: usize,
    #[serde(default)]
    pub registry_entry_payload_bytes: usize,
    #[serde(default)]
    pub registry_content_registries_tracked: usize,
    #[serde(default)]
    pub registry_content_packets_tracked: usize,
    #[serde(default)]
    pub registry_content_entries_tracked: usize,
    #[serde(default)]
    pub registry_duplicate_entries: usize,
    #[serde(default)]
    pub registry_duplicate_entry_ids_tracked: usize,
    #[serde(default)]
    pub last_registry_data_registry: Option<String>,
    #[serde(default)]
    pub last_registry_data_entry_count: usize,
    pub first_chunk: Option<ChunkPos>,
    pub first_chunk_summary: Option<ChunkProbeSummary>,
    pub first_chunk_center_block: Option<BlockProbe>,
    #[serde(default)]
    pub outward_walk: Option<OutwardWalkReport>,
    #[serde(default)]
    pub interaction_replay: Option<InteractionReplayReport>,
    #[serde(default)]
    pub peer_observation: Option<PeerObservationReport>,
    #[serde(default)]
    pub session_authority: Option<SessionAuthorityReport>,
    #[serde(default)]
    pub refusal_probes: Option<RefusalProbeReport>,
    /// What a second session holding operator permission found: what each
    /// command it ran answered with, and the command tree it was published.
    ///
    /// Absent when the probe was not asked to replay interactions, for the
    /// same reason the refusal probes are.
    #[serde(default)]
    pub operator_session: Option<OperatorSessionReport>,
    #[serde(default)]
    pub hold: Option<HoldReport>,
    /// The level the login said the session is in, as the client resolved it.
    ///
    /// Absent only for a session that never reached play.
    #[serde(default)]
    pub session_level: Option<SessionLevelReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration_catalog: Option<ConfigurationCatalogReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension_transitions: Option<DimensionTransitionsReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_runtime_replay: Option<EntityRuntimeReplayReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_menu: Option<ContainerMenuReport>,
    pub chunk_delivery: ChunkDeliveryReport,
    /// Every distinct play-phase serverbound wire id the replay put on the
    /// wire, across both of its sessions, in id order.
    ///
    /// This is what says which server mechanisms the replay actually drove:
    /// the matrix generator maps each id through the generated packet catalog
    /// to the Vanilla handler that owns it, so a row closes on a packet this
    /// run really sent rather than on a list somebody kept up to date by hand.
    #[serde(default)]
    pub serverbound_play_ids: Vec<i32>,
    pub world_counters: WorldCounters,
    #[serde(default)]
    pub world_apply_errors: Vec<String>,
    #[serde(skip)]
    pub world: WorldStore,
}

/// Dimension identity and resolved `minecraft:dimension_type` fields observed at join.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionLevelReport {
    /// `CommonPlayerSpawnInfo.dimension`, the level's own resource key.
    pub dimension: String,
    /// `CommonPlayerSpawnInfo.dimensionType` as it travelled: a registry index.
    pub dimension_type_id: i32,
    /// The registry element that index names, or the built-in profile's name
    /// when no registry arrived to resolve it against.
    pub dimension_type_name: Option<String>,
    /// `logical_height` of the resolved element (`DimensionType.java:94`).
    pub dimension_type_logical_height: i32,
    /// `infiniburn` of the resolved element (`DimensionType.java:95`).
    pub dimension_type_infiniburn_tag: Option<String>,
    /// `CommonPlayerSpawnInfo.seaLevel`, which is the generator's answer rather
    /// than the dimension type's.
    pub sea_level: i32,
}

impl SessionLevelReport {
    pub fn from_level_info(level: &bbb_client_world::WorldLevelInfo) -> Self {
        Self {
            dimension: level.dimension.clone(),
            dimension_type_id: level.dimension_type_id,
            dimension_type_name: level.dimension_type_name.clone(),
            dimension_type_logical_height: level.dimension_type_properties.logical_height,
            dimension_type_infiniburn_tag: level.dimension_type_properties.infiniburn_tag.clone(),
            sea_level: level.sea_level,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DimensionLevelReport {
    #[serde(flatten)]
    pub session: SessionLevelReport,
    pub min_y: i32,
    pub height: i32,
    pub is_debug: bool,
    pub is_flat: bool,
}

impl DimensionLevelReport {
    pub fn from_world(world: &WorldStore) -> Option<Self> {
        let level = world.level_info()?;
        let dimension = world.dimension();
        Some(Self {
            session: SessionLevelReport::from_level_info(level),
            min_y: dimension.min_y,
            height: dimension.height,
            is_debug: level.is_debug,
            is_flat: level.is_flat,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationCatalogReport {
    pub selected_known_packs: usize,
    pub packet_order: Vec<String>,
    pub registries: Vec<ConfigurationRegistryReport>,
    pub tags: Vec<ConfigurationRegistryTagsReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationRegistryReport {
    pub registry: String,
    pub entries: Vec<ConfigurationRegistryEntryReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationRegistryEntryReport {
    pub resource_id: String,
    pub canonical_nbt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationRegistryTagsReport {
    pub registry: String,
    pub tags: Vec<ConfigurationTagReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigurationTagReport {
    pub tag: String,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DimensionTransitionsReport {
    pub initial: DimensionLevelReport,
    pub legs: Vec<DimensionTransitionLegReport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DimensionTransitionLegReport {
    pub target_dimension: String,
    pub respawn_count_delta: usize,
    pub data_to_keep: i8,
    pub level: DimensionLevelReport,
    pub player_position: [f64; 3],
    pub milestone_order: Vec<String>,
    pub target_chunk: ChunkPos,
    pub first_chunk: ChunkPos,
    pub target_chunk_completed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityRuntimeReplayReport {
    pub staged_entity_types: Vec<String>,
    pub milestone_order: Vec<String>,
    pub general: GeneralEntityReplayReport,
    pub item_frame: ItemFrameReplayReport,
    pub painting: PaintingReplayReport,
    pub glow_item_frame_spawn_position: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneralEntityReplayReport {
    pub spawn_position: [f64; 3],
    pub moved_position: [f64; 3],
    pub equipped_item_id: i32,
    pub first_attack_event: i8,
    pub removed_after_second_attack: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemFrameReplayReport {
    pub spawn_position: [f64; 3],
    pub inserted_item_id: i32,
    pub rotation_after_second_interaction: i32,
    pub item_removed_after_first_attack: bool,
    pub removed_after_second_attack: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintingReplayReport {
    pub spawn_position: [f64; 3],
    pub removed_after_move: bool,
}

/// How many ticks the container menu session steps a frozen server through
/// after each packet it sends.
///
/// A frozen server still answers a packet where it reads it, but the block
/// event a chest lid crossing queues waits for `runBlockEvents`, which
/// `ServerLevel.tick` keeps behind the frozen gate (ServerLevel.java:388-391).
pub const CONTAINER_MENU_STEPPED_TICKS: usize = 5;

/// The three steps every container menu session publishes, in the order the
/// probe drives them.
pub const CONTAINER_MENU_STEP_LABELS: [&str; 3] = ["opened", "clicked", "closed"];

/// The container slot every session quick-moves out of the chest.
///
/// `ChestMenu` numbers its own grid before the player inventory
/// (ChestMenu.java:55-58), so menu slot 0 is always container slot 0.
pub const CONTAINER_MENU_QUICK_MOVED_SLOT: i16 = 0;

/// The chest each container menu session opens, in the order one connection
/// runs them.
///
/// Both are opened by the same session, which is what fixes their container
/// ids: `nextContainerCounter` is `containerCounter % 100 + 1` on the player
/// itself (ServerPlayer.java:1314-1316), so the first menu a fresh player
/// opens is 1 and the second is 2. The double chest is clicked on its `right`
/// half because that is the one `ChestBlock.getBlockType`
/// (ChestBlock.java:148-155) answers `FIRST` for, and `CompoundContainer`
/// numbers `first` ahead of `second` (CompoundContainer.java:31-33).
pub const CONTAINER_MENU_SESSIONS: [ContainerMenuSessionContract; 2] = [
    ContainerMenuSessionContract {
        name: "single_chest",
        block: [2, CONTAINER_MENU_SURFACE_Y, 0],
    },
    ContainerMenuSessionContract {
        name: "double_chest",
        block: [-2, CONTAINER_MENU_SURFACE_Y, 0],
    },
];

/// The height the harness stages the chests at: a flat world's layers are
/// bedrock, two dirt and grass from the build floor
/// (FlatLevelGeneratorSettings.java:217-219), so the grass is at y=-61 and a
/// block placed on the surface sits one above it.
const CONTAINER_MENU_SURFACE_Y: i32 = -60;

/// One staged chest the session opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerMenuSessionContract {
    pub name: &'static str,
    pub block: [i32; 3],
}

/// What the two chest menus said at each step of the session that drove them.
///
/// The whole menu is published rather than the moved stack alone: what
/// `server-container-menu-surface` compares is every container slot and every
/// player-inventory slot the menu holds, at each step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerMenuReport {
    pub stepped_ticks: usize,
    pub sessions: Vec<ContainerMenuSessionReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerMenuSessionReport {
    pub name: String,
    pub block: [i32; 3],
    pub menu_type: String,
    pub title_key: String,
    pub container_id: i32,
    pub steps: Vec<ContainerMenuStepReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerMenuStepReport {
    pub label: String,
    pub state_id: i32,
    pub container_id: i32,
    /// Every slot the menu holds, `<index>=<stack>` in slot order.
    pub slots: Vec<String>,
    pub carried: String,
    /// The chest lid block events the step's opener-count crossings queued,
    /// `<x>,<y>,<z>=<b0>/<b1>`.
    ///
    /// Crossings only. `ContainerOpenersCounter.recheckOpeners`
    /// calls `openerCountChanged`
    /// whether or not the count moved, and reschedules itself every five
    /// ticks through `ChestBlock.tick`, so an open
    /// chest republishes the same count for as long as it is open.
    pub lid_events: Vec<String>,
}

/// What a session found after walking out of the column it spawned in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutwardWalkReport {
    pub start_chunk: ChunkPos,
    pub walked_chunk: ChunkPos,
    pub walked_chunk_center_block: BlockProbe,
    pub walked_chunk_summary: ChunkProbeSummary,
}

pub const OUTWARD_WALK_CHUNKS: i32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefusalProbeReport {
    pub cookie_response: RefusalObservation,
    pub unsolicited_keep_alive: RefusalObservation,
    pub configuration_acknowledged: RefusalObservation,
    pub malformed_packet_body: RefusalObservation,
    pub malformed_component_nbt: RefusalObservation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefusalObservation {
    pub reached_play: bool,
    pub session_ended: bool,
    pub disconnect_reason: Option<String>,
}

impl RefusalObservation {
    pub fn completed(&self, reason: Option<&str>) -> bool {
        self.reached_play
            && self.session_ended
            && match reason {
                Some(reason) => self.disconnect_reason.as_deref() == Some(reason),
                None => true,
            }
    }
}

impl RefusalProbeReport {
    pub fn completed(&self) -> bool {
        self.cookie_response
            .completed(Some(REFUSED_COOKIE_RESPONSE_REASON))
            && self
                .unsolicited_keep_alive
                .completed(Some(REFUSED_KEEP_ALIVE_REASON))
            && self.configuration_acknowledged.completed(None)
            && self.malformed_packet_body.completed(None)
            && self.malformed_component_nbt.completed(None)
    }
}

// ServerCommonPacketListenerImpl.java:37-38, :81.
pub const REFUSED_COOKIE_RESPONSE_REASON: &str = "multiplayer.disconnect.unexpected_query_response";
pub const REFUSED_KEEP_ALIVE_REASON: &str = "disconnect.timeout";
pub const REFUSED_COOKIE_KEY: &str = "bbb:refusal_probe";
pub const REFUSED_KEEP_ALIVE_ID: i64 = -1;
pub const MALFORMED_BODY_MESSAGE: &str = "bbb malformed body probe";
pub const MALFORMED_BODY_TIMESTAMP_MILLIS: i64 = 0;
pub const MALFORMED_BODY_SALT: i64 = 0;
pub const MALFORMED_NBT_SLOT: i16 = 44;
pub const MALFORMED_NBT_ENTRY_NAME: &str = "bbb";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeerObservationReport {
    pub name: String,
    pub listed: bool,
    pub game_mode: String,
    pub show_hat: bool,
    pub entity_type_id: i32,
    pub removed_from_player_list: bool,
    pub entity_removed: bool,
    pub shared_flags: PeerSharedFlagsReport,
    pub sounds_heard: Vec<SoundObservation>,
}

mod interaction_replay;
pub use interaction_replay::*;

#[cfg(test)]
mod compatibility_tests {
    use super::{DimensionLevelReport, SessionLevelReport};

    #[test]
    fn ordinary_probe_session_level_keeps_legacy_json_shape() {
        let legacy = serde_json::json!({
            "dimension": "minecraft:overworld",
            "dimension_type_id": 0,
            "dimension_type_name": "minecraft:overworld",
            "dimension_type_logical_height": 384,
            "dimension_type_infiniburn_tag": "minecraft:infiniburn_overworld",
            "sea_level": 63,
        });
        let report: SessionLevelReport = serde_json::from_value(legacy).unwrap();
        let value = serde_json::to_value(report).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 6);
        assert!(value.get("min_y").is_none());
        assert!(value.get("height").is_none());
        assert!(value.get("is_debug").is_none());
        assert!(value.get("is_flat").is_none());
    }

    #[test]
    fn dimension_level_report_flattens_transition_only_fields() {
        let value = serde_json::to_value(DimensionLevelReport {
            session: SessionLevelReport {
                dimension: "minecraft:overworld".to_string(),
                dimension_type_id: 0,
                dimension_type_name: Some("minecraft:overworld".to_string()),
                dimension_type_logical_height: 384,
                dimension_type_infiniburn_tag: Some("minecraft:infiniburn_overworld".to_string()),
                sea_level: 63,
            },
            min_y: -64,
            height: 384,
            is_debug: false,
            is_flat: false,
        })
        .unwrap();
        assert_eq!(value.as_object().unwrap().len(), 10);
        assert_eq!(value["min_y"], -64);
        assert_eq!(value["height"], 384);
        assert_eq!(value["is_debug"], false);
        assert_eq!(value["is_flat"], false);
    }
}
