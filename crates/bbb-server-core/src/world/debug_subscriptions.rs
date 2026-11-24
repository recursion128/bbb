use crate::world::LevelId;
use rustc_hash::FxHashMap;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::block_entity::game_event::GameEventListenerOwner;
use crate::chunk_sender::ChunkKey;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::{EntityStore, EntityTracker};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::debug_subscription_types::{
    VANILLA_DEBUG_POI_TYPES, VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID,
    VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID, VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID, VANILLA_DEBUG_SUBSCRIPTION_POIS_ID,
    VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID,
};
use bbb_protocol::packets::{
    BlockPos, ChunkPos, DebugBlockValue, DebugBrainDump, DebugChunkValue, DebugEntityValue,
    DebugEvent, DebugGameEventInfo, DebugGameEventListenerInfo, DebugPoiInfo,
    DebugSubscriptionEvent, DebugSubscriptionUpdate, DebugSubscriptionValue,
};

use super::{send, ChunkStore, ServerLevel, ServerWorld, WorldEvent};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct PlayerDebugSubscriptions {
    requested: BTreeSet<i32>,
}

impl PlayerDebugSubscriptions {
    pub(super) fn replace(&mut self, requested: Vec<i32>) {
        self.requested = requested.into_iter().collect();
    }

    fn iter(&self) -> impl Iterator<Item = i32> + '_ {
        self.requested.iter().copied()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ServerDebugSubscriptions {
    enabled: BTreeSet<i32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct EntityValuePublication {
    last_values: BTreeMap<i32, Arc<DebugEntityValue>>,
    interested: BTreeMap<i32, BTreeSet<ConnectionId>>,
}

impl EntityValuePublication {
    fn clear(&mut self) {
        self.last_values.clear();
        self.interested.clear();
    }

    fn publish<F>(
        &mut self,
        subscription_id: i32,
        subscribers: &BTreeSet<ConnectionId>,
        (entities, tracker): (&EntityStore, &EntityTracker),
        clear_missing: bool,
        mut source: F,
        events: &mut Vec<WorldEvent>,
    ) where
        F: FnMut(i32) -> Option<DebugSubscriptionValue>,
    {
        let tracked: BTreeSet<i32> = entities
            .bodies()
            .filter_map(|entity| {
                let id = entity.id();
                (!tracker.viewers_of(id).is_empty()).then_some(id)
            })
            .collect();
        let entity_ids: Vec<i32> = tracked.iter().copied().collect();
        self.last_values
            .retain(|entity_id, _| tracked.contains(entity_id));
        self.interested
            .retain(|entity_id, _| tracked.contains(entity_id));

        if subscribers.is_empty() {
            self.interested.clear();
            return;
        }

        let mut interested_by_entity = BTreeMap::new();
        for &entity_id in &entity_ids {
            let interested = tracker
                .viewers_of(entity_id)
                .into_iter()
                .filter(|viewer| subscribers.contains(viewer))
                .collect::<BTreeSet<_>>();
            let previous = self.interested.get(&entity_id).cloned().unwrap_or_default();
            if let Some(cached) = self.last_values.get(&entity_id) {
                for viewer in interested.difference(&previous) {
                    events.push(send(
                        *viewer,
                        ClientboundPlayPacket::DebugEntityValue(Arc::clone(cached)),
                    ));
                }
            }
            if interested.is_empty() {
                self.interested.remove(&entity_id);
            } else {
                self.interested.insert(entity_id, interested.clone());
            }
            interested_by_entity.insert(entity_id, interested);
        }

        for entity_id in entity_ids {
            let interested = &interested_by_entity[&entity_id];
            match source(entity_id) {
                Some(value) => {
                    let update = Arc::new(DebugEntityValue {
                        entity_id,
                        update: DebugSubscriptionUpdate {
                            subscription_id,
                            value: Some(value),
                        },
                    });
                    if self
                        .last_values
                        .get(&entity_id)
                        .is_none_or(|last| last.as_ref() != update.as_ref())
                    {
                        for viewer in interested {
                            events.push(send(
                                *viewer,
                                ClientboundPlayPacket::DebugEntityValue(Arc::clone(&update)),
                            ));
                        }
                        self.last_values.insert(entity_id, update);
                    }
                }
                None => {
                    let had_value = self.last_values.remove(&entity_id).is_some();
                    if clear_missing && had_value {
                        let clear = Arc::new(DebugEntityValue {
                            entity_id,
                            update: DebugSubscriptionUpdate {
                                subscription_id,
                                value: None,
                            },
                        });
                        for viewer in interested {
                            events.push(send(
                                *viewer,
                                ClientboundPlayPacket::DebugEntityValue(Arc::clone(&clear)),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// One level channel's published values, keyed by the chunk or the block that
/// owns them.
///
/// The two channels this covers reach a subscriber tracking that key's column
/// (`sendToPlayersTrackingChunk`, TrackingDebugSynchronizer.java:65-73) and
/// replay what they last published when a subscriber starts tracking it
/// (`startTrackingChunk`, `:80-84`).
#[derive(Debug, Clone, PartialEq)]
struct LevelValuePublication<K: Ord> {
    last_values: BTreeMap<K, DebugSubscriptionValue>,
    interested: BTreeMap<K, BTreeSet<ConnectionId>>,
}

impl<K: Ord> Default for LevelValuePublication<K> {
    fn default() -> Self {
        Self {
            last_values: BTreeMap::new(),
            interested: BTreeMap::new(),
        }
    }
}

impl<K: Ord + Copy> LevelValuePublication<K> {
    fn clear(&mut self) {
        self.last_values.clear();
        self.interested.clear();
    }

    fn publish<C, P>(
        &mut self,
        (chunk_trackers, subscribers): (
            &BTreeMap<ChunkKey, BTreeSet<ConnectionId>>,
            &BTreeSet<ConnectionId>,
        ),
        mut values: BTreeMap<K, DebugSubscriptionValue>,
        clear_missing: bool,
        chunk_of: C,
        packet: P,
        events: &mut Vec<WorldEvent>,
    ) where
        C: Fn(K) -> ChunkKey,
        P: Fn(K, DebugSubscriptionUpdate) -> ClientboundPlayPacket,
    {
        // A key whose column nobody tracks any more is dropped rather than
        // cleared: `SourceSynchronizer.dropChunk` (`:196-199`) sends nothing.
        self.last_values
            .retain(|key, _| chunk_trackers.contains_key(&chunk_of(*key)));
        self.interested
            .retain(|key, _| chunk_trackers.contains_key(&chunk_of(*key)));
        if subscribers.is_empty() {
            self.interested.clear();
            return;
        }

        let keys = self
            .last_values
            .keys()
            .chain(values.keys())
            .copied()
            .filter(|key| chunk_trackers.contains_key(&chunk_of(*key)))
            .collect::<BTreeSet<_>>();

        let mut interested_by_key = BTreeMap::new();
        for &key in &keys {
            let interested = chunk_trackers[&chunk_of(key)]
                .iter()
                .copied()
                .filter(|viewer| subscribers.contains(viewer))
                .collect::<BTreeSet<_>>();
            let previous = self.interested.get(&key).cloned().unwrap_or_default();
            if let Some(cached) = self.last_values.get(&key) {
                let replay = Arc::new(packet(
                    key,
                    DebugSubscriptionUpdate {
                        subscription_id: subscription_id_of(cached),
                        value: Some(cached.clone()),
                    },
                ));
                for viewer in interested.difference(&previous) {
                    events.push(send(*viewer, (*replay).clone()));
                }
            }
            if interested.is_empty() {
                self.interested.remove(&key);
            } else {
                self.interested.insert(key, interested.clone());
            }
            interested_by_key.insert(key, interested);
        }

        for key in keys {
            let interested = &interested_by_key[&key];
            match values.remove(&key) {
                Some(value) => {
                    if self
                        .last_values
                        .get(&key)
                        .is_some_and(|last| *last == value)
                    {
                        continue;
                    }
                    let update = packet(
                        key,
                        DebugSubscriptionUpdate {
                            subscription_id: subscription_id_of(&value),
                            value: Some(value.clone()),
                        },
                    );
                    crate::world::player_packets::broadcast_to(
                        interested.iter().copied(),
                        update,
                        events,
                    );
                    self.last_values.insert(key, value);
                }
                None => {
                    let Some(last) = self.last_values.remove(&key) else {
                        continue;
                    };
                    if !clear_missing {
                        continue;
                    }
                    let clear = packet(
                        key,
                        DebugSubscriptionUpdate {
                            subscription_id: subscription_id_of(&last),
                            value: None,
                        },
                    );
                    crate::world::player_packets::broadcast_to(
                        interested.iter().copied(),
                        clear,
                        events,
                    );
                }
            }
        }
    }
}

/// The level state the two chunk-tracked channels read.
struct LevelChannelSources<'a> {
    chunks: &'a mut ChunkStore,
    raids: &'a super::raids::LevelRaids,
}

/// The `Registries.POINT_OF_INTEREST_TYPE` id `DebugPoiInfo.STREAM_CODEC`
/// (`DebugPoiInfo.java:13-21`) writes the record's type as.
fn poi_type_holder_id(resource_id: &str) -> Option<i32> {
    VANILLA_DEBUG_POI_TYPES
        .iter()
        .find(|entry| entry.resource_id == resource_id)
        .map(|entry| entry.holder_id)
}

/// The channel a level value belongs to, which is what
/// `DebugSubscription.Update.STREAM_CODEC` (`DebugSubscription.java:63-69`)
/// writes before the value itself.
fn subscription_id_of(value: &DebugSubscriptionValue) -> i32 {
    match value {
        DebugSubscriptionValue::Poi(_) => VANILLA_DEBUG_SUBSCRIPTION_POIS_ID,
        DebugSubscriptionValue::Raids(_) => VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID,
        DebugSubscriptionValue::GameEventListener(_) => {
            VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID
        }
        _ => unreachable!("level channels publish only POI, raid and listener values"),
    }
}

/// `ChunkMap.isChunkTracked` for every operator session
/// of one level, which is the population every level channel broadcasts within.
fn chunk_trackers(
    players: &FxHashMap<ConnectionId, super::PlayerSession>,
    audience: &BTreeSet<ConnectionId>,
) -> BTreeMap<ChunkKey, BTreeSet<ConnectionId>> {
    let mut trackers: BTreeMap<ChunkKey, BTreeSet<ConnectionId>> = BTreeMap::new();
    for connection_id in audience {
        let Some(player) = players.get(connection_id) else {
            continue;
        };
        let Some(view) = player.chunk_tracking_view else {
            continue;
        };
        for chunk in view.for_each() {
            if player.chunk_sender.is_pending(chunk) {
                continue;
            }
            trackers.entry(chunk).or_default().insert(*connection_id);
        }
    }
    trackers
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct LevelDebugSubscriptions {
    sleeping: bool,
    enabled: BTreeSet<i32>,
    subscribers: BTreeMap<i32, BTreeSet<ConnectionId>>,
    goals: EntityValuePublication,
    paths: EntityValuePublication,
    brains: EntityValuePublication,
    pois: LevelValuePublication<[i32; 3]>,
    raids: LevelValuePublication<ChunkKey>,
    game_event_listeners: LevelValuePublication<[i32; 3]>,
}

impl Default for LevelDebugSubscriptions {
    fn default() -> Self {
        Self {
            sleeping: true,
            enabled: BTreeSet::new(),
            subscribers: BTreeMap::new(),
            goals: EntityValuePublication::default(),
            paths: EntityValuePublication::default(),
            brains: EntityValuePublication::default(),
            pois: LevelValuePublication::default(),
            raids: LevelValuePublication::default(),
            game_event_listeners: LevelValuePublication::default(),
        }
    }
}

impl LevelDebugSubscriptions {
    fn update_activation(&mut self, enabled: BTreeSet<i32>, entities: &mut EntityStore) -> bool {
        let sleeping = enabled.is_empty();
        if sleeping != self.sleeping {
            self.subscribers.clear();
            self.goals.clear();
            self.paths.clear();
            self.brains.clear();
            self.pois.clear();
            self.raids.clear();
            self.game_event_listeners.clear();
        }
        self.sleeping = sleeping;
        self.enabled = enabled;
        entities.set_entity_path_debug_capture(
            self.enabled
                .contains(&VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID),
        );
        !self.sleeping
    }

    fn publish(
        &mut self,
        subscribers: BTreeMap<i32, BTreeSet<ConnectionId>>,
        chunk_trackers: &BTreeMap<ChunkKey, BTreeSet<ConnectionId>>,
        level: LevelChannelSources<'_>,
        brain_dumps: &BTreeMap<i32, DebugBrainDump>,
        (entities, tracker): (&EntityStore, &EntityTracker),
        events: &mut Vec<WorldEvent>,
    ) {
        self.subscribers = subscribers;
        let goal_subscribers = self
            .subscribers
            .get(&VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID)
            .cloned()
            .unwrap_or_default();
        self.goals.publish(
            VANILLA_DEBUG_SUBSCRIPTION_GOAL_SELECTORS_ID,
            &goal_subscribers,
            (entities, tracker),
            false,
            |entity_id| {
                entities
                    .mob_goal_debug_info(entity_id)
                    .map(DebugSubscriptionValue::Goals)
            },
            events,
        );

        let path_subscribers = self
            .subscribers
            .get(&VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID)
            .cloned()
            .unwrap_or_default();
        self.paths.publish(
            VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID,
            &path_subscribers,
            (entities, tracker),
            true,
            |entity_id| {
                entities
                    .mob_path_debug_info(entity_id)
                    .map(DebugSubscriptionValue::Path)
            },
            events,
        );

        // `Mob.registerDebugValues` (Mob.java:1439-1441) registers the source
        // only for a brain that is not brain-dead, so a mob this answers
        // nothing for is never cleared either.
        let brain_subscribers = self
            .subscribers
            .get(&VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID)
            .cloned()
            .unwrap_or_default();
        self.brains.publish(
            VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID,
            &brain_subscribers,
            (entities, tracker),
            false,
            |entity_id| {
                brain_dumps
                    .get(&entity_id)
                    .cloned()
                    .map(DebugSubscriptionValue::Brain)
            },
            events,
        );

        let LevelChannelSources { chunks, raids } = level;
        // `PoiSynchronizer.sendInitialChunk` (TrackingDebugSynchronizer.java:110-117)
        // publishes every record a tracked column holds; `onPoiRemoved` (`:127-129`)
        // clears the position a record left.
        let poi_subscribers = self
            .subscribers
            .get(&VANILLA_DEBUG_SUBSCRIPTION_POIS_ID)
            .cloned()
            .unwrap_or_default();
        let poi_values = if poi_subscribers.is_empty() {
            BTreeMap::new()
        } else {
            chunk_trackers
                .keys()
                .flat_map(|&chunk| chunks.poi_mut().records_in_chunk(chunk))
                .filter_map(|record| {
                    let [x, y, z] = record.pos;
                    Some((
                        record.pos,
                        DebugSubscriptionValue::Poi(DebugPoiInfo {
                            pos: BlockPos { x, y, z },
                            poi_type_holder_id: poi_type_holder_id(record.type_id)?,
                            free_ticket_count: record.free_tickets,
                        }),
                    ))
                })
                .collect()
        };
        self.pois.publish(
            (chunk_trackers, &poi_subscribers),
            poi_values,
            true,
            |[x, _, z]| (x >> 4, z >> 4),
            |[x, y, z], update| {
                ClientboundPlayPacket::DebugBlockValue(Arc::new(DebugBlockValue {
                    pos: BlockPos { x, y, z },
                    update,
                }))
            },
            events,
        );

        // `EuclideanGameEventListenerRegistry.sendDebugInfo`
        // (`EuclideanGameEventListenerRegistry.java:45-59`): a
        // `BlockPositionSource` listener is keyed by its block position. An
        // `EntityPositionSource` one goes out as an entity value (`:54-58`),
        // which this publication does not carry.
        let listener_subscribers = self
            .subscribers
            .get(&VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENT_LISTENERS_ID)
            .cloned()
            .unwrap_or_default();
        let listener_values = if listener_subscribers.is_empty() {
            BTreeMap::new()
        } else {
            chunks
                .block_entities()
                .game_event_listeners()
                .listeners()
                .filter(|listener| {
                    matches!(listener.owner, GameEventListenerOwner::BlockEntity)
                        && chunk_trackers.contains_key(&(listener.pos.x >> 4, listener.pos.z >> 4))
                })
                .map(|listener| {
                    (
                        [listener.pos.x, listener.pos.y, listener.pos.z],
                        DebugSubscriptionValue::GameEventListener(DebugGameEventListenerInfo {
                            listener_radius: listener.radius,
                        }),
                    )
                })
                .collect()
        };
        self.game_event_listeners.publish(
            (chunk_trackers, &listener_subscribers),
            listener_values,
            false,
            |[x, _, z]| (x >> 4, z >> 4),
            |[x, y, z], update| {
                ClientboundPlayPacket::DebugBlockValue(Arc::new(DebugBlockValue {
                    pos: BlockPos { x, y, z },
                    update,
                }))
            },
            events,
        );

        // `LevelChunk.registerDebugValues` (LevelChunk.java:638-661) registers the
        // raid channel for every column, so a column with no raid in it
        // answers the empty list rather than nothing.
        let raid_subscribers = self
            .subscribers
            .get(&VANILLA_DEBUG_SUBSCRIPTION_RAIDS_ID)
            .cloned()
            .unwrap_or_default();
        let raid_values = if raid_subscribers.is_empty() {
            BTreeMap::new()
        } else {
            chunk_trackers
                .keys()
                .map(|&chunk| {
                    (
                        chunk,
                        DebugSubscriptionValue::Raids(raids.raid_centers_in_chunk(chunk)),
                    )
                })
                .collect()
        };
        self.raids.publish(
            (chunk_trackers, &raid_subscribers),
            raid_values,
            false,
            |chunk| chunk,
            |chunk, update| {
                ClientboundPlayPacket::DebugChunkValue(Arc::new(DebugChunkValue {
                    pos: ChunkPos {
                        x: chunk.0,
                        z: chunk.1,
                    },
                    update,
                }))
            },
            events,
        );
    }

    #[cfg(test)]
    fn publish_missing_path_for_test(
        &mut self,
        missing_entity_id: i32,
        entities: &EntityStore,
        tracker: &EntityTracker,
        events: &mut Vec<WorldEvent>,
    ) {
        let subscribers = self
            .subscribers
            .get(&VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID)
            .cloned()
            .unwrap_or_default();
        self.paths.publish(
            VANILLA_DEBUG_SUBSCRIPTION_ENTITY_PATHS_ID,
            &subscribers,
            (entities, tracker),
            true,
            |entity_id| {
                (entity_id != missing_entity_id)
                    .then(|| entities.mob_path_debug_info(entity_id))
                    .flatten()
                    .map(DebugSubscriptionValue::Path)
            },
            events,
        );
    }
}

impl ServerWorld {
    pub(super) fn tick_level_debug_subscriptions(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let enabled = self.cold.debug_subscriptions.enabled.clone();
        let subscribers = self
            .levels
            .player_entries()
            .filter(|(_, player)| {
                &player.dimension == dimension && self.admin.is_operator(player.profile.uuid)
            })
            .fold(
                BTreeMap::new(),
                |mut subscribers, (connection_id, player)| {
                    for subscription_id in player.cold.requested_debug_subscriptions.iter() {
                        subscribers
                            .entry(subscription_id)
                            .or_insert_with(BTreeSet::new)
                            .insert(*connection_id);
                    }
                    subscribers
                },
            );

        let audience = subscribers.values().flatten().copied().collect();
        let chunk_trackers = chunk_trackers(&self.sessions, &audience);
        let brain_dumps = if subscribers.contains_key(&VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID) {
            self.brain_debug_dumps(dimension)
        } else {
            BTreeMap::new()
        };

        let level = self.level_mut_by_id(dimension);
        let ServerLevel {
            entities,
            entity_tracker,
            debug_subscriptions,
            chunks,
            raids,
            ..
        } = level;
        if debug_subscriptions.update_activation(enabled, entities) {
            debug_subscriptions.publish(
                subscribers,
                &chunk_trackers,
                LevelChannelSources { chunks, raids },
                &brain_dumps,
                (entities, entity_tracker),
                events,
            );
        }
    }

    /// `RemoteSampleLogger.useSample` (`RemoteSampleLogger.java:21-25`): every
    /// operator subscribed to `DEDICATED_SERVER_TICK_TIME`.
    pub(super) fn send_tick_time_sample(&self, sample: [i64; 4], events: &mut Vec<WorldEvent>) {
        let packet = ClientboundPlayPacket::DebugSample(bbb_protocol::packets::DebugSample {
            sample: sample.to_vec(),
            sample_type: bbb_protocol::packets::RemoteDebugSampleType::TickTime,
        });
        for (&connection_id, player) in self.levels.player_entries() {
            if self.admin.is_operator(player.profile.uuid)
                && player
                    .cold
                    .requested_debug_subscriptions
                    .requested
                    .contains(&VANILLA_DEBUG_SUBSCRIPTION_DEDICATED_SERVER_TICK_TIME_ID)
            {
                events.push(send(connection_id, packet.clone()));
            }
        }
    }

    pub(super) fn refresh_debug_subscriptions(&mut self) {
        self.cold.debug_subscriptions.enabled = self
            .levels
            .players()
            .filter(|player| self.admin.is_operator(player.profile.uuid))
            .flat_map(|player| player.cold.requested_debug_subscriptions.iter())
            .collect();
    }

    /// `LevelDebugSynchronizers.broadcastEventToTracking`
    /// (`LevelDebugSynchronizers.java:205-209`) for a game event that reached
    /// at least one listener: send the event value in a
    /// `ClientboundDebugEventPacket` to every operator subscribed to
    /// `DebugSubscriptions.GAME_EVENTS` whose chunk tracking view covers the
    /// event's block column.
    ///
    /// Vanilla reads the subscription set through `ServerPlayer.debugSubscriptions`
    /// (`ServerPlayer.java:2269-2271`), so the permission gate is re-read here
    /// instead of relying on the level's cached enabled set.
    pub(super) fn broadcast_game_event_debug(
        &self,
        dimension: &LevelId,
        game_event: VanillaGameEvent,
        position: bbb_protocol::packets::Vec3d,
        events: &mut Vec<WorldEvent>,
    ) {
        let block_pos = crate::block_entity::game_event::containing_block_pos(position);
        let chunk: ChunkKey = (block_pos.x >> 4, block_pos.z >> 4);
        for (connection_id, player) in self.levels.player_entries() {
            if &player.dimension != dimension {
                continue;
            }
            if !self.admin.is_operator(player.profile.uuid) {
                continue;
            }
            if !player
                .cold
                .requested_debug_subscriptions
                .iter()
                .any(|subscription_id| subscription_id == VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID)
            {
                continue;
            }
            let session = &self.sessions[connection_id];
            let Some(view) = session.chunk_tracking_view else {
                continue;
            };
            if session.chunk_sender.is_pending(chunk) || !view.contains(chunk) {
                continue;
            }
            events.push(send(
                *connection_id,
                ClientboundPlayPacket::DebugEvent(DebugEvent {
                    event: DebugSubscriptionEvent {
                        subscription_id: VANILLA_DEBUG_SUBSCRIPTION_GAME_EVENTS_ID,
                        value: DebugSubscriptionValue::GameEvent(DebugGameEventInfo {
                            game_event_holder_id: game_event.holder_id(),
                            pos: position,
                        }),
                    },
                }),
            ));
        }
    }

    #[cfg(test)]
    pub(crate) fn publish_missing_path_for_test(
        &mut self,
        dimension: LevelId,
        entity_id: i32,
    ) -> Vec<WorldEvent> {
        let level = self.level_mut_by_id(&dimension);
        let ServerLevel {
            entities,
            entity_tracker,
            debug_subscriptions,
            ..
        } = level;
        let mut events = Vec::new();
        debug_subscriptions.publish_missing_path_for_test(
            entity_id,
            entities,
            entity_tracker,
            &mut events,
        );
        events
    }
}
