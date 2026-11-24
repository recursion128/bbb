use super::super::session::{RULE_ELYTRA_MOVEMENT_CHECK, RULE_PLAYER_MOVEMENT_CHECK};
use super::super::{TeleportTransition, WorldCloseReason};
use super::*;
use bbb_protocol::packets::MovePlayer;

impl ServerWorld {
    pub(in crate::world) fn apply_move_player(
        &mut self,
        connection_id: ConnectionId,
        movement: MovePlayer,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        if movement_is_invalid(movement.position, movement.rotation) {
            Some(WorldCloseReason::InvalidMovement)
        } else {
            let _ = self.apply_move_player_checked(connection_id, movement, events);
            None
        }
    }

    /// `updateAwaitingTeleport` (`ServerGamePacketListenerImpl.java:1206-1223`):
    /// a teleport the client has not answered for more than 20 ticks is sent
    /// again from where the session stands.
    pub(in crate::world) fn update_awaiting_teleport(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        let now = self.tick;
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        if !player.client_loaded() {
            return;
        }
        if player.awaiting_teleport.is_none() {
            player.awaiting_teleport_tick = now;
            return;
        }
        if now.saturating_sub(player.awaiting_teleport_tick) <= 20 {
            return;
        }
        let rotation = player.rotation();
        let transition = TeleportTransition {
            new_level: player.dimension,
            position: player.position(),
            y_rot: rotation.y_rot,
            x_rot: rotation.x_rot,
            relatives_mask: 0,
        };
        self.teleport_set_position(connection_id, &transition, events);
    }

    fn apply_move_player_checked(
        &mut self,
        connection_id: ConnectionId,
        movement: MovePlayer,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        // `ServerGamePacketListenerImpl.java:1059`: the End credits own the
        // player until it respawns.
        if self.levels[connection_id].cold.won_game {
            return Some(());
        }
        self.update_awaiting_teleport(connection_id, events);
        let facts = MovementValidationFacts {
            movement_check: self.game_rule_boolean(RULE_PLAYER_MOVEMENT_CHECK),
            elytra_movement_check: self.game_rule_boolean(RULE_ELYTRA_MOVEMENT_CHECK),
            runs_normally: self.tick_rate.runs_normally(),
            is_passenger: self.player_is_passenger(connection_id),
        };
        // The probe sweeps the packet's own delta, so it only
        // runs once the arms before `player.move` let the packet
        // through.
        let sleeping = self.levels[connection_id].cold.sleeping_pos.is_some();
        let refile_tickets = self.levels[connection_id].client_loaded()
            && self.levels[connection_id].awaiting_teleport.is_none();
        let decided = movement_precheck(
            self.levels
                .player_mut(connection_id)
                .expect("connection presence was checked"),
            &movement,
            facts,
        );
        let packet_start = self.levels[connection_id].position();
        let glide_movement = {
            let player = &self.levels[connection_id];
            player
                .shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING)
                .then_some(player.known_movement)
        };
        let outcome = match decided {
            Some(outcome) => outcome,
            None => {
                let target = connection_id;
                let moved = (|| {
                    let mut probe = self.player_move_physics_probe(target, &movement)?;
                    if probe.reset_fall_distance {
                        self.reset_player_fall_distance(target, events);
                    }
                    apply_player_move_position_prefix(
                        self.levels.player_mut(target)?,
                        &mut self.scoreboard,
                        movement,
                        &probe,
                    )?;
                    self.update_player_query_section(target);
                    self.finish_player_move_support(target, &mut probe)?;
                    let content_registry = self.registries.content_registry_arc().clone();
                    apply_player_move_velocity_prefix(
                        self.levels.player_mut(target)?,
                        &probe,
                        &content_registry,
                    )?;
                    self.finish_player_move_emission(target, &probe, events)?;
                    self.finish_player_move_speed(target)?;
                    self.finish_player_collision_queries(target, &movement, &mut probe)?;
                    self.finish_player_floating_query(target, &movement, &mut probe)?;
                    probe.prefix_applied = true;
                    Some(apply_movement(
                        self.levels.player_mut(target)?,
                        &mut self.scoreboard,
                        movement,
                        probe,
                    ))
                })();
                moved?
            }
        };
        let refile_rider = refile_tickets
            && !sleeping
            && matches!(outcome, MovementOutcome::Ignored)
            && facts.is_passenger;
        match outcome {
            MovementOutcome::Accepted => {
                if movement.position.is_some()
                    && self.levels[connection_id].position() != packet_start
                {
                    self.move_session_chunk_tracking(connection_id, events)?;
                }
                self.update_player_query_section(connection_id);
                self.update_player_contact_support_checked(connection_id, events)?;
                // `:1175`, over the same delta
                // `handlePlayerKnownMovement` was handed, and over the
                // block the accepted position now stands in.
                let player = self.levels.player_at(connection_id);
                let landed_in_liquid = player.delta_movement().y < f64::from(1.0e-5_f32)
                    && (player
                        .fluid_interaction
                        .is_in_fluid(crate::entity::FluidTag::Water)
                        || player
                            .fluid_interaction
                            .is_in_fluid(crate::entity::FluidTag::Lava));
                let on_climbable = if !movement.on_ground && !landed_in_liquid {
                    self.player_on_climbable_checked(connection_id)?
                } else {
                    false
                };
                self.try_reset_player_impulse_after_movement(
                    connection_id,
                    movement.on_ground,
                    on_climbable,
                );
                let player = self.levels.player_at(connection_id);
                let checks_climbable = player.cold.last_vehicle.is_none()
                    && player.known_movement != Vec3d::default()
                    && !player.shared_flag(
                        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
                    )
                    && !player
                        .fluid_interaction
                        .is_eye_in_fluid(crate::entity::FluidTag::Water)
                    && !player
                        .fluid_interaction
                        .is_in_fluid(crate::entity::FluidTag::Water);
                let on_climbable = if checks_climbable {
                    self.player_on_climbable_checked(connection_id)?
                } else {
                    false
                };
                check_movement_statistics(
                    self.levels
                        .player_mut(connection_id)
                        .expect("connection presence was checked"),
                    &mut self.scoreboard,
                    on_climbable,
                );
                self.waypoint_update_player_source(connection_id, events);
                self.waypoint_update_player_receiver(connection_id, events);
                if let Some(before) = glide_movement.filter(|_| movement.horizontal_collision) {
                    self.crash_fall_flying_player(connection_id, before, events)?;
                }
            }
            // `:1104-1106`: the position is left where the server
            // had it and the client is sent back to it.
            MovementOutcome::TooQuickly => {
                let (dimension, position, rotation) = {
                    let player = self.levels.player_at(connection_id);
                    (player.dimension, player.position(), player.rotation())
                };
                self.teleport_set_position(
                    connection_id,
                    &TeleportTransition {
                        new_level: dimension,
                        position,
                        y_rot: rotation.y_rot,
                        x_rot: rotation.x_rot,
                        relatives_mask: 0,
                    },
                    events,
                );
            }
            // `:1180`: the packet is corrected back to the
            // position it started from, with the rotation it
            // asked for.
            MovementOutcome::MovedWrongly(rotation) => {
                let (dimension, position) = {
                    let player = self.levels.player_at(connection_id);
                    (player.dimension, packet_start)
                };
                self.teleport_set_position(
                    connection_id,
                    &TeleportTransition {
                        new_level: dimension,
                        position,
                        y_rot: rotation.y_rot,
                        x_rot: rotation.x_rot,
                        relatives_mask: 0,
                    },
                    events,
                );
                // ServerGamePacketListenerImpl.java:1180-1183: correction precedes the fall check.
                self.check_player_fall_state_checked(
                    connection_id,
                    Vec3d::default(),
                    movement.on_ground,
                    events,
                )?;
                self.levels
                    .player_mut(connection_id)?
                    .block_contact
                    .pending
                    .pop_back();
            }
            MovementOutcome::SleepingMoved(rotation) => {
                let player = self.levels.player_at(connection_id);
                let transition = TeleportTransition {
                    new_level: player.dimension,
                    position: player.position(),
                    y_rot: rotation.y_rot,
                    x_rot: rotation.x_rot,
                    relatives_mask: 0,
                };
                self.teleport_set_position(connection_id, &transition, events);
            }
            MovementOutcome::Ignored => {}
        }
        if refile_rider {
            self.move_session_chunk_tracking(connection_id, events)?;
        }
        Some(())
    }

    /// `LivingEntity.handleFallFlyingCollisions` (`LivingEntity.java:2567-2576`). The server
    /// does not run the glide's own travel, so the previous accepted delta stands in for the
    /// pre-move velocity and a server sweep of it from the new position clips it.
    fn crash_fall_flying_player(
        &mut self,
        connection_id: ConnectionId,
        before: Vec3d,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let position = self.levels[connection_id].position();
        let target = Vec3d {
            x: position.x + before.x,
            y: position.y + before.y,
            z: position.z + before.z,
        };
        let response = self
            .player_move_physics_probe_for_mover(
                connection_id,
                target,
                before,
                false,
                crate::entity::MoverType::Self_,
            )?
            .collision_response?;
        let after_x = if response.clip_x { 0.0 } else { before.x };
        let after_z = if response.clip_z { 0.0 } else { before.z };
        let damage = ((before.x.hypot(before.z) - after_x.hypot(after_z)) * 10.0 - 3.0) as f32;
        if damage <= 0.0 {
            return Some(());
        }
        let sound = crate::entity::living::fall_damage_sound(
            damage as i32,
            "minecraft:entity.player.small_fall",
            "minecraft:entity.player.big_fall",
        );
        if let Some(registry_id) =
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(sound)
        {
            self.play_player_sound(
                connection_id,
                Some(connection_id),
                SoundEventHolder::Reference { registry_id },
                bbb_protocol::packets::SoundSource::Players,
                position,
                1.0,
                1.0,
                events,
            );
        }
        self.hurt_connected_player_checked(
            connection_id,
            "minecraft:fly_into_wall",
            damage,
            None,
            None,
            None,
            events,
        )?;
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{creative_fixture_config, join_loaded, mailbox};
    use crate::world::WorldCommand;
    use bbb_protocol::game_rules::GameRuleValue;
    use bbb_protocol::packets::PlayServerbound;

    fn assert_extreme_displacement_snaps_back(freeze_ticks: bool) {
        let (sender, receiver) = mailbox();
        let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
        world.set_test_respawn_radius(0);
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        if freeze_ticks {
            world.set_tick_frozen(true, &mut Vec::new());
        } else {
            assert!(world.set_game_rule_for_test(
                "minecraft:player_movement_check",
                GameRuleValue::Boolean(false),
            ));
        }
        let start = world.player_movement_validation_for_test(ConnectionId(1)).0;
        sender
            .try_send(WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: PlayServerbound::MovePlayer(MovePlayer {
                    position: Some(Vec3d {
                        x: start.x + 10_000_000.0,
                        y: start.y,
                        z: start.z + 200.0,
                    }),
                    rotation: None,
                    on_ground: true,
                    horizontal_collision: false,
                }),
            })
            .unwrap();
        let extreme = world.tick(receiver.drain());
        assert!(
            extreme.events.iter().any(|event| matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::PlayerPosition(_),
                    ..
                }
            )),
            "extreme displacement should snap player back: {:?}",
            extreme.events
        );
        assert_eq!(
            world
                .player_movement_validation_for_test(ConnectionId(1))
                .0
                .x,
            start.x
        );
    }

    #[test]
    fn extreme_move_player_displacement_snaps_back_when_movement_check_off_or_frozen() {
        assert_extreme_displacement_snaps_back(false);
        assert_extreme_displacement_snaps_back(true);
    }
}
