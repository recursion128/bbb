//! `ChunkMap`'s entity half for one level: who is watching, and the ordered
//! `addEntity`/`removeEntity` journal the store observed.

use super::map_frames::{map_carrier_facts, map_frame_direction};
use super::*;
use crate::entity::tracking::TrackingPacket;
use crate::world::LevelId;

impl ServerWorld {
    pub(in crate::world) fn publish_tracking_packets(
        entities: &crate::entity::EntityStore,
        mut packets: Vec<TrackingPacket>,
        events: &mut Vec<WorldEvent>,
    ) {
        for index in 0..packets.len() {
            let ClientboundPlayPacket::AddEntity(entity) = &packets[index].packet else {
                continue;
            };
            if entities.player(entity.id).is_some() {
                continue;
            }
            let Some(vehicle_id) = entities.valid_vehicle(entity.id) else {
                continue;
            };
            // ServerEntity.java:307-309: pairing a rider also publishes its parent's relation.
            // The original batch must introduce every rider before clients can apply it.
            packets.push(TrackingPacket {
                recipients: packets[index].recipients.clone(),
                packet: ClientboundPlayPacket::SetPassengers(
                    bbb_protocol::packets::SetPassengers {
                        vehicle_id,
                        passenger_ids: entities.passengers(vehicle_id).to_vec(),
                    },
                ),
            });
        }
        for packet in packets {
            crate::world::player_packets::broadcast_to(packet.recipients, packet.packet, events);
        }
    }

    /// The sessions `ChunkMap.tick` hands its trackers, which is
    /// `this.level.players()` (`ChunkMap.java:1186`): one `ChunkMap` belongs to
    /// one `ServerLevel`, so a session watching another dimension is never a
    /// candidate.
    ///
    /// `ServerLevel.EntityCallbacks.onTrackingEnd` removes a dead avatar from
    /// this list (`ServerLevel.java:1945-1949`).
    pub(in crate::world) fn tracking_viewers(&self, dimension: &LevelId) -> Vec<TrackingViewer> {
        self.level_by_id(dimension)
            .player_order
            .iter()
            .filter(|connection_id| {
                self.levels
                    .player(**connection_id)
                    .is_some_and(|player| !player.avatar_removed)
            })
            .filter_map(|connection_id| self.tracking_viewer(*connection_id))
            .collect()
    }

    /// One session as a viewer, whether or not its avatar is still present.
    pub(in crate::world) fn tracking_viewer(
        &self,
        connection_id: ConnectionId,
    ) -> Option<TrackingViewer> {
        let player = self.levels.player(connection_id)?;
        Some(TrackingViewer {
            connection_id,
            position: player.position(),
            view_distance: self.player_view_distance(connection_id),
            uuid: player.profile.uuid,
            entity_id: player.entity_id(),
        })
    }

    pub(in crate::world) fn pair_delivered_entity_chunks(
        &mut self,
        dimension: &LevelId,
        connection_id: ConnectionId,
        chunks: &[(i32, i32)],
        events: &mut Vec<WorldEvent>,
    ) {
        let viewers = self.tracking_viewers(dimension);
        let Some(viewer) = viewers
            .iter()
            .find(|viewer| viewer.connection_id == connection_id)
        else {
            return;
        };
        let Self {
            sessions, levels, ..
        } = self;
        let Some(level) = levels.get_mut(dimension) else {
            return;
        };
        let mut packets = Vec::new();
        level.entity_tracker.pair_delivered_chunks(
            chunks,
            viewer,
            &viewers,
            &level.entities,
            |connection_id, chunk_x, chunk_z| {
                is_chunk_tracked(sessions, connection_id, (chunk_x, chunk_z))
            },
            &mut packets,
        );
        Self::publish_tracking_packets(&level.entities, packets, events);
    }

    /// `ChunkMap.tick()`'s entity half (`ChunkMap.java:1180-1212`) for one
    /// level, with the packets it produced turned into events.
    pub(super) fn tick_level_entity_tracking(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        self.tick_level_player_chunk_tracking(dimension, events);
        // `ServerEntity.sendChanges` runs `updateDataBeforeSync` before it packs
        // a byte (`ServerEntity.java:90-91`).
        self.refresh_level_player_dirty_effects(dimension);
        let viewers = self.tracking_viewers(dimension);
        let needs_carriers = {
            let level = self.level_by_id(dimension);
            !level.player_order.is_empty()
                && level.entity_tracker.any_framed_map_due(&level.entities)
        };
        let carriers = if needs_carriers {
            let resolver = RuntimePersistentRegistryResolver {
                datapacks: self.cold.config.datapacks.as_deref(),
                content_registry: self.registries.content_registry(),
            };
            self.levels
                .player_entries()
                .map(|(&id, player)| {
                    map_carrier_facts(
                        id,
                        player,
                        self.levels.names.key(player.dimension).as_str(),
                        &resolver,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let broadcast_range = self.admin.settings().entity_broadcast_range_percentage;
        let minecart_improvements = self.feature_flags.minecart_improvements;
        let Self {
            game_time,
            maps,
            sessions,
            levels,
            ..
        } = self;
        let sessions = &*sessions;
        let Some(level) = levels.get_mut(dimension) else {
            return;
        };
        let ServerLevel {
            entities,
            entity_tracker,
            simulation_levels,
            player_order,
            ..
        } = level;
        let level_players = &*player_order;
        entity_tracker.set_range_percentage(broadcast_range, entities);
        entity_tracker.set_minecart_improvements(minecart_improvements);
        let mut packets = Vec::new();
        entity_tracker.tick_with_players_and_framed_maps(
            entities,
            &viewers,
            |connection_id, chunk_x, chunk_z| {
                is_chunk_tracked(sessions, connection_id, (chunk_x, chunk_z))
            },
            |chunk_x, chunk_z| simulation_levels.is_entity_ticking((chunk_x, chunk_z)),
            |frame| {
                let mut live_carriers = Vec::new();
                for facts in &carriers {
                    match facts {
                        Ok((carrier, held)) => {
                            live_carriers.push(carrier.holding(held, frame.map_id))
                        }
                        Err(error) => {
                            tracing::warn!("framed map carrier: {error}");
                            return Vec::new();
                        }
                    }
                }
                let placed = ServerMapPlacedFrame {
                    entity_id: frame.entity_id,
                    pos: frame.attached,
                    direction: map_frame_direction(frame.direction),
                };
                let mut direct = Vec::new();
                for &connection_id in level_players {
                    if live_carriers
                        .iter()
                        .find(|carrier| carrier.connection_id == connection_id)
                        .is_none_or(|carrier| carrier.removed)
                    {
                        continue;
                    }
                    let carrier = live_carriers
                        .iter()
                        .find(|carrier| carrier.connection_id == connection_id)
                        .expect("a level player is present in the live carrier snapshot")
                        .clone();
                    if let Err(error) = maps.tick_carried_by(
                        frame.map_id,
                        carrier,
                        &live_carriers,
                        Some(placed),
                        &frame.static_decorations,
                        *game_time,
                    ) {
                        tracing::warn!(
                            event = "world.framed_map_tick_failed",
                            map_id = frame.map_id,
                            ?error,
                        );
                        continue;
                    }
                    if let Some(packet) = maps.next_update_packet(frame.map_id, connection_id) {
                        direct.push((connection_id, ClientboundPlayPacket::MapItemData(packet)));
                    }
                }
                direct
            },
            &mut packets,
        );
        Self::publish_tracking_packets(entities, packets, events);
    }

    /// Applies `ChunkMap.addEntity` / `removeEntity` in the order the level's
    /// store observed them; see [`Self::tick_level_entity_tracking`] for why
    /// the two halves of `ChunkMap` run at different points of the tick.
    pub(in crate::world) fn apply_level_entity_lifecycle_by_key(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.run_entity_commands(dimension, events);
        // The nearest point in every path that reaches the flush and owns the
        // event sink the criterion's packets are written to.
        self.apply_pending_kill_credits(dimension, events);
        // The listener registry reads the same journal the tracker drains.
        self.sync_entity_game_event_listeners(dimension);
        let lifecycle_events = self
            .level_mut_by_id(dimension)
            .entities
            .drain_lifecycle_events();
        if lifecycle_events.is_empty() {
            return;
        }
        let viewers = self.tracking_viewers(dimension);
        // `Player.removeVehicle` resets the cooldown after the relation edge;
        // the tracker below consumes that same ordered vector.
        for event in &lifecycle_events {
            let crate::entity::EntityLifecycleEvent::ExternalPlayerBoardingCooldownReset {
                player_entity_id,
                dismount,
            } = event
            else {
                continue;
            };
            if let Some(player) = self
                .levels
                .players_mut()
                .find(|player| player.entity_id() == *player_entity_id)
            {
                player.boarding_cooldown = 0;
            }
            if let Some(dismount) = dismount {
                let _ = self.finish_external_player_ejection(
                    dimension,
                    *player_entity_id,
                    *dismount,
                    events,
                );
            }
        }
        self.update_boss_bar_entity_index(dimension, &lifecycle_events);
        self.apply_waypoint_entity_lifecycle(dimension, &lifecycle_events, events);
        // `ServerLevel.EntityCallbacks.onDestroyed` (`ServerLevel.java:1894-1899`)
        // into `Scoreboard.entityRemoved` (`Scoreboard.java:342-347`).
        for event in &lifecycle_events {
            if let crate::entity::EntityLifecycleEvent::Removed { entity, .. } = event {
                if entity
                    .removal_reason()
                    .is_some_and(|reason| reason.should_destroy())
                    && !matches!(entity.data(), crate::entity::EntityData::Player)
                {
                    self.scoreboard
                        .queue_entity_removed(entity.uuid().to_string());
                }
            }
        }
        let destroyed_crystals: Vec<_> = lifecycle_events
            .iter()
            .filter_map(|event| match event {
                crate::entity::EntityLifecycleEvent::Removed { entity, .. }
                    if matches!(entity.data(), crate::entity::EntityData::EndCrystal(_))
                        && entity.removal_reason()
                            == Some(crate::entity::RemovalReason::Killed) =>
                {
                    Some((
                        entity.id(),
                        entity.uuid(),
                        entity.position(),
                        crate::entity::end_crystal::destroyed_by(entity),
                    ))
                }
                _ => None,
            })
            .collect();
        let Self {
            sessions, levels, ..
        } = self;
        let sessions = &*sessions;
        let Some(level) = levels.get_mut(dimension) else {
            return;
        };
        let ServerLevel { entity_tracker, .. } = level;
        let mut packets = Vec::new();
        entity_tracker.apply_lifecycle_events(
            &lifecycle_events,
            &viewers,
            |connection_id, chunk_x, chunk_z| {
                is_chunk_tracked(sessions, connection_id, (chunk_x, chunk_z))
            },
            &mut packets,
        );
        Self::publish_tracking_packets(&level.entities, packets, events);
        self.dragon_fight_crystals_destroyed(dimension, destroyed_crystals, events);
    }

    /// Publishes every level's ordered entity journal at one server-owner
    /// boundary, preserving the same level order as `tickChildren`.
    pub(in crate::world) fn apply_all_level_entity_lifecycle(
        &mut self,
        events: &mut Vec<WorldEvent>,
    ) {
        let mut previous = None;
        while let Some(dimension) = self.levels.next_key(previous) {
            self.apply_level_entity_lifecycle_by_key(&dimension, events);
            previous = Some(dimension);
        }
    }

    #[cfg(test)]
    pub(crate) fn apply_level_entity_lifecycle_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        events: &mut Vec<WorldEvent>,
    ) {
        self.apply_level_entity_lifecycle_by_key(&dimension.into(), events);
    }
}
