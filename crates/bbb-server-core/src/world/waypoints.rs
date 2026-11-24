//! Per-level waypoint connection state and its packet decisions.
use crate::world::LevelId;
use std::collections::{BTreeMap, BTreeSet};

use crate::connection::ClientboundPlayPacket;
use crate::entity::{
    AttributeModifier, AttributeOperation, Entity, EntityData, EntityLifecycleEvent,
};
use bbb_protocol::packets::{
    BlockPos, ChunkPos, TrackedWaypoint, TrackedWaypointPacket, Vec3d, WaypointData, WaypointIcon,
    WaypointIdentifier, WaypointOperation, WaypointVec3i,
};
use bbb_protocol::{
    attribute_types::{
        VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
        VANILLA_ATTRIBUTE_WAYPOINT_RECEIVE_RANGE_ID, VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID,
    },
    component_translatable_nbt,
    game_rules::GameRuleValue,
    game_types::GameType,
    ComponentStyle, NbtSummaryValue,
};
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;
use uuid::Uuid;

use crate::commands::{CommandAttributeTarget, CommandWaypointState};

use super::display_names::{
    entity_display_name, player_display_name, remove_sibling_click_actions,
};
use super::player_packets::block_position;
use super::scoreboard::PlayerTeam;
use super::{send, ChunkTrackingView, ConnectionId, PlayerState, ServerWorld, WorldEvent};

const REALLY_FAR_DISTANCE: f32 = 332.0;
const AZIMUTH_UPDATE_RADIANS: f32 = 0.008_726_646;
const DEFAULT_TEAM_COLOR_RGB: u32 = 0x30_30_30;
const RULE_LOCATOR_BAR: &str = "minecraft:locator_bar";
const WAYPOINT_TRANSMIT_RANGE_CROUCH_MODIFIER: &str = "minecraft:waypoint_transmit_range_crouch";

pub(super) fn default_waypoint_icon() -> WaypointIcon {
    WaypointIcon {
        style: "minecraft:default".to_owned(),
        color_rgb: None,
    }
}

/// `Entity.getDisplayName` (`Entity.java:3207-3209,3358-3360`) through
/// `PlayerTeam.formatNameForTeam` (`PlayerTeam.java:116-127`).
pub(super) fn command_stored_display_name(
    entity: &Entity,
    team: Option<&PlayerTeam>,
) -> NbtSummaryValue {
    let name = entity.custom_name().map_or_else(
        || {
            let id = entity.data().entity_resource_id();
            let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
            let mut key = format!("entity.{namespace}.{}", path.replace('/', "."));
            // `Villager.getTypeName` (Villager.java:702-704) names the profession
            // (VillagerProfession.java:112).
            if let (EntityData::GeneralLiving(data), "minecraft:villager") = (entity.data(), id) {
                if let Some((profession, _)) = data.villager_profession_and_level() {
                    let (namespace, path) = profession
                        .split_once(':')
                        .unwrap_or(("minecraft", profession));
                    key = format!("entity.{namespace}.villager.{path}");
                }
            }
            component_translatable_nbt(&key, Vec::new(), &ComponentStyle::default())
                .expect("an entity description is a valid component")
        },
        |name| remove_sibling_click_actions(name.component()),
    );
    entity_display_name(
        entity.data().entity_resource_id(),
        entity.uuid(),
        name,
        team,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum WaypointSourceKey {
    Player(ConnectionId),
    StoredEntity { dimension: LevelId, entity_id: i32 },
}

#[derive(Debug, Clone)]
pub(super) struct WaypointSource {
    pub key: WaypointSourceKey,
    pub identifier: Uuid,
    pub position: Vec3d,
    pub icon: WaypointIcon,
    pub team_color_rgb: Option<u32>,
    pub transmit_range: f64,
    pub spectator: bool,
    pub first_tick: bool,
    pub indirect_passengers: BTreeSet<i32>,
}

impl WaypointSource {
    fn block_position(&self) -> BlockPos {
        BlockPos {
            x: self.position.x.floor() as i32,
            y: self.position.y.floor() as i32,
            z: self.position.z.floor() as i32,
        }
    }

    fn chunk_position(&self) -> ChunkPos {
        let block = self.block_position();
        ChunkPos {
            x: block.x >> 4,
            z: block.z >> 4,
        }
    }

    fn effective_icon(&self) -> WaypointIcon {
        let mut icon = self.icon.clone();
        icon.color_rgb = icon.color_rgb.map(|color| color & 0x00ff_ffff);
        if icon.color_rgb.is_none() {
            icon.color_rgb = self.team_color_rgb.map(|color| {
                if color == 0 {
                    DEFAULT_TEAM_COLOR_RGB
                } else {
                    color & 0x00ff_ffff
                }
            });
        }
        if icon.style.len() > 256 || bbb_protocol::normalize_resource_location(&icon.style).is_err()
        {
            "minecraft:default".clone_into(&mut icon.style);
        }
        icon
    }
}

#[derive(Debug, Clone)]
pub(super) struct WaypointReceiver {
    pub connection_id: ConnectionId,
    pub entity_id: i32,
    pub position: Vec3d,
    pub receive_range: f64,
    pub spectator: bool,
    pub locator_bar_enabled: bool,
    pub chunk_tracking_view: Option<ChunkTrackingView>,
}

#[derive(Debug, Clone, PartialEq)]
enum WaypointConnection {
    Block {
        identifier: Uuid,
        icon: WaypointIcon,
        last_position: BlockPos,
    },
    Chunk {
        identifier: Uuid,
        icon: WaypointIcon,
        last_position: ChunkPos,
    },
    Azimuth {
        identifier: Uuid,
        icon: WaypointIcon,
        last_angle: f32,
    },
}

impl WaypointConnection {
    fn identifier(&self) -> Uuid {
        match self {
            Self::Block { identifier, .. }
            | Self::Chunk { identifier, .. }
            | Self::Azimuth { identifier, .. } => *identifier,
        }
    }

    fn tracked_waypoint(&self) -> TrackedWaypoint {
        match self {
            Self::Block {
                identifier,
                icon,
                last_position,
            } => tracked_waypoint(
                *identifier,
                icon.clone(),
                WaypointData::Position(WaypointVec3i {
                    x: last_position.x,
                    y: last_position.y,
                    z: last_position.z,
                }),
            ),
            Self::Chunk {
                identifier,
                icon,
                last_position,
            } => tracked_waypoint(
                *identifier,
                icon.clone(),
                WaypointData::Chunk(*last_position),
            ),
            Self::Azimuth {
                identifier,
                icon,
                last_angle,
            } => tracked_waypoint(
                *identifier,
                icon.clone(),
                WaypointData::Azimuth(*last_angle),
            ),
        }
    }

    fn update(&mut self, source: &WaypointSource, receiver: &WaypointReceiver) -> bool {
        match self {
            Self::Block { last_position, .. } => {
                let current = source.block_position();
                if *last_position == current {
                    false
                } else {
                    *last_position = current;
                    true
                }
            }
            Self::Chunk { last_position, .. } => {
                let current = source.chunk_position();
                if *last_position == current {
                    false
                } else {
                    *last_position = current;
                    true
                }
            }
            Self::Azimuth { last_angle, .. } => {
                let current = azimuth(source, receiver);
                if (current - *last_angle).abs() > AZIMUTH_UPDATE_RADIANS {
                    *last_angle = current;
                    true
                } else {
                    false
                }
            }
        }
    }

    fn is_broken(&self, source: &WaypointSource, receiver: &WaypointReceiver) -> bool {
        match self {
            Self::Block { last_position, .. } => {
                manhattan(*last_position, source.block_position()) > 1
                    || source_ignores_receiver(source, receiver)
            }
            Self::Chunk { last_position, .. } => {
                chessboard(*last_position, source.chunk_position()) > 1
                    || source_ignores_receiver(source, receiver)
                    || is_chunk_visible(*last_position, receiver)
            }
            Self::Azimuth { .. } => {
                source_ignores_receiver(source, receiver)
                    || is_chunk_visible(source.chunk_position(), receiver)
                    || !is_really_far(source, receiver)
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
struct WaypointConnections {
    connections: BTreeMap<(ConnectionId, WaypointSourceKey), WaypointConnection>,
    by_player: rustc_hash::FxHashMap<ConnectionId, BTreeSet<WaypointSourceKey>>,
    by_source: rustc_hash::FxHashMap<WaypointSourceKey, BTreeSet<ConnectionId>>,
}

impl WaypointConnections {
    fn has_connections_for(&self, connection_id: ConnectionId) -> bool {
        self.by_player
            .get(&connection_id)
            .is_some_and(|keys| !keys.is_empty())
    }

    fn has_connection(&self, connection_id: ConnectionId, key: &WaypointSourceKey) -> bool {
        self.by_player
            .get(&connection_id)
            .is_some_and(|keys| keys.contains(key))
    }

    fn insert_connection(
        &mut self,
        connection_id: ConnectionId,
        key: WaypointSourceKey,
        connection: WaypointConnection,
    ) {
        self.by_player
            .entry(connection_id)
            .or_default()
            .insert(key.clone());
        self.by_source
            .entry(key.clone())
            .or_default()
            .insert(connection_id);
        self.connections.insert((connection_id, key), connection);
    }

    fn remove_connection(
        &mut self,
        connection_id: ConnectionId,
        key: &WaypointSourceKey,
    ) -> Option<WaypointConnection> {
        let removed = self.connections.remove(&(connection_id, key.clone()))?;
        if let Some(keys) = self.by_player.get_mut(&connection_id) {
            keys.remove(key);
            if keys.is_empty() {
                self.by_player.remove(&connection_id);
            }
        }
        if let Some(players) = self.by_source.get_mut(key) {
            players.remove(&connection_id);
            if players.is_empty() {
                self.by_source.remove(key);
            }
        }
        Some(removed)
    }

    fn create_connection(
        &mut self,
        receiver: &WaypointReceiver,
        source: &WaypointSource,
        packets: &mut Vec<(ConnectionId, TrackedWaypointPacket)>,
    ) {
        if source.key == WaypointSourceKey::Player(receiver.connection_id)
            || !receiver.locator_bar_enabled
        {
            return;
        }
        match make_connection(source, receiver) {
            Some(connection) => {
                packets.push((
                    receiver.connection_id,
                    packet(WaypointOperation::Track, connection.tracked_waypoint()),
                ));
                self.insert_connection(receiver.connection_id, source.key.clone(), connection);
            }
            None => {
                if let Some(connection) =
                    self.remove_connection(receiver.connection_id, &source.key)
                {
                    packets.push((receiver.connection_id, untrack(connection.identifier())));
                }
            }
        }
    }

    fn update_connection(
        &mut self,
        receiver: &WaypointReceiver,
        source: &WaypointSource,
        packets: &mut Vec<(ConnectionId, TrackedWaypointPacket)>,
    ) {
        if source.key == WaypointSourceKey::Player(receiver.connection_id)
            || !receiver.locator_bar_enabled
        {
            return;
        }
        let key = (receiver.connection_id, source.key.clone());
        let broken = self
            .connections
            .get(&key)
            .is_some_and(|connection| connection.is_broken(source, receiver));
        if broken {
            match make_connection(source, receiver) {
                Some(connection) => {
                    packets.push((
                        receiver.connection_id,
                        packet(WaypointOperation::Track, connection.tracked_waypoint()),
                    ));
                    self.insert_connection(receiver.connection_id, source.key.clone(), connection);
                }
                None => {
                    if let Some(connection) =
                        self.remove_connection(receiver.connection_id, &source.key)
                    {
                        packets.push((receiver.connection_id, untrack(connection.identifier())));
                    }
                }
            }
            return;
        }
        let Some(connection) = self.connections.get_mut(&key) else {
            return;
        };
        if connection.update(source, receiver) {
            packets.push((
                receiver.connection_id,
                packet(WaypointOperation::Update, connection.tracked_waypoint()),
            ));
        }
    }
}

/// `ServerWaypointManager.java:18-138`: the transmitter set, receiver set and
/// connection table are level-local and ephemeral.
#[derive(Clone, Debug, Default)]
pub(super) struct ServerWaypointManager {
    transmitters: BTreeSet<WaypointSourceKey>,
    players: BTreeSet<ConnectionId>,
    table: WaypointConnections,
}

impl ServerWaypointManager {
    pub(super) fn transmitters(&self) -> impl Iterator<Item = WaypointSourceKey> + '_ {
        self.transmitters.iter().cloned()
    }

    fn has_other_players(&self, connection_id: ConnectionId) -> bool {
        self.players.len() > usize::from(self.players.contains(&connection_id))
    }

    fn has_other_transmitters(&self, key: &WaypointSourceKey) -> bool {
        self.transmitters.len() > usize::from(self.transmitters.contains(key))
    }

    fn has_connections_for(&self, connection_id: ConnectionId) -> bool {
        self.table.has_connections_for(connection_id)
    }

    fn is_tracking(&self, key: &WaypointSourceKey) -> bool {
        self.transmitters.contains(key)
    }

    pub(super) fn track_waypoint(
        &mut self,
        source: &WaypointSource,
        receiver_for: impl Fn(ConnectionId) -> Option<WaypointReceiver>,
    ) -> Vec<(ConnectionId, TrackedWaypointPacket)> {
        self.transmitters.insert(source.key.clone());
        let mut packets = Vec::new();
        for &connection_id in &self.players {
            if source.key == WaypointSourceKey::Player(connection_id) {
                continue;
            }
            let Some(receiver) = receiver_for(connection_id) else {
                continue;
            };
            self.table
                .create_connection(&receiver, source, &mut packets);
        }
        packets
    }

    pub(super) fn update_waypoint(
        &mut self,
        source: &WaypointSource,
        receiver_for: impl Fn(ConnectionId) -> Option<WaypointReceiver>,
    ) -> Vec<(ConnectionId, TrackedWaypointPacket)> {
        if !self.transmitters.contains(&source.key) {
            return Vec::new();
        }
        let mut packets = Vec::new();
        for &connection_id in &self.players {
            if source.key == WaypointSourceKey::Player(connection_id) {
                continue;
            }
            let Some(receiver) = receiver_for(connection_id) else {
                continue;
            };
            if self.table.has_connection(connection_id, &source.key) {
                self.table
                    .update_connection(&receiver, source, &mut packets);
            } else {
                self.table
                    .create_connection(&receiver, source, &mut packets);
            }
        }
        packets
    }

    pub(super) fn untrack_waypoint(
        &mut self,
        key: WaypointSourceKey,
        identifier: Uuid,
    ) -> Vec<(ConnectionId, TrackedWaypointPacket)> {
        let mut packets = Vec::new();
        if let Some(players) = self.table.by_source.remove(&key) {
            for connection_id in players {
                self.table.connections.remove(&(connection_id, key.clone()));
                if let Some(keys) = self.table.by_player.get_mut(&connection_id) {
                    keys.remove(&key);
                    if keys.is_empty() {
                        self.table.by_player.remove(&connection_id);
                    }
                }
                packets.push((connection_id, untrack(identifier)));
            }
        }
        self.transmitters.remove(&key);
        packets
    }

    pub(super) fn add_player(
        &mut self,
        receiver: &WaypointReceiver,
        source_for: impl Fn(&WaypointSourceKey) -> Option<WaypointSource>,
        player_source: Option<&WaypointSource>,
        receiver_for: impl Fn(ConnectionId) -> Option<WaypointReceiver>,
    ) -> Vec<(ConnectionId, TrackedWaypointPacket)> {
        self.players.insert(receiver.connection_id);
        let mut packets = Vec::new();
        let self_key = WaypointSourceKey::Player(receiver.connection_id);
        for key in &self.transmitters {
            if *key == self_key {
                continue;
            }
            let Some(source) = source_for(key) else {
                continue;
            };
            self.table
                .create_connection(receiver, &source, &mut packets);
        }
        if let Some(source) = player_source.filter(|source| source.transmit_range > 0.0) {
            packets.extend(self.track_waypoint(source, receiver_for));
        }
        packets
    }

    pub(super) fn update_player(
        &mut self,
        receiver: &WaypointReceiver,
        source_for: impl Fn(&WaypointSourceKey) -> Option<WaypointSource>,
    ) -> Vec<(ConnectionId, TrackedWaypointPacket)> {
        let mut packets = Vec::new();
        let self_key = WaypointSourceKey::Player(receiver.connection_id);
        for key in &self.transmitters {
            if *key == self_key {
                continue;
            }
            let Some(source) = source_for(key) else {
                continue;
            };
            if self.table.has_connection(receiver.connection_id, key) {
                self.table
                    .update_connection(receiver, &source, &mut packets);
            } else {
                self.table
                    .create_connection(receiver, &source, &mut packets);
            }
        }
        packets
    }

    pub(super) fn remove_player(
        &mut self,
        connection_id: ConnectionId,
        identifier: Uuid,
    ) -> Vec<(ConnectionId, TrackedWaypointPacket)> {
        let mut packets = Vec::new();
        if let Some(inbound) = self.table.by_player.remove(&connection_id) {
            for key in inbound {
                if let Some(players) = self.table.by_source.get_mut(&key) {
                    players.remove(&connection_id);
                    if players.is_empty() {
                        self.table.by_source.remove(&key);
                    }
                }
                if let Some(connection) = self.table.connections.remove(&(connection_id, key)) {
                    packets.push((connection_id, untrack(connection.identifier())));
                }
            }
        }
        packets.extend(self.untrack_waypoint(WaypointSourceKey::Player(connection_id), identifier));
        self.players.remove(&connection_id);
        packets
    }

    pub(super) fn break_all_connections(&mut self) -> Vec<(ConnectionId, TrackedWaypointPacket)> {
        self.table.by_player.clear();
        self.table.by_source.clear();
        let connections = std::mem::take(&mut self.table.connections);
        connections
            .into_iter()
            .map(|((connection_id, _), connection)| {
                (connection_id, untrack(connection.identifier()))
            })
            .collect()
    }

    pub(super) fn remake_connections(
        &mut self,
        source: &WaypointSource,
        receiver_for: impl Fn(ConnectionId) -> Option<WaypointReceiver>,
    ) -> Vec<(ConnectionId, TrackedWaypointPacket)> {
        let mut packets = Vec::new();
        for &connection_id in &self.players {
            if source.key == WaypointSourceKey::Player(connection_id) {
                continue;
            }
            let Some(receiver) = receiver_for(connection_id) else {
                continue;
            };
            self.table
                .create_connection(&receiver, source, &mut packets);
        }
        packets
    }

    #[cfg(test)]
    pub(super) fn connection_count(&self) -> usize {
        self.table.connections.len()
    }
}

impl PlayerState {
    fn waypoint_transmit_range(&self) -> f64 {
        self.attributes
            .value(VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID)
            .expect("the player supplier declares waypoint transmit range")
    }

    fn waypoint_receive_range(&self) -> f64 {
        self.attributes
            .value(VANILLA_ATTRIBUTE_WAYPOINT_RECEIVE_RANGE_ID)
            .expect("the player supplier declares waypoint receive range")
    }
}

impl ServerWorld {
    /// The waypoint `target` transmits, if it is a living transmitter.
    pub(crate) fn command_waypoint(
        &self,
        target: &CommandAttributeTarget,
    ) -> Option<CommandWaypointState> {
        let (key, _) = self.waypoint_key_and_identifier(target)?;
        self.command_waypoint_state(key)
    }

    /// The transmitters the waypoint manager of `level` tracks.
    pub(crate) fn command_level_waypoints(
        &self,
        level: &bbb_server_types::save::Java26LevelKey,
    ) -> Vec<CommandWaypointState> {
        self.level_by_id(&self.require_level_id(level))
            .waypoint_manager
            .transmitters()
            .filter_map(|key| self.command_waypoint_state(key))
            .collect()
    }

    fn command_waypoint_state(&self, key: WaypointSourceKey) -> Option<CommandWaypointState> {
        match key {
            WaypointSourceKey::Player(connection_id) => {
                let player = self.levels.player(connection_id)?;
                let team = self.scoreboard.board().players_team(&player.profile.name);
                Some(CommandWaypointState {
                    target: CommandAttributeTarget::Player(crate::commands::CommandPlayerTarget {
                        profile: crate::admin::NameAndId {
                            id: player.profile.uuid,
                            name: (&player.profile.name).into(),
                        },
                    }),
                    display_name: player_display_name(&player.profile, team),
                    position: block_position(player.position()),
                    icon: player.cold.locator_bar_icon.clone(),
                })
            }
            WaypointSourceKey::StoredEntity {
                dimension,
                entity_id,
            } => {
                let entity = self.level_by_id(&dimension).entities.get(entity_id)?;
                let living = entity.living()?;
                let team = self
                    .scoreboard
                    .board()
                    .players_team(&entity.uuid().to_string());
                Some(CommandWaypointState {
                    target: CommandAttributeTarget::Stored {
                        dimension: self.level_key(dimension).clone(),
                        entity_id,
                    },
                    display_name: command_stored_display_name(entity, team),
                    position: block_position(entity.position()),
                    icon: living.waypoint_icon().clone(),
                })
            }
        }
    }

    pub(crate) fn apply_set_waypoint_icon_by_key(
        &mut self,
        manager_level: &LevelId,
        target: CommandAttributeTarget,
        icon: WaypointIcon,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some((key, identifier)) = self.waypoint_key_and_identifier(&target) else {
            return;
        };
        let packets = self
            .level_mut_by_id(manager_level)
            .waypoint_manager
            .untrack_waypoint(key, identifier);
        self.emit_waypoint_packets(packets, events);

        match &target {
            CommandAttributeTarget::Player(user) => {
                let Some(connection_id) = self.command_player_connection(user) else {
                    return;
                };
                let Some(player) = self.levels.player_mut(connection_id) else {
                    return;
                };
                player.cold.locator_bar_icon = icon;
            }
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => {
                let Some(living) = self
                    .level_mut_by_id(&self.require_level_id(dimension))
                    .entities
                    .get_mut(*entity_id)
                    .and_then(Entity::living_mut)
                else {
                    return;
                };
                living.set_waypoint_icon(icon);
            }
        }

        let Some(source) = (match target {
            CommandAttributeTarget::Player(user) => self
                .command_player_connection(&user)
                .and_then(|connection_id| self.player_waypoint_source(connection_id)),
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => self
                .level_by_id(&self.require_level_id(&dimension))
                .entities
                .get(entity_id)
                .and_then(|entity| {
                    self.stored_waypoint_source(&self.require_level_id(&dimension), entity)
                }),
        }) else {
            return;
        };
        let packets = self.with_waypoint_manager(manager_level, |world, manager| {
            manager.track_waypoint(&source, |id| world.waypoint_receiver(id))
        });
        self.emit_waypoint_packets(packets, events);
    }

    pub(crate) fn apply_untrack_waypoint_by_key(
        &mut self,
        manager_level: &LevelId,
        target: CommandAttributeTarget,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some((key, identifier)) = self.waypoint_key_and_identifier(&target) else {
            return;
        };
        let packets = self
            .level_mut_by_id(manager_level)
            .waypoint_manager
            .untrack_waypoint(key, identifier);
        self.emit_waypoint_packets(packets, events);
    }

    fn waypoint_key_and_identifier(
        &self,
        target: &CommandAttributeTarget,
    ) -> Option<(WaypointSourceKey, Uuid)> {
        match target {
            CommandAttributeTarget::Player(user) => {
                let connection_id = self.command_player_connection(user)?;
                let player = self.levels.player(connection_id)?;
                Some((
                    WaypointSourceKey::Player(connection_id),
                    player.profile.uuid,
                ))
            }
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => {
                let entity = self
                    .level_by_id(&self.require_level_id(dimension))
                    .entities
                    .get(*entity_id)?;
                entity.living()?;
                Some((
                    WaypointSourceKey::StoredEntity {
                        dimension: self.require_level_id(dimension),
                        entity_id: *entity_id,
                    },
                    entity.uuid(),
                ))
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn set_player_waypoint_icon_for_test(
        &mut self,
        connection_id: ConnectionId,
        icon: WaypointIcon,
    ) {
        self.levels[connection_id].cold.locator_bar_icon = icon;
    }

    #[cfg(test)]
    pub(crate) fn player_waypoint_icon_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> &WaypointIcon {
        &self.levels[connection_id].cold.locator_bar_icon
    }

    #[cfg(test)]
    pub(crate) fn set_player_waypoint_transmit_range_for_test(
        &mut self,
        connection_id: ConnectionId,
        range: f64,
    ) {
        self.levels[connection_id]
            .attributes
            .get_instance(VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID)
            .expect("the player supplier declares waypoint transmit range")
            .set_base_value(range);
    }

    #[cfg(test)]
    pub(crate) fn stored_waypoint_icon_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) -> &WaypointIcon {
        self.level(dimension)
            .entities
            .get(entity_id)
            .and_then(Entity::living)
            .expect("the test entity is living")
            .waypoint_icon()
    }

    #[cfg(test)]
    pub(crate) fn waypoint_manager_tracks_target_for_test(
        &self,
        manager_level: Java26BuiltinDimension,
        target: &CommandAttributeTarget,
    ) -> bool {
        let key = match target {
            CommandAttributeTarget::Player(user) => self
                .levels
                .player_entries()
                .find(|(_, player)| player.profile.uuid == user.id)
                .map(|(&connection_id, _)| WaypointSourceKey::Player(connection_id)),
            CommandAttributeTarget::Stored {
                dimension,
                entity_id,
            } => Some(WaypointSourceKey::StoredEntity {
                dimension: self.require_level_id(dimension),
                entity_id: *entity_id,
            }),
        };
        key.is_some_and(|key| self.level(manager_level).waypoint_manager.is_tracking(&key))
    }

    fn locator_bar_enabled(&self) -> bool {
        matches!(
            self.admin.game_rule_value(RULE_LOCATOR_BAR),
            Some(GameRuleValue::Boolean(true))
        )
    }

    fn waypoint_team_color(&self, scoreboard_name: &str) -> Option<u32> {
        self.scoreboard
            .board()
            .players_team(scoreboard_name)
            .and_then(|team| team.color.color())
    }

    fn player_waypoint_source(&self, target: ConnectionId) -> Option<WaypointSource> {
        let player = self.levels.player(target)?;
        Some(WaypointSource {
            key: WaypointSourceKey::Player(target),
            identifier: player.profile.uuid,
            position: player.position(),
            icon: player.cold.locator_bar_icon.clone(),
            team_color_rgb: self.waypoint_team_color(&player.profile.name),
            transmit_range: player.waypoint_transmit_range(),
            spectator: player.game_type == GameType::Spectator,
            first_tick: player.tick_count == 0,
            indirect_passengers: BTreeSet::new(),
        })
    }

    fn with_waypoint_manager<R>(
        &mut self,
        dimension: &LevelId,
        f: impl FnOnce(&ServerWorld, &mut ServerWaypointManager) -> R,
    ) -> R {
        let mut manager = std::mem::take(&mut self.level_mut_by_id(dimension).waypoint_manager);
        let result = f(self, &mut manager);
        self.level_mut_by_id(dimension).waypoint_manager = manager;
        result
    }

    fn stored_waypoint_source(
        &self,
        dimension: &LevelId,
        entity: &Entity,
    ) -> Option<WaypointSource> {
        let living = entity.living()?;
        let store = &self.level_by_id(dimension).entities;
        let indirect_passengers = self
            .levels
            .players()
            .filter(|player| &player.dimension == dimension && !player.avatar_removed)
            .filter(|player| store.has_indirect_passenger(entity.id(), player.entity_id()))
            .map(|player| player.entity_id())
            .collect();
        let mut uuid_buf = Uuid::encode_buffer();
        let team_color_rgb =
            self.waypoint_team_color(entity.uuid().hyphenated().encode_lower(&mut uuid_buf));
        Some(WaypointSource {
            key: WaypointSourceKey::StoredEntity {
                dimension: *dimension,
                entity_id: entity.id(),
            },
            identifier: entity.uuid(),
            position: entity.position(),
            icon: living.waypoint_icon().clone(),
            team_color_rgb,
            transmit_range: living.waypoint_transmit_range(),
            spectator: false,
            first_tick: entity.is_first_tick(),
            indirect_passengers,
        })
    }

    fn waypoint_source(&self, key: &WaypointSourceKey) -> Option<WaypointSource> {
        match key {
            WaypointSourceKey::Player(connection_id) => self.player_waypoint_source(*connection_id),
            WaypointSourceKey::StoredEntity {
                dimension,
                entity_id,
            } => self
                .level_by_id(dimension)
                .entities
                .get(*entity_id)
                .and_then(|entity| self.stored_waypoint_source(dimension, entity)),
        }
    }

    fn waypoint_receiver(&self, target: ConnectionId) -> Option<WaypointReceiver> {
        let player = self.levels.player(target)?;
        let connection_id = player.connection_id;
        Some(WaypointReceiver {
            connection_id,
            entity_id: player.entity_id(),
            position: player.position(),
            receive_range: player.waypoint_receive_range(),
            spectator: player.game_type == GameType::Spectator,
            locator_bar_enabled: self.locator_bar_enabled(),
            chunk_tracking_view: self.sessions.get(&target)?.chunk_tracking_view,
        })
    }

    fn emit_waypoint_packets(
        &mut self,
        packets: Vec<(ConnectionId, TrackedWaypointPacket)>,
        events: &mut Vec<WorldEvent>,
    ) {
        events.extend(packets.into_iter().map(|(connection_id, packet)| {
            send(
                connection_id,
                ClientboundPlayPacket::TrackedWaypoint(packet),
            )
        }));
    }

    pub(super) fn waypoint_add_player(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(receiver) = self.waypoint_receiver(connection_id) else {
            return;
        };
        if receiver.receive_range <= 0.0 {
            return;
        }
        let dimension = self.levels[connection_id].dimension;
        let player_source = self.player_waypoint_source(connection_id);
        let packets = self.with_waypoint_manager(&dimension, |world, manager| {
            manager.add_player(
                &receiver,
                |key| world.waypoint_source(key),
                player_source.as_ref(),
                |id| world.waypoint_receiver(id),
            )
        });
        self.emit_waypoint_packets(packets, events);
    }

    pub(super) fn waypoint_remove_player(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let dimension = player.dimension;
        let identifier = player.profile.uuid;
        let packets = self
            .level_mut_by_id(&dimension)
            .waypoint_manager
            .remove_player(connection_id, identifier);
        self.emit_waypoint_packets(packets, events);
    }

    pub(super) fn waypoint_update_player_source(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player.tick_count == 0 || player.waypoint_transmit_range() <= 0.0 {
            return;
        }
        let dimension = player.dimension;
        let manager = &self.level_by_id(&dimension).waypoint_manager;
        if !manager.is_tracking(&WaypointSourceKey::Player(connection_id))
            || !manager.has_other_players(connection_id)
        {
            return;
        }
        let Some(source) = self.player_waypoint_source(connection_id) else {
            return;
        };
        let packets = self.with_waypoint_manager(&dimension, |world, manager| {
            manager.update_waypoint(&source, |id| world.waypoint_receiver(id))
        });
        self.emit_waypoint_packets(packets, events);
    }

    pub(super) fn waypoint_update_player_receiver(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        if player.waypoint_receive_range() <= 0.0 {
            return;
        }
        let dimension = player.dimension;
        let manager = &self.level_by_id(&dimension).waypoint_manager;
        if !manager.has_other_transmitters(&WaypointSourceKey::Player(connection_id))
            && !manager.has_connections_for(connection_id)
        {
            return;
        }
        let Some(receiver) = self.waypoint_receiver(connection_id) else {
            return;
        };
        let packets = self.with_waypoint_manager(&dimension, |world, manager| {
            manager.update_player(&receiver, |key| world.waypoint_source(key))
        });
        self.emit_waypoint_packets(packets, events);
    }

    pub(super) fn apply_waypoint_entity_lifecycle(
        &mut self,
        dimension: &LevelId,
        lifecycle_events: &[EntityLifecycleEvent],
        events: &mut Vec<WorldEvent>,
    ) {
        let mut receiver_updates = BTreeSet::new();
        for lifecycle in lifecycle_events {
            match lifecycle {
                EntityLifecycleEvent::PassengersChanged { passenger_ids, .. } => {
                    for &entity_id in passenger_ids {
                        if let Some((&connection_id, _)) = self
                            .levels
                            .player_entries()
                            .find(|(_, player)| player.entity_id() == entity_id)
                        {
                            receiver_updates.insert(connection_id);
                        }
                    }
                }
                EntityLifecycleEvent::ExternalPlayerBoardingCooldownReset {
                    player_entity_id,
                    ..
                } => {
                    if let Some((&connection_id, _)) = self
                        .levels
                        .player_entries()
                        .find(|(_, player)| player.entity_id() == *player_entity_id)
                    {
                        receiver_updates.insert(connection_id);
                    }
                }
                _ => {}
            }
            let packets = match lifecycle {
                EntityLifecycleEvent::Added { entity, .. }
                | EntityLifecycleEvent::TrackingStarted { entity } => {
                    let Some(source) = self.stored_waypoint_source(dimension, entity) else {
                        continue;
                    };
                    if source.transmit_range <= 0.0 {
                        continue;
                    }
                    self.with_waypoint_manager(dimension, |world, manager| {
                        manager.track_waypoint(&source, |id| world.waypoint_receiver(id))
                    })
                }
                EntityLifecycleEvent::WaypointUpdated { entity_id }
                | EntityLifecycleEvent::PassengersChanged {
                    vehicle_id: entity_id,
                    ..
                } => {
                    let source = self
                        .level_by_id(dimension)
                        .entities
                        .get(*entity_id)
                        .and_then(|entity| self.stored_waypoint_source(dimension, entity));
                    let Some(source) = source else {
                        continue;
                    };
                    if source.transmit_range <= 0.0 {
                        self.level_mut_by_id(dimension)
                            .waypoint_manager
                            .untrack_waypoint(source.key, source.identifier)
                    } else {
                        self.with_waypoint_manager(dimension, |world, manager| {
                            if manager.is_tracking(&source.key) {
                                manager.update_waypoint(&source, |id| world.waypoint_receiver(id))
                            } else {
                                manager.track_waypoint(&source, |id| world.waypoint_receiver(id))
                            }
                        })
                    }
                }
                EntityLifecycleEvent::Removed { entity, .. } if entity.living().is_some() => self
                    .level_mut_by_id(dimension)
                    .waypoint_manager
                    .untrack_waypoint(
                        WaypointSourceKey::StoredEntity {
                            dimension: *dimension,
                            entity_id: entity.id(),
                        },
                        entity.uuid(),
                    ),
                EntityLifecycleEvent::Moved { .. }
                | EntityLifecycleEvent::TrackingStopped { .. }
                | EntityLifecycleEvent::Removed { .. }
                | EntityLifecycleEvent::Broadcast { .. }
                | EntityLifecycleEvent::BlockDestruction { .. }
                | EntityLifecycleEvent::ExternalPlayerBoardingCooldownReset { .. }
                | EntityLifecycleEvent::Sound { .. }
                | EntityLifecycleEvent::SoundExcept { .. }
                | EntityLifecycleEvent::SoundFromEntity { .. }
                | EntityLifecycleEvent::Particles { .. }
                | EntityLifecycleEvent::MobEffectChanged { .. } => continue,
            };
            self.emit_waypoint_packets(packets, events);
        }
        for connection_id in receiver_updates {
            self.waypoint_update_player_receiver(connection_id, events);
        }
    }

    pub(super) fn refresh_player_dirty_attributes(&mut self, events: &mut Vec<WorldEvent>) {
        let mut dirty = Vec::new();
        for player in self.levels.players_mut() {
            let mut attributes = player.attributes.drain_attributes_to_update();
            if let Some(first) = attributes.next() {
                dirty.push((player.connection_id, first, attributes));
            }
        }
        for (connection_id, first, rest) in dirty {
            for attribute_id in std::iter::once(first).chain(rest) {
                self.apply_player_dirty_attribute(connection_id, attribute_id, events);
            }
        }
    }

    pub(super) fn refresh_one_player_dirty_attributes(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        let attributes = player.attributes.drain_attributes_to_update();
        for attribute_id in attributes {
            self.apply_player_dirty_attribute(connection_id, attribute_id, events);
        }
    }

    fn apply_player_dirty_attribute(
        &mut self,
        connection_id: ConnectionId,
        attribute_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        if attribute_id == VANILLA_ATTRIBUTE_MAX_HEALTH_ID {
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("the player remains connected");
            let max_health = player.max_health();
            if player.damage.health > max_health {
                player.damage.set_health(player.damage.health, max_health);
            }
        } else if attribute_id == VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID {
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("the player remains connected");
            let max_absorption = player.max_absorption();
            player.damage.absorption_amount =
                player.damage.absorption_amount.clamp(0.0, max_absorption);
        } else if attribute_id == VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID {
            let Some(source) = self.player_waypoint_source(connection_id) else {
                return;
            };
            let dimension = self.levels[connection_id].dimension;
            let packets = if source.transmit_range > 0.0 {
                self.with_waypoint_manager(&dimension, |world, manager| {
                    manager.track_waypoint(&source, |id| world.waypoint_receiver(id))
                })
            } else {
                self.level_mut_by_id(&dimension)
                    .waypoint_manager
                    .untrack_waypoint(source.key, source.identifier)
            };
            self.emit_waypoint_packets(packets, events);
        } else if attribute_id == VANILLA_ATTRIBUTE_WAYPOINT_RECEIVE_RANGE_ID {
            let receiving = self
                .levels
                .player(connection_id)
                .is_some_and(|player| player.waypoint_receive_range() > 0.0);
            if receiving {
                self.waypoint_add_player(connection_id, events);
            } else {
                self.waypoint_remove_player(connection_id, events);
            }
        }
    }

    pub(super) fn update_player_waypoint_crouch_modifiers(&mut self) {
        for player in self.levels.players_mut() {
            let crouching = player.shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            );
            let instance = player
                .attributes
                .get_instance(VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID)
                .expect("the player supplier declares waypoint transmit range");
            if crouching {
                let already_present = instance
                    .modifier(WAYPOINT_TRANSMIT_RANGE_CROUCH_MODIFIER)
                    .is_some_and(|modifier| {
                        modifier.amount == -1.0
                            && modifier.operation == AttributeOperation::AddMultipliedTotal
                    });
                if !already_present {
                    instance.add_or_update_transient_modifier(AttributeModifier {
                        id: WAYPOINT_TRANSMIT_RANGE_CROUCH_MODIFIER.to_owned(),
                        amount: -1.0,
                        operation: AttributeOperation::AddMultipliedTotal,
                    });
                }
            } else if instance.has_modifier(WAYPOINT_TRANSMIT_RANGE_CROUCH_MODIFIER) {
                instance.remove_modifier(WAYPOINT_TRANSMIT_RANGE_CROUCH_MODIFIER);
            }
        }
    }

    pub(super) fn waypoint_remake_player_by_name(
        &mut self,
        name: &str,
        team_color_rgb: Option<u32>,
        all_levels: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(connection_id) = self
            .levels
            .player_entries()
            .find(|(_, player)| player.profile.name.eq_ignore_ascii_case(name))
            .map(|(&connection_id, _)| connection_id)
        else {
            return;
        };
        let Some(mut source) = self.player_waypoint_source(connection_id) else {
            return;
        };
        source.team_color_rgb = team_color_rgb;
        let player_level = self.levels[connection_id].dimension;
        for dimension in self.level_ids() {
            if !all_levels && dimension != player_level {
                continue;
            }
            let packets = self.with_waypoint_manager(&dimension, |world, manager| {
                manager.remake_connections(&source, |id| world.waypoint_receiver(id))
            });
            self.emit_waypoint_packets(packets, events);
        }
    }

    pub(super) fn waypoint_locator_bar_changed(
        &mut self,
        enabled: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        for dimension in self.level_ids() {
            if !enabled {
                let packets = self
                    .level_mut_by_id(&dimension)
                    .waypoint_manager
                    .break_all_connections();
                self.emit_waypoint_packets(packets, events);
                continue;
            }
            let receiver_ids: Vec<ConnectionId> = self
                .levels
                .player_entries()
                .filter(|(_, player)| player.dimension == dimension)
                .map(|(&connection_id, _)| connection_id)
                .collect();
            for connection_id in receiver_ids {
                let Some(receiver) = self.waypoint_receiver(connection_id) else {
                    continue;
                };
                let packets = self.with_waypoint_manager(&dimension, |world, manager| {
                    manager.update_player(&receiver, |key| world.waypoint_source(key))
                });
                self.emit_waypoint_packets(packets, events);
            }
        }
    }
}

fn make_connection(
    source: &WaypointSource,
    receiver: &WaypointReceiver,
) -> Option<WaypointConnection> {
    if source.first_tick || source_ignores_receiver(source, receiver) {
        return None;
    }
    let icon = source.effective_icon();
    if is_really_far(source, receiver) {
        Some(WaypointConnection::Azimuth {
            identifier: source.identifier,
            icon,
            last_angle: azimuth(source, receiver),
        })
    } else if !is_chunk_visible(source.chunk_position(), receiver) {
        Some(WaypointConnection::Chunk {
            identifier: source.identifier,
            icon,
            last_position: source.chunk_position(),
        })
    } else {
        Some(WaypointConnection::Block {
            identifier: source.identifier,
            icon,
            last_position: source.block_position(),
        })
    }
}

fn source_ignores_receiver(source: &WaypointSource, receiver: &WaypointReceiver) -> bool {
    if receiver.spectator {
        return false;
    }
    if source.spectator || source.indirect_passengers.contains(&receiver.entity_id) {
        return true;
    }
    f64::from(distance(source.position, receiver.position))
        >= source.transmit_range.min(receiver.receive_range)
}

fn is_really_far(source: &WaypointSource, receiver: &WaypointReceiver) -> bool {
    distance(source.position, receiver.position) > REALLY_FAR_DISTANCE
}

fn distance(source: Vec3d, receiver: Vec3d) -> f32 {
    let x = source.x - receiver.x;
    let y = source.y - receiver.y;
    let z = source.z - receiver.z;
    (x * x + y * y + z * z).sqrt() as f32
}

fn is_chunk_visible(chunk: ChunkPos, receiver: &WaypointReceiver) -> bool {
    receiver
        .chunk_tracking_view
        .is_some_and(|view| view.is_in_view_distance((chunk.x, chunk.z)))
}

fn azimuth(source: &WaypointSource, receiver: &WaypointReceiver) -> f32 {
    let direction_x = receiver.position.x - source.position.x;
    let direction_z = receiver.position.z - source.position.z;
    direction_x.atan2(-direction_z) as f32
}

fn manhattan(left: BlockPos, right: BlockPos) -> i64 {
    i64::from(left.x).abs_diff(i64::from(right.x)) as i64
        + i64::from(left.y).abs_diff(i64::from(right.y)) as i64
        + i64::from(left.z).abs_diff(i64::from(right.z)) as i64
}

fn chessboard(left: ChunkPos, right: ChunkPos) -> i64 {
    i64::from(left.x)
        .abs_diff(i64::from(right.x))
        .max(i64::from(left.z).abs_diff(i64::from(right.z))) as i64
}

fn tracked_waypoint(identifier: Uuid, icon: WaypointIcon, data: WaypointData) -> TrackedWaypoint {
    TrackedWaypoint {
        identifier: WaypointIdentifier::Uuid(identifier),
        icon,
        data,
    }
}

fn packet(operation: WaypointOperation, waypoint: TrackedWaypoint) -> TrackedWaypointPacket {
    TrackedWaypointPacket {
        operation,
        waypoint,
    }
}

fn untrack(identifier: Uuid) -> TrackedWaypointPacket {
    packet(
        WaypointOperation::Untrack,
        tracked_waypoint(
            identifier,
            WaypointIcon {
                style: "minecraft:default".to_string(),
                color_rgb: None,
            },
            WaypointData::Empty,
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn icon() -> WaypointIcon {
        WaypointIcon {
            style: "minecraft:default".to_string(),
            color_rgb: None,
        }
    }

    fn source(key: WaypointSourceKey, position: Vec3d) -> WaypointSource {
        WaypointSource {
            key,
            identifier: Uuid::from_u128(10),
            position,
            icon: icon(),
            team_color_rgb: None,
            transmit_range: 60_000_000.0,
            spectator: false,
            first_tick: false,
            indirect_passengers: BTreeSet::new(),
        }
    }

    fn receiver(id: u64, position: Vec3d) -> WaypointReceiver {
        WaypointReceiver {
            connection_id: ConnectionId(id),
            entity_id: id as i32,
            position,
            receive_range: 60_000_000.0,
            spectator: false,
            locator_bar_enabled: true,
            chunk_tracking_view: Some(ChunkTrackingView {
                center: (0, 0),
                view_distance: 2,
            }),
        }
    }

    fn only_packet(packets: Vec<(ConnectionId, TrackedWaypointPacket)>) -> TrackedWaypointPacket {
        assert_eq!(packets.len(), 1);
        packets.into_iter().next().unwrap().1
    }

    #[test]
    fn connections_update_and_change_representation_without_an_untrack() {
        let mut manager = ServerWaypointManager::default();
        let mut source = source(
            WaypointSourceKey::StoredEntity {
                dimension: Java26BuiltinDimension::Overworld.into(),
                entity_id: 4,
            },
            Vec3d {
                x: 1.0,
                y: 64.0,
                z: 1.0,
            },
        );
        let receiver = receiver(1, Vec3d::default());
        let receivers = BTreeMap::from([(receiver.connection_id, receiver.clone())]);
        manager.players.insert(receiver.connection_id);

        let packet = only_packet(manager.track_waypoint(&source, |id| receivers.get(&id).cloned()));
        assert_eq!(packet.operation, WaypointOperation::Track);
        assert!(matches!(packet.waypoint.data, WaypointData::Position(_)));

        source.position.x = 2.0;
        let packet =
            only_packet(manager.update_waypoint(&source, |id| receivers.get(&id).cloned()));
        assert_eq!(packet.operation, WaypointOperation::Update);
        assert!(matches!(packet.waypoint.data, WaypointData::Position(_)));

        source.position.x = 80.0;
        let packet =
            only_packet(manager.update_waypoint(&source, |id| receivers.get(&id).cloned()));
        assert_eq!(packet.operation, WaypointOperation::Track);
        assert!(matches!(packet.waypoint.data, WaypointData::Chunk(_)));

        source.position.x = 400.0;
        let packet =
            only_packet(manager.update_waypoint(&source, |id| receivers.get(&id).cloned()));
        assert_eq!(packet.operation, WaypointOperation::Track);
        assert!(matches!(packet.waypoint.data, WaypointData::Azimuth(_)));

        assert_eq!(manager.connection_count(), 1);
        let packet = only_packet(manager.untrack_waypoint(source.key, source.identifier));
        assert_eq!(packet.operation, WaypointOperation::Untrack);
        assert_eq!(packet.waypoint.data, WaypointData::Empty);
        assert_eq!(manager.connection_count(), 0);
    }

    #[test]
    fn receiver_filtering_matches_range_spectator_passenger_and_gamerule_rules() {
        let mut manager = ServerWaypointManager::default();
        let mut source = source(
            WaypointSourceKey::StoredEntity {
                dimension: Java26BuiltinDimension::Overworld.into(),
                entity_id: 4,
            },
            Vec3d {
                x: 20.0,
                y: 0.0,
                z: 0.0,
            },
        );
        source.transmit_range = 20.0;
        let mut receiver = receiver(1, Vec3d::default());
        receiver.receive_range = 100.0;
        manager.players.insert(receiver.connection_id);

        assert!(manager
            .track_waypoint(&source, |id| {
                (id == receiver.connection_id).then(|| receiver.clone())
            })
            .is_empty());
        receiver.spectator = true;
        assert_eq!(
            manager
                .update_waypoint(&source, |id| {
                    (id == receiver.connection_id).then(|| receiver.clone())
                })
                .len(),
            1
        );

        receiver.spectator = false;
        source.transmit_range = 100.0;
        source.indirect_passengers.insert(receiver.entity_id);
        assert_eq!(
            only_packet(manager.update_waypoint(&source, |id| {
                (id == receiver.connection_id).then(|| receiver.clone())
            }))
            .operation,
            WaypointOperation::Untrack
        );

        source.indirect_passengers.clear();
        receiver.locator_bar_enabled = false;
        assert!(manager
            .update_waypoint(&source, |id| {
                (id == receiver.connection_id).then(|| receiver.clone())
            })
            .is_empty());
    }

    #[test]
    fn remake_replaces_the_icon_with_team_default_in_one_track_packet() {
        let mut manager = ServerWaypointManager::default();
        let mut source = source(
            WaypointSourceKey::Player(ConnectionId(2)),
            Vec3d {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        );
        let receiver = receiver(1, Vec3d::default());
        let receivers = BTreeMap::from([(receiver.connection_id, receiver.clone())]);
        manager.players.insert(receiver.connection_id);
        manager.track_waypoint(&source, |id| receivers.get(&id).cloned());

        source.team_color_rgb = Some(0);
        let packet =
            only_packet(manager.remake_connections(&source, |id| receivers.get(&id).cloned()));
        assert_eq!(packet.operation, WaypointOperation::Track);
        assert_eq!(packet.waypoint.icon.color_rgb, Some(DEFAULT_TEAM_COLOR_RGB));
        assert_eq!(manager.connection_count(), 1);
    }

    #[test]
    fn chunk_visibility_uses_the_non_neighbor_buffer() {
        let receiver = receiver(1, Vec3d::default());
        assert!(is_chunk_visible(ChunkPos { x: 2, z: 0 }, &receiver));
        assert!(!is_chunk_visible(ChunkPos { x: 3, z: 0 }, &receiver));
    }

    #[test]
    fn player_add_update_remove_and_break_keep_both_table_axes_consistent() {
        let mut manager = ServerWaypointManager::default();
        let source = source(
            WaypointSourceKey::StoredEntity {
                dimension: Java26BuiltinDimension::Overworld.into(),
                entity_id: 4,
            },
            Vec3d {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        );
        let sources = BTreeMap::from([(source.key.clone(), source.clone())]);
        let receiver = receiver(1, Vec3d::default());
        manager.transmitters.insert(source.key);

        let receivers = BTreeMap::from([(receiver.connection_id, receiver.clone())]);
        assert_eq!(
            manager
                .add_player(
                    &receiver,
                    |key| sources.get(key).cloned(),
                    None,
                    |id| receivers.get(&id).cloned(),
                )
                .len(),
            1
        );
        assert!(manager
            .update_player(&receiver, |key| sources.get(key).cloned())
            .is_empty());
        assert_eq!(manager.connection_count(), 1);
        assert_eq!(
            manager
                .remove_player(receiver.connection_id, Uuid::from_u128(1))
                .len(),
            1
        );
        assert_eq!(manager.connection_count(), 0);

        manager.add_player(
            &receiver,
            |key| sources.get(key).cloned(),
            None,
            |id| receivers.get(&id).cloned(),
        );
        assert_eq!(manager.break_all_connections().len(), 1);
        assert_eq!(manager.connection_count(), 0);
    }

    #[test]
    fn adding_a_transmitting_player_connects_the_existing_receivers() {
        let mut manager = ServerWaypointManager::default();
        let existing = receiver(1, Vec3d::default());
        manager.players.insert(existing.connection_id);
        let joining = receiver(
            2,
            Vec3d {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
        );
        let player_source = source(
            WaypointSourceKey::Player(joining.connection_id),
            joining.position,
        );
        let receivers = BTreeMap::from([
            (existing.connection_id, existing),
            (joining.connection_id, joining.clone()),
        ]);

        let packets = manager.add_player(
            &joining,
            |_| None,
            Some(&player_source),
            |id| receivers.get(&id).cloned(),
        );
        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].0, ConnectionId(1));
        assert_eq!(packets[0].1.operation, WaypointOperation::Track);
    }
}
