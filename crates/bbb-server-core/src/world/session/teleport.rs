//! Player respawn entry point and same-level / cross-dimension teleportation.

use bbb_protocol::packets::{
    ChangeDifficulty, PlayerPositionState, PlayerPositionUpdate, PlayerRotation, Respawn,
    SetHeldSlot, Vec3d,
};
use bbb_server_types::save::Java26BuiltinDimension;

use crate::commands;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::world::external_player_ride_events::DismountLanding;
use crate::world::{
    block_position, send, LevelId, ServerWorld, TeleportTransition, WorldEvent, UNSENT_HEALTH,
};

impl ServerWorld {
    // PlayerList.java:385-451: resolve the spawn block before replacing the player.
    pub(in crate::world) fn perform_respawn(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.perform_respawn_checked(connection_id, false, events);
    }

    /// `ServerPlayer.teleport(TeleportTransition)` (`ServerPlayer.java:1079-1135`).
    pub fn teleport_player(
        &mut self,
        connection_id: ConnectionId,
        transition: TeleportTransition,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some(new_level) = self.level_id(&transition.new_level) else {
            return false;
        };
        self.teleport_player_id(
            connection_id,
            TeleportTransition {
                new_level,
                position: transition.position,
                y_rot: transition.y_rot,
                x_rot: transition.x_rot,
                relatives_mask: transition.relatives_mask,
            },
            events,
        )
    }

    pub(in crate::world) fn teleport_player_id(
        &mut self,
        connection_id: ConnectionId,
        transition: TeleportTransition<LevelId>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        self.teleport_player_checked(connection_id, transition, events)
            .unwrap_or(false)
    }

    pub(in crate::world) fn teleport_player_checked(
        &mut self,
        connection_id: ConnectionId,
        transition: TeleportTransition<LevelId>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        let Some((old_level, entity_id, game_type)) =
            self.levels.player(connection_id).and_then(|player| {
                if player.avatar_removed {
                    return None;
                }
                Some((player.dimension, player.entity_id(), player.game_type))
            })
        else {
            return Some(false);
        };
        if !self.has_level_id(&transition.new_level) {
            return Some(false);
        }
        if self.levels[connection_id].cold.sleeping_pos.is_some() {
            self.stop_player_sleep_for_teleport(connection_id, events);
        }
        // ServerPlayer.java:1091-1093.
        if self.stop_external_player_ride(
            &old_level,
            connection_id,
            DismountLanding::Stay,
            events,
        )? {
            self.apply_level_entity_lifecycle_by_key(&old_level, events);
        }
        self.levels[connection_id].boarding_cooldown = 0;

        // `:1095-1100`.
        if transition.new_level == old_level {
            self.teleport_set_position(connection_id, &transition, events);
            // `ServerPlayer.java:1097`: `this.connection.resetPosition()`.
            let player = &mut self.levels[connection_id];
            player.first_good = player.position();
            player.last_good = player.position();
            return Some(true);
        }

        // `:1102`.
        self.levels[connection_id].is_changing_dimension = true;
        // `:1104`.
        const KEEP_ALL_DATA: i8 = 3;
        events.push(send(
            connection_id,
            ClientboundPlayPacket::Respawn(Respawn {
                common_spawn_info: self.common_spawn_info(
                    &transition.new_level,
                    game_type,
                    self.levels[connection_id].cold.previous_game_type,
                    self.levels[connection_id].cold.portal_cooldown,
                    self.levels[connection_id].cold.last_death_location.clone(),
                ),
                data_to_keep: KEEP_ALL_DATA,
            }),
        ));
        events.push(send(
            connection_id,
            ClientboundPlayPacket::ChangeDifficulty(ChangeDifficulty {
                difficulty: self.admin.settings().difficulty,
                locked: self.cold.difficulty_locked,
            }),
        ));
        // `:1107`.
        self.send_player_permission_level(connection_id, events);
        // `:1108`: `oldLevel.removePlayerImmediately(this, CHANGED_DIMENSION)`
        // reaches `Player.remove` (`Player.java:1244-1250`), closing any open
        // container against `old_level` and returning carried / 2x2 grid items
        // to the inventory (`AbstractContainerMenu.java:598-606`).
        if self.levels[connection_id].inventory.container_id() != 0 {
            self.close_container(connection_id, false, events);
        }
        self.close_container(connection_id, false, events);
        self.apply_chunk_tracking_view(connection_id, None, events);
        self.remove_player_entity_tracking(&old_level, connection_id, entity_id, true, events);
        self.waypoint_remove_player(connection_id, events);
        self.remove_level_player(&old_level, connection_id);
        self.update_sleeping_player_list(&old_level, events);
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the teleporting session is still connected");
        // `:1112-1114`.
        if old_level.builtin() == Some(Java26BuiltinDimension::Overworld)
            && transition.new_level.builtin() == Some(Java26BuiltinDimension::Nether)
        {
            player.cold.entered_nether_position = Some(player.position());
        }
        // `:1118`.
        self.levels.move_player(connection_id, transition.new_level);
        self.append_level_player(&transition.new_level, connection_id);
        self.update_sleeping_player_list(&transition.new_level, events);
        self.refresh_cross_level_entities_across(&old_level, &transition.new_level);
        // `:1119-1120`.
        self.teleport_set_position(connection_id, &transition, events);
        let position = self.levels[connection_id].position();
        self.levels[connection_id].first_good = position;
        self.levels[connection_id].last_good = position;
        let pos = block_position(position);
        self.levels[connection_id].cold.last_section_pos = (pos.x >> 4, pos.y >> 4, pos.z >> 4);
        // `:1121`.
        let _ =
            self.add_player_entity_tracking_checked(&transition.new_level, connection_id, events);
        self.update_chunk_tracking(connection_id, events);
        let _ = self.move_session_entity_tracking(connection_id, events);
        self.waypoint_add_player(connection_id, events);
        // `:1123`.
        self.trigger_dimension_change(connection_id, &old_level, events);
        if self.levels.player(connection_id).is_none() {
            return Some(true);
        }
        // `:1124`.
        self.stop_using_item(connection_id);
        // `:1125`.
        let Some(abilities) = self
            .levels
            .player(connection_id)
            .map(|player| player.abilities)
        else {
            return Some(true);
        };
        events.push(send(
            connection_id,
            ClientboundPlayPacket::PlayerAbilities(abilities),
        ));
        // `:1126`.
        self.send_level_info(connection_id, events);
        // `:1127`.
        let (menu, selected_slot) = {
            let Some(player) = self.levels.player_mut(connection_id) else {
                return Some(true);
            };
            let menu = player.inventory.send_all_data_to_remote();
            (menu, player.inventory.selected_slot())
        };
        match menu {
            Ok(menu) => Self::send_menu_sync(connection_id, [menu], events),
            Err(error) => tracing::warn!("dimension change inventory publication: {error}"),
        }
        // `:1128`.
        self.send_active_player_effects(connection_id, events);
        // `:1130-1132`.
        self.levels[connection_id].last_sent_exp = -1;
        self.levels[connection_id].last_sent_health = UNSENT_HEALTH;
        events.push(send(
            connection_id,
            ClientboundPlayPacket::SetHeldSlot(SetHeldSlot {
                slot: i32::from(selected_slot),
            }),
        ));
        self.teleport_camera_watchers(
            &old_level,
            &commands::CommandCameraTarget::Player {
                uuid: self.levels[connection_id].profile.uuid,
            },
            &transition,
            events,
        );
        Some(true)
    }

    pub(in crate::world) fn teleport_set_position(
        &mut self,
        target: ConnectionId,
        transition: &TeleportTransition<LevelId>,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player_mut(target) else {
            return;
        };
        player.set_on_ground(false);
        self.sync_teleport_position(target, transition, events);
    }

    /// `ServerGamePacketListenerImpl.teleport(PositionMoveRotation, relatives)`
    /// (`ServerGamePacketListenerImpl.java:1246-1255`).
    pub(in crate::world) fn sync_teleport_position(
        &mut self,
        connection_id: ConnectionId,
        transition: &TeleportTransition<LevelId>,
        events: &mut Vec<WorldEvent>,
    ) {
        let now = self.tick;
        let teleport_id = self.next_teleport_id;
        self.next_teleport_id = self.next_teleport_id.wrapping_add(1);
        let change = PlayerPositionUpdate {
            id: teleport_id,
            position: transition.position,
            delta_movement: Vec3d::default(),
            y_rot: transition.y_rot,
            x_rot: transition.x_rot,
            relatives_mask: transition.relatives_mask,
        };
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the teleporting session is still connected");
        // Entity.java:3079-3089: relative velocity uses the body's current movement.
        let absolute = change.apply_to_state(PlayerPositionState {
            position: player.position(),
            delta_movement: player.delta_movement(),
            y_rot: player.rotation().y_rot,
            x_rot: player.rotation().x_rot,
        });
        let rotation = PlayerRotation {
            y_rot: absolute.y_rot,
            x_rot: absolute.x_rot,
        };
        let moved = player.position() != absolute.position;
        player.set_position(absolute.position);
        player.set_delta_movement(absolute.delta_movement);
        player.block_contact.old_position = absolute.position;
        player.block_contact.pending.clear();
        player.set_rotation(rotation);
        player.awaiting_teleport = Some(teleport_id);
        player.awaiting_teleport_tick = now;
        self.update_player_query_section(connection_id);
        if moved {
            self.waypoint_update_player_source(connection_id, events);
            self.waypoint_update_player_receiver(connection_id, events);
        }
        events.push(send(
            connection_id,
            ClientboundPlayPacket::PlayerPosition(change),
        ));
    }
}
