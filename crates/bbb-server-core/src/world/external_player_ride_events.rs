//! Ride events and world-collision dismounts.

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::level_event::VanillaGameEvent;
use crate::world::LevelId;
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID;
use bbb_protocol::packets::{
    PlayerPositionUpdate, Vec3d, PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y,
    PLAYER_RELATIVE_DELTA_Z, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y_ROT,
};

use super::dismount_search::{DismountLevel, DismountRider};
use super::{send, ServerWorld, WorldEvent};

/// A detached session rider and the vehicle position to use if its body was removed.
#[derive(Clone, Copy)]
pub(in crate::world) struct ExternalPlayerRide {
    connection_id: ConnectionId,
    vehicle_entity_id: i32,
    vehicle_position: Vec3d,
}

/// Whether detach runs `LivingEntity.dismountVehicle`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::world) enum DismountLanding {
    /// `LivingEntity.stopRiding` (`LivingEntity.java:3217-3223`).
    Search,
    /// Direct `removeVehicle` or an avatar already removed (`LivingEntity.java:2303`).
    Stay,
}

impl ServerWorld {
    /// `Entity.startRiding` fires for every indirect passenger after attach
    /// (`Entity.java:2380-2385`, `:3453-3455`).
    pub(in crate::world) fn raise_external_player_mount(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let dimension = player.dimension;
        let player_entity_id = player.entity_id();
        let entities = &self.level_by_id(&dimension).entities;
        let Some(vehicle_entity_id) = entities.external_player_vehicle(player_entity_id) else {
            return;
        };
        let Some(vehicle_position) = entities
            .get(vehicle_entity_id)
            .map(crate::entity::Entity::position)
        else {
            return;
        };
        let riders = self.sessions_riding(&dimension, vehicle_entity_id);
        self.levels[connection_id].set_pose(VANILLA_ENTITY_POSE_STANDING_ID);
        self.player_game_event_at(
            connection_id,
            VanillaGameEvent::ENTITY_MOUNT,
            vehicle_position,
            None,
        );
        for rider in riders {
            self.fire_player_trigger(rider, "minecraft:started_riding", events);
        }
    }

    /// Detach outside an entity tick; ejections inside the borrowed store use
    /// the pre-tick snapshot path below.
    pub(in crate::world) fn stop_external_player_ride(
        &mut self,
        dimension: &LevelId,
        connection_id: ConnectionId,
        landing: DismountLanding,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        self.stop_external_player_ride_checked(dimension, connection_id, landing, events)
    }

    pub(in crate::world) fn stop_external_player_ride_checked(
        &mut self,
        dimension: &LevelId,
        connection_id: ConnectionId,
        landing: DismountLanding,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        let Some((player_entity_id, player_position)) = self
            .levels
            .player(connection_id)
            .map(|player| (player.entity_id(), player.position()))
        else {
            return Some(false);
        };
        let entities = &self.level_by_id(dimension).entities;
        let Some(vehicle_entity_id) = entities.external_player_vehicle(player_entity_id) else {
            return Some(false);
        };
        let vehicle_position = entities
            .get(vehicle_entity_id)
            .map_or(player_position, crate::entity::Entity::position);
        let Some(detached_vehicle_id) = self
            .level_mut_by_id(dimension)
            .entities
            .detach_external_player(player_entity_id)
        else {
            return Some(false);
        };
        debug_assert_eq!(detached_vehicle_id, vehicle_entity_id);
        self.finish_external_player_dismount(
            dimension,
            ExternalPlayerRide {
                connection_id,
                vehicle_entity_id,
                vehicle_position,
            },
            landing,
            events,
        )?;
        Some(true)
    }

    /// Preserve the sole-root unload decision after the save, while the
    /// disconnecting session still exists (`PlayerList.java:302-312`).
    pub(in crate::world) fn unload_external_player_ride(
        &mut self,
        dimension: &LevelId,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some((player_entity_id, player_position)) = self
            .levels
            .player(connection_id)
            .map(|player| (player.entity_id(), player.position()))
        else {
            return;
        };
        let entities = &self.level_by_id(dimension).entities;
        let ride = entities
            .external_player_vehicle(player_entity_id)
            .map(|vehicle_entity_id| ExternalPlayerRide {
                connection_id,
                vehicle_entity_id,
                vehicle_position: entities
                    .get(vehicle_entity_id)
                    .map_or(player_position, crate::entity::Entity::position),
            });
        let unloaded = self
            .level_mut_by_id(dimension)
            .entities
            .unload_external_player_vehicle(player_entity_id);
        if let (Some(ride), Some(_)) = (ride, unloaded) {
            // The disconnected session has no consumer for the landing packet.
            self.finish_external_player_dismount(dimension, ride, DismountLanding::Stay, events)
                .expect("Stay landing does not move player tracking");
        }
    }

    pub(in crate::world) fn finish_external_player_ejection(
        &mut self,
        dimension: &LevelId,
        player_entity_id: i32,
        dismount: crate::entity::store::ExternalPlayerDismount,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let Some(connection_id) =
            self.levels
                .player_entries()
                .find_map(|(&connection_id, player)| {
                    (&player.dimension == dimension
                        && player.entity_id() == player_entity_id
                        && !player.avatar_removed)
                        .then_some(connection_id)
                })
        else {
            return Some(());
        };
        self.finish_external_player_dismount(
            dimension,
            ExternalPlayerRide {
                connection_id,
                vehicle_entity_id: dismount.vehicle_id,
                vehicle_position: dismount.vehicle_position,
            },
            DismountLanding::Search,
            events,
        )
    }

    /// `Entity.removeVehicle` raises before the optional landing teleport
    /// (`Entity.java:2403-2412`; `LivingEntity.java:3217-3223`).
    fn finish_external_player_dismount(
        &mut self,
        dimension: &LevelId,
        ride: ExternalPlayerRide,
        landing: DismountLanding,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        if !self.levels.contains_player(ride.connection_id) {
            return Some(());
        }
        let vehicle_position = ride.vehicle_position;
        self.player_game_event_at(
            ride.connection_id,
            VanillaGameEvent::ENTITY_DISMOUNT,
            vehicle_position,
            None,
        );
        // ServerPlayer.java:2137-2148: removeVehicle sends to its connection before landing.
        if let Some(living) = self
            .level_by_id(dimension)
            .entities
            .get(ride.vehicle_entity_id)
            .and_then(crate::entity::Entity::living)
        {
            for effect in living.active_effects() {
                events.push(send(
                    ride.connection_id,
                    ClientboundPlayPacket::RemoveMobEffect(
                        crate::entity::living::mob_effects::client_update::remove_mob_effect_packet(
                            ride.vehicle_entity_id,
                            effect,
                        ),
                    ),
                ));
            }
        }
        events.push(send(
            ride.connection_id,
            ClientboundPlayPacket::SetPassengers(bbb_protocol::packets::SetPassengers {
                vehicle_id: ride.vehicle_entity_id,
                passenger_ids: self
                    .level_by_id(dimension)
                    .entities
                    .passengers(ride.vehicle_entity_id)
                    .to_vec(),
            }),
        ));
        if landing == DismountLanding::Stay {
            return Some(());
        }
        let (target, pose_id) = self.external_player_dismount_target(dimension, ride);
        self.levels[ride.connection_id].set_pose(pose_id);
        self.dismount_external_player_to(ride.connection_id, target, events)
    }

    /// `LivingEntity.dismountVehicle` (`LivingEntity.java:2301-2323`).
    fn external_player_dismount_target(
        &self,
        dimension: &LevelId,
        ride: ExternalPlayerRide,
    ) -> (Vec3d, i32) {
        let player = self.levels.player_at(ride.connection_id);
        if player.avatar_removed {
            return (player.position(), player.pose_id);
        }
        self.dismount_level(dimension).living_dismount_target(
            self.level_by_id(dimension)
                .entities
                .get(ride.vehicle_entity_id),
            ride.vehicle_position,
            DismountRider::Player(player),
        )
    }

    #[cfg(test)]
    pub(crate) fn external_player_dismount_target_for_test(
        &self,
        connection_id: ConnectionId,
        vehicle_entity_id: i32,
        vehicle_position_before_tick: Vec3d,
    ) -> Vec3d {
        let dimension = &self.levels[connection_id].dimension;
        self.external_player_dismount_target(
            dimension,
            ExternalPlayerRide {
                connection_id,
                vehicle_entity_id,
                vehicle_position: vehicle_position_before_tick,
            },
        )
        .0
    }

    fn dismount_level(&self, dimension: &LevelId) -> DismountLevel<'_> {
        let level = self.level_by_id(dimension);
        DismountLevel {
            chunks: &level.chunks,
            border: level.world_border,
            game_time: self.game_time(),
        }
    }

    pub(in crate::world) fn stop_stored_command_ride(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
    ) -> bool {
        let store = &self.level_by_id(dimension).entities;
        let Some(vehicle_id) = store.vehicle(entity_id) else {
            return false;
        };
        let Some(entity) = store.get(entity_id) else {
            return false;
        };
        let vehicle_position = store
            .get(vehicle_id)
            .map_or(entity.position(), crate::entity::Entity::position);
        let landing = entity.living().is_some().then(|| {
            self.dismount_level(dimension).living_dismount_target(
                store.get(vehicle_id),
                vehicle_position,
                DismountRider::Stored(entity),
            )
        });
        let level = self.level_mut_by_id(dimension);
        if !level.entities.stop_stored_riding(entity_id) {
            return false;
        }
        level.event_sink.game_event_with_source_at(
            VanillaGameEvent::ENTITY_DISMOUNT,
            vehicle_position,
            Some(entity_id),
            None,
        );
        if let Some((position, pose)) = landing {
            if let Some(entity) = level.entities.get_mut(entity_id) {
                entity.set_pose_id(pose);
                entity.set_pos(position);
            }
            level.entities.refile(entity_id);
        }
        true
    }

    /// `ServerPlayer.teleportTo` preserves relative delta and rotation
    /// (`Entity.java:3260-3262`; `ServerPlayer.java:1669-1671`).
    fn dismount_external_player_to(
        &mut self,
        connection_id: ConnectionId,
        target: Vec3d,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let teleport_id = self.next_teleport_id;
        self.next_teleport_id = self.next_teleport_id.wrapping_add(1);
        let now = self.tick;
        let Some(player) = self.levels.player_mut(connection_id) else {
            return Some(());
        };
        let moved = player.position() != target;
        player.set_position(target);
        player.set_on_ground(false);
        player.known_movement = Vec3d::default();
        player.received_movement_this_tick = false;
        player.awaiting_teleport = Some(teleport_id);
        player.awaiting_teleport_tick = now;
        events.push(send(
            connection_id,
            ClientboundPlayPacket::PlayerPosition(PlayerPositionUpdate {
                id: teleport_id,
                position: target,
                delta_movement: Vec3d::default(),
                y_rot: 0.0,
                x_rot: 0.0,
                relatives_mask: PLAYER_RELATIVE_DELTA_X
                    | PLAYER_RELATIVE_DELTA_Y
                    | PLAYER_RELATIVE_DELTA_Z
                    | PLAYER_RELATIVE_Y_ROT
                    | PLAYER_RELATIVE_X_ROT,
            }),
        ));
        if moved {
            self.waypoint_update_player_source(connection_id, events);
            self.waypoint_update_player_receiver(connection_id, events);
        }
        self.move_session_chunk_tracking(connection_id, events)
    }

    /// `Entity.getIndirectPassengersStream` is DFS in passenger-list order
    /// (`Entity.java:3453-3455`).
    pub(in crate::world) fn sessions_riding(
        &self,
        dimension: &LevelId,
        vehicle_entity_id: i32,
    ) -> Vec<ConnectionId> {
        let entities = &self.level_by_id(dimension).entities;
        let sessions = self
            .levels
            .player_entries()
            .filter(|(_, player)| &player.dimension == dimension && !player.avatar_removed)
            .map(|(&connection_id, player)| (player.entity_id(), connection_id))
            .collect::<std::collections::BTreeMap<_, _>>();
        ordered_riding_sessions(
            vehicle_entity_id,
            |entity_id| entities.passengers(entity_id),
            |entity_id| sessions.get(&entity_id).copied(),
        )
    }
}

fn ordered_riding_sessions<'a>(
    vehicle_entity_id: i32,
    passengers: impl Fn(i32) -> &'a [i32],
    session: impl Fn(i32) -> Option<ConnectionId>,
) -> Vec<ConnectionId> {
    fn visit<'a>(
        parent: i32,
        passengers: &impl Fn(i32) -> &'a [i32],
        session: &impl Fn(i32) -> Option<ConnectionId>,
        seen: &mut std::collections::BTreeSet<i32>,
        result: &mut Vec<ConnectionId>,
    ) {
        for &passenger in passengers(parent) {
            if !seen.insert(passenger) {
                continue;
            }
            if let Some(connection_id) = session(passenger) {
                result.push(connection_id);
            }
            visit(passenger, passengers, session, seen, result);
        }
    }

    let mut seen = std::collections::BTreeSet::from([vehicle_entity_id]);
    let mut result = Vec::new();
    visit(
        vehicle_entity_id,
        &passengers,
        &session,
        &mut seen,
        &mut result,
    );
    result
}
