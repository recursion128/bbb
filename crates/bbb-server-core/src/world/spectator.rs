//! Session entity selection and the single spectator camera owner.

use super::*;
use crate::commands::{CommandCameraTarget, FetchProfileResult, FetchProfileTarget};
use crate::world::LevelId;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::packets::{PickItemFromEntity, SetCamera, SystemChat};
use bbb_server_types::geometry::Aabb;
use bbb_server_types::interaction::player_pose_bounding_box;

struct CameraEntity {
    target: CommandCameraTarget,
    dimension: LevelId,
    entity_id: i32,
    position: Vec3d,
    rotation: PlayerRotation,
    bounds: Aabb,
    alive: bool,
    pickable: bool,
}

impl ServerWorld {
    fn camera_entity(&self, target: &CommandCameraTarget) -> Option<CameraEntity> {
        match target {
            CommandCameraTarget::Player { uuid } => {
                let player = self.levels.player(self.player_reference(*uuid)?)?;
                Some(CameraEntity {
                    target: target.clone(),
                    dimension: player.dimension,
                    entity_id: player.entity_id(),
                    position: player.position(),
                    rotation: player.rotation(),
                    bounds: player_pose_bounding_box(
                        player.position(),
                        player.pose_id,
                        player.pose_scale(),
                    )
                    .ok()?,
                    alive: !player.avatar_removed && !player.damage.is_dead_or_dying(),
                    pickable: player.game_type != GameType::Spectator,
                })
            }
            CommandCameraTarget::Stored {
                dimension,
                entity_id,
                uuid,
            } => {
                let entity = self
                    .levels
                    .get(&self.level_id(dimension)?)?
                    .entities
                    .get(*entity_id)?;
                if entity.uuid() != *uuid || entity.removal_reason().is_some() {
                    return None;
                }
                let position = entity.position();
                Some(CameraEntity {
                    target: target.clone(),
                    dimension: self.require_level_id(dimension),
                    entity_id: *entity_id,
                    position: Vec3d {
                        x: position.x,
                        y: position.y,
                        z: position.z,
                    },
                    rotation: PlayerRotation {
                        y_rot: entity.y_rot(),
                        x_rot: entity.x_rot(),
                    },
                    bounds: entity.bounding_box(),
                    alive: entity.is_alive()
                        && entity.living().is_none_or(|living| living.health() > 0.0),
                    pickable: entity.is_pickable(),
                })
            }
        }
    }

    fn camera_entity_by_id(&self, dimension: &LevelId, entity_id: i32) -> Option<CameraEntity> {
        let target = if let Some(player) = self
            .levels
            .players()
            .find(|player| player.entity_id() == entity_id && player.dimension == *dimension)
        {
            CommandCameraTarget::Player {
                uuid: player.profile.uuid,
            }
        } else {
            let entity = self.levels.get(dimension)?.entities.get(entity_id)?;
            CommandCameraTarget::Stored {
                dimension: self.level_key(*dimension).clone(),
                entity_id,
                uuid: entity.uuid(),
            }
        };
        self.camera_entity(&target)
    }

    /// `Player.isWithinEntityInteractionRange` (Player.java:1977-1984).
    fn player_reaches_camera_entity(
        &self,
        connection_id: ConnectionId,
        target: &CameraEntity,
    ) -> bool {
        let player = self.levels.player_at(connection_id);
        let Some(range) = player
            .attributes
            .value(VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID)
        else {
            return false;
        };
        let Ok(eye) = player.eye_position() else {
            return false;
        };
        target.bounds.distance_to_sqr(eye) < (range + 3.0).powi(2)
    }

    /// `ServerGamePacketListenerImpl.handleSpectateEntity` (ServerGamePacketListenerImpl.java:1843-1856).
    pub(super) fn apply_spectate_entity(
        &mut self,
        connection_id: ConnectionId,
        entity_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self.levels.player_at(connection_id);
        if !player.client_loaded() || player.game_type != GameType::Spectator {
            return;
        }
        let dimension = player.dimension;
        self.levels[connection_id].cold.last_action_millis =
            super::idle_timeout::monotonic_millis();
        let Some(target) = self.camera_entity_by_id(&dimension, entity_id) else {
            return;
        };
        if !self
            .level_by_id(&target.dimension)
            .world_border
            .contains_block_at(block_position(target.position), self.game_time())
            || !self.player_reaches_camera_entity(connection_id, &target)
            || !target.pickable
        {
            return;
        }
        self.set_player_camera(connection_id, Some(target.target), events);
    }

    /// `ServerGamePacketListenerImpl.handleTeleportToEntityPacket` (ServerGamePacketListenerImpl.java:1423-1433).
    pub(super) fn apply_teleport_to_entity(
        &mut self,
        connection_id: ConnectionId,
        uuid: Uuid,
        events: &mut Vec<WorldEvent>,
    ) {
        let player = self.levels.player_at(connection_id);
        if player.game_type != GameType::Spectator {
            return;
        }
        let target = if self.player_reference(uuid).is_some() {
            CommandCameraTarget::Player { uuid }
        } else {
            let Some((dimension, entity_id)) = self.entity_reference(&player.dimension, uuid)
            else {
                return;
            };
            CommandCameraTarget::Stored {
                dimension: self.level_key(dimension).clone(),
                entity_id,
                uuid,
            }
        };
        let Some(target) = self.camera_entity(&target) else {
            return;
        };
        self.set_player_camera(connection_id, None, events);
        self.teleport_player_id(
            connection_id,
            TeleportTransition {
                new_level: target.dimension,
                position: target.position,
                y_rot: target.rotation.y_rot,
                x_rot: target.rotation.x_rot,
                relatives_mask: 0,
            },
            events,
        );
    }

    pub(crate) fn set_camera_by_command(
        &mut self,
        player: &crate::commands::CommandPlayerTarget,
        target: Option<&CommandCameraTarget>,
        events: &mut Vec<WorldEvent>,
    ) {
        if let Some(connection_id) = self.command_player_connection(player) {
            self.set_player_camera(connection_id, target.cloned(), events);
        }
    }

    /// `ServerPlayer.setCamera` (ServerPlayer.java:1916-1930).
    pub(super) fn set_player_camera(
        &mut self,
        player: ConnectionId,
        target: Option<CommandCameraTarget>,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.set_player_camera_checked(player, target, events);
    }
    fn set_player_camera_checked(
        &mut self,
        player: ConnectionId,
        target: Option<CommandCameraTarget>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let move_chunks = target.is_some();
        let connection_id = player;
        let receiver = self.levels.player(connection_id)?;
        let own = CommandCameraTarget::Player {
            uuid: receiver.profile.uuid,
        };
        let target = target.filter(|target| *target != own);
        if receiver.cold.camera == target {
            return Some(());
        }
        let entity = self.camera_entity(target.as_ref().unwrap_or(&own))?;
        let rotation = receiver.rotation();
        self.levels.player_mut(connection_id)?.cold.camera = target;
        self.teleport_player_checked(
            connection_id,
            TeleportTransition {
                new_level: entity.dimension,
                position: entity.position,
                y_rot: rotation.y_rot,
                x_rot: rotation.x_rot,
                relatives_mask: 0,
            },
            events,
        )?;
        if move_chunks {
            self.move_session_chunk_tracking(connection_id, events)?;
        }
        events.push(send(
            connection_id,
            ClientboundPlayPacket::SetCamera(SetCamera {
                camera_id: entity.entity_id,
            }),
        ));
        // ServerGamePacketListenerImpl.java:371-378: resetPosition reads the listener's current player.
        if let Some(current) = self.levels.player_mut(connection_id) {
            current.first_good = current.position();
            current.last_good = current.position();
        }
        Some(())
    }

    /// `Entity.teleportSpectators` (Entity.java:3038-3045).
    pub(super) fn teleport_camera_watchers(
        &mut self,
        old_level: &LevelId,
        target: &CommandCameraTarget,
        transition: &TeleportTransition<LevelId>,
        events: &mut Vec<WorldEvent>,
    ) {
        let watchers: Vec<_> = self
            .levels
            .player_entries()
            .filter(|(_, player)| {
                &player.dimension == old_level && player.cold.camera.as_ref() == Some(target)
            })
            .map(|(id, _)| *id)
            .collect();
        for connection_id in watchers {
            self.teleport_player_id(connection_id, transition.clone(), events);
            self.set_player_camera(connection_id, None, events);
        }
    }

    pub(super) fn flush_camera_transitions(&mut self, events: &mut Vec<WorldEvent>) {
        for (old_level, target, transition) in std::mem::take(&mut self.pending_camera_transitions)
        {
            self.teleport_camera_watchers(&old_level, &target, &transition, events);
        }
    }

    /// `ServerPlayer.tick` (ServerPlayer.java:586-597).
    pub(super) fn tick_spectator_cameras(&mut self, events: &mut Vec<WorldEvent>) {
        for connection_id in self.levels.player_keys().copied().collect::<Vec<_>>() {
            if self.levels[connection_id].cold.camera.is_none() {
                continue;
            }
            let _ = self.tick_spectator_camera_checked(connection_id, events);
        }
    }
    fn tick_spectator_camera_checked(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let Some(target) = self.levels[connection_id].cold.camera.clone() else {
            return Some(());
        };
        let Some(entity) = self.camera_entity(&target).filter(|entity| entity.alive) else {
            return self.set_player_camera_checked(connection_id, None, events);
        };
        let player = self.levels.player_mut(connection_id).unwrap();
        let moved = player.position() != entity.position;
        player.set_position(entity.position);
        player.set_rotation(entity.rotation);
        let stop = player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN);
        if moved {
            self.waypoint_update_player_source(connection_id, events);
            self.waypoint_update_player_receiver(connection_id, events);
        }
        self.move_session_chunk_tracking(connection_id, events)?;
        if stop {
            self.set_player_camera_checked(connection_id, None, events)?;
        }
        Some(())
    }

    /// `ServerGamePacketListenerImpl.handlePickItemFromEntity` (ServerGamePacketListenerImpl.java:725-738).
    pub(super) fn apply_pick_item_from_entity(
        &mut self,
        connection_id: ConnectionId,
        packet: PickItemFromEntity,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(target) =
            self.camera_entity_by_id(&self.levels[connection_id].dimension, packet.entity_id)
        else {
            return;
        };
        if !self.player_reaches_camera_entity(connection_id, &target) {
            return;
        }
        let profile_target = match target.target {
            CommandCameraTarget::Player { uuid } => FetchProfileTarget::Player { uuid },
            CommandCameraTarget::Stored {
                dimension,
                entity_id,
                uuid,
            } => {
                let stack = self
                    .level_by_id(&self.require_level_id(&dimension))
                    .entities
                    .get(entity_id)
                    .and_then(|entity| entity.pick_result());
                if let Some(stack) = stack {
                    let stack = match stack {
                        crate::entity::EntityPickResult::Existing(stack) => stack,
                        crate::entity::EntityPickResult::Fresh { item_id } => {
                            bbb_server_types::item_stack::RuntimeItemStack::from_summary(
                                bbb_protocol::packets::ItemStackSummary {
                                    item_id: Some(item_id),
                                    count: 1,
                                    component_patch: Default::default(),
                                },
                            )
                        }
                    };
                    let picked = self.try_pick_item(connection_id, stack, events);
                    if let Err(error) = picked {
                        tracing::warn!("entity pick item: {error}");
                    }
                }
                FetchProfileTarget::Stored {
                    dimension,
                    entity_id,
                    uuid,
                }
            }
        };
        if packet.include_data && self.can_use_game_master_blocks(connection_id) {
            if let FetchProfileResult::Avatar {
                profile,
                display_name,
            } = self.fetch_profile(&profile_target)
            {
                match commands::avatar_profile_feedback(&profile, display_name) {
                    Ok(message) if self.accepts_command_success() => self.send_system_message(
                        connection_id,
                        SystemChat::from_component(
                            RichTextComponent::try_from_component_nbt_with_registry(
                                message,
                                self.plugins().content_registry(),
                            )
                            .expect("validated profile feedback"),
                            false,
                        ),
                        events,
                    ),
                    Err(error) => self.send_system_message(
                        connection_id,
                        SystemChat::from_component(
                            commands::failure_feedback(commands::translatable(
                                "commands.fetchprofile.failed_to_serialize",
                                vec![bbb_protocol::NbtSummaryValue::String(error.into())],
                            ))
                            .message,
                            false,
                        ),
                        events,
                    ),
                    _ => {}
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
