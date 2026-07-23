use std::{
    collections::{BTreeMap, BTreeSet},
    ops::{Deref, DerefMut},
};

use bbb_protocol::debug_subscription_types::{
    vanilla_debug_game_event_type_by_id, vanilla_debug_poi_type_by_id,
    vanilla_debug_subscription_type_by_id, VANILLA_DEBUG_SUBSCRIPTION_BEES_ID,
    VANILLA_DEBUG_SUBSCRIPTION_BEE_HIVES_ID, VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_BREEZES_ID,
    VANILLA_DEBUG_SUBSCRIPTION_ENTITY_BLOCK_INTERSECTIONS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID, VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID, VANILLA_DEBUG_SUBSCRIPTION_NEIGHBOR_UPDATES_ID,
    VANILLA_DEBUG_SUBSCRIPTION_POIS_ID, VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_REDSTONE_WIRE_ORIENTATIONS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_STRUCTURES_ID, VANILLA_DEBUG_SUBSCRIPTION_VILLAGE_SECTIONS_ID,
};
use bbb_protocol::packets::{
    DebugBeeInfo, DebugBlockValue as ProtocolDebugBlockValue, DebugBoundingBox, DebugBrainDump,
    DebugBreezeInfo, DebugChunkValue as ProtocolDebugChunkValue, DebugEntityBlockIntersection,
    DebugEntityValue as ProtocolDebugEntityValue, DebugEvent as ProtocolDebugEvent,
    DebugGameEventListenerInfo, DebugGoalInfo, DebugPathInfo, DebugRedstoneOrientation,
    DebugSample as ProtocolDebugSample, DebugStructureInfo,
    DebugSubscriptionUpdate as ProtocolDebugSubscriptionUpdate,
    DebugSubscriptionValue as ProtocolDebugSubscriptionValue,
    GameTestHighlightPos as ProtocolGameTestHighlightPos,
    TestInstanceBlockStatus as ProtocolTestInstanceBlockStatus, Vec3i as ProtocolVec3i,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::{protocol_block_pos, BlockPos, ChunkPos, WorldStore};

use super::game_rules::GameRuleValuesState;

pub const GAME_TEST_HIGHLIGHT_DURATION_MILLIS: u64 = 10_000;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ClientDebugGameState {
    #[serde(default)]
    pub requested_subscriptions: BTreeSet<i32>,
    #[serde(default)]
    pub values_by_subscription: BTreeMap<i32, DebugSubscriptionValuesState>,
    #[serde(default)]
    pub last_debug_block_value: Option<DebugBlockValueState>,
    #[serde(default)]
    pub last_debug_chunk_value: Option<DebugChunkValueState>,
    #[serde(default)]
    pub last_debug_entity_value: Option<DebugEntityValueState>,
    #[serde(default)]
    pub last_debug_event: Option<DebugEventState>,
    #[serde(default)]
    pub last_debug_sample: Option<DebugSampleState>,
    #[serde(default)]
    pub last_game_rule_values: Option<GameRuleValuesState>,
    #[serde(default)]
    pub last_game_test_highlight_pos: Option<GameTestHighlightPosState>,
    #[serde(default)]
    pub game_test_highlight_markers: DebugValueMap<BlockPos, GameTestHighlightMarkerState>,
    #[serde(default)]
    pub last_test_instance_block_status: Option<TestInstanceBlockStatusState>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DebugSubscriptionValuesState {
    #[serde(default)]
    pub chunk_values: DebugValueMap<ChunkPos, DebugTimedValueState>,
    #[serde(default)]
    pub block_values: DebugValueMap<BlockPos, DebugTimedValueState>,
    #[serde(default)]
    pub entity_values: DebugValueMap<Uuid, DebugTimedValueState>,
    #[serde(default)]
    pub events: Vec<DebugTimedValueState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DebugValueMap<K, V>(BTreeMap<K, V>);

impl<K, V> Default for DebugValueMap<K, V> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<K, V> Deref for DebugValueMap<K, V> {
    type Target = BTreeMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V> DerefMut for DebugValueMap<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K: Serialize, V: Serialize> Serialize for DebugValueMap<K, V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.0.iter())
    }
}

impl<'de, K, V> Deserialize<'de> for DebugValueMap<K, V>
where
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<(K, V)>::deserialize(deserializer).map(|values| Self(values.into_iter().collect()))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugTimedValueState {
    pub value: DebugSubscriptionValueState,
    pub expires_after_game_time: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DebugSubscriptionValueState {
    Bee(DebugBeeInfo),
    Brain(DebugBrainDump),
    Breeze(DebugBreezeInfo),
    Goals(DebugGoalInfo),
    Path(DebugPathInfo),
    EntityBlockIntersection(DebugEntityBlockIntersection),
    Hive(DebugHiveInfoState),
    Poi(DebugPoiInfoState),
    RedstoneOrientation(DebugRedstoneOrientation),
    VillageSection,
    Raids(Vec<BlockPos>),
    Structures(Vec<DebugStructureInfoState>),
    GameEventListener(DebugGameEventListenerInfo),
    NeighborUpdate(BlockPos),
    GameEvent(DebugGameEventInfoState),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugHiveInfoState {
    pub block_resource_id: String,
    pub occupant_count: i32,
    pub honey_level: i32,
    pub sedated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugPoiInfoState {
    pub pos: BlockPos,
    pub poi_type_resource_id: String,
    pub free_ticket_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugStructureInfoState {
    pub bounding_box: DebugBoundingBoxState,
    pub pieces: Vec<DebugStructurePieceState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugStructurePieceState {
    pub bounding_box: DebugBoundingBoxState,
    pub is_start: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBoundingBoxState {
    pub min: BlockPos,
    pub max: BlockPos,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DebugGameEventInfoState {
    pub game_event_resource_id: String,
    pub pos: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid debug subscription id {subscription_id}")]
pub struct InvalidDebugSubscriptionId {
    pub subscription_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugBlockValueState {
    pub pos: BlockPos,
    #[serde(default)]
    pub subscription_id: i32,
    #[serde(default)]
    pub value_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugChunkValueState {
    pub pos: ChunkPos,
    #[serde(default)]
    pub subscription_id: i32,
    #[serde(default)]
    pub value_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugEntityValueState {
    pub entity_id: i32,
    #[serde(default)]
    pub subscription_id: i32,
    #[serde(default)]
    pub value_present: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugEventState {
    #[serde(default)]
    pub subscription_id: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugSampleState {
    pub sample_len: usize,
    pub sample_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTestHighlightPosState {
    pub absolute_pos: BlockPos,
    pub relative_pos: BlockPos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameTestHighlightMarkerState {
    pub relative_pos: BlockPos,
    pub remove_at_monotonic_millis: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestInstanceBlockStatusState {
    pub status: bbb_protocol::RichTextComponent,
    pub size: Option<DebugVec3iState>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugVec3iState {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl WorldStore {
    pub fn apply_debug_block_value(&mut self, packet: ProtocolDebugBlockValue) {
        self.counters.debug_block_value_packets += 1;
        let pos = protocol_block_pos(packet.pos);
        self.client_debug_game.last_debug_block_value = Some(DebugBlockValueState {
            pos,
            subscription_id: packet.update.subscription_id,
            value_present: packet.update.value.is_some(),
        });
        self.apply_debug_subscription_update(DebugValueKey::Block(pos), packet.update);
    }

    pub fn apply_debug_chunk_value(&mut self, packet: ProtocolDebugChunkValue) {
        self.counters.debug_chunk_value_packets += 1;
        let pos = ChunkPos {
            x: packet.pos.x,
            z: packet.pos.z,
        };
        self.client_debug_game.last_debug_chunk_value = Some(DebugChunkValueState {
            pos,
            subscription_id: packet.update.subscription_id,
            value_present: packet.update.value.is_some(),
        });
        self.apply_debug_subscription_update(DebugValueKey::Chunk(pos), packet.update);
    }

    pub fn apply_debug_entity_value(&mut self, packet: ProtocolDebugEntityValue) {
        self.counters.debug_entity_value_packets += 1;
        self.client_debug_game.last_debug_entity_value = Some(DebugEntityValueState {
            entity_id: packet.entity_id,
            subscription_id: packet.update.subscription_id,
            value_present: packet.update.value.is_some(),
        });
        let Some(uuid) = self.entities.uuid_for_protocol_id(packet.entity_id) else {
            return;
        };
        self.apply_debug_subscription_update(DebugValueKey::Entity(uuid), packet.update);
    }

    pub fn apply_debug_event(&mut self, packet: ProtocolDebugEvent) {
        self.counters.debug_event_packets += 1;
        self.client_debug_game.last_debug_event = Some(DebugEventState {
            subscription_id: packet.event.subscription_id,
        });
        if !self
            .client_debug_game
            .values_by_subscription
            .contains_key(&packet.event.subscription_id)
        {
            return;
        }
        let subscription_id = packet.event.subscription_id;
        let value =
            match self.canonical_debug_subscription_value(subscription_id, packet.event.value) {
                Ok(value) => value,
                Err(error) => {
                    self.record_apply_error("debug_subscription_value", &error);
                    return;
                }
            };
        let timed = debug_timed_value(subscription_id, self.debug_game_time(), value);
        self.client_debug_game
            .values_by_subscription
            .get_mut(&subscription_id)
            .expect("requested debug subscription must own initialized value maps")
            .events
            .push(timed);
    }

    pub fn set_requested_debug_subscriptions(
        &mut self,
        subscription_ids: &[i32],
    ) -> Result<bool, InvalidDebugSubscriptionId> {
        let requested = subscription_ids.iter().copied().collect::<BTreeSet<_>>();
        if let Some(subscription_id) = requested
            .iter()
            .copied()
            .find(|id| vanilla_debug_subscription_type_by_id(*id).is_none())
        {
            return Err(InvalidDebugSubscriptionId { subscription_id });
        }
        if requested == self.client_debug_game.requested_subscriptions {
            return Ok(false);
        }
        self.client_debug_game
            .values_by_subscription
            .retain(|id, _| requested.contains(id));
        for id in &requested {
            self.client_debug_game
                .values_by_subscription
                .entry(*id)
                .or_default();
        }
        self.client_debug_game.requested_subscriptions = requested;
        Ok(true)
    }

    pub fn requested_debug_subscriptions(&self) -> &BTreeSet<i32> {
        &self.client_debug_game.requested_subscriptions
    }

    pub fn debug_subscription_values(
        &self,
        subscription_id: i32,
    ) -> Option<&DebugSubscriptionValuesState> {
        self.client_debug_game
            .values_by_subscription
            .get(&subscription_id)
    }

    pub fn tick_debug_subscriber(&mut self, game_time: i64) {
        for (subscription_id, values) in &mut self.client_debug_game.values_by_subscription {
            let expires = vanilla_debug_subscription_type_by_id(*subscription_id)
                .is_some_and(|subscription| subscription.expire_after_ticks != 0);
            if !expires {
                continue;
            }
            values
                .chunk_values
                .retain(|_, value| !debug_value_expired(value, game_time));
            values
                .block_values
                .retain(|_, value| !debug_value_expired(value, game_time));
            values
                .entity_values
                .retain(|_, value| !debug_value_expired(value, game_time));
            values
                .events
                .retain(|value| !debug_value_expired(value, game_time));
        }
    }

    pub fn clear_requested_debug_subscriptions(&mut self) {
        self.client_debug_game.requested_subscriptions.clear();
        self.client_debug_game.values_by_subscription.clear();
    }

    /// Drops the connection-owned remote subscriber without resetting packet
    /// counters or the independent debug overlay diagnostics.
    pub fn clear_debug_subscriber(&mut self) {
        self.clear_requested_debug_subscriptions();
        self.clear_debug_subscriber_values();
    }

    pub(crate) fn drop_debug_chunk(&mut self, pos: ChunkPos) {
        for values in self.client_debug_game.values_by_subscription.values_mut() {
            values.chunk_values.remove(&pos);
            values.block_values.retain(|block_pos, _| {
                block_pos.x.div_euclid(16) != pos.x || block_pos.z.div_euclid(16) != pos.z
            });
        }
    }

    pub(crate) fn drop_debug_entity_uuid(&mut self, uuid: Uuid) {
        for values in self.client_debug_game.values_by_subscription.values_mut() {
            values.entity_values.remove(&uuid);
        }
    }

    pub fn apply_debug_sample(&mut self, packet: ProtocolDebugSample) {
        self.counters.debug_sample_packets += 1;
        self.client_debug_game.last_debug_sample = Some(DebugSampleState {
            sample_len: packet.sample.len(),
            sample_type: packet.sample_type.as_str().to_string(),
        });
    }

    pub fn apply_game_test_highlight_pos(&mut self, packet: ProtocolGameTestHighlightPos) {
        self.apply_game_test_highlight_pos_at(packet, 0);
    }

    pub fn apply_game_test_highlight_pos_at(
        &mut self,
        packet: ProtocolGameTestHighlightPos,
        now_monotonic_millis: u64,
    ) {
        self.counters.game_test_highlight_pos_packets += 1;
        let state = GameTestHighlightPosState {
            absolute_pos: protocol_block_pos(packet.absolute_pos),
            relative_pos: protocol_block_pos(packet.relative_pos),
        };
        self.client_debug_game.last_game_test_highlight_pos = Some(state.clone());
        self.client_debug_game.game_test_highlight_markers.insert(
            state.absolute_pos,
            GameTestHighlightMarkerState {
                relative_pos: state.relative_pos,
                remove_at_monotonic_millis: now_monotonic_millis
                    .saturating_add(GAME_TEST_HIGHLIGHT_DURATION_MILLIS),
            },
        );
        self.counters.game_test_highlights_tracked =
            self.client_debug_game.game_test_highlight_markers.len();
    }

    pub fn expire_game_test_highlight_markers_at(&mut self, now_monotonic_millis: u64) {
        self.client_debug_game
            .game_test_highlight_markers
            .retain(|_, marker| now_monotonic_millis <= marker.remove_at_monotonic_millis);
        self.counters.game_test_highlights_tracked =
            self.client_debug_game.game_test_highlight_markers.len();
    }

    pub(crate) fn clear_game_test_highlight_markers(&mut self) {
        self.client_debug_game.game_test_highlight_markers.clear();
        self.client_debug_game.last_game_test_highlight_pos = None;
        self.counters.game_test_highlights_tracked = 0;
    }

    pub fn apply_test_instance_block_status(&mut self, packet: ProtocolTestInstanceBlockStatus) {
        self.counters.test_instance_block_status_packets += 1;
        self.apply_test_instance_block_status_to_editor(&packet);
        self.client_debug_game.last_test_instance_block_status =
            Some(TestInstanceBlockStatusState {
                status: packet.status,
                size: packet.size.map(debug_vec3i_state),
            });
    }

    pub fn client_debug_game(&self) -> &ClientDebugGameState {
        &self.client_debug_game
    }

    pub fn last_debug_block_value(&self) -> Option<&DebugBlockValueState> {
        self.client_debug_game.last_debug_block_value.as_ref()
    }

    pub fn last_debug_chunk_value(&self) -> Option<&DebugChunkValueState> {
        self.client_debug_game.last_debug_chunk_value.as_ref()
    }

    pub fn last_debug_entity_value(&self) -> Option<&DebugEntityValueState> {
        self.client_debug_game.last_debug_entity_value.as_ref()
    }

    pub fn last_debug_event(&self) -> Option<&DebugEventState> {
        self.client_debug_game.last_debug_event.as_ref()
    }

    pub fn last_debug_sample(&self) -> Option<&DebugSampleState> {
        self.client_debug_game.last_debug_sample.as_ref()
    }

    pub fn last_game_test_highlight_pos(&self) -> Option<&GameTestHighlightPosState> {
        self.client_debug_game.last_game_test_highlight_pos.as_ref()
    }

    pub fn game_test_highlight_markers(
        &self,
    ) -> &DebugValueMap<BlockPos, GameTestHighlightMarkerState> {
        &self.client_debug_game.game_test_highlight_markers
    }

    pub fn last_test_instance_block_status(&self) -> Option<&TestInstanceBlockStatusState> {
        self.client_debug_game
            .last_test_instance_block_status
            .as_ref()
    }

    fn apply_debug_subscription_update(
        &mut self,
        key: DebugValueKey,
        update: ProtocolDebugSubscriptionUpdate,
    ) {
        if !self
            .client_debug_game
            .values_by_subscription
            .contains_key(&update.subscription_id)
        {
            return;
        }
        let subscription_id = update.subscription_id;
        let value = match update.value {
            Some(value) => match self.canonical_debug_subscription_value(subscription_id, value) {
                Ok(value) => Some(value),
                Err(error) => {
                    self.record_apply_error("debug_subscription_value", &error);
                    return;
                }
            },
            None => None,
        };
        let game_time = self.debug_game_time();
        let values = self
            .client_debug_game
            .values_by_subscription
            .get_mut(&subscription_id)
            .expect("requested debug subscription must own initialized value maps");
        match (key, value) {
            (DebugValueKey::Block(key), Some(value)) => {
                values
                    .block_values
                    .insert(key, debug_timed_value(subscription_id, game_time, value));
            }
            (DebugValueKey::Block(key), None) => {
                values.block_values.remove(&key);
            }
            (DebugValueKey::Chunk(key), Some(value)) => {
                values
                    .chunk_values
                    .insert(key, debug_timed_value(subscription_id, game_time, value));
            }
            (DebugValueKey::Chunk(key), None) => {
                values.chunk_values.remove(&key);
            }
            (DebugValueKey::Entity(key), Some(value)) => {
                values
                    .entity_values
                    .insert(key, debug_timed_value(subscription_id, game_time, value));
            }
            (DebugValueKey::Entity(key), None) => {
                values.entity_values.remove(&key);
            }
        }
    }

    fn canonical_debug_subscription_value(
        &self,
        subscription_id: i32,
        value: ProtocolDebugSubscriptionValue,
    ) -> Result<DebugSubscriptionValueState, String> {
        let mismatch = || {
            format!("debug subscription {subscription_id} received a value from a different codec")
        };
        Ok(match (subscription_id, value) {
            (VANILLA_DEBUG_SUBSCRIPTION_BEES_ID, ProtocolDebugSubscriptionValue::Bee(value)) => {
                DebugSubscriptionValueState::Bee(value)
            }
            (
                VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID,
                ProtocolDebugSubscriptionValue::Brain(value),
            ) => DebugSubscriptionValueState::Brain(value),
            (
                VANILLA_DEBUG_SUBSCRIPTION_BREEZES_ID,
                ProtocolDebugSubscriptionValue::Breeze(value),
            ) => DebugSubscriptionValueState::Breeze(value),
            (
                VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID,
                ProtocolDebugSubscriptionValue::Goals(value),
            ) => DebugSubscriptionValueState::Goals(value),
            (
                VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID,
                ProtocolDebugSubscriptionValue::Path(value),
            ) => DebugSubscriptionValueState::Path(value),
            (
                VANILLA_DEBUG_SUBSCRIPTION_ENTITY_BLOCK_INTERSECTIONS_ID,
                ProtocolDebugSubscriptionValue::EntityBlockIntersection(value),
            ) => DebugSubscriptionValueState::EntityBlockIntersection(value),
            (
                VANILLA_DEBUG_SUBSCRIPTION_BEE_HIVES_ID,
                ProtocolDebugSubscriptionValue::Hive(value),
            ) => {
                let Some(block_resource_id) = self
                    .registry_resource_id_by_holder_id("minecraft:block", value.block_registry_id)
                else {
                    return Err(format!(
                        "unknown block registry id {} in debug hive value",
                        value.block_registry_id
                    ));
                };
                DebugSubscriptionValueState::Hive(DebugHiveInfoState {
                    block_resource_id: block_resource_id.to_string(),
                    occupant_count: value.occupant_count,
                    honey_level: value.honey_level,
                    sedated: value.sedated,
                })
            }
            (VANILLA_DEBUG_SUBSCRIPTION_POIS_ID, ProtocolDebugSubscriptionValue::Poi(value)) => {
                let Some(poi_type) = vanilla_debug_poi_type_by_id(value.poi_type_holder_id) else {
                    return Err(format!(
                        "unknown point of interest type registry id {} in debug POI value",
                        value.poi_type_holder_id
                    ));
                };
                DebugSubscriptionValueState::Poi(DebugPoiInfoState {
                    pos: protocol_block_pos(value.pos),
                    poi_type_resource_id: poi_type.resource_id.to_string(),
                    free_ticket_count: value.free_ticket_count,
                })
            }
            (
                VANILLA_DEBUG_SUBSCRIPTION_REDSTONE_WIRE_ORIENTATIONS_ID,
                ProtocolDebugSubscriptionValue::RedstoneOrientation(value),
            ) => DebugSubscriptionValueState::RedstoneOrientation(value),
            (
                VANILLA_DEBUG_SUBSCRIPTION_VILLAGE_SECTIONS_ID,
                ProtocolDebugSubscriptionValue::VillageSection,
            ) => DebugSubscriptionValueState::VillageSection,
            (
                VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID,
                ProtocolDebugSubscriptionValue::Raids(values),
            ) => DebugSubscriptionValueState::Raids(
                values.into_iter().map(protocol_block_pos).collect(),
            ),
            (
                VANILLA_DEBUG_SUBSCRIPTION_STRUCTURES_ID,
                ProtocolDebugSubscriptionValue::Structures(values),
            ) => DebugSubscriptionValueState::Structures(
                values.into_iter().map(debug_structure_state).collect(),
            ),
            (
                VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID,
                ProtocolDebugSubscriptionValue::GameEventListener(value),
            ) => DebugSubscriptionValueState::GameEventListener(value),
            (
                VANILLA_DEBUG_SUBSCRIPTION_NEIGHBOR_UPDATES_ID,
                ProtocolDebugSubscriptionValue::NeighborUpdate(value),
            ) => DebugSubscriptionValueState::NeighborUpdate(protocol_block_pos(value)),
            (
                VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID,
                ProtocolDebugSubscriptionValue::GameEvent(value),
            ) => {
                let Some(game_event) =
                    vanilla_debug_game_event_type_by_id(value.game_event_holder_id)
                else {
                    return Err(format!(
                        "unknown game event registry id {} in debug event value",
                        value.game_event_holder_id
                    ));
                };
                DebugSubscriptionValueState::GameEvent(DebugGameEventInfoState {
                    game_event_resource_id: game_event.resource_id.to_string(),
                    pos: [value.pos.x, value.pos.y, value.pos.z],
                })
            }
            _ => return Err(mismatch()),
        })
    }

    fn debug_game_time(&self) -> i64 {
        self.world_time().map(|time| time.game_time).unwrap_or(0)
    }

    pub(crate) fn clear_debug_subscriber_values(&mut self) {
        self.client_debug_game.last_debug_block_value = None;
        self.client_debug_game.last_debug_chunk_value = None;
        self.client_debug_game.last_debug_entity_value = None;
        self.client_debug_game.last_debug_event = None;
        self.client_debug_game.values_by_subscription.clear();
        for id in &self.client_debug_game.requested_subscriptions {
            self.client_debug_game
                .values_by_subscription
                .insert(*id, DebugSubscriptionValuesState::default());
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DebugValueKey {
    Block(BlockPos),
    Chunk(ChunkPos),
    Entity(Uuid),
}

fn debug_timed_value(
    subscription_id: i32,
    game_time: i64,
    value: DebugSubscriptionValueState,
) -> DebugTimedValueState {
    let expire_after_ticks = vanilla_debug_subscription_type_by_id(subscription_id)
        .map(|subscription| subscription.expire_after_ticks)
        .unwrap_or(0);
    DebugTimedValueState {
        value,
        expires_after_game_time: (expire_after_ticks != 0)
            .then(|| game_time.saturating_add(i64::from(expire_after_ticks))),
    }
}

fn debug_value_expired(value: &DebugTimedValueState, game_time: i64) -> bool {
    value
        .expires_after_game_time
        .is_some_and(|expires_after| game_time >= expires_after)
}

fn debug_structure_state(value: DebugStructureInfo) -> DebugStructureInfoState {
    DebugStructureInfoState {
        bounding_box: debug_bounding_box_state(value.bounding_box),
        pieces: value
            .pieces
            .into_iter()
            .map(|piece| DebugStructurePieceState {
                bounding_box: debug_bounding_box_state(piece.bounding_box),
                is_start: piece.is_start,
            })
            .collect(),
    }
}

fn debug_bounding_box_state(value: DebugBoundingBox) -> DebugBoundingBoxState {
    DebugBoundingBoxState {
        min: protocol_block_pos(value.min),
        max: protocol_block_pos(value.max),
    }
}

fn debug_vec3i_state(pos: ProtocolVec3i) -> DebugVec3iState {
    DebugVec3iState {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

#[cfg(test)]
mod tests {
    use super::super::game_rules::GameRuleValueState;
    use super::*;
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID;
    use bbb_protocol::packets::{
        AddEntity, BlockPos as ProtocolBlockPos, ChunkPos as ProtocolChunkPos, DebugBlockValue,
        DebugChunkValue, DebugEntityBlockIntersection, DebugEntityValue, DebugEvent,
        DebugGameEventInfo, DebugHiveInfo, DebugPoiInfo, DebugSample, DebugSubscriptionEvent,
        DebugSubscriptionUpdate, DebugSubscriptionValue, GameRuleValue, GameRuleValues,
        GameTestHighlightPos, RemoteDebugSampleType, RemoveEntities, TestInstanceBlockStatus,
        Vec3d,
    };

    fn block_update(pos: ProtocolBlockPos) -> DebugBlockValue {
        DebugBlockValue {
            pos,
            update: DebugSubscriptionUpdate {
                subscription_id: 6,
                value: Some(DebugSubscriptionValue::EntityBlockIntersection(
                    DebugEntityBlockIntersection::InAir,
                )),
            },
        }
    }

    fn chunk_remove(pos: ProtocolChunkPos) -> DebugChunkValue {
        DebugChunkValue {
            pos,
            update: DebugSubscriptionUpdate {
                subscription_id: 11,
                value: None,
            },
        }
    }

    fn entity_remove(entity_id: i32) -> DebugEntityValue {
        DebugEntityValue {
            entity_id,
            update: DebugSubscriptionUpdate {
                subscription_id: 3,
                value: None,
            },
        }
    }

    fn neighbor_event(pos: ProtocolBlockPos) -> DebugEvent {
        DebugEvent {
            event: DebugSubscriptionEvent {
                subscription_id: 14,
                value: DebugSubscriptionValue::NeighborUpdate(pos),
            },
        }
    }

    #[test]
    fn tracks_debug_game_events_and_counters() {
        let mut store = WorldStore::new();
        store
            .set_requested_debug_subscriptions(&[3, 6, 11, 14])
            .unwrap();

        store.apply_debug_block_value(block_update(ProtocolBlockPos { x: 1, y: 64, z: -2 }));
        store.apply_debug_chunk_value(chunk_remove(ProtocolChunkPos { x: 3, z: -4 }));
        store.apply_debug_entity_value(entity_remove(123));
        store.apply_debug_event(neighbor_event(ProtocolBlockPos { x: 4, y: 5, z: 6 }));
        store.apply_debug_sample(DebugSample {
            sample: vec![100, -50],
            sample_type: RemoteDebugSampleType::TickTime,
        });
        store.apply_game_rule_values(GameRuleValues {
            values: vec![
                GameRuleValue {
                    rule: "minecraft:do_daylight_cycle".to_string(),
                    value: "false".to_string(),
                },
                GameRuleValue {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                },
            ],
        });
        store.apply_game_test_highlight_pos(GameTestHighlightPos {
            absolute_pos: ProtocolBlockPos {
                x: -10,
                y: 70,
                z: 22,
            },
            relative_pos: ProtocolBlockPos { x: 1, y: 2, z: 3 },
        });
        store.apply_test_instance_block_status(TestInstanceBlockStatus {
            status: bbb_protocol::RichTextComponent::plain("Ready"),
            size: Some(ProtocolVec3i { x: 3, y: 4, z: 5 }),
        });

        assert_eq!(
            store.last_debug_block_value(),
            Some(&DebugBlockValueState {
                pos: BlockPos { x: 1, y: 64, z: -2 },
                subscription_id: 6,
                value_present: true,
            })
        );
        assert_eq!(
            store.last_debug_chunk_value(),
            Some(&DebugChunkValueState {
                pos: ChunkPos { x: 3, z: -4 },
                subscription_id: 11,
                value_present: false,
            })
        );
        assert_eq!(
            store.last_debug_entity_value(),
            Some(&DebugEntityValueState {
                entity_id: 123,
                subscription_id: 3,
                value_present: false,
            })
        );
        assert_eq!(
            store.last_debug_event(),
            Some(&DebugEventState {
                subscription_id: 14
            })
        );
        assert_eq!(
            store.last_debug_sample(),
            Some(&DebugSampleState {
                sample_len: 2,
                sample_type: "tick_time".to_string(),
            })
        );
        assert_eq!(
            store.last_game_rule_values(),
            Some(&GameRuleValuesState {
                values: vec![
                    GameRuleValueState {
                        rule: "minecraft:do_daylight_cycle".to_string(),
                        value: "false".to_string(),
                    },
                    GameRuleValueState {
                        rule: "minecraft:random_tick_speed".to_string(),
                        value: "3".to_string(),
                    },
                ],
            })
        );
        assert_eq!(store.last_game_rule_values().unwrap().len(), 2);
        assert_eq!(
            store.last_game_test_highlight_pos(),
            Some(&GameTestHighlightPosState {
                absolute_pos: BlockPos {
                    x: -10,
                    y: 70,
                    z: 22,
                },
                relative_pos: BlockPos { x: 1, y: 2, z: 3 },
            })
        );
        assert_eq!(
            store.last_test_instance_block_status(),
            Some(&TestInstanceBlockStatusState {
                status: bbb_protocol::RichTextComponent::plain("Ready"),
                size: Some(DebugVec3iState { x: 3, y: 4, z: 5 }),
            })
        );

        let counters = store.counters();
        assert_eq!(counters.debug_block_value_packets, 1);
        assert_eq!(counters.debug_chunk_value_packets, 1);
        assert_eq!(counters.debug_entity_value_packets, 1);
        assert_eq!(counters.debug_event_packets, 1);
        assert_eq!(counters.debug_sample_packets, 1);
        assert_eq!(counters.game_rule_value_packets, 1);
        assert_eq!(counters.game_test_highlight_pos_packets, 1);
        assert_eq!(counters.game_test_highlights_tracked, 1);
        assert_eq!(counters.test_instance_block_status_packets, 1);
    }

    #[test]
    fn game_test_highlight_markers_refresh_expire_strictly_and_clear_with_level() {
        let mut store = WorldStore::new();
        let absolute_a = ProtocolBlockPos { x: 1, y: 2, z: 3 };
        let absolute_b = ProtocolBlockPos { x: -4, y: 5, z: -6 };

        store.apply_game_test_highlight_pos_at(
            GameTestHighlightPos {
                absolute_pos: absolute_a,
                relative_pos: ProtocolBlockPos { x: 7, y: 8, z: 9 },
            },
            100,
        );
        store.apply_game_test_highlight_pos_at(
            GameTestHighlightPos {
                absolute_pos: absolute_b,
                relative_pos: ProtocolBlockPos {
                    x: -10,
                    y: 11,
                    z: 12,
                },
            },
            200,
        );
        store.apply_game_test_highlight_pos_at(
            GameTestHighlightPos {
                absolute_pos: absolute_a,
                relative_pos: ProtocolBlockPos {
                    x: 13,
                    y: 14,
                    z: 15,
                },
            },
            300,
        );

        assert_eq!(store.game_test_highlight_markers().len(), 2);
        assert_eq!(store.counters().game_test_highlight_pos_packets, 3);
        assert_eq!(store.counters().game_test_highlights_tracked, 2);
        assert_eq!(
            store.game_test_highlight_markers()[&BlockPos { x: 1, y: 2, z: 3 }],
            GameTestHighlightMarkerState {
                relative_pos: BlockPos {
                    x: 13,
                    y: 14,
                    z: 15
                },
                remove_at_monotonic_millis: 10_300,
            }
        );

        store.expire_game_test_highlight_markers_at(10_200);
        assert_eq!(store.game_test_highlight_markers().len(), 2);
        store.expire_game_test_highlight_markers_at(10_201);
        assert_eq!(store.game_test_highlight_markers().len(), 1);
        store.expire_game_test_highlight_markers_at(10_300);
        assert_eq!(store.game_test_highlight_markers().len(), 1);
        store.expire_game_test_highlight_markers_at(10_301);
        assert!(store.game_test_highlight_markers().is_empty());

        store.apply_game_test_highlight_pos_at(
            GameTestHighlightPos {
                absolute_pos: absolute_a,
                relative_pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            },
            20_000,
        );
        store.clear_client_level();
        assert!(store.game_test_highlight_markers().is_empty());
        assert!(store.last_game_test_highlight_pos().is_none());
        assert_eq!(store.counters().game_test_highlights_tracked, 0);
    }

    #[test]
    fn debug_game_state_survives_world_store_json_round_trip() {
        let mut store = WorldStore::new();
        store.set_requested_debug_subscriptions(&[6]).unwrap();

        store.apply_debug_block_value(block_update(ProtocolBlockPos { x: 1, y: 64, z: -2 }));
        store.apply_game_rule_values(GameRuleValues {
            values: vec![GameRuleValue {
                rule: "minecraft:random_tick_speed".to_string(),
                value: "3".to_string(),
            }],
        });
        store.apply_test_instance_block_status(TestInstanceBlockStatus {
            status: bbb_protocol::RichTextComponent::plain("Ready"),
            size: Some(ProtocolVec3i { x: 3, y: 4, z: 5 }),
        });

        let json = serde_json::to_string(&store).unwrap();
        let restored: WorldStore = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.counters().debug_block_value_packets, 1);
        assert_eq!(restored.counters().game_rule_value_packets, 1);
        assert_eq!(restored.counters().test_instance_block_status_packets, 1);
        assert_eq!(
            restored.last_debug_block_value(),
            Some(&DebugBlockValueState {
                pos: BlockPos { x: 1, y: 64, z: -2 },
                subscription_id: 6,
                value_present: true,
            })
        );
        assert_eq!(
            restored.last_game_rule_values(),
            Some(&GameRuleValuesState {
                values: vec![GameRuleValueState {
                    rule: "minecraft:random_tick_speed".to_string(),
                    value: "3".to_string(),
                }],
            })
        );
        assert_eq!(
            restored.last_test_instance_block_status(),
            Some(&TestInstanceBlockStatusState {
                status: bbb_protocol::RichTextComponent::plain("Ready"),
                size: Some(DebugVec3iState { x: 3, y: 4, z: 5 }),
            })
        );
    }

    #[test]
    fn listener_clear_drops_requests_and_values_but_preserves_overlay_diagnostics() {
        let mut store = WorldStore::new();
        store
            .set_requested_debug_subscriptions(&[3, 6, 11, 14])
            .unwrap();
        store.apply_debug_block_value(block_update(ProtocolBlockPos { x: 1, y: 64, z: -2 }));
        store.apply_debug_chunk_value(chunk_remove(ProtocolChunkPos { x: 3, z: -4 }));
        store.apply_debug_entity_value(entity_remove(123));
        store.apply_debug_event(neighbor_event(ProtocolBlockPos { x: 4, y: 5, z: 6 }));
        store.apply_debug_sample(DebugSample {
            sample: vec![100, -50],
            sample_type: RemoteDebugSampleType::TickTime,
        });

        store.clear_client_level();

        assert!(store.last_debug_block_value().is_none());
        assert!(store.last_debug_chunk_value().is_none());
        assert!(store.last_debug_entity_value().is_none());
        assert!(store.last_debug_event().is_none());
        assert_eq!(
            store.last_debug_sample(),
            Some(&DebugSampleState {
                sample_len: 2,
                sample_type: "tick_time".to_string(),
            })
        );
        assert_eq!(store.counters().debug_block_value_packets, 1);
        assert_eq!(store.counters().debug_chunk_value_packets, 1);
        assert_eq!(store.counters().debug_entity_value_packets, 1);
        assert_eq!(store.counters().debug_event_packets, 1);
        assert_eq!(store.counters().debug_sample_packets, 1);
        assert!(store.requested_debug_subscriptions().is_empty());
        assert!(store.client_debug_game.values_by_subscription.is_empty());
    }

    #[test]
    fn registry_session_reset_drops_remote_debug_owner_state() {
        let mut store = WorldStore::new();
        store.set_requested_debug_subscriptions(&[6]).unwrap();
        store.apply_debug_block_value(block_update(ProtocolBlockPos { x: 1, y: 64, z: -2 }));

        store.reset_registry_session();

        assert!(store.requested_debug_subscriptions().is_empty());
        assert!(store.client_debug_game.values_by_subscription.is_empty());
        assert!(store.last_debug_block_value().is_none());
        assert_eq!(store.counters().debug_block_value_packets, 1);
    }

    #[test]
    fn level_drop_clears_values_and_reinitializes_requested_maps() {
        let mut store = WorldStore::new();
        store.set_requested_debug_subscriptions(&[6, 14]).unwrap();
        store.apply_debug_block_value(block_update(ProtocolBlockPos { x: 1, y: 64, z: -2 }));
        store.apply_debug_event(neighbor_event(ProtocolBlockPos { x: 4, y: 5, z: 6 }));

        store.clear_debug_subscriber_values();

        assert_eq!(
            store.requested_debug_subscriptions(),
            &BTreeSet::from([6, 14])
        );
        assert!(store
            .client_debug_game
            .values_by_subscription
            .values()
            .all(|values| values.block_values.is_empty()
                && values.chunk_values.is_empty()
                && values.entity_values.is_empty()
                && values.events.is_empty()));
    }

    #[test]
    fn requested_subscriptions_gate_typed_maps_and_resolve_holder_identity() {
        let mut store = WorldStore::new();
        store
            .set_requested_debug_subscriptions(&[6, 7, 8, 15])
            .unwrap();

        let block = ProtocolBlockPos { x: 1, y: 64, z: -2 };
        store.apply_debug_block_value(block_update(block));
        store.apply_debug_block_value(DebugBlockValue {
            pos: ProtocolBlockPos { x: 2, y: 64, z: -2 },
            update: DebugSubscriptionUpdate {
                subscription_id: 7,
                value: Some(DebugSubscriptionValue::Hive(DebugHiveInfo {
                    block_registry_id: 0,
                    occupant_count: 2,
                    honey_level: 4,
                    sedated: true,
                })),
            },
        });
        store.apply_debug_block_value(DebugBlockValue {
            pos: ProtocolBlockPos { x: 3, y: 64, z: -2 },
            update: DebugSubscriptionUpdate {
                subscription_id: 8,
                value: Some(DebugSubscriptionValue::Poi(DebugPoiInfo {
                    pos: ProtocolBlockPos { x: 3, y: 64, z: -2 },
                    poi_type_holder_id: 13,
                    free_ticket_count: 1,
                })),
            },
        });
        store.apply_debug_event(DebugEvent {
            event: DebugSubscriptionEvent {
                subscription_id: 15,
                value: DebugSubscriptionValue::GameEvent(DebugGameEventInfo {
                    game_event_holder_id: 0,
                    pos: Vec3d {
                        x: 3.5,
                        y: 64.0,
                        z: -1.5,
                    },
                }),
            },
        });

        assert!(matches!(
            &store.debug_subscription_values(6).unwrap().block_values
                [&BlockPos { x: 1, y: 64, z: -2 }]
                .value,
            DebugSubscriptionValueState::EntityBlockIntersection(
                DebugEntityBlockIntersection::InAir
            )
        ));
        assert!(matches!(
            &store.debug_subscription_values(7).unwrap().block_values
                [&BlockPos { x: 2, y: 64, z: -2 }]
                .value,
            DebugSubscriptionValueState::Hive(DebugHiveInfoState {
                block_resource_id,
                occupant_count: 2,
                honey_level: 4,
                sedated: true,
            }) if block_resource_id == "minecraft:air"
        ));
        assert!(matches!(
            &store.debug_subscription_values(8).unwrap().block_values
                [&BlockPos { x: 3, y: 64, z: -2 }]
                .value,
            DebugSubscriptionValueState::Poi(DebugPoiInfoState {
                poi_type_resource_id,
                ..
            }) if poi_type_resource_id == "minecraft:home"
        ));
        assert!(matches!(
            &store.debug_subscription_values(15).unwrap().events[0].value,
            DebugSubscriptionValueState::GameEvent(DebugGameEventInfoState {
                game_event_resource_id,
                ..
            }) if game_event_resource_id == "minecraft:block_activate"
        ));

        store.set_requested_debug_subscriptions(&[7]).unwrap();
        assert!(store.debug_subscription_values(6).is_none());
        assert_eq!(
            store
                .debug_subscription_values(7)
                .unwrap()
                .block_values
                .len(),
            1
        );
        store.apply_debug_block_value(block_update(ProtocolBlockPos { x: 9, y: 9, z: 9 }));
        assert!(store.debug_subscription_values(6).is_none());
        assert!(store.set_requested_debug_subscriptions(&[16]).is_err());
    }

    #[test]
    fn temporary_values_expire_and_chunk_entity_drop_hooks_remove_owned_keys() {
        let mut store = WorldStore::new();
        store
            .set_requested_debug_subscriptions(&[3, 6, 14])
            .unwrap();
        let uuid = Uuid::from_u128(0x12345678123456781234567812345678);
        store.apply_add_entity(AddEntity {
            id: 123,
            uuid,
            entity_type_id: VANILLA_ENTITY_TYPE_BEE_ID,
            position: Vec3d::default(),
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
        store.apply_debug_entity_value(DebugEntityValue {
            entity_id: 123,
            update: DebugSubscriptionUpdate {
                subscription_id: 3,
                value: Some(DebugSubscriptionValue::Breeze(DebugBreezeInfo {
                    attack_target: None,
                    jump_target: None,
                })),
            },
        });
        store.apply_debug_block_value(block_update(ProtocolBlockPos {
            x: 17,
            y: 64,
            z: -1,
        }));
        store.apply_debug_event(neighbor_event(ProtocolBlockPos { x: 4, y: 5, z: 6 }));

        assert!(store
            .debug_subscription_values(3)
            .unwrap()
            .entity_values
            .contains_key(&uuid));
        assert_eq!(
            store
                .debug_subscription_values(6)
                .unwrap()
                .block_values
                .len(),
            1
        );
        store.tick_debug_subscriber(99);
        assert_eq!(
            store
                .debug_subscription_values(6)
                .unwrap()
                .block_values
                .len(),
            1
        );
        store.tick_debug_subscriber(100);
        assert!(store
            .debug_subscription_values(6)
            .unwrap()
            .block_values
            .is_empty());
        assert_eq!(store.debug_subscription_values(14).unwrap().events.len(), 1);

        store.apply_debug_block_value(block_update(ProtocolBlockPos {
            x: 17,
            y: 64,
            z: -1,
        }));
        assert!(!store.forget_chunk(ChunkPos { x: 1, z: -1 }));
        assert!(store
            .debug_subscription_values(6)
            .unwrap()
            .block_values
            .is_empty());
        assert_eq!(
            store.apply_remove_entities(RemoveEntities {
                entity_ids: vec![123],
            }),
            1
        );
        assert!(store
            .debug_subscription_values(3)
            .unwrap()
            .entity_values
            .is_empty());
        store.tick_debug_subscriber(200);
        assert!(store
            .debug_subscription_values(14)
            .unwrap()
            .events
            .is_empty());
    }
}
