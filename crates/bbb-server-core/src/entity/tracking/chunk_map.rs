//! `ChunkMap`'s entity half: which sessions are paired with each tracker.
//!
//! Vanilla re-pairs every tracker against a session on each of its moves and
//! every viewer against a tracker whenever that tracker changes section. The
//! verdicts here are the same, but a moving session is only offered the
//! non-avatar trackers filed in the chunk columns its view distance reaches
//! plus those it already sees; the rest cannot pass the range test. A tracker
//! that moved since the last sweep is still filed under its old column until
//! the sweep's own section-change pass pairs it with everyone.

use std::collections::{BTreeMap, BTreeSet};

use super::*;

/// `SoundEvent.getRange(float)` (`net/minecraft/sounds/SoundEvent.java:40-42`)
/// for a sound holder: `volume > 1.0F ? 16.0F * volume : 16.0F`.
fn sound_holder_range(holder: &bbb_protocol::packets::SoundEventHolder, volume: f32) -> f64 {
    let fixed_range = match holder {
        bbb_protocol::packets::SoundEventHolder::Reference { .. } => None,
        bbb_protocol::packets::SoundEventHolder::Direct { fixed_range, .. } => *fixed_range,
    };
    f64::from(fixed_range.unwrap_or(if volume > 1.0 { 16.0 * volume } else { 16.0 }))
}

fn sound_range(sound: &bbb_protocol::packets::SoundEvent) -> f64 {
    sound_holder_range(&sound.sound, sound.volume)
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackingPacket {
    pub recipients: Vec<ConnectionId>,
    pub packet: ClientboundPlayPacket,
}

#[cfg(test)]
pub(crate) fn recipient_packets(
    out: &[TrackingPacket],
) -> impl Iterator<Item = (&ConnectionId, &ClientboundPlayPacket)> {
    out.iter().flat_map(|targeted| {
        targeted
            .recipients
            .iter()
            .map(move |recipient| (recipient, &targeted.packet))
    })
}

fn push_tracking_packet(
    out: &mut Vec<TrackingPacket>,
    recipients: impl IntoIterator<Item = ConnectionId>,
    packet: ClientboundPlayPacket,
) {
    let recipients: Vec<_> = recipients.into_iter().collect();
    push_tracking_packet_vec(out, recipients, packet);
}

fn push_tracking_packet_vec(
    out: &mut Vec<TrackingPacket>,
    recipients: Vec<ConnectionId>,
    packet: ClientboundPlayPacket,
) {
    if !recipients.is_empty() {
        out.push(TrackingPacket { recipients, packet });
    }
}

fn index_viewers_by_entity_id(viewers: &[TrackingViewer], index: &mut FxHashMap<i32, usize>) {
    index.clear();
    index.reserve(viewers.len());
    for (slot, viewer) in viewers.iter().enumerate() {
        index.insert(viewer.entity_id, slot);
    }
}

/// The inverse of every tracker's `seen_by`: what each session is paired with.
type SeenIndex = FxHashMap<ConnectionId, FxHashSet<i32>>;

fn unlink(seen: &mut SeenIndex, connection_id: ConnectionId, entity_id: i32) {
    if let Some(ids) = seen.get_mut(&connection_id) {
        ids.remove(&entity_id);
        if ids.is_empty() {
            seen.remove(&connection_id);
        }
    }
}

/// `TrackedEntity.updatePlayer`'s horizontal range test (`ChunkMap.java:1387-1392`).
fn within_range(range: i32, viewer: Vec3d, view_distance: i32, target: Vec3d) -> bool {
    let delta_x = viewer.x - target.x;
    let delta_z = viewer.z - target.z;
    let visible_range = f64::from(range.min(view_distance * CHUNK_BLOCKS));
    delta_x * delta_x + delta_z * delta_z <= visible_range * visible_range
}

/// One `addPairing` bundle (`ServerEntity.java:262-267`) to every new viewer.
fn push_pairing(
    out: &mut Vec<TrackingPacket>,
    recipients: &[ConnectionId],
    packets: Vec<ClientboundPlayPacket>,
) {
    let delimiter = || std::iter::once(ClientboundPlayPacket::BundleDelimiter);
    for packet in delimiter().chain(packets).chain(delimiter()) {
        push_tracking_packet(out, recipients.iter().copied(), packet);
    }
}

/// The level facts a non-player pairing reads besides the viewers it decides.
struct Pairing<'a, F> {
    /// Every session of the level, any of which may own a projectile.
    sessions: &'a [TrackingViewer],
    entity_ids: &'a FxHashMap<uuid::Uuid, i32>,
    tracks_chunk: &'a F,
}

/// `ChunkMap.TrackedEntity` (`ChunkMap.java:1322-1432`).
#[derive(Debug, Clone, PartialEq)]
struct TrackedEntity {
    server_entity: ServerEntity,
    /// `EntityType.clientTrackingRange() * 16` (`ChunkMap.java:1142`), in
    /// blocks.
    range: i32,
    /// `TrackedEntity.getEffectiveRange` (`ChunkMap.java:1414-1425`), scaled
    /// and cached until the passenger tree or the percentage changes.
    effective_range: i32,
    last_section_pos: SectionKey,
    /// `TrackedEntity.seenBy` (`ChunkMap.java:1327`), an identity set of
    /// connections.
    seen_by: FxHashSet<ConnectionId>,
    last_passengers: Vec<i32>,
    player: Option<TrackedPlayer>,
}

#[derive(Debug, Clone, PartialEq)]
struct TrackedPlayer {
    connection_id: ConnectionId,
    uuid: uuid::Uuid,
}

impl TrackedPlayer {
    fn refers_to(&self, viewer: &TrackingViewer) -> bool {
        self.connection_id == viewer.connection_id && self.uuid == viewer.uuid
    }
}

impl TrackedEntity {
    /// The `TrackedEntity` constructor (`ChunkMap.java:1329-1334`).
    fn new(
        entity: &Entity,
        range: i32,
        update_interval: i32,
        track_delta: bool,
        item_stack_registries: Arc<ItemStackRegistryPublication>,
    ) -> Self {
        Self {
            server_entity: ServerEntity::new_with_item_stack_registries(
                entity,
                update_interval,
                track_delta,
                item_stack_registries,
            ),
            range,
            effective_range: range,
            last_section_pos: section_of_block(entity.block_position()),
            seen_by: FxHashSet::default(),
            last_passengers: Vec::new(),
            player: None,
        }
    }

    fn new_player(
        player: &crate::world::PlayerState,
        frame: &PlayerSyncFrame,
        item_stack_registries: Arc<ItemStackRegistryPublication>,
    ) -> Self {
        let range = vanilla_entity_type(VANILLA_ENTITY_TYPE_PLAYER_ID)
            .expect("the generated entity catalog contains PLAYER")
            .client_tracking_range
            * CHUNK_BLOCKS;
        let mut server_entity = ServerEntity::for_player(frame);
        server_entity.item_stack_registries = item_stack_registries;
        Self {
            server_entity,
            range,
            effective_range: range,
            last_section_pos: section_of_block(crate::entity::containing_block(frame.position)),
            seen_by: FxHashSet::default(),
            last_passengers: Vec::new(),
            player: Some(TrackedPlayer {
                connection_id: player.connection_id(),
                uuid: player.uuid(),
            }),
        }
    }

    const fn id(&self) -> i32 {
        self.server_entity.entity_id
    }

    fn avatar_in_indexed<'a>(
        &self,
        viewers: &'a [TrackingViewer],
        index: &FxHashMap<i32, usize>,
    ) -> Option<&'a TrackingViewer> {
        let identity = self.player.as_ref()?;
        let viewer = viewers.get(*index.get(&self.id())?)?;
        identity.refers_to(viewer).then_some(viewer)
    }

    /// `TrackedEntity.broadcastRemoved` (`ChunkMap.java:1370-1374`).
    fn broadcast_removed(&self, seen: &mut SeenIndex, out: &mut Vec<TrackingPacket>) {
        if self.seen_by.is_empty() {
            return;
        }
        for &connection_id in &self.seen_by {
            unlink(seen, connection_id, self.id());
        }
        push_tracking_packet(
            out,
            self.seen_by.iter().copied(),
            self.server_entity.remove_pairing(),
        );
    }

    /// `TrackedEntity.removePlayer` (`ChunkMap.java:1376-1383`) minus the debug
    /// synchronizer, which has no owner here.
    fn remove_player(
        &mut self,
        connection_id: ConnectionId,
        seen: &mut SeenIndex,
        out: &mut Vec<TrackingPacket>,
    ) {
        if self.seen_by.remove(&connection_id) {
            unlink(seen, connection_id, self.id());
            push_tracking_packet(out, [connection_id], self.server_entity.remove_pairing());
        }
    }

    /// Applies one `updatePlayer` verdict per viewer, unpairing the viewers
    /// that lost sight in one packet, and answers the ones newly paired.
    fn apply_visibility(
        &mut self,
        verdicts: impl Iterator<Item = (ConnectionId, bool)>,
        seen: &mut SeenIndex,
        out: &mut Vec<TrackingPacket>,
    ) -> Vec<ConnectionId> {
        let id = self.id();
        let mut added = Vec::new();
        let mut removed = Vec::new();
        for (connection_id, visible) in verdicts {
            if visible {
                if self.seen_by.insert(connection_id) {
                    seen.entry(connection_id).or_default().insert(id);
                    added.push(connection_id);
                }
            } else if self.seen_by.remove(&connection_id) {
                unlink(seen, connection_id, id);
                removed.push(connection_id);
            }
        }
        if !removed.is_empty() {
            push_tracking_packet_vec(out, removed, self.server_entity.remove_pairing());
        }
        added
    }

    /// `TrackedEntity.updatePlayer` (`ChunkMap.java:1385-1408`).
    ///
    /// `Entity.broadcastToPlayer` (`Entity.java:3362-3364`) is `true` for
    /// every non-player kind.
    fn update_players<F>(
        &mut self,
        entity: &Entity,
        viewers: &[TrackingViewer],
        level: &Pairing<'_, F>,
        seen: &mut SeenIndex,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
    {
        let position = entity.position();
        let block = entity.block_position();
        let range = self.effective_range;
        let added = self.apply_visibility(
            viewers.iter().map(|viewer| {
                let visible = within_range(range, viewer.position, viewer.view_distance, position)
                    && (level.tracks_chunk)(viewer.connection_id, block.x >> 4, block.z >> 4);
                (viewer.connection_id, visible)
            }),
            seen,
            out,
        );
        if added.is_empty() {
            return;
        }
        let data = add_entity_data(entity.data(), level.sessions, level.entity_ids);
        let mut packets = self.server_entity.send_pairing_data_with_data(entity, data);
        if !self.last_passengers.is_empty() {
            packets.push(ClientboundPlayPacket::SetPassengers(SetPassengers {
                vehicle_id: entity.id(),
                passenger_ids: self.last_passengers.clone(),
            }));
        }
        push_pairing(out, &added, packets);
    }

    /// `updatePlayer` for a session's avatar, whose `broadcastToPlayer` hides a
    /// spectator from everyone but spectators watching through their own eyes.
    fn update_player_avatars<F>(
        &mut self,
        store: &EntityStore,
        viewers: &[TrackingViewer],
        tracks_chunk: &F,
        seen: &mut SeenIndex,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
    {
        let Some(identity) = &self.player else {
            return;
        };
        let (own_connection, uuid) = (identity.connection_id, identity.uuid);
        let Some(target) = store.player(self.id()) else {
            return;
        };
        let position = target.body().position();
        let block = crate::entity::containing_block(position);
        let (target_spectator, camera_is_self) = (target.is_spectator(), target.camera_is_self());
        let range = self.effective_range;
        let added = self.apply_visibility(
            viewers
                .iter()
                .filter(|viewer| !(viewer.connection_id == own_connection && viewer.uuid == uuid))
                .map(|viewer| {
                    let viewer_spectator = store
                        .player(viewer.entity_id)
                        .is_some_and(crate::world::PlayerState::is_spectator);
                    let broadcast = if viewer_spectator {
                        camera_is_self
                    } else {
                        !target_spectator
                    };
                    let visible =
                        within_range(range, viewer.position, viewer.view_distance, position)
                            && broadcast
                            && tracks_chunk(viewer.connection_id, block.x >> 4, block.z >> 4);
                    (viewer.connection_id, visible)
                }),
            seen,
            out,
        );
        if added.is_empty() {
            return;
        }
        let packets = self
            .server_entity
            .send_player_pairing_data(uuid, target.pairing_data(store));
        push_pairing(out, &added, packets);
    }

    fn route_player_changes(
        &self,
        owner: ConnectionId,
        packets: PlayerSyncPackets,
        out: &mut Vec<TrackingPacket>,
    ) {
        for targeted in packets.ordered {
            let include_owner = targeted.audience == PlayerSyncAudience::TrackingAndSelf
                && !self.seen_by.contains(&owner);
            let total = self.seen_by.len() + usize::from(include_owner);
            if total == 0 {
                continue;
            }
            let mut recipients = Vec::with_capacity(total);
            recipients.extend(self.seen_by.iter().copied());
            if include_owner {
                recipients.push(owner);
            }
            push_tracking_packet_vec(out, recipients, targeted.packet);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntityTrackingSpec {
    client_tracking_range: i32,
    update_interval: i32,
    track_deltas: bool,
}

fn entity_tracking_spec(entity: &Entity) -> Option<EntityTrackingSpec> {
    if let Some(plugin) = entity.plugin_mob() {
        return (plugin.resource_id == entity.data().entity_resource_id()).then_some(
            EntityTrackingSpec {
                client_tracking_range: plugin.client_tracking_range,
                update_interval: plugin.update_interval,
                track_deltas: plugin.track_deltas,
            },
        );
    }
    let catalog = vanilla_entity_type(entity.entity_type_id())?;
    (catalog.resource_id == entity.data().entity_resource_id()).then_some(EntityTrackingSpec {
        client_tracking_range: catalog.client_tracking_range,
        update_interval: catalog.update_interval,
        track_deltas: catalog.track_deltas,
    })
}

/// Reusable scratch buffers for [`EntityTracker`] ticks, ignored by `PartialEq`
/// so steady-state tracking sweeps avoid per-tick heap allocations.
#[derive(Debug, Clone, Default)]
struct TrackingScratch {
    stale_avatars: Vec<(ConnectionId, i32)>,
    absent_viewers: Vec<ConnectionId>,
    moved_viewers: Vec<TrackingViewer>,
    viewer_by_entity_id: FxHashMap<i32, usize>,
    range_stack: Vec<i32>,
    range_visited: BTreeSet<i32>,
    candidate_ids: Vec<i32>,
}

impl PartialEq for TrackingScratch {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

/// `ChunkMap.entityMap` (`ChunkMap.java:146`) with the indexes that keep
/// pairing incremental.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EntityTracker {
    entities: FxHashMap<i32, TrackedEntity>,
    entity_ids: FxHashMap<uuid::Uuid, i32>,
    /// Tracked item frames / glow item frames, for O(#frames) map-due probes.
    item_frame_ids: FxHashSet<i32>,
    item_stack_registries: Arc<ItemStackRegistryPublication>,
    /// Session avatars by the connection that owns them.
    avatars: FxHashMap<ConnectionId, i32>,
    /// Non-avatar trackers by the chunk column of their `last_section_pos`.
    by_column: FxHashMap<(i32, i32), Vec<i32>>,
    seen: SeenIndex,
    /// Trackers whose passenger tree changed since their effective range was
    /// read; their vehicles above them are refreshed with them.
    stale_ranges: Vec<i32>,
    range_percentage: BroadcastRangePercentage,
    minecart_improvements: bool,
    /// Stored-only callers keep their pre-player-tracker section history here.
    /// The unified player path uses each avatar's `TrackedEntity.last_section_pos`.
    legacy_viewer_sections: FxHashMap<ConnectionId, SectionKey>,
    scratch: TrackingScratch,
}

/// `entity-broadcast-range-percentage` (`DedicatedServerProperties.java:114-116`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BroadcastRangePercentage(i32);

impl Default for BroadcastRangePercentage {
    fn default() -> Self {
        Self(100)
    }
}

impl BroadcastRangePercentage {
    /// `DedicatedServer.getScaledTrackingDistance` (`DedicatedServer.java:764-766`).
    const fn scale(self, range: i32) -> i32 {
        self.0 * range / 100
    }
}

const fn column_of(section: SectionKey) -> (i32, i32) {
    section.column()
}

fn file_in_column(by_column: &mut FxHashMap<(i32, i32), Vec<i32>>, section: SectionKey, id: i32) {
    by_column.entry(column_of(section)).or_default().push(id);
}

fn unfile_from_column(
    by_column: &mut FxHashMap<(i32, i32), Vec<i32>>,
    section: SectionKey,
    id: i32,
) {
    let column = column_of(section);
    if let Some(ids) = by_column.get_mut(&column) {
        if let Some(index) = ids.iter().position(|filed| *filed == id) {
            ids.swap_remove(index);
        }
        if ids.is_empty() {
            by_column.remove(&column);
        }
    }
}

impl EntityTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn set_minecart_improvements(&mut self, enabled: bool) {
        self.minecart_improvements = enabled;
    }

    pub(crate) fn any_framed_map_due(&self, store: &EntityStore) -> bool {
        self.item_frame_ids.iter().any(|&id| {
            self.entities.get(&id).is_some_and(|tracked| {
                tracked.player.is_none()
                    && store.get(id).is_some_and(|entity| {
                        tracked.server_entity.framed_map_due(entity).is_some()
                    })
            })
        })
    }

    pub fn new_with_item_stack_holder_ids(
        item_stack_holder_ids: BTreeMap<(String, String), i32>,
    ) -> Self {
        let registry_ids = item_stack_holder_ids
            .keys()
            .map(|(registry, _)| registry.clone())
            .collect();
        Self {
            item_stack_registries: Arc::new(ItemStackRegistryPublication {
                registry_ids,
                holder_ids: item_stack_holder_ids,
                tags: None,
            }),
            ..Self::default()
        }
    }

    pub fn new_with_item_stack_registry_view(
        registry_ids: BTreeSet<String>,
        holder_ids: BTreeMap<(String, String), i32>,
        tags: Arc<RegistryTagSnapshot>,
    ) -> Self {
        let mut tracker = Self::default();
        tracker.replace_item_stack_registry_view(registry_ids, holder_ids, tags);
        tracker
    }

    pub fn replace_item_stack_registry_view(
        &mut self,
        mut registry_ids: BTreeSet<String>,
        holder_ids: BTreeMap<(String, String), i32>,
        tags: Arc<RegistryTagSnapshot>,
    ) {
        registry_ids.extend(holder_ids.keys().map(|(registry, _)| registry.clone()));
        let publication = Arc::new(ItemStackRegistryPublication {
            registry_ids,
            holder_ids,
            tags: Some(tags),
        });
        for entity in self.entities.values_mut() {
            entity.server_entity.item_stack_registries = Arc::clone(&publication);
        }
        self.item_stack_registries = publication;
    }

    /// The publication this level's item-stack reads project through, which is
    /// the holder table the equipment row already uses.
    pub fn item_stack_resolver(&self) -> TrackingItemStackResolver<'_> {
        self.item_stack_registries.resolver()
    }

    /// How many entities of this level are tracked, for tests and receipts.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// The sessions currently paired with one entity, ascending.
    pub fn viewers_of(&self, entity_id: i32) -> Vec<ConnectionId> {
        let mut viewers: Vec<ConnectionId> = self
            .entities
            .get(&entity_id)
            .map(|tracked| tracked.seen_by.iter().copied().collect())
            .unwrap_or_default();
        viewers.sort_unstable();
        viewers
    }

    /// `ChunkMap.sendToTrackingPlayersAndSelf` drops the packet when its
    /// entity id has no tracker (`ChunkMap.java:1230-1234`).
    pub fn player_viewers_of(
        &self,
        connection_id: ConnectionId,
        entity_id: i32,
    ) -> Option<Vec<ConnectionId>> {
        let tracked = self.entities.get(&entity_id)?;
        tracked
            .player
            .as_ref()
            .is_some_and(|player| player.connection_id == connection_id)
            .then(|| {
                let mut viewers: Vec<ConnectionId> = tracked.seen_by.iter().copied().collect();
                viewers.sort_unstable();
                viewers
            })
    }

    /// `ChunkMap.removeEntity(ServerPlayer)` (`ChunkMap.java:1166-1172`): every
    /// tracker drops the session.
    ///
    /// The returned packets are the ones that session would be sent. A
    /// disconnect discards them, exactly as `ServerWorld::remove_player` drops
    /// the leaver's own copies of the departure packets, because its route is
    /// already gone; a dimension change sends them, because the session is
    /// still there and has to forget the level it left.
    pub fn viewer_left(&mut self, connection_id: ConnectionId) -> Vec<ClientboundPlayPacket> {
        self.legacy_viewer_sections.remove(&connection_id);
        let mut out = Vec::new();
        self.unpair_session(connection_id, &mut out);
        out.into_iter().map(|targeted| targeted.packet).collect()
    }

    fn unpair_session(&mut self, connection_id: ConnectionId, out: &mut Vec<TrackingPacket>) {
        let Some(ids) = self.seen.remove(&connection_id) else {
            return;
        };
        for id in ids {
            if let Some(tracked) = self.entities.get_mut(&id) {
                if tracked.seen_by.remove(&connection_id) {
                    push_tracking_packet(
                        out,
                        [connection_id],
                        tracked.server_entity.remove_pairing(),
                    );
                }
            }
        }
    }

    /// `ChunkMap.addEntity(ServerPlayer)`'s target admission phase
    /// (`ChunkMap.java:1139-1159`) for the player `store` holds under
    /// `entity_id`.
    pub fn add_player<F>(
        &mut self,
        entity_id: i32,
        viewers: &[TrackingViewer],
        store: &EntityStore,
        tracks_chunk: F,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
    {
        let Some(player) = store.player(entity_id) else {
            return;
        };
        let (connection_id, uuid) = (player.connection_id(), player.uuid());
        if self.entities.contains_key(&entity_id)
            || self.avatars.contains_key(&connection_id)
            || self
                .entity_ids
                .get(&uuid)
                .is_some_and(|&id| id != entity_id)
        {
            return;
        }
        self.entity_ids.insert(uuid, entity_id);
        self.avatars.insert(connection_id, entity_id);
        let riding = store.external_player_vehicle(entity_id).is_some();
        let registries = Arc::clone(&self.item_stack_registries);
        let percentage = self.range_percentage;
        let tracked = self.entities.entry(entity_id).or_insert_with(|| {
            let mut tracked =
                TrackedEntity::new_player(player, &player.tracking_frame(riding), registries);
            tracked.effective_range = percentage.scale(tracked.range);
            tracked
        });
        tracked.update_player_avatars(store, viewers, &tracks_chunk, &mut self.seen, out);
    }

    /// `ChunkMap.move` for a session (`ChunkMap.java:1065-1072`): its avatar
    /// first, then the session as a viewer of everything else.
    pub fn move_player_body<F>(
        &mut self,
        mover: &TrackingViewer,
        viewers: &[TrackingViewer],
        store: &EntityStore,
        tracks_chunk: F,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
    {
        self.refresh_ranges(store);
        index_viewers_by_entity_id(viewers, &mut self.scratch.viewer_by_entity_id);
        if let Some(tracked) = self.entities.get_mut(&mover.entity_id) {
            if tracked
                .avatar_in_indexed(viewers, &self.scratch.viewer_by_entity_id)
                .is_some()
            {
                tracked.update_player_avatars(store, viewers, &tracks_chunk, &mut self.seen, out);
            }
        }
        self.reconcile_moved_player(mover, viewers, store, &tracks_chunk, out);
    }

    /// `ChunkMap.removeEntity(ServerPlayer)` (`ChunkMap.java:1165-1177`).
    pub fn remove_player(
        &mut self,
        connection_id: ConnectionId,
        _expected_entity_id: i32,
        out: &mut Vec<TrackingPacket>,
    ) {
        self.unpair_session(connection_id, out);
        let Some(entity_id) = self.avatars.remove(&connection_id) else {
            return;
        };
        if let Some(tracked) = self.entities.remove(&entity_id) {
            if let Some(player) = tracked.player.as_ref() {
                if self.entity_ids.get(&player.uuid).copied() == Some(entity_id) {
                    self.entity_ids.remove(&player.uuid);
                }
            }
            tracked.broadcast_removed(&mut self.seen, out);
        }
    }

    /// `TrackedEntity.updatePlayer(player)` for every tracker but the
    /// session's own (`ChunkMap.java:1065-1072`, `:1207-1211`).
    fn reconcile_moved_player<F>(
        &mut self,
        mover: &TrackingViewer,
        viewers: &[TrackingViewer],
        store: &EntityStore,
        tracks_chunk: &F,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
    {
        let Self {
            entities,
            avatars,
            seen,
            scratch,
            ..
        } = self;
        for &target_id in avatars.values() {
            let Some(tracked) = entities.get_mut(&target_id) else {
                continue;
            };
            let Some(identity) = tracked.player.as_ref() else {
                continue;
            };
            if target_id == mover.entity_id && identity.refers_to(mover) {
                continue;
            }
            if tracked
                .avatar_in_indexed(viewers, &scratch.viewer_by_entity_id)
                .is_none()
            {
                tracked.remove_player(mover.connection_id, seen, out);
                continue;
            }
            tracked.update_player_avatars(
                store,
                std::slice::from_ref(mover),
                tracks_chunk,
                seen,
                out,
            );
        }
        self.update_viewer(mover, viewers, store, tracks_chunk, out);
    }

    pub(crate) fn pair_delivered_chunks<F>(
        &mut self,
        chunks: &[(i32, i32)],
        viewer: &TrackingViewer,
        sessions: &[TrackingViewer],
        store: &EntityStore,
        tracks_chunk: F,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
    {
        if chunks.is_empty() {
            return;
        }
        self.refresh_ranges(store);
        let Self {
            entities,
            entity_ids,
            by_column,
            seen,
            ..
        } = self;
        let level = Pairing {
            sessions,
            entity_ids,
            tracks_chunk: &tracks_chunk,
        };
        let viewers = std::slice::from_ref(viewer);
        for &chunk in chunks {
            let Some(ids) = by_column.get(&chunk) else {
                continue;
            };
            for &id in ids {
                let (Some(tracked), Some(entity)) = (entities.get_mut(&id), store.get(id)) else {
                    continue;
                };
                let block = entity.block_position();
                if (block.x >> 4, block.z >> 4) == chunk {
                    tracked.update_players(entity, viewers, &level, seen, out);
                }
            }
        }
        // Avatars are not filed in by_column; visit the level's sessions once.
        for session in sessions {
            let Some(player) = store.player(session.entity_id) else {
                continue;
            };
            let block = player.body().block_position();
            if !chunks.contains(&(block.x >> 4, block.z >> 4)) {
                continue;
            }
            let Some(tracked) = entities.get_mut(&session.entity_id) else {
                continue;
            };
            if tracked
                .player
                .as_ref()
                .is_some_and(|identity| identity.refers_to(session))
            {
                tracked.update_player_avatars(store, viewers, &tracks_chunk, seen, out);
            }
        }
    }

    /// Re-pairs one moved viewer with every non-avatar tracker it could now
    /// see or stop seeing.
    fn update_viewer<F>(
        &mut self,
        viewer: &TrackingViewer,
        sessions: &[TrackingViewer],
        store: &EntityStore,
        tracks_chunk: &F,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
    {
        let mut candidates = std::mem::take(&mut self.scratch.candidate_ids);
        self.fill_candidates_for(viewer, &mut candidates);
        let Self {
            entities,
            entity_ids,
            seen,
            ..
        } = self;
        let level = Pairing {
            sessions,
            entity_ids,
            tracks_chunk,
        };
        for &id in &candidates {
            let (Some(tracked), Some(entity)) = (entities.get_mut(&id), store.get(id)) else {
                continue;
            };
            tracked.update_players(entity, std::slice::from_ref(viewer), &level, seen, out);
        }
        candidates.clear();
        self.scratch.candidate_ids = candidates;
    }

    /// The non-avatar trackers filed within the viewer's view distance, then
    /// the ones it sees from outside that square.
    fn fill_candidates_for(&self, viewer: &TrackingViewer, ids: &mut Vec<i32>) {
        ids.clear();
        let reach = f64::from(viewer.view_distance.max(0) * CHUNK_BLOCKS);
        let bound = |coordinate: f64| {
            (
                (coordinate - reach).floor() as i32 >> 4,
                (coordinate + reach).floor() as i32 >> 4,
            )
        };
        let (min_x, max_x) = bound(viewer.position.x);
        let (min_z, max_z) = bound(viewer.position.z);
        let inside =
            |(x, z): (i32, i32)| (min_x..=max_x).contains(&x) && (min_z..=max_z).contains(&z);
        let columns = (i64::from(max_x - min_x) + 1) * (i64::from(max_z - min_z) + 1);
        if columns < self.by_column.len() as i64 {
            for x in min_x..=max_x {
                for z in min_z..=max_z {
                    if let Some(filed) = self.by_column.get(&(x, z)) {
                        ids.extend_from_slice(filed);
                    }
                }
            }
        } else {
            for (&column, filed) in &self.by_column {
                if inside(column) {
                    ids.extend_from_slice(filed);
                }
            }
        }
        if let Some(seen) = self.seen.get(&viewer.connection_id) {
            ids.extend(seen.iter().copied().filter(|id| {
                self.entities.get(id).is_some_and(|tracked| {
                    tracked.player.is_none() && !inside(column_of(tracked.last_section_pos))
                })
            }));
        }
    }

    /// Applies a new `entity-broadcast-range-percentage` to every tracker.
    pub fn set_range_percentage(&mut self, percentage: i32, store: &EntityStore) {
        let percentage = BroadcastRangePercentage(percentage);
        if self.range_percentage == percentage {
            return;
        }
        self.range_percentage = percentage;
        for (&id, tracked) in &mut self.entities {
            if tracked.player.is_some() {
                tracked.effective_range = percentage.scale(tracked.range);
            } else {
                self.stale_ranges.push(id);
            }
        }
        self.refresh_ranges(store);
    }

    /// Re-reads the effective range of every tracker in each passenger tree
    /// that changed since the last read.
    fn refresh_ranges(&mut self, store: &EntityStore) {
        if self.stale_ranges.is_empty() {
            return;
        }
        let Self {
            stale_ranges,
            entities,
            range_percentage,
            scratch,
            ..
        } = self;
        let stack = &mut scratch.range_stack;
        let visited = &mut scratch.range_visited;
        stack.clear();
        while let Some(mut root) = stale_ranges.pop() {
            while let Some(vehicle) = store.valid_vehicle(root) {
                root = vehicle;
            }
            stack.push(root);
            while let Some(id) = stack.pop() {
                if let Some(tracked) = entities.get_mut(&id) {
                    if tracked.player.is_none() {
                        visited.clear();
                        tracked.effective_range = range_percentage.scale(effective_tracking_range(
                            store,
                            id,
                            tracked.range,
                            visited,
                        ));
                    }
                }
                stack.extend(
                    store
                        .passengers(id)
                        .iter()
                        .filter(|&&passenger| store.valid_vehicle(passenger) == Some(id)),
                );
            }
        }
        visited.clear();
    }

    /// Applies the ordered tracking callbacks emitted by the entity store.
    ///
    /// `PersistentEntitySectionManager` invokes `onTrackingStart` and
    /// `onTrackingEnd` synchronously with each add and removal
    /// (`PersistentEntitySectionManager.java:61-100`, `:365-449`). Those
    /// callbacks reach `ChunkMap.addEntity` and `removeEntity`
    /// (`ChunkMap.java:1139-1177`), so this must consume the edges in their
    /// original order rather than reconcile the store's final live set.
    /// In particular, the owned snapshot on `TrackingStarted` is the entity passed to
    /// `ServerEntity.sendPairingData`; the live store may already have moved
    /// or removed it by the time this method is called.
    ///
    /// A move does not itself add or remove a tracker. `ChunkMap.tick` notices
    /// the changed section and runs the normal player publication sweep, which
    /// is likewise left to [`EntityTracker::tick`] here.
    pub fn apply_lifecycle_events<F>(
        &mut self,
        events: &[EntityLifecycleEvent],
        viewers: &[TrackingViewer],
        tracks_chunk: F,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
    {
        for event in events {
            match event {
                EntityLifecycleEvent::Added { tracked: false, .. }
                | EntityLifecycleEvent::Removed { tracked: false, .. } => {}
                EntityLifecycleEvent::Added {
                    entity,
                    tracked: true,
                }
                | EntityLifecycleEvent::TrackingStarted { entity } => {
                    let Some(catalog) = entity_tracking_spec(entity) else {
                        continue;
                    };
                    self.entity_ids.insert(entity.uuid(), entity.id());
                    self.stale_ranges.push(entity.id());
                    self.add_entity(entity, catalog, viewers, &tracks_chunk, out);
                }
                EntityLifecycleEvent::Moved { .. } => {}
                EntityLifecycleEvent::WaypointUpdated { .. } => {}
                EntityLifecycleEvent::TrackingStopped { entity }
                | EntityLifecycleEvent::Removed {
                    entity,
                    tracked: true,
                } => {
                    if entity_tracking_spec(entity).is_none()
                        || self.entity_ids.get(&entity.uuid()).copied() != Some(entity.id())
                    {
                        continue;
                    }
                    self.entity_ids.remove(&entity.uuid());
                    self.item_frame_ids.remove(&entity.id());
                    if let Some(tracked) = self.entities.remove(&entity.id()) {
                        unfile_from_column(
                            &mut self.by_column,
                            tracked.last_section_pos,
                            entity.id(),
                        );
                        tracked.broadcast_removed(&mut self.seen, out);
                    }
                }
                // `ChunkMap.sendToTrackingPlayers` (`ChunkMap.java:1214-1219`):
                // `seenBy` only; the entity itself is never a session here.
                EntityLifecycleEvent::Broadcast { entity_id, packet } => {
                    if let Some(tracked) = self.entities.get(entity_id) {
                        if !tracked.seen_by.is_empty() {
                            push_tracking_packet(
                                out,
                                tracked.seen_by.iter().copied(),
                                packet.clone(),
                            );
                        }
                    }
                }
                // ServerLevel.java:1009-1018: all players in this level, independent of tracking.
                EntityLifecycleEvent::BlockDestruction {
                    entity_id,
                    pos,
                    stage,
                } => {
                    let recipients = viewers.iter().filter_map(|viewer| {
                        let dx = f64::from(pos.x) - viewer.position.x;
                        let dy = f64::from(pos.y) - viewer.position.y;
                        let dz = f64::from(pos.z) - viewer.position.z;
                        if viewer.entity_id != *entity_id && dx * dx + dy * dy + dz * dz < 1024.0 {
                            Some(viewer.connection_id)
                        } else {
                            None
                        }
                    });
                    push_tracking_packet(
                        out,
                        recipients,
                        ClientboundPlayPacket::BlockDestruction(
                            bbb_protocol::packets::BlockDestruction {
                                id: *entity_id,
                                pos: *pos,
                                progress: *stage as u8,
                            },
                        ),
                    );
                }
                EntityLifecycleEvent::PassengersChanged {
                    vehicle_id,
                    passenger_ids,
                } => {
                    self.stale_ranges.push(*vehicle_id);
                    if let Some(tracked) = self.entities.get_mut(vehicle_id) {
                        if tracked.last_passengers == *passenger_ids {
                            continue;
                        }
                        let previous =
                            std::mem::replace(&mut tracked.last_passengers, passenger_ids.clone());
                        if tracked.seen_by.is_empty() {
                            continue;
                        }
                        let recipients: Vec<ConnectionId> = tracked
                            .seen_by
                            .iter()
                            .copied()
                            .filter(|&connection_id| {
                                // ServerEntity.java:92-98: changed passengers receive their direct session packet.
                                viewers
                                    .iter()
                                    .find(|viewer| viewer.connection_id == connection_id)
                                    .is_some_and(|viewer| {
                                        previous.contains(&viewer.entity_id)
                                            == passenger_ids.contains(&viewer.entity_id)
                                    })
                            })
                            .collect();
                        if !recipients.is_empty() {
                            let packet = ClientboundPlayPacket::SetPassengers(SetPassengers {
                                vehicle_id: *vehicle_id,
                                passenger_ids: passenger_ids.clone(),
                            });
                            push_tracking_packet_vec(out, recipients, packet);
                        }
                    }
                }
                EntityLifecycleEvent::ExternalPlayerBoardingCooldownReset { .. } => {}
                // `PlayerList.broadcast` (`PlayerList.java:606-616`): every
                // session of this level strictly inside the range, which is
                // `SoundEvent.getRange(volume)` (`SoundEvent.java:40-42`).
                EntityLifecycleEvent::Sound { sound } => {
                    let range = sound_range(sound);
                    let recipients = viewers.iter().filter_map(|viewer| {
                        let dx = sound.position.x - viewer.position.x;
                        let dy = sound.position.y - viewer.position.y;
                        let dz = sound.position.z - viewer.position.z;
                        if dx * dx + dy * dy + dz * dz < range * range {
                            Some(viewer.connection_id)
                        } else {
                            None
                        }
                    });
                    push_tracking_packet(
                        out,
                        recipients,
                        ClientboundPlayPacket::Sound(sound.clone()),
                    );
                }
                EntityLifecycleEvent::SoundExcept {
                    sound,
                    except_entity_id,
                } => {
                    let range = sound_range(sound);
                    let recipients = viewers.iter().filter_map(|viewer| {
                        if viewer.entity_id == *except_entity_id {
                            return None;
                        }
                        let dx = sound.position.x - viewer.position.x;
                        let dy = sound.position.y - viewer.position.y;
                        let dz = sound.position.z - viewer.position.z;
                        if dx * dx + dy * dy + dz * dz < range * range {
                            Some(viewer.connection_id)
                        } else {
                            None
                        }
                    });
                    push_tracking_packet(
                        out,
                        recipients,
                        ClientboundPlayPacket::Sound(sound.clone()),
                    );
                }
                EntityLifecycleEvent::SoundFromEntity {
                    sound,
                    position_at_emit,
                } => {
                    let range = sound_holder_range(&sound.sound, sound.volume);
                    let recipients = viewers.iter().filter_map(|viewer| {
                        (distance_to_sqr(*position_at_emit, viewer.position) < range * range)
                            .then_some(viewer.connection_id)
                    });
                    push_tracking_packet(
                        out,
                        recipients,
                        ClientboundPlayPacket::SoundEntity(sound.clone()),
                    );
                }
                // `serverPlayer.connection.send` for each passenger the emit
                // already resolved (`LivingEntity.java:1062-1068`); this is not
                // a `ChunkMap.broadcast`, so `seen_by` does not gate it.
                EntityLifecycleEvent::MobEffectChanged {
                    recipient_entity_ids,
                    packet,
                } => {
                    let recipients = viewers.iter().filter_map(|viewer| {
                        recipient_entity_ids
                            .contains(&viewer.entity_id)
                            .then_some(viewer.connection_id)
                    });
                    push_tracking_packet(out, recipients, packet.clone());
                }
                // `ServerLevel.sendParticles` (`ServerLevel.java:1308-1334`)
                // over its per-player range test (`:1356-1370`).
                EntityLifecycleEvent::Particles { particles } => {
                    let recipients = viewers.iter().filter_map(|viewer| {
                        crate::level_event::particle_reaches(
                            particles.position.x,
                            particles.position.y,
                            particles.position.z,
                            viewer.position.x,
                            viewer.position.y,
                            viewer.position.z,
                            particles.override_limiter,
                        )
                        .then_some(viewer.connection_id)
                    });
                    push_tracking_packet(
                        out,
                        recipients,
                        ClientboundPlayPacket::LevelParticles(particles.clone()),
                    );
                }
            }
        }
    }

    /// `ChunkMap.tick()`'s entity half (`ChunkMap.java:1180-1212`).
    ///
    /// `updateChunkTracking` for every session (`:1181-1183`) is not here: the
    /// caller already owns the chunk-tracking view and refreshes it where
    /// vanilla does. Filing and dropping trackers is
    /// [`EntityTracker::apply_lifecycle_events`], which consumes the store's
    /// ordered projection of vanilla's entity callbacks.
    pub fn tick<F, T>(
        &mut self,
        store: &mut EntityStore,
        viewers: &[TrackingViewer],
        tracks_chunk: F,
        entity_ticking: T,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
        T: Fn(i32, i32) -> bool,
    {
        self.tick_with_framed_maps(
            store,
            viewers,
            tracks_chunk,
            entity_ticking,
            |_| Vec::new(),
            out,
        );
    }

    /// `ServerEntity.sendChanges`' framed-map arm before its generic sync
    /// (`ServerEntity.java:101-120`). Returned packets are already addressed
    /// and are never projected through the frame's `seenBy` set.
    pub fn tick_with_framed_maps<F, T, M>(
        &mut self,
        store: &mut EntityStore,
        viewers: &[TrackingViewer],
        tracks_chunk: F,
        entity_ticking: T,
        framed_map_packets: M,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
        T: Fn(i32, i32) -> bool,
        M: FnMut(&FramedMapFrame) -> Vec<(ConnectionId, ClientboundPlayPacket)>,
    {
        let moved_viewers = self.legacy_moved_viewers(viewers);
        self.sweep(
            store,
            viewers,
            &tracks_chunk,
            entity_ticking,
            framed_map_packets,
            out,
        );
        for viewer in &moved_viewers {
            self.update_viewer(viewer, viewers, store, &tracks_chunk, out);
        }
    }

    /// The shared player/stored `ChunkMap.tick` walk (`ChunkMap.java:1180-1212`).
    pub fn tick_with_players_and_framed_maps<F, T, M>(
        &mut self,
        store: &mut EntityStore,
        viewers: &[TrackingViewer],
        tracks_chunk: F,
        entity_ticking: T,
        framed_map_packets: M,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
        T: Fn(i32, i32) -> bool,
        M: FnMut(&FramedMapFrame) -> Vec<(ConnectionId, ClientboundPlayPacket)>,
    {
        index_viewers_by_entity_id(viewers, &mut self.scratch.viewer_by_entity_id);
        if !self.avatars.is_empty() {
            let mut stale_avatars = std::mem::take(&mut self.scratch.stale_avatars);
            stale_avatars.clear();
            for (&connection_id, &entity_id) in &self.avatars {
                if self.entities.get(&entity_id).is_none_or(|tracked| {
                    tracked
                        .avatar_in_indexed(viewers, &self.scratch.viewer_by_entity_id)
                        .is_none()
                }) {
                    stale_avatars.push((connection_id, entity_id));
                }
            }
            for &(connection_id, entity_id) in &stale_avatars {
                self.remove_player(connection_id, entity_id, out);
            }
            stale_avatars.clear();
            self.scratch.stale_avatars = stale_avatars;
        }
        if !self.seen.is_empty() {
            let mut absent = std::mem::take(&mut self.scratch.absent_viewers);
            absent.clear();
            for &connection_id in self.seen.keys() {
                if !viewers
                    .iter()
                    .any(|viewer| viewer.connection_id == connection_id)
                {
                    absent.push(connection_id);
                }
            }
            for &connection_id in &absent {
                self.unpair_session(connection_id, out);
            }
            absent.clear();
            self.scratch.absent_viewers = absent;
        }
        self.sweep(
            store,
            viewers,
            &tracks_chunk,
            entity_ticking,
            framed_map_packets,
            out,
        );
        if !self.scratch.moved_viewers.is_empty() {
            let mut moved = std::mem::take(&mut self.scratch.moved_viewers);
            for mover in moved.drain(..) {
                self.reconcile_moved_player(&mover, viewers, store, &tracks_chunk, out);
            }
            self.scratch.moved_viewers = moved;
        }
    }

    /// The `ChunkMap.tick` loop over every tracker (`ChunkMap.java:1188-1205`),
    /// recording the sessions whose avatars changed section in
    /// `self.scratch.moved_viewers`.
    #[allow(clippy::too_many_arguments)]
    fn sweep<F, T, M>(
        &mut self,
        store: &mut EntityStore,
        viewers: &[TrackingViewer],
        tracks_chunk: &F,
        entity_ticking: T,
        mut framed_map_packets: M,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
        T: Fn(i32, i32) -> bool,
        M: FnMut(&FramedMapFrame) -> Vec<(ConnectionId, ClientboundPlayPacket)>,
    {
        self.refresh_ranges(store);
        let Self {
            entities,
            entity_ids,
            by_column,
            seen,
            minecart_improvements,
            scratch,
            ..
        } = self;
        let minecart_improvements = *minecart_improvements;
        let level = Pairing {
            sessions: viewers,
            entity_ids,
            tracks_chunk,
        };
        index_viewers_by_entity_id(viewers, &mut scratch.viewer_by_entity_id);
        let viewer_index = &scratch.viewer_by_entity_id;
        let moved = &mut scratch.moved_viewers;
        moved.clear();
        for (&id, tracked) in entities.iter_mut() {
            if tracked.player.is_some() {
                let Some(mover) = tracked.avatar_in_indexed(viewers, viewer_index).copied() else {
                    continue;
                };
                let section = section_of_block(crate::entity::containing_block(mover.position));
                if tracked.last_section_pos != section {
                    tracked.update_player_avatars(store, viewers, tracks_chunk, seen, out);
                    moved.push(mover);
                    tracked.last_section_pos = section;
                }
                let riding = store.external_player_vehicle(id).is_some();
                let Some(player) = store.player_mut(id) else {
                    continue;
                };
                let packets = tracked
                    .server_entity
                    .send_player_changes(player.drain_tracking_frame(riding));
                tracked.route_player_changes(mover.connection_id, packets, out);
                continue;
            }
            let is_riding = store.valid_vehicle(id).is_some();
            let Some(entity) = store.get_mut(id) else {
                continue;
            };
            // `:1189-1200`.
            let section = section_of_block(entity.block_position());
            let section_changed = tracked.last_section_pos != section;
            if section_changed {
                tracked.update_players(entity, viewers, &level, seen, out);
                if column_of(tracked.last_section_pos) != column_of(section) {
                    unfile_from_column(by_column, tracked.last_section_pos, id);
                    file_in_column(by_column, section, id);
                }
                tracked.last_section_pos = section;
            }
            // `:1202-1204`.
            let block = entity.block_position();
            if section_changed || entity.needs_sync() || entity_ticking(block.x >> 4, block.z >> 4)
            {
                if let Some(frame) = tracked.server_entity.framed_map_due(entity) {
                    out.extend(framed_map_packets(&frame).into_iter().map(
                        |(connection_id, packet)| TrackingPacket {
                            recipients: vec![connection_id],
                            packet,
                        },
                    ));
                }
                let emit_packets = !tracked.seen_by.is_empty();
                for packet in tracked.server_entity.send_changes_with_riding(
                    entity,
                    is_riding,
                    minecart_improvements,
                    emit_packets,
                ) {
                    push_tracking_packet(out, tracked.seen_by.iter().copied(), packet);
                }
            }
        }
    }

    /// The `ChunkMap.addEntity` half used by the callback path.
    fn add_entity<F>(
        &mut self,
        entity: &Entity,
        catalog: EntityTrackingSpec,
        viewers: &[TrackingViewer],
        tracks_chunk: &F,
        out: &mut Vec<TrackingPacket>,
    ) where
        F: Fn(ConnectionId, i32, i32) -> bool,
    {
        let id = entity.id();
        if self.entities.contains_key(&id) {
            return;
        }
        // `ChunkMap.java:1142-1150`; `track_deltas` is generated from
        // `EntityType.trackDeltas()` (`EntityType.java:1556-1566`).
        let range = catalog.client_tracking_range * CHUNK_BLOCKS;
        // `ChunkMap.java:1143`: a type with no tracking range is never tracked
        // at all.
        if range == 0 {
            return;
        }
        let Self {
            entities,
            entity_ids,
            item_frame_ids,
            by_column,
            seen,
            item_stack_registries,
            range_percentage,
            ..
        } = self;
        // `ChunkMap.java:1149-1150`.
        let tracked = entities.entry(id).or_insert_with(|| {
            let mut tracked = TrackedEntity::new(
                entity,
                range,
                catalog.update_interval,
                catalog.track_deltas,
                Arc::clone(item_stack_registries),
            );
            tracked.effective_range = range_percentage.scale(range);
            tracked
        });
        if matches!(
            entity.data(),
            EntityData::ItemFrame(_) | EntityData::GlowItemFrame(_)
        ) {
            item_frame_ids.insert(id);
        }
        file_in_column(by_column, tracked.last_section_pos, id);
        // `ChunkMap.java:1151`.
        let level = Pairing {
            sessions: viewers,
            entity_ids,
            tracks_chunk,
        };
        tracked.update_players(entity, viewers, &level, seen, out);
    }

    fn legacy_moved_viewers(&mut self, viewers: &[TrackingViewer]) -> Vec<TrackingViewer> {
        let present = viewers
            .iter()
            .map(|viewer| viewer.connection_id)
            .collect::<BTreeSet<_>>();
        self.legacy_viewer_sections
            .retain(|connection_id, _| present.contains(connection_id));
        viewers
            .iter()
            .filter_map(|viewer| {
                let section = section_of_block(crate::entity::containing_block(viewer.position));
                (self
                    .legacy_viewer_sections
                    .insert(viewer.connection_id, section)
                    != Some(section))
                .then_some(*viewer)
            })
            .collect()
    }
}

fn effective_tracking_range(
    store: &EntityStore,
    entity_id: i32,
    own_range: i32,
    visited: &mut BTreeSet<i32>,
) -> i32 {
    let passengers = store.passengers(entity_id);
    if passengers.is_empty() || !visited.insert(entity_id) {
        return own_range;
    }
    passengers.iter().fold(own_range, |range, passenger_id| {
        if store.is_external_player_passenger(*passenger_id) {
            if store.external_player_structural_vehicle(*passenger_id) != Some(entity_id) {
                return range;
            }
            let player_range = vanilla_entity_type(VANILLA_ENTITY_TYPE_PLAYER_ID)
                .map(|catalog| catalog.client_tracking_range * CHUNK_BLOCKS)
                .unwrap_or(0);
            return range.max(player_range);
        }
        if store.valid_vehicle(*passenger_id) != Some(entity_id) {
            return range;
        }
        let passenger_range = store
            .get(*passenger_id)
            .and_then(entity_tracking_spec)
            .map(|spec| spec.client_tracking_range * CHUNK_BLOCKS)
            .unwrap_or(0);
        range.max(effective_tracking_range(
            store,
            *passenger_id,
            passenger_range,
            visited,
        ))
    })
}
