//! External-player projection into the level-owned entity tracker.

use crate::world::LevelId;
use std::collections::{BTreeMap, BTreeSet};

use crate::connection::ConnectionId;
use crate::entity::tracking::attribute_snapshot;
use crate::entity::tracking::player_metadata::{player_metadata, PlayerRows};
use crate::entity::{EntityStore, PlayerPairingData, PlayerSyncFrame};
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{AttributeSnapshot, EquipmentSlotUpdate, SetPassengers};
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;

use super::{
    block_position, is_chunk_tracked, send, ClientboundPlayPacket, PlayerState, ServerWorld,
    WorldEvent,
};

#[derive(Clone, Default)]
pub(super) struct PlayerChunkMembership {
    players: BTreeMap<i32, bool>,
    distance: BTreeMap<crate::chunk_sender::ChunkKey, BTreeSet<i32>>,
}

impl PlayerChunkMembership {
    fn add(&mut self, id: i32, chunk: crate::chunk_sender::ChunkKey, ignored: bool) {
        self.players.insert(id, ignored);
        if !ignored {
            self.distance.entry(chunk).or_default().insert(id);
        }
    }

    fn remove_distance(&mut self, chunk: crate::chunk_sender::ChunkKey, id: i32) -> Option<()> {
        let members = self.distance.get_mut(&chunk)?;
        members.remove(&id);
        if members.is_empty() {
            self.distance.remove(&chunk);
        }
        Some(())
    }

    pub(super) fn remove(&mut self, id: i32, chunk: crate::chunk_sender::ChunkKey) {
        let ignored = self.players.get(&id).copied().unwrap_or(true);
        if !ignored {
            let _ = self.remove_distance(chunk, id);
        }
        self.players.remove(&id);
    }

    fn move_player(
        &mut self,
        id: i32,
        old: (i32, i32, i32),
        next: (i32, i32, i32),
        ignored: bool,
    ) -> Option<bool> {
        let was_ignored = self.players.get(&id).copied().unwrap_or(false);
        if old == next && was_ignored == ignored {
            return Some(false);
        }
        if !was_ignored {
            self.remove_distance((old.0, old.2), id)?;
        }
        if !ignored {
            self.distance
                .entry((next.0, next.2))
                .or_default()
                .insert(id);
        }
        if let Some(filed) = self.players.get_mut(&id) {
            *filed = ignored;
        }
        Some(true)
    }

    pub(super) fn centres(&self) -> impl Iterator<Item = crate::chunk_sender::ChunkKey> + '_ {
        self.distance.keys().copied()
    }
}

/// The synched items `ServerEntity.sendChanges` packs this tick
/// (`ServerEntity.java:216`, `SynchedEntityData.java:74-90`).
#[derive(Default)]
struct PlayerDirtyRows {
    shoulders: bool,
    silent: bool,
    air_supply: bool,
    ticks_frozen: bool,
    shared_flags: bool,
    pose: bool,
    sleeping_pos: bool,
    living_entity_flags: bool,
    arrow_count: bool,
    stinger_count: bool,
    effect_particles: bool,
    effect_ambience: bool,
    main_hand: bool,
    model_customization: bool,
}

impl PlayerState {
    pub(crate) fn uuid(&self) -> uuid::Uuid {
        self.profile.uuid
    }

    pub(crate) fn is_spectator(&self) -> bool {
        self.game_type == GameType::Spectator
    }

    pub(crate) fn camera_is_self(&self) -> bool {
        self.cold.camera.is_none()
    }

    /// Player-owned rows of `ServerEntity.sendPairingData`
    /// (`ServerEntity.java:269-309`).
    pub(crate) fn pairing_data(&self, entities: &EntityStore) -> PlayerPairingData {
        let equipment = self
            .inventory
            .equipment_slots()
            .filter_map(|(slot, index)| {
                let item = self.inventory.slot(index)?;
                (!item.is_empty()).then_some((slot, item))
            })
            .map(|(slot, item)| EquipmentSlotUpdate {
                slot,
                item: item.summary().clone(),
            })
            .collect::<Vec<_>>();
        let vehicle_passengers =
            entities
                .external_player_vehicle(self.entity_id())
                .map(|vehicle_id| SetPassengers {
                    vehicle_id,
                    passenger_ids: entities.passengers(vehicle_id).to_vec(),
                });
        let mut frame = self.tracking_frame(vehicle_passengers.is_some());
        frame.effect_particles = self.effect_particles.particles().to_vec();
        PlayerPairingData {
            metadata: player_metadata(&frame, PlayerRows::NonDefault),
            syncable_attributes: self
                .attributes
                .syncable_attributes()
                .map(attribute_snapshot)
                .collect(),
            equipment,
            vehicle_passengers,
        }
    }

    /// Current scalar state for tracker initialization, with no dirty rows.
    pub(crate) fn tracking_frame(&self, riding: bool) -> PlayerSyncFrame {
        self.sync_frame(riding, PlayerDirtyRows::default(), false, false, Vec::new())
    }

    /// `ServerEntity.sendChanges`'s read of the player, which clears every
    /// dirty bit it publishes (`ServerEntity.java:216`).
    pub(crate) fn drain_tracking_frame(&mut self, riding: bool) -> PlayerSyncFrame {
        let (effect_particles, effect_ambience) = self.effect_particles.take_dirty();
        let dirty = PlayerDirtyRows {
            shoulders: std::mem::take(&mut self.cold.shoulders.dirty),
            silent: std::mem::take(&mut self.silent_dirty),
            air_supply: std::mem::take(&mut self.air_supply_dirty),
            ticks_frozen: std::mem::take(&mut self.ticks_frozen_dirty),
            shared_flags: std::mem::take(&mut self.shared_flags_dirty),
            pose: std::mem::take(&mut self.pose_dirty),
            sleeping_pos: std::mem::take(&mut self.cold.sleeping_pos_dirty),
            living_entity_flags: self.using_item.take_living_entity_flags_dirty(),
            arrow_count: std::mem::take(&mut self.cold.arrow_count_dirty),
            stinger_count: std::mem::take(&mut self.cold.stinger_count_dirty),
            effect_particles,
            effect_ambience,
            main_hand: std::mem::take(&mut self.cold.main_hand_dirty),
            model_customization: std::mem::take(&mut self.cold.model_customization_dirty),
        };
        let needs_sync = std::mem::take(&mut self.needs_sync);
        let hurt_marked = std::mem::take(&mut self.hurt_marked);
        let dirty_attributes = self
            .attributes
            .drain_attributes_to_sync()
            .map(attribute_snapshot)
            .collect();
        self.sync_frame(riding, dirty, needs_sync, hurt_marked, dirty_attributes)
    }

    fn sync_frame(
        &self,
        riding: bool,
        dirty: PlayerDirtyRows,
        needs_sync: bool,
        hurt_marked: bool,
        dirty_attributes: Vec<AttributeSnapshot>,
    ) -> PlayerSyncFrame {
        PlayerSyncFrame {
            shoulder_parrots: self.cold.shoulders.variants(),
            shoulders_dirty: dirty.shoulders,
            silent: self.silent,
            silent_dirty: dirty.silent,
            air_supply: self.air_supply,
            air_supply_dirty: dirty.air_supply,
            ticks_frozen: self.ticks_frozen,
            ticks_frozen_dirty: dirty.ticks_frozen,
            entity_id: self.entity_id(),
            position: self.position(),
            delta_movement: self.delta_movement(),
            known_movement: self.known_movement,
            needs_sync,
            y_rot: self.rotation().y_rot,
            x_rot: self.rotation().x_rot,
            y_head_rot: self.rotation().y_rot,
            on_ground: self.on_ground(),
            riding,
            shared_flags: self.shared_flags,
            shared_flags_dirty: dirty.shared_flags,
            pose_id: self.pose_id,
            pose_dirty: dirty.pose,
            sleeping_pos: self.cold.sleeping_pos,
            sleeping_pos_dirty: dirty.sleeping_pos,
            living_entity_flags: self.using_item.living_entity_flags(),
            living_entity_flags_dirty: dirty.living_entity_flags,
            arrow_count: self.cold.arrows.count,
            arrow_count_dirty: dirty.arrow_count,
            stinger_count: self.cold.stingers.count,
            stinger_count_dirty: dirty.stinger_count,
            health: self.damage.health,
            absorption: self.damage.absorption_amount,
            effect_particles: if dirty.effect_particles {
                self.effect_particles.particles().to_vec()
            } else {
                Vec::new()
            },
            effect_particles_dirty: dirty.effect_particles,
            effect_ambience: self.effect_particles.ambience(),
            effect_ambience_dirty: dirty.effect_ambience,
            main_hand: self.cold.client_information.main_hand,
            main_hand_dirty: dirty.main_hand,
            model_customization: self.cold.client_information.displayed_skin_parts,
            model_customization_dirty: dirty.model_customization,
            dirty_attributes,
            hurt_marked,
        }
    }
}

impl ServerWorld {
    /// `ServerLevel.broadcastEntityEvent` and `broadcastDamageEvent`
    /// (`ServerLevel.java:1181-1188`) delegate to the tracked entity, which
    /// sends its `seenBy` set before the owning player (`ChunkMap.java:1347-1358`).
    pub(super) fn send_to_player_tracking_and_self(
        &self,
        connection_id: ConnectionId,
        entity_id: i32,
        packet: ClientboundPlayPacket,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        self.send_to_player_tracking_and_self_in_level(
            connection_id,
            entity_id,
            &player.dimension,
            packet,
            events,
        );
    }

    // ChunkMap.java:1230-1235: callbacks retain their level when a player changes dimension.
    pub(super) fn send_to_player_tracking_and_self_in_level(
        &self,
        connection_id: ConnectionId,
        entity_id: i32,
        dimension: &LevelId,
        packet: ClientboundPlayPacket,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.send_to_player_tracking_in_level(
            connection_id,
            entity_id,
            dimension,
            &packet,
            events,
        ) {
            events.push(send(connection_id, packet));
        }
    }

    /// `ChunkMap.TrackedEntity.sendToTrackingPlayers` for a session's own
    /// entity; false where the session has no tracked body under that id.
    pub(super) fn send_to_player_tracking(
        &self,
        target: ConnectionId,
        entity_id: i32,
        packet: &ClientboundPlayPacket,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some(player) = self.levels.player(target) else {
            return false;
        };
        self.send_to_player_tracking_in_level(
            player.connection_id,
            entity_id,
            &player.dimension,
            packet,
            events,
        )
    }

    fn send_to_player_tracking_in_level(
        &self,
        connection_id: ConnectionId,
        entity_id: i32,
        dimension: &LevelId,
        packet: &ClientboundPlayPacket,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some(_) = self
            .levels
            .player(connection_id)
            .filter(|player| player.entity_id() == entity_id)
        else {
            return false;
        };
        let Some(level) = self.level_if_present_by_id(dimension) else {
            return false;
        };
        let Some(viewers) = level
            .entity_tracker
            .player_viewers_of(connection_id, entity_id)
        else {
            return false;
        };
        crate::world::player_packets::broadcast_to(
            viewers
                .into_iter()
                .filter(|viewer| *viewer != connection_id),
            packet.clone(),
            events,
        );
        true
    }

    pub(super) fn add_player_entity_tracking_checked(
        &mut self,
        dimension: &LevelId,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let entity_id = self
            .levels
            .player(connection_id)
            .filter(|player| &player.dimension == dimension && !player.avatar_removed)?
            .entity_id();
        // `ServerEntity.sendPairingData` runs `updateDataBeforeSync` before the
        // row a new viewer pairs from (`ServerEntity.java:269-270`).
        self.refresh_level_player_dirty_effects(dimension);
        let viewers = self.tracking_viewers(dimension);
        let Self {
            sessions, levels, ..
        } = self;
        let sessions = &*sessions;
        let level = levels.get_mut(dimension)?;
        let mut packets = Vec::new();
        level.entity_tracker.add_player(
            entity_id,
            &viewers,
            &level.entities,
            |viewer, chunk_x, chunk_z| is_chunk_tracked(sessions, viewer, (chunk_x, chunk_z)),
            &mut packets,
        );
        Self::publish_tracking_packets(&level.entities, packets, events);
        let ignored = self.player_loading_ticket_ignored(connection_id);
        let player = self.levels.player_at(connection_id);
        let entity_id = player.entity_id();
        let pos = block_position(player.position());
        self.level_mut_by_id(dimension).player_chunk_membership.add(
            entity_id,
            (pos.x >> 4, pos.z >> 4),
            ignored,
        );
        Some(())
    }

    #[cfg(test)]
    pub(super) fn move_player_entity_tracking(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.move_session_entity_tracking(connection_id, events);
    }

    pub(super) fn move_session_entity_tracking(
        &mut self,
        target: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let connection_id = target;
        let dimension = self.levels.player(connection_id)?.dimension;
        self.update_player_query_section(connection_id);
        self.refresh_level_player_dirty_effects(&dimension);
        let viewers = self.tracking_viewers(&dimension);
        let mover = self.tracking_viewer(connection_id)?;
        let Self {
            sessions, levels, ..
        } = self;
        let sessions = &*sessions;
        let level = levels.get_mut(&dimension)?;
        let mut packets = Vec::new();
        level.entity_tracker.move_player_body(
            &mover,
            &viewers,
            &level.entities,
            |viewer, chunk_x, chunk_z| is_chunk_tracked(sessions, viewer, (chunk_x, chunk_z)),
            &mut packets,
        );
        Self::publish_tracking_packets(&level.entities, packets, events);
        Some(())
    }

    pub(super) fn remove_player_entity_tracking(
        &mut self,
        dimension: &LevelId,
        connection_id: ConnectionId,
        entity_id: i32,
        send_to_leaver: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(level) = self.level_mut_if_present_by_id(dimension) else {
            return;
        };
        let mut packets = Vec::new();
        level
            .entity_tracker
            .remove_player(connection_id, entity_id, &mut packets);
        for mut packet in packets {
            if !send_to_leaver {
                packet
                    .recipients
                    .retain(|recipient| *recipient != connection_id);
            }
            crate::world::player_packets::broadcast_to(packet.recipients, packet.packet, events);
        }
        if let Some(player) = self.levels.player(connection_id) {
            let (x, _, z) = player.cold.last_section_pos;
            self.level_mut_by_id(dimension)
                .player_chunk_membership
                .remove(entity_id, (x, z));
        }
    }

    pub(super) fn player_loading_ticket_ignored(&self, connection_id: ConnectionId) -> bool {
        self.levels.player(connection_id).is_none_or(|player| {
            player.game_type == GameType::Spectator
                && !matches!(
                    self.admin
                        .game_rule_value(super::RULE_SPECTATORS_GENERATE_CHUNKS),
                    Some(bbb_protocol::game_rules::GameRuleValue::Boolean(true))
                )
        })
    }

    /// `ChunkMap.move` updates entity admission before its chunk view
    /// (`ChunkMap.java:1065-1099`), including moves within one section.
    #[cfg(test)]
    pub(super) fn move_player_chunk_tracking(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.move_session_chunk_tracking(connection_id, events);
    }

    pub(super) fn move_session_chunk_tracking(
        &mut self,
        target: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        self.move_session_entity_tracking(target, events)?;
        let player = self.levels.player(target)?;
        let entity_id = player.entity_id();
        let dimension = player.dimension;
        let old = player.cold.last_section_pos;
        let pos = block_position(player.position());
        let section = (pos.x >> 4, pos.y >> 4, pos.z >> 4);
        let ignored = player.game_type == GameType::Spectator
            && !matches!(
                self.admin
                    .game_rule_value(super::RULE_SPECTATORS_GENERATE_CHUNKS),
                Some(bbb_protocol::game_rules::GameRuleValue::Boolean(true)),
            );
        let moved = self
            .level_mut_by_id(&dimension)
            .player_chunk_membership
            .move_player(entity_id, old, section, ignored)?;
        if !moved {
            return Some(());
        }
        let player = self.levels.player_mut(target)?;
        player.cold.last_section_pos = section;
        player.cold.filed_ticket_ignored = ignored;
        self.update_session_chunk_tracking(target, events)
    }

    pub(super) fn tick_level_player_chunk_tracking(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let players = self.level_by_id(dimension).player_order.clone();
        for connection_id in players {
            if self
                .levels
                .player(connection_id)
                .is_none_or(|player| player.avatar_removed)
            {
                continue;
            }
            self.update_chunk_tracking(connection_id, events);
        }
    }

    #[cfg(test)]
    pub(crate) fn player_tracking_viewers_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) -> Vec<ConnectionId> {
        self.level(dimension).entity_tracker.viewers_of(entity_id)
    }

    #[cfg(test)]
    pub(crate) fn reconcile_player_tracking_for_test(&mut self, connection_id: ConnectionId) {
        self.move_player_entity_tracking(connection_id, &mut Vec::new());
    }

    #[cfg(test)]
    pub(crate) fn set_player_shared_flag_for_test(
        &mut self,
        connection_id: ConnectionId,
        flag: i8,
        value: bool,
    ) {
        self.levels[connection_id].set_shared_flag(flag, value);
    }
}

impl ServerWorld {
    pub(super) fn update_player_query_section(&mut self, target: ConnectionId) {
        let Some(player) = self
            .levels
            .player(target)
            .filter(|player| !player.avatar_removed)
        else {
            return;
        };
        let (dimension, entity_id) = (player.dimension, player.entity_id());
        self.level_mut_by_id(&dimension)
            .entities
            .refile_player(entity_id);
    }
}

#[cfg(test)]
mod tests;
