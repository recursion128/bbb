//! Production packet and tick ownership for an external Player riding a Boat.

use crate::world::LevelId;
mod known_movement;

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::{
    ExternalPlayerBoatAdmissionFacts, ExternalPlayerBoatCollisionProbe,
    ExternalPlayerBoatCollisionResolution, ExternalPlayerBoatMoveOutcome,
    ExternalPlayerBoatMoveRequest, ExternalPlayerVehiclePlacement,
};
use bbb_protocol::packets::{
    InteractEntity, MoveVehicle, MoveVehicleCommand, PaddleBoat, PlayerPositionUpdate, Vec3d,
    PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y_ROT,
};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_types::geometry::{collide_with_boxes, collide_with_step, Vec3Ops};
use bbb_server_types::interaction::is_within_entity_interaction_range;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::custom_stat;

use super::movement_collision::{
    add, block_collision_boxes, level_entity_boxes_in, maximum_flying_ticks, no_blocks_around,
    subtract, CollisionSnapshot,
};
use super::{
    block_position, send, stored_block_state, GameType, ServerLevel, ServerWorld, WorldCloseReason,
    WorldEvent, ENTITY_INTERACTION_BUFFER,
};

impl ServerWorld {
    /// `AbstractBoat.interact` followed by `ServerPlayer.startRiding`
    /// (`AbstractBoat.java:686-693`; `ServerPlayer.java:2121-2130`).
    pub(super) fn apply_external_player_boat_interact(
        &mut self,
        connection_id: ConnectionId,
        packet: InteractEntity,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        self.apply_external_player_boat_interact_checked(connection_id, packet, events)
            .unwrap_or(true)
    }

    pub(super) fn apply_external_player_boat_interact_checked(
        &mut self,
        connection_id: ConnectionId,
        packet: InteractEntity,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        let player = self.levels.player_at(connection_id);
        let dimension = player.dimension;
        let player_loaded = player.client_loaded();
        let Some((target_position, target_bounds)) = self
            .level_by_id(&dimension)
            .entities
            .get(packet.entity_id)
            .filter(|target| crate::entity::boat::is_boat(target.data()))
            .map(|target| {
                let target_position = target.position();
                (target_position, target.bounding_box())
            })
        else {
            return Some(false);
        };
        if !player_loaded {
            return Some(true);
        }
        self.levels[connection_id].set_shared_flag(
            bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            packet.using_secondary_action,
        );
        let player = self.levels.player_at(connection_id);
        let range = player.entity_interaction_range();
        let eye = match player.eye_position() {
            Ok(eye) => eye,
            Err(_) => return Some(true),
        };
        let border = self.level_by_id(&dimension).world_border;
        let game_time = self.game_time();
        if !border_contains_position(border, target_position, game_time)
            || !is_within_entity_interaction_range(
                eye,
                &target_bounds,
                range,
                ENTITY_INTERACTION_BUFFER,
            )
        {
            return Some(true);
        }

        if self.levels[connection_id].game_type == GameType::Spectator {
            self.open_vehicle_container(connection_id, packet.entity_id, events);
            return Some(true);
        }

        // `AbstractBoat.interact` runs `super.interact`'s leash branches
        // before the ride attempt (`AbstractBoat.java:686-693`,
        // `Entity.java:2168-2242`).
        let hand = packet.hand;
        let (original_hand, player_entity_id, player_uuid, player_pos, infinite_materials) = {
            let player = self.levels.player_at(connection_id);
            let stack = match hand {
                bbb_protocol::packets::InteractionHand::MainHand => {
                    player.inventory.selected_stack().clone()
                }
                bbb_protocol::packets::InteractionHand::OffHand => {
                    player.inventory.off_hand_stack().clone()
                }
            };
            (
                stack,
                player.entity_id(),
                player.profile.uuid,
                player.position(),
                player.abilities.instabuild,
            )
        };
        let original_summary = original_hand.summary().clone();
        let mut hand_stack = original_summary.clone();
        let leash_outcome = crate::entity::leash::interact_leash(
            &mut self.level_mut_by_id(&dimension).entities,
            packet.entity_id,
            &mut crate::entity::leash::LeashInteractingPlayer {
                mob_item_used: false,
                hand_stack: &mut hand_stack,
                entity_id: player_entity_id,
                uuid: player_uuid,
                position: player_pos,
                secondary_use_active: packet.using_secondary_action,
                infinite_materials,
            },
        );
        // `Player.interactOn` (`Player.java:836-838`) restores a creative
        // shrink that ran on the same hand stack.
        if infinite_materials && hand_stack.count < original_hand.count() {
            hand_stack = original_summary.clone();
        }
        let hand_stack = RuntimeItemStack::from_summary(hand_stack);
        *self.levels[connection_id].inventory.item_in_hand_mut(hand) = hand_stack;
        if leash_outcome != crate::entity::leash::LeashInteraction::Pass {
            let awarded_item = if matches!(
                leash_outcome,
                crate::entity::leash::LeashInteraction::BoundHeld
                    | crate::entity::leash::LeashInteraction::Unleashed
            ) {
                bbb_protocol::packets::ItemStackSummary::empty()
            } else {
                original_summary.clone()
            };
            let succeeded = leash_outcome != crate::entity::leash::LeashInteraction::Refused;
            self.finish_leash_interaction(
                connection_id,
                dimension,
                super::command_execution::entity_interaction::LeashInteractionFinish {
                    outcome: leash_outcome,
                    target_position,
                    target_block_pos: block_position(target_position),
                    off_hand: hand == bbb_protocol::packets::InteractionHand::OffHand,
                },
                events,
            )?;
            if succeeded {
                self.trigger_player_entity_interaction(
                    connection_id,
                    "minecraft:player_interacted_with_entity",
                    dimension,
                    packet.entity_id,
                    &awarded_item,
                    events,
                );
            }
            return Some(true);
        }

        // AbstractChestBoat.java:43-49,80-100: failed riding alone does not open cargo.
        let entities = &self.level_by_id(&dimension).entities;
        let target = entities.get(packet.entity_id)?;
        if crate::entity::chest_boat::chest_boat(target.data()).is_some()
            && (packet.using_secondary_action
                || !entities.passengers(packet.entity_id).is_empty()
                || target.is_eye_in_fluid(crate::entity::FluidTag::Water))
        {
            self.open_vehicle_container(connection_id, packet.entity_id, events);
            self.trigger_player_entity_interaction(
                connection_id,
                "minecraft:player_interacted_with_entity",
                dimension,
                packet.entity_id,
                &original_summary,
                events,
            );
            return Some(true);
        }

        let player = self.levels.player_at(connection_id);
        let facts = ExternalPlayerBoatAdmissionFacts {
            can_ride: player.external_player_can_ride(),
            secondary_use_active: packet.using_secondary_action,
            eye_in_water: eye_in_water(self.level_by_id(&dimension), eye),
        };
        let player_entity_id = player.entity_id();
        let player_y_rot = player.rotation().y_rot;
        if !self
            .level_mut_by_id(&dimension)
            .entities
            .admit_external_player_to_boat(player_entity_id, packet.entity_id, facts)
        {
            return Some(true);
        }
        let player_scale = self.levels.player_at(connection_id).pose_scale();
        let player_pose_id = self.levels.player_at(connection_id).pose_id;
        let Some(placement) = self
            .level_by_id(&dimension)
            .entities
            .external_player_vehicle_placement_with_rider(
                player_entity_id,
                player_y_rot,
                player_scale,
                player_pose_id,
            )
        else {
            self.level_mut_by_id(&dimension)
                .entities
                .detach_external_player(player_entity_id);
            return Some(true);
        };
        self.raise_external_player_mount(connection_id, events);
        self.place_newly_mounted_player(connection_id, placement, None, events)?;
        self.trigger_player_entity_interaction(
            connection_id,
            "minecraft:player_interacted_with_entity",
            dimension,
            packet.entity_id,
            &original_summary,
            events,
        );
        Some(true)
    }

    /// `AbstractHorse.doPlayerRide`'s server half (`AbstractHorse.java:497-501`):
    /// the rider takes the horse's rotation and then `startRiding` asks the
    /// relation store for admission (`Entity.java:2353-2391`).
    #[cfg(test)]
    pub(in crate::world) fn start_external_player_ride(
        &mut self,
        connection_id: ConnectionId,
        dimension: super::Java26BuiltinDimension,
        vehicle_entity_id: i32,
        y_rot: f32,
        x_rot: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        let _ = self.start_external_player_ride_checked(
            connection_id,
            &dimension.into(),
            vehicle_entity_id,
            crate::entity::passenger::RelationAdmission::Normal,
            Some((y_rot, x_rot)),
            events,
        );
    }

    pub(in crate::world) fn start_external_player_ride_checked(
        &mut self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        vehicle_entity_id: i32,
        admission: crate::entity::passenger::RelationAdmission,
        rotation: Option<(f32, f32)>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<bool> {
        let player = self.levels.player_at(connection_id);
        let player_y_rot = player.rotation().y_rot;
        let player_entity_id = player.entity_id();
        let can_ride = player.external_player_can_ride();
        let entities = &mut self.level_mut_by_id(dimension).entities;
        let attached = match admission {
            crate::entity::passenger::RelationAdmission::Normal => {
                entities.attach_external_player(player_entity_id, vehicle_entity_id, can_ride)
            }
            crate::entity::passenger::RelationAdmission::Forced => entities
                .attach_external_player_forced(
                    player_entity_id,
                    vehicle_entity_id,
                    entities.external_player_insertion_index(vehicle_entity_id),
                ),
        };
        if !attached {
            return Some(false);
        }
        let player_scale = self.levels.player_at(connection_id).pose_scale();
        let player_pose_id = self.levels.player_at(connection_id).pose_id;
        let Some(placement) = self
            .level_by_id(dimension)
            .entities
            .external_player_vehicle_placement_with_rider(
                player_entity_id,
                rotation.map_or(player_y_rot, |(y_rot, _)| y_rot),
                player_scale,
                player_pose_id,
            )
        else {
            self.level_mut_by_id(dimension)
                .entities
                .detach_external_player(player_entity_id);
            return Some(false);
        };
        self.raise_external_player_mount(connection_id, events);
        self.place_newly_mounted_player(connection_id, placement, rotation, events)?;
        Some(true)
    }

    /// `rotation` is the rider rotation the mount itself wrote, which the
    /// teleport then has to carry absolutely; `None` keeps the client's own.
    fn place_newly_mounted_player(
        &mut self,
        connection_id: ConnectionId,
        placement: ExternalPlayerVehiclePlacement,
        rotation: Option<(f32, f32)>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let teleport_id = self.next_teleport_id;
        self.next_teleport_id = self.next_teleport_id.wrapping_add(1);
        let now = self.tick;
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the mounted session remains connected");
        player.set_position(placement.position);
        player.set_y_rot(placement.y_rot);
        if let Some((_, x_rot)) = rotation {
            player.set_x_rot(x_rot);
        }
        player.set_on_ground(false);
        player.known_movement = Vec3d::default();
        player.received_movement_this_tick = false;
        player.awaiting_teleport = Some(teleport_id);
        player.awaiting_teleport_tick = now;
        // Rotation is relative so the packet cannot undo the yaw
        // `positionRider` just constrained to the Boat; a mount that wrote the
        // rider's rotation itself sends that rotation instead.
        let (y_rot, x_rot, relatives_mask) = match rotation {
            Some((_, x_rot)) => (placement.y_rot, x_rot, 0),
            None => (0.0, 0.0, PLAYER_RELATIVE_Y_ROT | PLAYER_RELATIVE_X_ROT),
        };
        events.push(send(
            connection_id,
            ClientboundPlayPacket::PlayerPosition(PlayerPositionUpdate {
                id: teleport_id,
                position: placement.position,
                delta_movement: Vec3d::default(),
                y_rot,
                x_rot,
                relatives_mask,
            }),
        ));
        let player = self.levels.player_at(connection_id);
        let dimension = player.dimension;
        let vehicle_id = self
            .level_by_id(&dimension)
            .entities
            .external_player_vehicle(player.entity_id())?;
        self.send_mounted_vehicle_state(connection_id, &dimension, vehicle_id, events);
        self.waypoint_update_player_source(connection_id, events);
        self.waypoint_update_player_receiver(connection_id, events);
        self.move_session_chunk_tracking(connection_id, events)
    }

    /// Sends a mounted player the vehicle's active effects and direct
    /// `SetPassengers` (`ServerPlayer.java:2121-2131`) before tracker broadcast.
    pub(super) fn send_mounted_vehicle_state(
        &self,
        connection_id: ConnectionId,
        dimension: &LevelId,
        vehicle_id: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let entities = &self.level_by_id(dimension).entities;
        if let Some(living) = entities
            .get(vehicle_id)
            .and_then(crate::entity::Entity::living)
        {
            for effect in living.active_effects() {
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::UpdateMobEffect(
                        crate::entity::living::mob_effects::client_update::update_mob_effect_packet(
                            vehicle_id, effect, false,
                        ),
                    ),
                ));
            }
        }
        // ServerPlayer.java:2121-2131: the new rider receives this before tracker broadcast.
        events.push(send(
            connection_id,
            ClientboundPlayPacket::SetPassengers(bbb_protocol::packets::SetPassengers {
                vehicle_id,
                passenger_ids: entities.passengers(vehicle_id).to_vec(),
            }),
        ));
    }

    /// `ServerGamePacketListenerImpl.handleMoveVehicle`
    /// (`ServerGamePacketListenerImpl.java:439-515`) through the typed entity
    /// kernel and the live world collision owner.
    pub(super) fn apply_external_player_boat_move(
        &mut self,
        connection_id: ConnectionId,
        update: MoveVehicleCommand,
        events: &mut Vec<WorldEvent>,
    ) -> Option<WorldCloseReason> {
        if vehicle_packet_has_invalid_values(update) {
            return Some(WorldCloseReason::InvalidVehicleMovement);
        }
        self.apply_external_player_boat_move_checked(connection_id, update, events)
            .flatten()
    }

    fn apply_external_player_boat_move_checked(
        &mut self,
        connection_id: ConnectionId,
        update: MoveVehicleCommand,
        events: &mut Vec<WorldEvent>,
    ) -> Option<Option<WorldCloseReason>> {
        // `ServerGamePacketListenerImpl.java:443`.
        self.update_awaiting_teleport(connection_id, events);
        let player = self.levels.player_at(connection_id);
        if !player.client_loaded() || player.awaiting_teleport.is_some() {
            return Some(None);
        }
        let Some(expected_vehicle_entity_id) = player.cold.last_vehicle else {
            return Some(None);
        };
        if self.external_player_controlled_vehicle(connection_id)
            != Some(expected_vehicle_entity_id)
        {
            return Some(None);
        }
        let player_entity_id = player.entity_id();
        let dimension = player.dimension;
        let request = ExternalPlayerBoatMoveRequest {
            player_entity_id,
            expected_vehicle_entity_id,
            first_good_position: player.cold.vehicle_first_good,
            last_good_position: player.cold.vehicle_last_good,
            target_position: update.position,
            y_rot: update.y_rot,
            x_rot: update.x_rot,
            on_ground: update.on_ground,
            singleplayer_owner: false,
            player_scale: player.pose_scale(),
            player_pose_id: player.pose_id,
        };
        let collision = self.external_player_vehicle_collision_snapshot(
            &dimension,
            expected_vehicle_entity_id,
            player_entity_id,
            update.position,
        );
        let outcome = {
            let ServerLevel {
                entities, chunks, ..
            } = self.level_mut_by_id(&dimension);
            entities.move_external_player_controlled_boat(request, |probe| {
                resolve_vehicle_collision(chunks, &collision, probe)
            })
        };
        Some(match outcome {
            ExternalPlayerBoatMoveOutcome::InvalidValues => {
                Some(WorldCloseReason::InvalidVehicleMovement)
            }
            ExternalPlayerBoatMoveOutcome::Ignored => None,
            ExternalPlayerBoatMoveOutcome::Correction {
                position,
                y_rot,
                x_rot,
                ..
            } => {
                events.push(send(
                    connection_id,
                    ClientboundPlayPacket::MoveVehicle(MoveVehicle {
                        position,
                        y_rot,
                        x_rot,
                    }),
                ));
                None
            }
            ExternalPlayerBoatMoveOutcome::Moved {
                placement,
                new_position,
                client_delta,
                attempted_delta,
                vehicle_was_supported,
                finalization,
                ..
            } => {
                self.finalize_external_player_boat_move_by_key(&dimension, finalization, events);
                let riding_stat = self
                    .level_by_id(&dimension)
                    .entities
                    .get(expected_vehicle_entity_id)
                    .and_then(riding_stat);
                let vehicle_can_float = self
                    .level_by_id(&dimension)
                    .entities
                    .get(expected_vehicle_entity_id)
                    // `HappyGhast.isFlyingVehicle` (`HappyGhast.java:580-582`).
                    .filter(|vehicle| {
                        vehicle.entity_type_id()
                            != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
                    })
                    .is_some_and(|vehicle| {
                        let query = vehicle
                            .bounding_box()
                            .inflate(0.0625, 0.0625, 0.0625)
                            .expand_towards(Vec3d {
                                x: 0.0,
                                y: -0.55,
                                z: 0.0,
                            });
                        !vehicle.no_gravity()
                            && no_blocks_around(&self.level_by_id(&dimension).chunks, query)
                    });
                let floating = attempted_delta.y >= -0.03125
                    && !vehicle_was_supported
                    && !self.admin.settings().allow_flight
                    && vehicle_can_float;
                // ServerGamePacketListenerImpl.java:498-499: move files the player's current position before positionRider.
                self.move_session_chunk_tracking(connection_id, events)?;
                let player = self
                    .levels
                    .player_mut(connection_id)
                    .expect("the moving rider remains connected");
                let riding_delta = subtract(placement.position, player.position());
                award_riding_distance(player, &mut self.scoreboard, riding_stat, riding_delta);
                player.set_position(placement.position);
                player.set_y_rot(placement.y_rot);
                player.set_on_ground(update.on_ground);
                player.known_movement = client_delta;
                player.received_movement_this_tick = true;
                player.cold.client_vehicle_is_floating = floating;
                player.cold.vehicle_last_good = new_position;
                self.waypoint_update_player_source(connection_id, events);
                self.waypoint_update_player_receiver(connection_id, events);
                self.move_session_entity_tracking(connection_id, events)?;
                None
            }
        })
    }

    /// `ServerGamePacketListenerImpl.handlePaddleBoat`
    /// (`ServerGamePacketListenerImpl.java:1436-1441`).
    pub(super) fn apply_external_player_boat_paddles(
        &mut self,
        connection_id: ConnectionId,
        update: PaddleBoat,
    ) {
        let player = self.levels.player_at(connection_id);
        let dimension = player.dimension;
        let player_entity_id = player.entity_id();
        self.level_mut_by_id(&dimension)
            .entities
            .apply_external_player_boat_paddles(player_entity_id, update.left, update.right);
    }

    /// The listener's per-connection vehicle identity/baseline refresh and
    /// floating timeout (`ServerGamePacketListenerImpl.java:329-363`).
    pub(super) fn tick_external_player_vehicle_listeners(&mut self, events: &mut Vec<WorldEvent>) {
        let connection_ids = self.levels.player_keys().copied().collect::<Vec<_>>();
        let mut floating_disconnects = Vec::new();
        for connection_id in connection_ids {
            let player = self.levels.player_at(connection_id);
            let controller = self
                .external_player_controlled_vehicle(connection_id)
                .and_then(|vehicle_entity_id| {
                    self.level_by_id(&player.dimension)
                        .entities
                        .external_player_vehicle_controller(player.entity_id(), vehicle_entity_id)
                });
            let maximum_flying_ticks = controller.as_ref().and_then(|controller| {
                self.level_by_id(&player.dimension)
                    .entities
                    .get(controller.vehicle_entity_id)
                    .map(|vehicle| maximum_flying_ticks(vehicle.gravity()))
            });
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("the listener owner remains connected");
            let Some(controller) = controller.filter(|controller| {
                controller.root_vehicle_entity_id == controller.vehicle_entity_id
            }) else {
                player.cold.last_vehicle = None;
                player.cold.client_vehicle_is_floating = false;
                player.cold.above_ground_vehicle_tick_count = 0;
                continue;
            };
            player.cold.last_vehicle = Some(controller.vehicle_entity_id);
            player.cold.vehicle_first_good = controller.vehicle_position;
            player.cold.vehicle_last_good = controller.vehicle_position;
            if player.cold.client_vehicle_is_floating {
                player.cold.above_ground_vehicle_tick_count =
                    player.cold.above_ground_vehicle_tick_count.wrapping_add(1);
                if maximum_flying_ticks
                    .is_some_and(|maximum| player.cold.above_ground_vehicle_tick_count > maximum)
                {
                    floating_disconnects.push(connection_id);
                }
            } else {
                player.cold.above_ground_vehicle_tick_count = 0;
            }
        }
        for connection_id in floating_disconnects {
            self.remove_player(connection_id, events);
            events.push(WorldEvent::Close {
                connection_id,
                reason: WorldCloseReason::Floating,
            });
        }
    }

    pub(super) fn sync_external_players_after_vehicle_tick(
        &mut self,
        dimension: &LevelId,
        ticked_entity_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        if self
            .level_by_id(dimension)
            .entities
            .passengers(ticked_entity_id)
            .is_empty()
        {
            return Some(());
        }
        let updates = self
            .levels
            .player_entries()
            .filter(|(_, player)| &player.dimension == dimension && !player.avatar_removed)
            .filter_map(|(&connection_id, player)| {
                let entities = &self.level_by_id(dimension).entities;
                (entities.external_player_root_vehicle(player.entity_id())
                    == Some(ticked_entity_id))
                .then(|| {
                    let placement = entities.external_player_vehicle_placement_with_rider(
                        player.entity_id(),
                        player.rotation().y_rot,
                        player.pose_scale(),
                        player.pose_id,
                    )?;
                    let direct_vehicle = entities.external_player_vehicle(player.entity_id())?;
                    let stat = entities.get(direct_vehicle).and_then(riding_stat);
                    Some((connection_id, placement, stat))
                })
                .flatten()
            })
            .collect::<Vec<_>>();
        for (connection_id, placement, stat) in updates {
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("the ticking rider remains connected");
            let old = player.position();
            player.set_position(placement.position);
            player.set_y_rot(placement.y_rot);
            // LivingEntity.rideTick (`LivingEntity.java:3226-3228`).
            player.block_contact.fall_distance = 0.0;
            award_riding_distance(
                player,
                &mut self.scoreboard,
                stat,
                subtract(placement.position, old),
            );
            self.waypoint_update_player_source(connection_id, events);
            self.waypoint_update_player_receiver(connection_id, events);
            // Entity.java:2305-2309: ordinary rider placement does not refile player tickets.
            self.move_session_entity_tracking(connection_id, events)?;
        }
        Some(())
    }

    /// `player.isPassenger()`, steering or not.
    pub(super) fn player_is_passenger(&self, connection_id: ConnectionId) -> bool {
        let player = self.levels.player_at(connection_id);
        self.level_by_id(&player.dimension)
            .entities
            .external_player_vehicle(player.entity_id())
            .is_some()
    }

    /// `player.getVehicle()` when its `getControllingPassenger()` is the
    /// player (`Entity.getControlledVehicle`, `Entity.java:3540-3542`).
    pub(super) fn external_player_controlled_vehicle(
        &self,
        connection_id: ConnectionId,
    ) -> Option<i32> {
        let player = self.levels.player_at(connection_id);
        let level = self.level_by_id(&player.dimension);
        let entities = &level.entities;
        if let Some(boat) = entities.external_player_direct_controlled_boat(player.entity_id()) {
            return Some(boat);
        }
        let vehicle_id = entities.external_player_vehicle(player.entity_id())?;
        let tags = level.chunks.registry_tags_shared().map_or(
            crate::entity::EntityRegistryTagView::Builtin26_1,
            crate::entity::EntityRegistryTagView::Frozen,
        );
        let facts = Self::nearby_player_facts(player);
        let controller = crate::entity::direct_controlling_player_with_registry_tags(
            entities.get(vehicle_id)?,
            Some(entities),
            &tags,
            |entity_id| (entity_id == facts.entity_id).then_some(facts),
        )??;
        (controller.entity_id == player.entity_id()).then_some(vehicle_id)
    }

    pub(super) fn external_player_carrier_roots(
        &self,
        dimension: &LevelId,
    ) -> std::collections::BTreeSet<i32> {
        self.levels
            .players()
            .filter(|player| &player.dimension == dimension && !player.avatar_removed)
            .filter_map(|player| {
                self.level_by_id(dimension)
                    .entities
                    .external_player_root_vehicle(player.entity_id())
            })
            .collect()
    }

    fn external_player_vehicle_collision_snapshot(
        &self,
        dimension: &LevelId,
        vehicle_entity_id: i32,
        player_entity_id: i32,
        target_position: Vec3d,
    ) -> CollisionSnapshot {
        let level = self.level_by_id(dimension);
        // `Entity.canCollideWith` excludes the whole same-root passenger tree
        // (`Entity.java:2297-2298`, `:3489-3499`).
        let mut related_entities = std::collections::BTreeSet::from([vehicle_entity_id]);
        let mut pending = vec![vehicle_entity_id];
        while let Some(parent) = pending.pop() {
            for &passenger in level.entities.passengers(parent) {
                if related_entities.insert(passenger) {
                    pending.push(passenger);
                }
            }
        }
        let (is_boat, query, entity_boxes) = if let Some(vehicle) =
            level.entities.get(vehicle_entity_id)
        {
            let is_boat = crate::entity::boat::is_boat(vehicle.data());
            let old_box = vehicle.bounding_box();
            let delta = subtract(target_position, vehicle.position());
            let query = old_box.expand_towards(delta).inflate(1.0, 2.0, 1.0);
            let boxes = if is_boat {
                // `AbstractBoat.canVehicleCollide` (`AbstractBoat.java:109-115`):
                // boats collide with `canBeCollidedWith || isPushable` entities outside the tree.
                let query_inflated = query.inflate(1.0e-7, 1.0e-7, 1.0e-7);
                let tags = level.chunks.registry_tags();
                let mut boat_boxes: Vec<_> = level
                    .entities
                    .ids_in_box_sections(query_inflated)
                    .into_iter()
                    .filter(|id| !related_entities.contains(id))
                    .filter_map(|id| level.entities.get(id))
                    .filter(|entity| {
                        entity.bounding_box().intersects(query_inflated)
                            && level
                                .entities
                                .section_visibility(crate::entity::store::section_of_block(
                                    entity.block_position(),
                                ))
                                .is_accessible()
                            && (entity.can_be_collided_with()
                                || match crate::entity::push::pushable_short_of_climbing(entity) {
                                    Some(true) => {
                                        !crate::entity::general_living::is_abstract_horse_entity_type(
                                            entity.entity_type_id(),
                                        ) || level.entities.passengers(entity.id()).is_empty()
                                    }
                                    Some(false) => false,
                                    None => !crate::entity::living::on_climbable_in_blocks(
                                        entity.block_position(),
                                        entity.is_fall_flying(),
                                        |pos| {
                                            if level.chunks.is_inside_build_height(pos.y) {
                                                level.chunks.get_block_state(pos.x, pos.y, pos.z)
                                            } else {
                                                Some(level.chunks.air_state_id())
                                            }
                                        },
                                        |state, tag| {
                                            bbb_protocol::block_state_types::vanilla_block_state_block_type_id(
                                                state,
                                            )
                                            .is_some_and(|block_id| {
                                                tags.is_some_and(|t| t.contains_block(tag, block_id))
                                            })
                                        },
                                    )
                                    .unwrap_or(false),
                                })
                    })
                    .map(|entity| (entity.id(), entity.bounding_box()))
                    .collect();
                boat_boxes.sort_unstable_by_key(|&(id, _)| id);
                boat_boxes.into_iter().map(|(_, bounds)| bounds).collect()
            } else {
                level_entity_boxes_in(level, query)
                    .into_iter()
                    .filter(|(id, _)| !related_entities.contains(id))
                    .map(|(_, bounds)| bounds)
                    .collect()
            };
            (is_boat, Some(query.inflate(1.0e-7, 1.0e-7, 1.0e-7)), boxes)
        } else {
            (false, None, Vec::new())
        };
        // `Entity.canCollideWith` (`Entity.java:2297-2303`) vs `AbstractBoat.canVehicleCollide`
        // (`AbstractBoat.java:109-115`): players have `canBeCollidedWith == false`, so only
        // boats collide with pushable players; living mounts never do.
        let player_boxes = if let (true, Some(query)) = (is_boat, query) {
            self.levels
                .players()
                .filter(|player| {
                    &player.dimension == dimension
                        && !player.avatar_removed
                        && player.game_type != GameType::Spectator
                        && !player.damage.is_dead_or_dying()
                        && player.cold.sleeping_pos.is_none()
                        && player.entity_id() != player_entity_id
                        && level
                            .entities
                            .external_player_root_vehicle(player.entity_id())
                            != Some(vehicle_entity_id)
                })
                .filter_map(|player| player.bounding_box().ok())
                .filter(|bounds| bounds.intersects(query))
                .collect()
        } else {
            Vec::new()
        };
        CollisionSnapshot {
            entity_boxes,
            player_boxes,
            border: level.world_border,
            game_time: self.game_time(),
        }
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn external_player_vehicle_listener_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> (Option<i32>, Vec3d, Vec3d, bool) {
        let player = self.levels.player_at(connection_id);
        (
            player.cold.last_vehicle,
            player.cold.vehicle_first_good,
            player.cold.vehicle_last_good,
            player.cold.client_vehicle_is_floating,
        )
    }

    pub(crate) fn boat_control_state_for_test(
        &self,
        dimension: super::Java26BuiltinDimension,
        entity_id: i32,
    ) -> (Vec3d, f32, f32, (bool, bool)) {
        let entity = self
            .level(dimension)
            .entities
            .get(entity_id)
            .expect("live Boat");
        (
            entity.position(),
            entity.y_rot(),
            entity.x_rot(),
            crate::entity::boat::boat(entity.data())
                .expect("Boat data")
                .paddle_state(),
        )
    }

    pub(crate) fn boat_out_of_control_ticks_for_test(
        &self,
        dimension: super::Java26BuiltinDimension,
        entity_id: i32,
    ) -> f32 {
        let entity = self
            .level(dimension)
            .entities
            .get(entity_id)
            .expect("live Boat");
        crate::entity::boat::boat(entity.data())
            .expect("Boat data")
            .out_of_control_ticks()
    }

    pub(crate) fn external_player_vehicle_placement_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> Option<ExternalPlayerVehiclePlacement> {
        let player = self.levels.player_at(connection_id);
        self.level_by_id(&player.dimension)
            .entities
            .external_player_vehicle_placement_with_rider(
                player.entity_id(),
                player.rotation().y_rot,
                player.pose_scale(),
                player.pose_id,
            )
    }

    pub(crate) fn set_world_border_for_test(
        &mut self,
        dimension: super::Java26BuiltinDimension,
        border: bbb_server_types::interaction::WorldBorder,
    ) {
        self.level_mut(dimension).world_border = border;
    }

    pub(crate) fn set_external_player_vehicle_floating_ticks_for_test(
        &mut self,
        connection_id: ConnectionId,
        ticks: i32,
    ) {
        self.levels[connection_id]
            .cold
            .above_ground_vehicle_tick_count = ticks;
    }

    pub(crate) fn set_player_fall_distance_for_test(
        &mut self,
        connection_id: ConnectionId,
        distance: f64,
    ) {
        self.levels[connection_id].block_contact.fall_distance = distance;
    }

    pub(crate) fn external_player_boat_distance_stat_for_test(
        &self,
        connection_id: ConnectionId,
    ) -> i32 {
        self.levels[connection_id]
            .cold
            .stats
            .value(custom_stat("minecraft:boat_one_cm"))
    }

    pub(crate) fn set_external_player_last_vehicle_for_test(
        &mut self,
        connection_id: ConnectionId,
        vehicle_entity_id: Option<i32>,
    ) {
        self.levels[connection_id].cold.last_vehicle = vehicle_entity_id;
    }

    pub(crate) fn spawn_boat_with_stored_passenger_for_test(
        &mut self,
        dimension: super::Java26BuiltinDimension,
        position: Vec3d,
    ) -> Option<(i32, i32)> {
        let boat_type = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID;
        let plan = crate::entity::AtomicSpawnPlan {
            tree: crate::entity::SpawnEntityTree {
                entity_type_id: boat_type,
                resource_id: "minecraft:oak_boat".to_owned(),
                entity_nbt: vec![bbb_protocol::NbtSummaryEntry {
                    name: "id".into(),
                    value: bbb_protocol::NbtSummaryValue::string("minecraft:oak_boat"),
                }],
                passengers: vec![crate::entity::SpawnEntityTree {
                    entity_type_id: bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
                    resource_id: "minecraft:armor_stand".to_owned(),
                    entity_nbt: vec![bbb_protocol::NbtSummaryEntry {
                        name: "id".into(),
                        value: bbb_protocol::NbtSummaryValue::String(
                            "minecraft:armor_stand".into(),
                        ),
                    }],
                    passengers: Vec::new(),
                }],
            },
            position,
            spawn_block_position: block_position(position),
            spawner_position: block_position(position),
            collision_box: bbb_server_types::geometry::Aabb::of_entity(position, 1.375, 0.5625),
            custom_spawn_rules: None,
            placement_probe: None,
            finalize_root_mob: false,
            equipment: None,
            nearby_box: bbb_server_types::geometry::Aabb::of_entity(position, 1.375, 0.5625),
            max_nearby_entities: 6,
        };
        let entities = &mut self.level_mut(dimension).entities;
        let boat = crate::entity::spawn_entity_tree(entities, &plan)?;
        let passenger = *entities.passengers(boat).first()?;
        Some((boat, passenger))
    }
}

fn resolve_vehicle_collision(
    chunks: &crate::chunk::ChunkStore,
    snapshot: &CollisionSnapshot,
    probe: ExternalPlayerBoatCollisionProbe,
) -> ExternalPlayerBoatCollisionResolution {
    // ServerGamePacketListenerImpl.java:475 -> Entity.java:1075-1104.
    let extra = snapshot.collision_boxes();
    let collect = |query| {
        let mut boxes = block_collision_boxes(chunks, query);
        boxes.extend_from_slice(&extra);
        boxes
    };
    let clipped = if probe.requested_delta.length_sqr() == 0.0 {
        probe.requested_delta
    } else {
        let swept = probe.old_bounding_box.expand_towards(probe.requested_delta);
        collide_with_boxes(
            probe.requested_delta,
            probe.old_bounding_box,
            &collect(swept),
        )
    };
    let resolved_delta = collide_with_step(
        probe.old_bounding_box,
        probe.requested_delta,
        clipped,
        probe.on_ground,
        probe.max_up_step,
        |query| Some(collect(query)),
    )
    .expect("vehicle collision collection always succeeds");
    let resolved_position = add(probe.old_position, resolved_delta);
    let target_delta = subtract(probe.target_position, probe.old_position);
    let target_box = probe
        .old_bounding_box
        .moved(target_delta)
        .inflate(-1.0e-5, -1.0e-5, -1.0e-5);
    let old_box = probe.old_bounding_box.inflate(-1.0e-5, -1.0e-5, -1.0e-5);
    let old_position_clear = !block_collision_boxes(chunks, probe.old_bounding_box)
        .into_iter()
        .chain(extra.iter().copied())
        .any(|shape| shape.intersects(probe.old_bounding_box));
    let collides_with_new_geometry = block_collision_boxes(chunks, target_box)
        .into_iter()
        .chain(extra)
        .any(|shape| shape.intersects(target_box) && !shape.intersects(old_box));
    ExternalPlayerBoatCollisionResolution {
        resolved_position,
        horizontal_collision: resolved_delta.x != probe.requested_delta.x
            || resolved_delta.z != probe.requested_delta.z,
        old_position_clear,
        collides_with_new_geometry,
    }
}

/// The stat `ServerPlayer.checkRidingStatistics` counts a ride on `vehicle`
/// under (`ServerPlayer.java:1470-1490`).
fn riding_stat(vehicle: &crate::entity::Entity) -> Option<&'static str> {
    use bbb_protocol::entity_types::*;
    if crate::entity::minecart::is_minecart(vehicle.data()) {
        return Some("minecraft:minecart_one_cm");
    }
    if crate::entity::boat::is_boat(vehicle.data()) {
        return Some("minecraft:boat_one_cm");
    }
    Some(match vehicle.entity_type_id() {
        VANILLA_ENTITY_TYPE_PIG_ID => "minecraft:pig_one_cm",
        VANILLA_ENTITY_TYPE_STRIDER_ID => "minecraft:strider_one_cm",
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => "minecraft:happy_ghast_one_cm",
        VANILLA_ENTITY_TYPE_NAUTILUS_ID | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
            "minecraft:nautilus_one_cm"
        }
        _ if matches!(vehicle.data(), crate::entity::EntityData::GeneralLiving(data)
            if data.abstract_horse_retained().is_some()) =>
        {
            "minecraft:horse_one_cm"
        }
        _ => return None,
    })
}

fn award_riding_distance(
    player: &mut super::PlayerState,
    scoreboard: &mut super::server_scoreboard::ServerScoreboard,
    stat: Option<&str>,
    delta: Vec3d,
) {
    // `ServerPlayer.rideTick` and `checkRidingStatistics`
    // (`ServerPlayer.java:1412-1418`, `:1471-1492`).
    if let Some(stat) = stat.filter(|_| !did_not_move(delta)) {
        let length = (delta.x * delta.x + delta.y * delta.y + delta.z * delta.z).sqrt() as f32;
        player.award_stat(
            scoreboard,
            custom_stat(stat),
            (length * 100.0).round() as i32,
        );
    }
}

/// `ServerPlayer.didNotMove` (`ServerPlayer.java:1492-1494`), which is the
/// whole of `checkRidingStatistics`'s guard (`:1470-1471`).
///
/// A rider that moved by less than half a centimetre still awards, so the
/// counter appears in the file at zero; only an exactly stationary tick is
/// skipped.
fn did_not_move(delta: Vec3d) -> bool {
    delta.x == 0.0 && delta.y == 0.0 && delta.z == 0.0
}

fn eye_in_water(level: &ServerLevel, eye: Vec3d) -> bool {
    let pos = block_position(eye);
    if pos.y < level.chunks.min_y()
        || pos.y >= level.chunks.min_y() + level.chunks.section_count() as i32 * 16
    {
        return false;
    }
    let state = stored_block_state(&level.chunks, pos);
    let fluid = block_state_fluid_state(state);
    fluid.family() == Some(FluidFamily::Water)
        && eye.y < f64::from(pos.y) + f64::from(fluid.own_height())
}

fn border_contains_position(
    border: bbb_server_types::interaction::WorldBorder,
    position: Vec3d,
    game_time: i64,
) -> bool {
    position.x >= border.min_x_at(game_time)
        && position.x < border.max_x_at(game_time)
        && position.z >= border.min_z_at(game_time)
        && position.z < border.max_z_at(game_time)
}

fn vehicle_packet_has_invalid_values(update: MoveVehicleCommand) -> bool {
    // `ServerGamePacketListenerImpl.java:426-443`: infinities reach the clamp,
    // while NaN coordinates and non-finite rotations end the connection.
    update.position.x.is_nan()
        || update.position.y.is_nan()
        || update.position.z.is_nan()
        || !update.y_rot.is_finite()
        || !update.x_rot.is_finite()
}

#[cfg(test)]
mod tests {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_PIG_ID,
    };
    use bbb_protocol::packets::{
        BlockPos, DataComponentPatchSummary, InteractEntity, InteractionHand, ItemStackSummary,
        MoveVehicleCommand, PlayServerbound, Vec3d,
    };
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    use bbb_server_types::save::Java26BuiltinDimension;

    use crate::connection::{ClientboundPlayPacket, ConnectionId};
    use crate::entity::{BoatData, EntityData, GeneralLivingData};
    use crate::mailbox::{WorldCommandReceiver, WorldCommandSender};
    use crate::world::{ServerWorld, TickResult, WorldCommand, WorldEvent};

    const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

    fn block_state(name: &str) -> i32 {
        let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .expect("built-in block");
        bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id)
            .expect("built-in block default state")
    }

    fn stack_of(resource_id: &str, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: (count > 0).then(|| {
                bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource_id)
                    .expect("built-in item")
                    .id
            }),
            count,
            component_patch: DataComponentPatchSummary::default().into(),
        }
    }

    fn play(
        sender: &WorldCommandSender,
        receiver: &WorldCommandReceiver,
        world: &mut ServerWorld,
        packet: PlayServerbound,
    ) -> TickResult {
        sender
            .try_send(WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet,
            })
            .unwrap();
        world.tick(receiver.drain())
    }

    fn interact(entity_id: i32) -> PlayServerbound {
        PlayServerbound::Interact(InteractEntity {
            entity_id,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        })
    }

    fn correction_sent(tick: &TickResult) -> bool {
        tick.events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::MoveVehicle(_),
                }
            )
        })
    }

    fn accept_mount_teleport(
        sender: &WorldCommandSender,
        receiver: &WorldCommandReceiver,
        world: &mut ServerWorld,
        mounted: &TickResult,
    ) {
        let id = mounted
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::PlayerPosition(update),
                } => Some(update.id),
                _ => None,
            })
            .expect("mount sends player teleport");
        play(
            sender,
            receiver,
            world,
            PlayServerbound::AcceptTeleportation { id },
        );
    }

    fn step_arena(
        obstacle_one_and_half: bool,
    ) -> (
        WorldCommandSender,
        WorldCommandReceiver,
        ServerWorld,
        BlockPos,
    ) {
        let mut config = crate::tests::datapack_config();
        config.game_type = bbb_protocol::game_types::GameType::Survival;
        let mut world = ServerWorld::new(config).unwrap();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let player_pos = world.publication().players[0].position;
        let base = BlockPos {
            x: player_pos.x.floor() as i32,
            y: player_pos.y.floor() as i32,
            z: player_pos.z.floor() as i32,
        };
        let stone = block_state("minecraft:stone");
        let slab = block_state("minecraft:smooth_stone_slab");
        let air = block_state("minecraft:air");
        for x in (base.x - 4)..=(base.x + 6) {
            for z in (base.z - 4)..=(base.z + 6) {
                world.set_block(
                    DIM,
                    BlockPos {
                        x,
                        y: base.y - 1,
                        z,
                    },
                    stone,
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                );
                for y in base.y..=(base.y + 5) {
                    world.set_block(
                        DIM,
                        BlockPos { x, y, z },
                        air,
                        crate::fluid::SET_BLOCK_UPDATE_ALL,
                    );
                }
            }
        }
        let obstacle = BlockPos {
            x: base.x + 2,
            y: base.y,
            z: base.z,
        };
        world.set_block(DIM, obstacle, stone, crate::fluid::SET_BLOCK_UPDATE_ALL);
        if obstacle_one_and_half {
            world.set_block(
                DIM,
                BlockPos {
                    y: base.y + 1,
                    ..obstacle
                },
                slab,
                crate::fluid::SET_BLOCK_UPDATE_ALL,
            );
        }
        (sender, receiver, world, base)
    }

    enum MountKind {
        Boat,
        Horse,
        Camel,
        Pig,
    }

    fn drive_across_obstacle(kind: MountKind, obstacle_one_and_half: bool, falling: bool) -> bool {
        let (sender, receiver, mut world, base) = step_arena(obstacle_one_and_half);
        let spawn_pos = Vec3d {
            x: f64::from(base.x) + 0.8,
            y: f64::from(base.y),
            z: f64::from(base.z) + 0.5,
        };
        let vehicle = match kind {
            MountKind::Boat => {
                let id = world
                    .spawn_stored_entity_for_test(
                        DIM,
                        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
                        spawn_pos,
                        EntityData::Boat(BoatData::new()),
                    )
                    .0;
                world.apply_level_entity_lifecycle_for_test(DIM, &mut Vec::new());
                let mounted = play(&sender, &receiver, &mut world, interact(id));
                accept_mount_teleport(&sender, &receiver, &mut world, &mounted);
                id
            }
            MountKind::Horse => {
                let id = world
                    .spawn_stored_entity_for_test(
                        DIM,
                        VANILLA_ENTITY_TYPE_HORSE_ID,
                        spawn_pos,
                        EntityData::GeneralLiving(
                            GeneralLivingData::new(VANILLA_ENTITY_TYPE_HORSE_ID).unwrap(),
                        ),
                    )
                    .0;
                let NbtSummaryValue::Compound(mut entries) =
                    world.stored_entity_command_data_for_test(DIM, id).unwrap()
                else {
                    panic!("compound");
                };
                entries.retain(|entry| entry.name != "Tame");
                entries.push(NbtSummaryEntry {
                    name: "Tame".into(),
                    value: NbtSummaryValue::Byte(1),
                });
                world.load_stored_entity_record_for_test(DIM, id, &entries);
                world.set_inventory_slot_for_test(
                    ConnectionId(1),
                    0,
                    stack_of("minecraft:saddle", 1),
                );
                play(&sender, &receiver, &mut world, interact(id));
                world.set_inventory_slot_for_test(
                    ConnectionId(1),
                    0,
                    stack_of("minecraft:stick", 1),
                );
                let mounted = play(&sender, &receiver, &mut world, interact(id));
                accept_mount_teleport(&sender, &receiver, &mut world, &mounted);
                id
            }
            MountKind::Camel => {
                let id = world
                    .spawn_stored_entity_for_test(
                        DIM,
                        VANILLA_ENTITY_TYPE_CAMEL_ID,
                        spawn_pos,
                        EntityData::GeneralLiving(
                            GeneralLivingData::new(VANILLA_ENTITY_TYPE_CAMEL_ID).unwrap(),
                        ),
                    )
                    .0;
                world.set_inventory_slot_for_test(
                    ConnectionId(1),
                    0,
                    stack_of("minecraft:saddle", 1),
                );
                play(&sender, &receiver, &mut world, interact(id));
                world.set_inventory_slot_for_test(ConnectionId(1), 0, stack_of("minecraft:air", 0));
                let mounted = play(&sender, &receiver, &mut world, interact(id));
                accept_mount_teleport(&sender, &receiver, &mut world, &mounted);
                id
            }
            MountKind::Pig => {
                let id = world
                    .spawn_stored_entity_for_test(
                        DIM,
                        VANILLA_ENTITY_TYPE_PIG_ID,
                        spawn_pos,
                        EntityData::GeneralLiving(
                            GeneralLivingData::new(VANILLA_ENTITY_TYPE_PIG_ID).unwrap(),
                        ),
                    )
                    .0;
                world.set_inventory_slot_for_test(
                    ConnectionId(1),
                    0,
                    stack_of("minecraft:saddle", 1),
                );
                play(&sender, &receiver, &mut world, interact(id));
                world.set_inventory_slot_for_test(
                    ConnectionId(1),
                    0,
                    stack_of("minecraft:carrot_on_a_stick", 1),
                );
                let mounted = play(&sender, &receiver, &mut world, interact(id));
                accept_mount_teleport(&sender, &receiver, &mut world, &mounted);
                id
            }
        };

        let start = Vec3d {
            y: if falling {
                spawn_pos.y + 0.25
            } else {
                spawn_pos.y
            },
            ..spawn_pos
        };
        let settled = play(
            &sender,
            &receiver,
            &mut world,
            PlayServerbound::MoveVehicle(MoveVehicleCommand {
                position: start,
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: !falling,
            }),
        );
        assert!(!correction_sent(&settled));

        // If `falling`, `requested_delta.y == -0.25` collides with a temporary
        // support step under `start` (`grounded_after == true` with `on_ground == false`).
        if falling {
            let slab = block_state("minecraft:smooth_stone_slab");
            for dx in -1..=1 {
                for dz in -1..=1 {
                    world.set_block(
                        DIM,
                        BlockPos {
                            x: base.x + dx,
                            y: base.y - 1,
                            z: base.z + dz,
                        },
                        block_state("minecraft:stone"),
                        crate::fluid::SET_BLOCK_UPDATE_ALL,
                    );
                }
            }
            // Lower destination floor by half a block so target `y = base.y - 0.5`
            // has `requested_delta.y = -0.75 < 0` while clamping on `base.y` (`clipped.y = -0.25`).
            for dx in 2..=5 {
                for dz in 1..=5 {
                    world.set_block(
                        DIM,
                        BlockPos {
                            x: base.x + dx,
                            y: base.y - 1,
                            z: base.z + dz,
                        },
                        slab,
                        crate::fluid::SET_BLOCK_UPDATE_ALL,
                    );
                }
            }
        }

        let target = Vec3d {
            x: f64::from(base.x) + 3.3,
            y: if falling {
                f64::from(base.y) - 0.5
            } else {
                f64::from(base.y)
            },
            z: f64::from(base.z) + 2.6,
        };
        let moved = play(
            &sender,
            &receiver,
            &mut world,
            PlayServerbound::MoveVehicle(MoveVehicleCommand {
                position: target,
                y_rot: 0.0,
                x_rot: 0.0,
                on_ground: true,
            }),
        );
        let corrected = correction_sent(&moved);
        if !corrected {
            assert_eq!(
                world
                    .stored_entity_for_test(DIM, vehicle)
                    .unwrap()
                    .position(),
                target
            );
        }
        corrected
    }

    #[test]
    fn move_vehicle_applies_living_step_height_and_keeps_boats_unstepped() {
        // Boat (`Entity.maxUpStep == 0.0F`) collides horizontally with a 1-block step.
        assert!(drive_across_obstacle(MountKind::Boat, false, false));

        // Horse (`maxUpStep == 1.0F`) steps up a 1-block obstacle without rubberbanding
        // both when grounded (`delta.y == 0`) and when landing (`delta.y < 0`),
        // but is blocked by a 1.5-block obstacle.
        assert!(!drive_across_obstacle(MountKind::Horse, false, false));
        assert!(!drive_across_obstacle(MountKind::Horse, false, true));
        assert!(drive_across_obstacle(MountKind::Horse, true, false));

        // Camel (`Camel.java:120`, `maxUpStep == 1.5F`) clears both 1.0 and 1.5 obstacles.
        assert!(!drive_across_obstacle(MountKind::Camel, false, false));
        assert!(!drive_across_obstacle(MountKind::Camel, true, false));
        assert!(!drive_across_obstacle(MountKind::Camel, true, true));

        // Pig (`step_height == 0.6`, clamped to `1.0F` when player-controlled,
        // `LivingEntity.java:3910-3913`) also clears a 1-block obstacle.
        assert!(!drive_across_obstacle(MountKind::Pig, false, false));
    }
}
