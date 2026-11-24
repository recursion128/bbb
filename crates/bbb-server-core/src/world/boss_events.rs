//! Live custom boss bars and their persistent UUID membership.

use std::collections::{BTreeMap, BTreeSet};

use uuid::Uuid;

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use bbb_facts::boss_bar_styles::{VANILLA_BOSS_BAR_COLORS, VANILLA_BOSS_BAR_OVERLAYS};
use bbb_protocol::packets::{
    BossBarColor, BossBarOverlay, BossEvent, BossEventFlags, BossEventOperation,
};
use bbb_protocol::RichTextComponent;
use bbb_server_persistence::{Java26CustomBossEventPacked, Java26CustomBossEventsPacked};

use crate::admin::NameAndId;
use crate::commands::{CommandBossBarMutation, CommandBossBarState};
use crate::entity::{Entity, EntityData};

use super::scoreboard::Scoreboard;
use super::{LevelId, ServerWorld, WorldEvent};

/// `WitherBoss.INVULNERABLE_TICKS` (`WitherBoss.java:66`).
const WITHER_INVULNERABLE_TICKS: f32 = 220.0;

#[derive(Debug, Clone)]
struct Event {
    runtime_id: Uuid,
    state: CommandBossBarState,
    darken_screen: bool,
    play_music: bool,
    create_world_fog: bool,
    live: BTreeMap<ConnectionId, Uuid>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ServerBossEvents {
    events: BTreeMap<String, Event>,
    /// Bars a living boss owns, shown to the sessions tracking it.
    /// Each wither's bar with the custom name it last took its name from.
    entities: BTreeMap<(LevelId, i32), (ServerBossEvent, Option<RichTextComponent>)>,
}

impl ServerBossEvents {
    /// `CustomBossEvents.java:19-24` assigns a fresh process UUID while loading;
    /// the persistent document carries only the resource id and packed state.
    #[cfg(test)]
    pub(super) fn restore(packed: &Java26CustomBossEventsPacked) -> Self {
        Self::restore_with_registry(
            packed,
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        )
    }

    pub(super) fn restore_with_registry(
        packed: &Java26CustomBossEventsPacked,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Self {
        let mut events = BTreeMap::new();
        for (id, packed) in &packed.0 {
            let Ok(name) = RichTextComponent::try_from_component_nbt_with_registry(
                packed.name.clone(),
                content_registry,
            ) else {
                continue;
            };
            let color = color_from_name(&packed.color).unwrap_or(BossBarColor::White);
            let overlay = overlay_from_name(&packed.overlay).unwrap_or(BossBarOverlay::Progress);
            events.insert(
                id.clone(),
                Event {
                    runtime_id: Uuid::new_v4(),
                    state: CommandBossBarState {
                        id: id.clone(),
                        name,
                        value: packed.value,
                        max: packed.max,
                        color,
                        overlay,
                        visible: packed.visible,
                        players: unique_players(&packed.players),
                        online_players: Vec::new(),
                    },
                    darken_screen: packed.darken_screen,
                    play_music: packed.play_boss_music,
                    create_world_fog: packed.create_world_fog,
                    live: BTreeMap::new(),
                },
            );
        }
        Self {
            events,
            entities: BTreeMap::new(),
        }
    }

    pub(super) fn persistent_snapshot(&self) -> Java26CustomBossEventsPacked {
        Java26CustomBossEventsPacked(
            self.events
                .iter()
                .map(|(id, event)| {
                    (
                        id.clone(),
                        Java26CustomBossEventPacked {
                            name: event.state.name.component().clone(),
                            visible: event.state.visible,
                            value: event.state.value,
                            max: event.state.max,
                            color: event.state.color.as_str().to_string(),
                            overlay: event.state.overlay.as_str().to_string(),
                            darken_screen: event.darken_screen,
                            play_boss_music: event.play_music,
                            create_world_fog: event.create_world_fog,
                            players: unique_players(&event.state.players),
                        },
                    )
                })
                .collect(),
        )
    }

    pub(super) fn contains(&self, id: &str) -> bool {
        self.events.contains_key(id)
    }

    pub(super) fn len(&self) -> usize {
        self.events.len()
    }

    pub(super) fn ids(&self) -> impl Iterator<Item = &str> + '_ {
        self.events.keys().map(String::as_str)
    }

    pub(super) fn state(&self, id: &str) -> Option<&CommandBossBarState> {
        self.events.get(id).map(|event| &event.state)
    }

    pub(super) fn states(&self) -> impl Iterator<Item = &CommandBossBarState> + '_ {
        self.events.values().map(|event| &event.state)
    }

    #[cfg(test)]
    pub(super) fn command_snapshot(&self, online: &[NameAndId]) -> Vec<CommandBossBarState> {
        self.events
            .values()
            .map(|event| {
                let mut state = event.state.clone();
                state.online_players = online
                    .iter()
                    .filter(|player| state.players.contains(&player.id))
                    .cloned()
                    .collect();
                state
            })
            .collect()
    }

    pub(super) fn apply_command(
        &mut self,
        mutation: CommandBossBarMutation,
        created_id: Option<Uuid>,
        online: &[(ConnectionId, NameAndId)],
    ) -> Vec<(ConnectionId, ClientboundPlayPacket)> {
        let mut packets = Vec::new();
        match mutation {
            CommandBossBarMutation::Create { id, name, .. } => {
                let event = Event {
                    runtime_id: created_id.expect("create bossbar requires runtime UUID"),
                    state: CommandBossBarState {
                        id: id.clone(),
                        name,
                        value: 0,
                        max: 100,
                        color: BossBarColor::White,
                        overlay: BossBarOverlay::Progress,
                        visible: true,
                        players: Vec::new(),
                        online_players: Vec::new(),
                    },
                    darken_screen: false,
                    play_music: false,
                    create_world_fog: false,
                    live: BTreeMap::new(),
                };
                assert!(
                    self.events.insert(id, event).is_none(),
                    "bossbar create references an unused id"
                );
            }
            CommandBossBarMutation::Remove { id } => {
                if let Some(event) = self.events.remove(&id) {
                    if event.state.visible {
                        packets.extend(
                            event
                                .live
                                .into_keys()
                                .map(|connection| (connection, remove(event.runtime_id))),
                        );
                    }
                }
            }
            mutation => {
                let id = mutation_id(&mutation);
                let event = self
                    .events
                    .get_mut(id)
                    .expect("bossbar mutation references an existing id");
                match mutation {
                    CommandBossBarMutation::SetName { name: new_name, .. } => {
                        if event.state.name != new_name {
                            event.state.name = new_name;
                            broadcast(event, &mut packets, name);
                        }
                    }
                    CommandBossBarMutation::SetColor { color, .. } => {
                        if event.state.color != color {
                            event.state.color = color;
                            broadcast(event, &mut packets, style);
                        }
                    }
                    CommandBossBarMutation::SetOverlay { overlay, .. } => {
                        if event.state.overlay != overlay {
                            event.state.overlay = overlay;
                            broadcast(event, &mut packets, style);
                        }
                    }
                    CommandBossBarMutation::SetValue { value, .. } => {
                        let old_progress = event_progress(event);
                        event.state.value = value;
                        if event_progress(event) != old_progress {
                            broadcast(event, &mut packets, progress);
                        }
                    }
                    CommandBossBarMutation::SetMax { max, .. } => {
                        let old_progress = event_progress(event);
                        event.state.max = max;
                        if event_progress(event) != old_progress {
                            broadcast(event, &mut packets, progress);
                        }
                    }
                    CommandBossBarMutation::SetVisible { visible, .. } => {
                        if event.state.visible != visible {
                            event.state.visible = visible;
                            packets.extend(event.live.keys().copied().map(|connection| {
                                (
                                    connection,
                                    if visible {
                                        add(event)
                                    } else {
                                        remove(event.runtime_id)
                                    },
                                )
                            }));
                        }
                    }
                    CommandBossBarMutation::SetPlayers { players, .. } => {
                        event.state.players = unique_players(&players);
                        let wanted = online
                            .iter()
                            .filter(|(_, player)| event.state.players.contains(&player.id))
                            .map(|(connection, player)| (*connection, player.id))
                            .collect::<BTreeMap<_, _>>();

                        // `CustomBossEvent.setPlayers` removes stale members before
                        // adding newcomers (`CustomBossEvent.java:127-140`).
                        let removed = event
                            .live
                            .keys()
                            .filter(|connection| !wanted.contains_key(connection))
                            .copied()
                            .collect::<Vec<_>>();
                        for connection in removed {
                            event.live.remove(&connection);
                            if event.state.visible {
                                packets.push((connection, remove(event.runtime_id)));
                            }
                        }
                        for (connection, player) in wanted {
                            if event.live.insert(connection, player).is_none()
                                && event.state.visible
                            {
                                packets.push((connection, add(event)));
                            }
                        }
                    }
                    CommandBossBarMutation::Create { .. }
                    | CommandBossBarMutation::Remove { .. } => {
                        unreachable!()
                    }
                }
            }
        }
        packets
    }

    pub(super) fn on_player_connect(
        &mut self,
        connection_id: ConnectionId,
        player: Uuid,
    ) -> Vec<ClientboundPlayPacket> {
        let mut packets = Vec::new();
        for event in self.events.values_mut() {
            if event.state.players.contains(&player)
                && event.live.insert(connection_id, player).is_none()
                && event.state.visible
            {
                packets.push(add(event));
            }
        }
        packets
    }

    pub(super) fn on_player_disconnect(&mut self, connection_id: ConnectionId, _player: Uuid) {
        for event in self.events.values_mut() {
            event.live.remove(&connection_id);
        }
        for (event, _) in self.entities.values_mut() {
            event.forget_player(connection_id);
        }
    }
}

impl ServerWorld {
    /// Incrementally tracks boss-bar entity IDs from entity lifecycle events so
    /// [`Self::sync_entity_boss_bars`] avoids scanning every loaded entity each tick.
    pub(super) fn update_boss_bar_entity_index(
        &mut self,
        dimension: &LevelId,
        lifecycle_events: &[crate::entity::EntityLifecycleEvent],
    ) {
        let plugins = &self.cold.config.plugins;
        let Some(level) = self.levels.get_mut(dimension) else {
            return;
        };
        if !level.boss_bar_entities_seeded {
            return;
        }
        for event in lifecycle_events {
            match event {
                crate::entity::EntityLifecycleEvent::Added { entity, .. }
                | crate::entity::EntityLifecycleEvent::TrackingStarted { entity } => {
                    if entity_can_have_boss_bar(entity, plugins) {
                        level.boss_bar_entities.insert(entity.id());
                    }
                }
                crate::entity::EntityLifecycleEvent::Removed { entity, .. } => {
                    level.boss_bar_entities.remove(&entity.id());
                }
                _ => {}
            }
        }
    }

    /// `WitherBoss.startSeenByPlayer`/`stopSeenByPlayer` and its
    /// `customServerAiStep` progress (`WitherBoss.java:261-264,351,370-379`):
    /// each wither's bar follows the sessions tracking it.
    pub(super) fn sync_entity_boss_bars(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(level) = self.levels.get_mut(dimension) else {
            return;
        };
        if !level.boss_bar_entities_seeded {
            for entity in level.entities.bodies() {
                if entity_can_have_boss_bar(entity, &self.cold.config.plugins) {
                    level.boss_bar_entities.insert(entity.id());
                }
            }
            level.boss_bar_entities_seeded = true;
        }
        let bars = &mut self.cold.boss_events.entities;
        if level.boss_bar_entities.is_empty() && !bars.keys().any(|(dim, _)| dim == dimension) {
            return;
        }
        let scoreboard = self.scoreboard.board();
        let registry = self.cold.config.plugins.content_registry();
        let mut packets = BossPackets::new();
        let mut live = BTreeSet::new();
        let mut stale_ids = Vec::new();
        for &entity_id in &level.boss_bar_entities {
            let Some(entity) = level.entities.get(entity_id) else {
                stale_ids.push(entity_id);
                continue;
            };
            let Some((color, overlay, flags, progress)) =
                entity_boss_bar_spec_and_progress(entity, &self.cold.config.plugins)
            else {
                continue;
            };
            let key = (*dimension, entity_id);
            live.insert(key);
            // The bar takes `getDisplayName()` when built and again only on
            // `setCustomName` (`WitherBoss.java:76,129,134-137`).
            let (bar, named_from) = bars.entry(key).or_insert_with(|| {
                (
                    ServerBossEvent::new(
                        boss_display_name(entity, scoreboard, registry),
                        color,
                        overlay,
                        flags,
                    ),
                    entity.custom_name().cloned(),
                )
            });
            if named_from.as_ref() != entity.custom_name() {
                *named_from = entity.custom_name().cloned();
                bar.set_name(
                    boss_display_name(entity, scoreboard, registry),
                    &mut packets,
                );
            }
            // `customServerAiStep` sets the progress, which neither a NoAI nor
            // a dying wither runs (`WitherBoss.java:264,351`).
            let ai_runs = entity
                .living()
                .is_some_and(|living| !living.is_dead_or_dying())
                && !matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_no_ai());
            if ai_runs {
                bar.set_progress(progress, &mut packets);
            }
            let viewers = level.entity_tracker.viewers_of(entity_id);
            bar.set_players(&viewers.into_iter().collect(), &mut packets);
        }
        for stale in stale_ids {
            level.boss_bar_entities.remove(&stale);
        }
        if !bars.is_empty() {
            bars.retain(|key, (bar, _)| {
                if key.0 != *dimension || live.contains(key) {
                    return true;
                }
                bar.remove_all_players(&mut packets);
                false
            });
        }
        for (connection_id, packet) in packets {
            events.push(WorldEvent::Send {
                connection_id,
                packet,
            });
        }
    }
}

fn entity_can_have_boss_bar(
    entity: &Entity,
    plugins: &crate::plugin::ServerPluginRegistry,
) -> bool {
    matches!(
        entity.data(),
        EntityData::GeneralLiving(data) if data.wither_retained().is_some()
    ) || plugins
        .mob_entry(entity.entity_type_id())
        .is_some_and(|entry| entry.spec.boss_bar.is_some())
}

fn entity_boss_bar_spec_and_progress(
    entity: &Entity,
    plugins: &crate::plugin::ServerPluginRegistry,
) -> Option<(BossBarColor, BossBarOverlay, BossEventFlags, f32)> {
    if let Some(progress) = wither_progress(entity) {
        return Some((
            BossBarColor::Purple,
            BossBarOverlay::Progress,
            BossEventFlags {
                darken_screen: true,
                ..BossEventFlags::default()
            },
            progress,
        ));
    }
    let entry = plugins.mob_entry(entity.entity_type_id())?;
    let bar_spec = entry.spec.boss_bar.as_ref()?;
    let progress = entry.behavior.boss_bar_progress(entity)?;
    Some((
        BossBarColor::from_ordinal(bar_spec.color_id).unwrap_or(BossBarColor::Purple),
        BossBarOverlay::from_ordinal(bar_spec.overlay_id).unwrap_or(BossBarOverlay::Progress),
        BossEventFlags {
            darken_screen: bar_spec.darken_screen,
            play_music: bar_spec.play_boss_music,
            create_world_fog: bar_spec.create_world_fog,
        },
        progress,
    ))
}

fn wither_progress(entity: &Entity) -> Option<f32> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    let invulnerable = data.wither_retained()?.invulnerable_ticks();
    if invulnerable > 0 {
        return Some(1.0 - invulnerable as f32 / WITHER_INVULNERABLE_TICKS);
    }
    let living = entity.living()?;
    Some(living.health() / living.max_health())
}

/// `Entity.getDisplayName` (`Entity.java:3358-3360`) over the entity's team.
pub(super) fn boss_display_name(
    entity: &Entity,
    scoreboard: &Scoreboard,
    registry: &bbb_protocol::content_registry::ContentRegistry,
) -> RichTextComponent {
    RichTextComponent::try_from_component_nbt_with_registry(
        super::display_names::stored_display_name(
            entity,
            scoreboard.players_team(&entity.uuid().to_string()),
        ),
        registry,
    )
    .expect("a display name is a valid component")
}

/// What one boss event publishes, addressed by session.
pub(super) type BossPackets = Vec<(ConnectionId, ClientboundPlayPacket)>;

/// `ServerBossEvent`: a server-owned bar (raid, wither, dragon fight) whose
/// members are live sessions.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ServerBossEvent {
    id: Uuid,
    name: RichTextComponent,
    progress: f32,
    color: BossBarColor,
    overlay: BossBarOverlay,
    flags: BossEventFlags,
    visible: bool,
    players: BTreeSet<ConnectionId>,
}

impl ServerBossEvent {
    pub(super) fn new(
        name: RichTextComponent,
        color: BossBarColor,
        overlay: BossBarOverlay,
        flags: BossEventFlags,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            progress: 1.0,
            color,
            overlay,
            flags,
            visible: true,
            players: BTreeSet::new(),
        }
    }

    #[cfg(test)]
    pub(super) const fn progress(&self) -> f32 {
        self.progress
    }

    pub(super) const fn players(&self) -> &BTreeSet<ConnectionId> {
        &self.players
    }

    pub(super) fn add_player(&mut self, connection: ConnectionId, packets: &mut BossPackets) {
        if self.players.insert(connection) && self.visible {
            packets.push((connection, self.add_packet()));
        }
    }

    pub(super) fn remove_player(&mut self, connection: ConnectionId, packets: &mut BossPackets) {
        if self.players.remove(&connection) && self.visible {
            packets.push((connection, self.remove_packet()));
        }
    }

    /// Drops a session that left the server; it has no client to tell.
    pub(super) fn forget_player(&mut self, connection: ConnectionId) {
        self.players.remove(&connection);
    }

    pub(super) fn remove_all_players(&mut self, packets: &mut BossPackets) {
        for connection in std::mem::take(&mut self.players) {
            if self.visible {
                packets.push((connection, self.remove_packet()));
            }
        }
    }

    /// Adds the newcomers and removes the members `wanted` no longer lists.
    pub(super) fn set_players(
        &mut self,
        wanted: &BTreeSet<ConnectionId>,
        packets: &mut BossPackets,
    ) {
        let current = self.players.clone();
        for &connection in wanted.difference(&current) {
            self.add_player(connection, packets);
        }
        for &connection in current.difference(wanted) {
            self.remove_player(connection, packets);
        }
    }

    /// A hidden bar keeps its members and re-adds them when shown again.
    pub(super) fn set_visible(&mut self, visible: bool, packets: &mut BossPackets) {
        if visible == self.visible {
            return;
        }
        self.visible = visible;
        for &connection in &self.players {
            let packet = if visible {
                self.add_packet()
            } else {
                self.remove_packet()
            };
            packets.push((connection, packet));
        }
    }

    pub(super) fn set_name(&mut self, name: RichTextComponent, packets: &mut BossPackets) {
        if name == self.name {
            return;
        }
        self.name = name;
        self.broadcast(packets, Self::name_packet);
    }

    pub(super) fn set_progress(&mut self, progress: f32, packets: &mut BossPackets) {
        if progress == self.progress {
            return;
        }
        self.progress = progress;
        self.broadcast(packets, Self::progress_packet);
    }

    fn broadcast(&self, packets: &mut BossPackets, packet: fn(&Self) -> ClientboundPlayPacket) {
        if !self.visible {
            return;
        }
        packets.extend(
            self.players
                .iter()
                .map(|&connection| (connection, packet(self))),
        );
    }

    fn add_packet(&self) -> ClientboundPlayPacket {
        ClientboundPlayPacket::BossEvent(BossEvent {
            id: self.id,
            operation: BossEventOperation::Add {
                name: Box::new(self.name.clone()),
                progress: self.progress,
                color: self.color,
                overlay: self.overlay,
                flags: self.flags,
            },
        })
    }

    fn remove_packet(&self) -> ClientboundPlayPacket {
        remove(self.id)
    }

    fn progress_packet(&self) -> ClientboundPlayPacket {
        ClientboundPlayPacket::BossEvent(BossEvent {
            id: self.id,
            operation: BossEventOperation::UpdateProgress {
                progress: self.progress,
            },
        })
    }

    fn name_packet(&self) -> ClientboundPlayPacket {
        ClientboundPlayPacket::BossEvent(BossEvent {
            id: self.id,
            operation: BossEventOperation::UpdateName {
                name: Box::new(self.name.clone()),
            },
        })
    }
}

fn mutation_id(mutation: &CommandBossBarMutation) -> &str {
    match mutation {
        CommandBossBarMutation::SetName { id, .. }
        | CommandBossBarMutation::SetColor { id, .. }
        | CommandBossBarMutation::SetOverlay { id, .. }
        | CommandBossBarMutation::SetValue { id, .. }
        | CommandBossBarMutation::SetMax { id, .. }
        | CommandBossBarMutation::SetVisible { id, .. }
        | CommandBossBarMutation::SetPlayers { id, .. } => id,
        CommandBossBarMutation::Create { .. } | CommandBossBarMutation::Remove { .. } => {
            unreachable!()
        }
    }
}

fn unique_players(players: &[Uuid]) -> Vec<Uuid> {
    players
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn color_from_name(name: &str) -> Option<BossBarColor> {
    VANILLA_BOSS_BAR_COLORS
        .iter()
        .find(|entry| entry.serialized_name == name)
        .map(|entry| entry.color)
}

fn overlay_from_name(name: &str) -> Option<BossBarOverlay> {
    VANILLA_BOSS_BAR_OVERLAYS
        .iter()
        .find(|entry| entry.serialized_name == name)
        .map(|entry| entry.overlay)
}

fn flags(event: &Event) -> BossEventFlags {
    BossEventFlags {
        darken_screen: event.darken_screen,
        play_music: event.play_music,
        create_world_fog: event.create_world_fog,
    }
}

/// `CustomBossEvent.java:72-75` and `CustomBossEvent.java:78-81` apply
/// `Mth.clamp((float)value / max, 0, 1)`;
/// explicit comparisons preserve its NaN and signed-zero behavior.
fn event_progress(event: &Event) -> f32 {
    let value = event.state.value as f32 / event.state.max as f32;
    value.clamp(0.0, 1.0)
}

fn broadcast(
    event: &Event,
    packets: &mut Vec<(ConnectionId, ClientboundPlayPacket)>,
    packet: fn(&Event) -> ClientboundPlayPacket,
) {
    if event.state.visible {
        packets.extend(
            event
                .live
                .keys()
                .copied()
                .map(|connection| (connection, packet(event))),
        );
    }
}

fn add(event: &Event) -> ClientboundPlayPacket {
    ClientboundPlayPacket::BossEvent(BossEvent {
        id: event.runtime_id,
        operation: BossEventOperation::Add {
            name: Box::new(event.state.name.clone()),
            progress: event_progress(event),
            color: event.state.color,
            overlay: event.state.overlay,
            flags: flags(event),
        },
    })
}

fn remove(id: Uuid) -> ClientboundPlayPacket {
    ClientboundPlayPacket::BossEvent(BossEvent {
        id,
        operation: BossEventOperation::Remove,
    })
}

fn progress(event: &Event) -> ClientboundPlayPacket {
    ClientboundPlayPacket::BossEvent(BossEvent {
        id: event.runtime_id,
        operation: BossEventOperation::UpdateProgress {
            progress: event_progress(event),
        },
    })
}

fn name(event: &Event) -> ClientboundPlayPacket {
    ClientboundPlayPacket::BossEvent(BossEvent {
        id: event.runtime_id,
        operation: BossEventOperation::UpdateName {
            name: Box::new(event.state.name.clone()),
        },
    })
}

fn style(event: &Event) -> ClientboundPlayPacket {
    ClientboundPlayPacket::BossEvent(BossEvent {
        id: event.runtime_id,
        operation: BossEventOperation::UpdateStyle {
            color: event.state.color,
            overlay: event.state.overlay,
        },
    })
}

#[cfg(test)]
impl super::ServerWorld {
    pub(crate) fn command_boss_bars_snapshot_for_test(&self) -> Vec<CommandBossBarState> {
        let online = self
            .levels
            .players()
            .map(|player| NameAndId {
                id: player.profile.uuid,
                name: (&player.profile.name).into(),
            })
            .collect::<Vec<_>>();
        self.cold.boss_events.command_snapshot(&online)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_server_types::save::Java26BuiltinDimension;

    const ID: &str = "minecraft:test";

    fn connection(id: u64) -> ConnectionId {
        ConnectionId(id)
    }

    fn player(name: &str, id: u128) -> NameAndId {
        NameAndId {
            id: Uuid::from_u128(id),
            name: name.into(),
        }
    }

    fn create(events: &mut ServerBossEvents, runtime_id: Uuid) {
        assert!(events
            .apply_command(
                CommandBossBarMutation::Create {
                    id: ID.to_string(),
                    name: RichTextComponent::plain("Test"),
                    source_level: Java26BuiltinDimension::Overworld.into(),
                },
                Some(runtime_id),
                &[],
            )
            .is_empty());
    }

    fn operation(packet: &ClientboundPlayPacket) -> &BossEventOperation {
        let ClientboundPlayPacket::BossEvent(event) = packet else {
            panic!("expected boss event packet")
        };
        &event.operation
    }

    #[test]
    fn bossbar_create_defaults_and_hidden_mutations_match_live_visibility() {
        let runtime_id = Uuid::from_u128(10);
        let alice = player("alice", 1);
        let bob = player("bob", 2);
        let mut events = ServerBossEvents::default();
        create(&mut events, runtime_id);
        let state = &events.command_snapshot(&[])[0];
        assert_eq!(state.value, 0);
        assert_eq!(state.max, 100);
        assert_eq!(state.color, BossBarColor::White);
        assert_eq!(state.overlay, BossBarOverlay::Progress);
        assert!(state.visible);

        let packets = events.apply_command(
            CommandBossBarMutation::SetPlayers {
                id: ID.to_string(),
                players: vec![alice.id],
            },
            None,
            &[(connection(1), alice.clone())],
        );
        assert!(matches!(
            operation(&packets[0].1),
            BossEventOperation::Add { .. }
        ));

        let packets = events.apply_command(
            CommandBossBarMutation::SetVisible {
                id: ID.to_string(),
                visible: false,
            },
            None,
            &[],
        );
        assert!(matches!(
            operation(&packets[0].1),
            BossEventOperation::Remove
        ));
        assert!(events
            .apply_command(
                CommandBossBarMutation::SetValue {
                    id: ID.to_string(),
                    value: 50,
                },
                None,
                &[],
            )
            .is_empty());
        assert!(events
            .apply_command(
                CommandBossBarMutation::SetPlayers {
                    id: ID.to_string(),
                    players: vec![bob.id],
                },
                None,
                &[(connection(1), alice), (connection(2), bob.clone())],
            )
            .is_empty());
        let packets = events.apply_command(
            CommandBossBarMutation::SetVisible {
                id: ID.to_string(),
                visible: true,
            },
            None,
            &[],
        );
        assert_eq!(packets[0].0, connection(2));
        assert!(matches!(
            operation(&packets[0].1),
            BossEventOperation::Add { progress, .. } if *progress == 0.5
        ));
    }

    #[test]
    fn bossbar_visible_updates_and_remove_emit_the_matching_operations() {
        let alice = player("alice", 1);
        let mut events = ServerBossEvents::default();
        create(&mut events, Uuid::from_u128(10));
        events.apply_command(
            CommandBossBarMutation::SetPlayers {
                id: ID.to_string(),
                players: vec![alice.id],
            },
            None,
            &[(connection(1), alice)],
        );

        let mutations = [
            CommandBossBarMutation::SetName {
                id: ID.to_string(),
                name: RichTextComponent::plain("Renamed"),
            },
            CommandBossBarMutation::SetColor {
                id: ID.to_string(),
                color: BossBarColor::Purple,
            },
            CommandBossBarMutation::SetOverlay {
                id: ID.to_string(),
                overlay: BossBarOverlay::Notched12,
            },
            CommandBossBarMutation::SetValue {
                id: ID.to_string(),
                value: 25,
            },
            CommandBossBarMutation::SetMax {
                id: ID.to_string(),
                max: 50,
            },
        ];
        let packets = mutations
            .into_iter()
            .flat_map(|mutation| events.apply_command(mutation, None, &[]))
            .collect::<Vec<_>>();
        assert!(matches!(
            operation(&packets[0].1),
            BossEventOperation::UpdateName { .. }
        ));
        assert!(matches!(
            operation(&packets[1].1),
            BossEventOperation::UpdateStyle { .. }
        ));
        assert!(matches!(
            operation(&packets[2].1),
            BossEventOperation::UpdateStyle { .. }
        ));
        assert!(matches!(
            operation(&packets[3].1),
            BossEventOperation::UpdateProgress { progress } if *progress == 0.25
        ));
        assert!(matches!(
            operation(&packets[4].1),
            BossEventOperation::UpdateProgress { progress } if *progress == 0.5
        ));

        let packets = events.apply_command(
            CommandBossBarMutation::Remove { id: ID.to_string() },
            None,
            &[],
        );
        assert!(matches!(
            operation(&packets[0].1),
            BossEventOperation::Remove
        ));
    }

    #[test]
    fn bossbar_player_replacement_removes_before_it_adds() {
        let alice = player("alice", 1);
        let bob = player("bob", 2);
        let mut events = ServerBossEvents::default();
        create(&mut events, Uuid::from_u128(10));
        events.apply_command(
            CommandBossBarMutation::SetPlayers {
                id: ID.to_string(),
                players: vec![alice.id],
            },
            None,
            &[(connection(1), alice.clone())],
        );

        let packets = events.apply_command(
            CommandBossBarMutation::SetPlayers {
                id: ID.to_string(),
                players: vec![bob.id],
            },
            None,
            &[(connection(1), alice), (connection(2), bob)],
        );
        assert_eq!(packets[0].0, connection(1));
        assert!(matches!(
            operation(&packets[0].1),
            BossEventOperation::Remove
        ));
        assert_eq!(packets[1].0, connection(2));
        assert!(matches!(
            operation(&packets[1].1),
            BossEventOperation::Add { .. }
        ));
    }

    #[test]
    fn bossbar_membership_survives_disconnect_but_live_membership_does_not() {
        let alice = player("alice", 1);
        let mut events = ServerBossEvents::default();
        create(&mut events, Uuid::from_u128(10));
        events.apply_command(
            CommandBossBarMutation::SetPlayers {
                id: ID.to_string(),
                players: vec![alice.id],
            },
            None,
            &[],
        );
        let packets = events.on_player_connect(connection(1), alice.id);
        assert!(matches!(
            operation(&packets[0]),
            BossEventOperation::Add { .. }
        ));
        assert_eq!(
            events.command_snapshot(std::slice::from_ref(&alice))[0].online_players,
            std::slice::from_ref(&alice)
        );

        events.on_player_disconnect(connection(1), alice.id);
        let state = &events.command_snapshot(&[])[0];
        assert_eq!(state.players, [alice.id]);
        assert!(state.online_players.is_empty());
        let packets = events.on_player_connect(connection(2), alice.id);
        assert!(matches!(
            operation(&packets[0]),
            BossEventOperation::Add { .. }
        ));
    }

    #[test]
    fn bossbar_restore_deduplicates_members_and_mints_runtime_ids() {
        let alice = Uuid::from_u128(1);
        let packed = Java26CustomBossEventsPacked(BTreeMap::from([(
            ID.to_string(),
            Java26CustomBossEventPacked {
                name: bbb_protocol::component_literal_nbt("Saved"),
                visible: true,
                value: 0,
                max: 0,
                color: "purple".to_string(),
                overlay: "notched_12".to_string(),
                darken_screen: true,
                play_boss_music: true,
                create_world_fog: true,
                players: vec![alice, alice],
            },
        )]));
        let mut first = ServerBossEvents::restore(&packed);
        assert_eq!(first.events[ID].state.players, [alice]);
        assert_eq!(first.persistent_snapshot().0[ID].players, [alice]);

        let packets = first.on_player_connect(connection(1), alice);
        assert!(matches!(
            operation(&packets[0]),
            BossEventOperation::Add { progress, flags, .. }
                if progress.is_nan()
                    && flags.darken_screen
                    && flags.play_music
                    && flags.create_world_fog
        ));
    }
}
