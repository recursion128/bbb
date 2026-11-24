//! The walking half of `ServerGamePacketListenerImpl.handleMovePlayer`: the
//! `player.move` collision probe behind moved-wrongly and the floating kick it
//! feeds.

use crate::connection::ConnectionId;
use crate::world::LevelId;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_GRAVITY_ID;
use bbb_protocol::packets::{BlockPos, MovePlayer, Vec3d};
use bbb_server_types::geometry::{
    collide_with_boxes, no_block_collision_with_scratch,
    visit_block_collision_shapes_checked_with_scratch, BoxShape, Vec3Ops,
};
use bbb_server_types::interaction::player_pose_bounding_box;

use super::movement_collision::{
    add, level_entity_boxes_in, maximum_flying_ticks, moving_piston_at, no_blocks_around_checked,
    push_player_block_collision_shape_checked, push_player_block_collision_shape_with_context,
    subtract, CollisionSnapshot,
};
use super::player_packets::clamped_target;
use super::{GameType, PlayerState, ServerWorld, WorldCloseReason, WorldEvent};

mod emission;

/// `ServerGamePacketListenerImpl.clientIsFloating` (`:261`) and the two flags
/// beside it that outlive one packet: `aboveGroundTickCount` (`:262`) and
/// `Entity.verticalCollisionBelow`, which `Entity.move` only rewrites on a move
/// with a vertical component (`Entity.java:757-761`).
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct MovementProbeState {
    pub(super) client_is_floating: bool,
    pub(super) above_ground_tick_count: i32,
    pub(super) vertical_collision_below: bool,
}

/// What one `player.move(MoverType.PLAYER, delta)` probe found
/// (`ServerGamePacketListenerImpl.java:1111-1145`).
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PlayerMoveProbe {
    pub(super) packet_start: Option<Vec3d>,
    pub(super) resolved_position: Option<Vec3d>,
    pub(super) prefix_applied: bool,
    pub(super) updates_ground: bool,
    pub(super) solved_delta: Vec3d,
    pub(super) jump_factor: Option<f32>,
    pub(super) collision_response: Option<PlayerMoveCollisionResponse>,
    pub(super) contact_movement: Option<crate::entity::EntityMovement>,
    pub(super) used_stuck_multiplier: bool,
    pub(super) reset_fall_distance: bool,
    /// `movedDist` (`:1129-1130`): how far the client's target is from where
    /// the server's own collision solve put the player. Vanilla zeroes the y
    /// term with an always-true test, so only x and z count.
    pub(super) moved_distance: f64,
    /// `level.noCollision(player, oldAABB)`: the box the session occupies
    /// is already inside geometry, which forgives a failed move (`:1144`).
    pub(super) old_position_clear: bool,
    /// `isEntityCollidingWithAnythingNew` (`:1216-1233`).
    pub(super) collides_with_new_geometry: bool,
    /// `Entity.verticalCollisionBelow` as the previous probe left it, which is
    /// what `playerStandsOnSomething` reads before the move (`:1120`).
    pub(super) vertical_collision_below: bool,
    /// `noBlocksAround(player)` at the target (`:1156`).
    pub(super) no_blocks_around: bool,
    /// The `verticalCollisionBelow` this probe's own solve produced, which the
    /// packet stores whether or not it is accepted (`Entity.java:757-761`).
    pub(super) next_vertical_collision_below: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PlayerMoveCollisionResponse {
    pub(super) clip_x: bool,
    pub(super) clip_z: bool,
    pub(super) vertical_collision: bool,
    pub(super) fall_state: Option<i32>,
    pub(super) supporting: Option<BlockPos>,
    pub(super) on_ground_no_blocks: bool,
}

impl PlayerMoveProbe {
    /// `clientIsFloating` (`:1148-1156`) for the packet's vertical delta.
    ///
    /// `server.allowFlight()` is applied where the floating count is read
    /// ([`ServerWorld::tick_player_floating`]).
    pub(super) fn client_is_floating(
        self,
        player: &PlayerState,
        vertical_delta: f64,
        fall_flying: bool,
    ) -> bool {
        vertical_delta >= -0.03125
            && !self.vertical_collision_below
            && player.game_type != GameType::Spectator
            && !player.abilities.can_fly
            && !player
                .has_effect_id(bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_LEVITATION_ID)
            && !fall_flying
            && !player.using_item.is_auto_spin_attack()
            && self.no_blocks_around
    }
}

impl ServerWorld {
    #[cfg(test)]
    pub(crate) fn start_player_auto_spin_attack_for_test(
        &mut self,
        connection_id: ConnectionId,
        ticks: i32,
    ) {
        self.levels[connection_id]
            .using_item
            .start_auto_spin_attack(
                ticks,
                8.0,
                bbb_server_types::item_stack::RuntimeItemStack::empty(),
            );
    }

    /// `player.move(MoverType.PLAYER, new Vec3(xDist, yDist, zDist))` and the
    /// `movedDist` it is measured by (`:1111-1130`), plus the two collision
    /// questions the accept condition asks (`:1142-1145`) and the floating
    /// inputs (`:1148-1156`).
    ///
    #[cfg(test)]
    pub(super) fn player_move_probe(
        &self,
        connection_id: ConnectionId,
        movement: &MovePlayer,
    ) -> Option<PlayerMoveProbe> {
        let mut probe = self.player_move_physics_probe(connection_id, movement)?;
        self.finish_player_move_support(connection_id, &mut probe)?;
        self.finish_player_collision_queries(connection_id, movement, &mut probe)?;
        self.finish_player_floating_query(connection_id, movement, &mut probe)?;
        Some(probe)
    }

    pub(super) fn player_move_physics_probe(
        &self,
        target: ConnectionId,
        movement: &MovePlayer,
    ) -> Option<PlayerMoveProbe> {
        let player = self.levels.player(target)?;
        let position = movement
            .position
            .map(clamped_target)
            .unwrap_or(player.position());
        self.player_move_physics_probe_for_mover(
            target,
            position,
            subtract(position, player.last_good),
            movement.on_ground,
            crate::entity::MoverType::Player,
        )
    }

    pub(in crate::world) fn move_player_by_external_force(
        &mut self,
        id: ConnectionId,
        delta: Vec3d,
        mover: crate::entity::MoverType,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        use super::player_packets::{
            apply_player_move_body_prefix, apply_player_move_velocity_prefix,
        };
        let previous = self.levels.player(id)?.position();
        let moved = (|| {
            if delta != Vec3d::default() || self.levels.player(id)?.game_type == GameType::Spectator
            {
                let target = add(previous, delta);
                let mut probe = self.player_move_physics_probe_for_mover(
                    id,
                    target,
                    delta,
                    self.levels.player(id)?.on_ground(),
                    mover,
                )?;
                if probe.reset_fall_distance {
                    self.reset_player_fall_distance(id, events);
                }
                apply_player_move_body_prefix(self.levels.player_mut(id)?, &probe)?;
                self.update_player_query_section(id);
                self.finish_player_move_support(id, &mut probe)?;
                if !self.levels.player(id)?.avatar_removed {
                    let content_registry =
                        std::sync::Arc::clone(self.registries.content_registry_arc());
                    apply_player_move_velocity_prefix(
                        self.levels.player_mut(id)?,
                        &probe,
                        &content_registry,
                    )?;
                    self.finish_player_move_emission(id, &probe, events)?;
                    self.finish_player_move_speed(id)?;
                }
            }
            let position = self.levels.player(id)?.position();
            self.apply_player_block_effect_movements_checked(
                id,
                &[crate::entity::EntityMovement::direct(previous, position)],
                events,
            )?;
            Some(())
        })();
        self.levels.player_mut(id)?.block_contact.pending.pop_back();
        moved
    }

    pub(in crate::world) fn player_move_physics_probe_for_mover(
        &self,
        connection_id: ConnectionId,
        target: Vec3d,
        mut delta: Vec3d,
        on_ground: bool,
        mover: crate::entity::MoverType,
    ) -> Option<PlayerMoveProbe> {
        let player = self.levels.player(connection_id)?;
        let dimension = player.dimension;
        let level = self.level_by_id(&dimension);
        let old_box =
            player_pose_bounding_box(player.position(), player.pose_id, player.pose_scale())
                .ok()?;
        let stuck = player.block_contact.stuck_multiplier;
        let used_stuck_multiplier =
            player.game_type != GameType::Spectator && stuck.length_sqr() > 1.0e-7;
        if used_stuck_multiplier && mover != crate::entity::MoverType::Piston {
            delta = Vec3d {
                x: delta.x * stuck.x,
                y: delta.y * stuck.y,
                z: delta.z * stuck.z,
            };
        }
        if player.game_type != GameType::Spectator
            && matches!(
                mover,
                crate::entity::MoverType::Self_ | crate::entity::MoverType::Player
            )
        {
            delta = self.player_back_off_from_edge(connection_id, delta, old_box)?;
        }
        let resolved_delta = if player.game_type == GameType::Spectator {
            delta
        } else {
            let query = old_box.expand_towards(delta);
            // Bound swept AABB block volume as defense-in-depth against huge deltas.
            const MAX_SWEEP_COLLISION_BLOCKS: u64 = 65_536;
            let span_x = (query.max_x.ceil() - query.min_x.floor()).max(0.0) as u64 + 1;
            let span_y = (query.max_y.ceil() - query.min_y.floor()).max(0.0) as u64 + 1;
            let span_z = (query.max_z.ceil() - query.min_z.floor()).max(0.0) as u64 + 1;
            if span_x.saturating_mul(span_y).saturating_mul(span_z) > MAX_SWEEP_COLLISION_BLOCKS {
                return None;
            }
            let step_height = player
                .attributes
                .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_STEP_HEIGHT_ID)?
                as f32;
            let sweep_with_step = query.expand_towards(Vec3d {
                x: 0.0,
                y: f64::from(step_height.max(0.0)),
                z: 0.0,
            });
            let snapshot =
                self.player_collision_snapshot_in(&dimension, connection_id, sweep_with_step);
            let initial_entities: Vec<_> = snapshot
                .entity_boxes
                .iter()
                .copied()
                .filter(|part| part.intersects(query.inflate(1.0e-7, 1.0e-7, 1.0e-7)))
                .collect();
            let push_block_boxes = |bounds, boxes: &mut Vec<_>| {
                visit_block_collision_shapes_checked_with_scratch(
                    bounds,
                    |pos| moving_piston_at(&level.chunks, pos),
                    |pos, out| {
                        push_player_block_collision_shape_checked(&level.chunks, player, pos, out)
                    },
                    |_, shape| {
                        boxes.extend(shape.boxes());
                        Some(true)
                    },
                )?;
                Some(())
            };
            let collect = |query| {
                let mut boxes = initial_entities.clone();
                boxes.extend(level.world_border.collision_boxes_at(
                    player.position(),
                    query,
                    self.game_time(),
                ));
                push_block_boxes(query, &mut boxes)?;
                Some(boxes)
            };
            let clipped = if delta.length_sqr() == 0.0 {
                delta
            } else {
                collide_with_boxes(delta, old_box, &collect(query)?)
            };
            bbb_server_types::geometry::collide_with_step(
                old_box,
                delta,
                clipped,
                player.on_ground(),
                step_height,
                collect,
            )?
        };
        let movement_length = resolved_delta.length_sqr();
        // Entity.java:734-750: a tiny clipped result does not update the body's position.
        let updates_position =
            movement_length > 1.0e-7 || delta.length_sqr() - movement_length < 1.0e-7;
        let resolved = if updates_position {
            add(player.position(), resolved_delta)
        } else {
            player.position()
        };
        // Entity.java:735-743: the reset ray follows the solved movement, capped at eight blocks.
        let reset_fall_distance = if player.game_type != GameType::Spectator
            && player.block_contact.fall_distance != 0.0
            && movement_length >= 1.0
        {
            let length = movement_length.sqrt();
            let distance = length.min(8.0);
            let check_to = add(
                player.position(),
                Vec3d {
                    x: resolved_delta.x / length * distance,
                    y: resolved_delta.y / length * distance,
                    z: resolved_delta.z / length * distance,
                },
            );
            let instant_portal = self
                .admin
                .game_rule_value("minecraft:players_nether_portal_default_delay")
                == Some(bbb_protocol::game_rules::GameRuleValue::Integer(0));
            super::pov_clip::fall_resetting_clip(
                &level.chunks,
                player.position(),
                check_to,
                instant_portal,
            )?
        } else {
            false
        };

        let x_dist = target.x - resolved.x;
        let z_dist = target.z - resolved.z;
        let moved_distance = x_dist * x_dist + z_dist * z_dist;
        // Entity.java:754-761: the horizontal axes use Mth.equal, Y uses exact equality.
        let vertical_collision = delta.y != resolved_delta.y;
        let collision_response = if player.game_type == GameType::Spectator {
            None
        } else {
            let supporting = player.block_contact.supporting;
            let no_blocks = player.block_contact.on_ground_no_blocks;
            let fall_state = None;
            Some(PlayerMoveCollisionResponse {
                clip_x: (delta.x - resolved_delta.x).abs() >= 1.0e-5,
                clip_z: (delta.z - resolved_delta.z).abs() >= 1.0e-5,
                vertical_collision,
                fall_state,
                supporting,
                on_ground_no_blocks: no_blocks,
            })
        };
        // Entity.java:1015-1018: jumping reads the pre-move support.
        let jump_factor = if mover == crate::entity::MoverType::Player
            && player.cold.last_vehicle.is_none()
            && player.on_ground()
            && !on_ground
            && target.y - player.last_good.y > 0.0
        {
            self.player_support_pos_at(
                connection_id,
                player.position(),
                player.block_contact.supporting,
                0.500001_f32,
            )
            .and_then(|below| {
                let state_at = |pos: BlockPos| {
                    if level.chunks.is_inside_build_height(pos.y) {
                        level.chunks.get_block_state(pos.x, pos.y, pos.z)
                    } else {
                        Some(level.chunks.air_state_id())
                    }
                };
                crate::entity::living::body_block_jump_factor_with_registry(
                    state_at(super::block_position(player.position()))?,
                    state_at(below)?,
                    level.chunks.content_registry(),
                )
            })
        } else {
            None
        };
        Some(PlayerMoveProbe {
            jump_factor,
            collision_response,
            contact_movement: (player.game_type != GameType::Spectator && updates_position)
                .then_some(crate::entity::EntityMovement::axis_dependent(
                    player.position(),
                    resolved,
                    delta,
                )),
            used_stuck_multiplier,
            reset_fall_distance,
            moved_distance,
            old_position_clear: false,
            collides_with_new_geometry: false,
            packet_start: Some(player.position()),
            resolved_position: Some(resolved),
            prefix_applied: false,
            solved_delta: resolved_delta,
            updates_ground: if player.game_type == GameType::Spectator {
                mover == crate::entity::MoverType::Player
            } else {
                delta.y != 0.0
            },
            vertical_collision_below: player.movement.vertical_collision_below,
            no_blocks_around: false,
            next_vertical_collision_below: if player.game_type == GameType::Spectator {
                false
            } else if delta.y != 0.0 {
                vertical_collision && delta.y < 0.0
            } else {
                player.movement.vertical_collision_below
            },
        })
    }

    /// The listener's floating timeout (`ServerGamePacketListenerImpl.java:318-327`).
    ///
    pub(super) fn tick_player_floating(&mut self, events: &mut Vec<WorldEvent>) {
        let mut kicked = Vec::new();
        let allow_flight = self.admin.settings().allow_flight;
        let passengers = self
            .levels
            .player_keys()
            .copied()
            .filter(|&connection_id| self.player_is_passenger(connection_id))
            .collect::<Vec<_>>();
        for player in self.levels.players_mut() {
            let connection_id = player.connection_id;
            if player.movement.client_is_floating
                && !allow_flight
                && player.cold.sleeping_pos.is_none()
                && player.damage.health > 0.0
                && !passengers.contains(&connection_id)
            {
                player.movement.above_ground_tick_count =
                    player.movement.above_ground_tick_count.wrapping_add(1);
                let gravity = player
                    .attributes
                    .value(VANILLA_ATTRIBUTE_GRAVITY_ID)
                    .unwrap_or(0.08);
                if player.movement.above_ground_tick_count > maximum_flying_ticks(gravity) {
                    kicked.push(connection_id);
                }
            } else {
                player.movement.client_is_floating = false;
                player.movement.above_ground_tick_count = 0;
            }
        }
        for connection_id in kicked {
            self.remove_player(connection_id, events);
            events.push(WorldEvent::Close {
                connection_id,
                reason: WorldCloseReason::Floating,
            });
        }
    }

    // EntityGetter.java:54-67: sessions are not canBeCollidedWith targets.
    pub(super) fn player_collision_snapshot_in(
        &self,
        dimension: &LevelId,
        target: ConnectionId,
        bounds: bbb_server_types::geometry::Aabb,
    ) -> CollisionSnapshot {
        let boxes = level_entity_boxes_in(self.level_by_id(dimension), bounds);
        self.player_collision_snapshot_with_boxes(dimension, target, &boxes)
    }

    pub(super) fn player_collision_snapshot_with_boxes(
        &self,
        dimension: &LevelId,
        target: ConnectionId,
        boxes: &[(i32, bbb_server_types::geometry::Aabb)],
    ) -> CollisionSnapshot {
        let level = self.level_by_id(dimension);
        let player = self
            .levels
            .player(target)
            .expect("the colliding player is online");
        let player_entity_id = player.entity_id();
        // `Entity.canCollideWith` excludes the whole same-root passenger tree
        // (`Entity.java:2297-2298`, `:3489-3499`).
        let entity_boxes = if boxes.is_empty() {
            Vec::new()
        } else if let Some(root) = level
            .entities
            .external_player_root_vehicle(player_entity_id)
        {
            let mut related_entities = std::collections::BTreeSet::from([player_entity_id, root]);
            let mut pending = vec![root];
            while let Some(parent) = pending.pop() {
                for &passenger in level.entities.passengers(parent) {
                    if related_entities.insert(passenger) {
                        pending.push(passenger);
                    }
                }
            }
            boxes
                .iter()
                .filter(|(id, _)| !related_entities.contains(id))
                .map(|&(_, bounds)| bounds)
                .collect()
        } else {
            boxes
                .iter()
                .filter(|(id, _)| *id != player_entity_id)
                .map(|&(_, bounds)| bounds)
                .collect()
        };
        let player_boxes = Vec::new();
        CollisionSnapshot {
            entity_boxes,
            player_boxes,
            border: level.world_border,
            game_time: self.game_time(),
        }
    }
    pub(super) fn player_no_collision_at(
        &self,
        id: ConnectionId,
        position: Vec3d,
        bounds: bbb_server_types::geometry::Aabb,
    ) -> Option<bool> {
        let player = &self.levels.player(id).expect("the player is online");
        let level = self.level_by_id(&player.dimension);
        if !no_block_collision_with_scratch(
            bounds,
            |pos| moving_piston_at(&level.chunks, pos),
            |pos, out| {
                push_player_block_collision_shape_with_context(
                    &level.chunks,
                    player,
                    pos,
                    position.y,
                    false,
                    out,
                )
            },
        )? {
            return Some(false);
        }
        if !self
            .player_collision_snapshot_in(&player.dimension, id, bounds)
            .entity_boxes
            .is_empty()
        {
            return Some(false);
        }
        Some(
            level
                .world_border
                .no_collision_at(position, bounds, self.game_time()),
        )
    }

    // Player.java:880-949: X, Z, then combined retreats share the same source context.
    fn player_back_off_from_edge(
        &self,
        id: ConnectionId,
        delta: Vec3d,
        bounds: bbb_server_types::geometry::Aabb,
    ) -> Option<Vec3d> {
        let player = self.levels.player(id)?;
        let max_down = f64::from(
            player
                .attributes
                .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_STEP_HEIGHT_ID)?
                as f32,
        );
        if player.abilities.flying
            || delta.y > 0.0
            || !player.shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            )
        {
            return Some(delta);
        }
        let can_fall = |x, z, height| {
            self.player_no_collision_at(
                id,
                player.position(),
                bbb_server_types::geometry::Aabb {
                    min_x: bounds.min_x + 1.0e-7 + x,
                    min_y: bounds.min_y - height - 1.0e-7,
                    min_z: bounds.min_z + 1.0e-7 + z,
                    max_x: bounds.max_x - 1.0e-7 + x,
                    max_y: bounds.min_y,
                    max_z: bounds.max_z - 1.0e-7 + z,
                },
            )
        };
        if !player.on_ground()
            && !(player.block_contact.fall_distance < max_down
                && !can_fall(0.0, 0.0, max_down - player.block_contact.fall_distance)?)
        {
            return Some(delta);
        }
        // Sneaking edge back-off applies only to walking-scale steps; cap distance
        // so large unchecked deltas cannot spin the 0.05-step loop millions of times.
        const MAX_SNEAK_BACKOFF_DELTA: f64 = 2.0;
        if delta.x.abs() > MAX_SNEAK_BACKOFF_DELTA || delta.z.abs() > MAX_SNEAK_BACKOFF_DELTA {
            return Some(delta);
        }
        let mut x = delta.x;
        let mut z = delta.z;
        let step_x = x.signum() * 0.05;
        let step_z = z.signum() * 0.05;
        while x != 0.0 && can_fall(x, 0.0, max_down)? {
            if x.abs() <= 0.05 {
                x = 0.0;
                break;
            }
            x -= step_x;
        }
        while z != 0.0 && can_fall(0.0, z, max_down)? {
            if z.abs() <= 0.05 {
                z = 0.0;
                break;
            }
            z -= step_z;
        }
        while x != 0.0 && z != 0.0 && can_fall(x, z, max_down)? {
            x = if x.abs() <= 0.05 { 0.0 } else { x - step_x };
            z = if z.abs() <= 0.05 { 0.0 } else { z - step_z };
        }
        Some(Vec3d { x, y: delta.y, z })
    }

    pub(super) fn finish_player_move_support(
        &self,
        id: ConnectionId,
        probe: &mut PlayerMoveProbe,
    ) -> Option<()> {
        let Some(response) = &mut probe.collision_response else {
            return Some(());
        };
        let player = self.levels.player(id)?;
        let position = probe.resolved_position.unwrap_or(player.position());
        if probe.updates_ground {
            let movement = probe.solved_delta;
            (response.supporting, response.on_ground_no_blocks) = self
                .player_support_for_movement(
                    id,
                    position,
                    movement,
                    probe.next_vertical_collision_below,
                )?;
        }
        if response.vertical_collision {
            let pos = self.player_legacy_support_pos_at(id, position, response.supporting)?;
            let level = self.level_by_id(&player.dimension);
            response.fall_state = Some(if level.chunks.is_inside_build_height(pos.y) {
                level.chunks.get_block_state(pos.x, pos.y, pos.z)?
            } else {
                level.block_state(pos)
            });
        }
        Some(())
    }

    pub(super) fn finish_player_move_speed(&mut self, id: ConnectionId) -> Option<()> {
        let player = self.levels.player(id)?;
        if player.game_type == GameType::Spectator
            || player.abilities.flying
            || player.shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING,
            )
        {
            return Some(());
        }
        let level = self.level_by_id(&player.dimension);
        let state_at = |pos: BlockPos| {
            if level.chunks.is_inside_build_height(pos.y) {
                level.chunks.get_block_state(pos.x, pos.y, pos.z)
            } else {
                Some(level.block_state(pos))
            }
        };
        let state = state_at(super::block_position(player.position()))?;
        let content_registry = level.chunks.content_registry();
        let here =
            crate::movement_speed::block_speed_factor_with_registry(state, content_registry)?;
        let base = crate::movement_speed::select_block_speed_factor_with_registry(
            state,
            here,
            content_registry,
            || {
                let below = self.player_support_pos_at(
                    id,
                    player.position(),
                    player.block_contact.supporting,
                    0.500001_f32,
                )?;
                crate::movement_speed::block_speed_factor_with_registry(
                    state_at(below)?,
                    content_registry,
                )
            },
        )?;
        let efficiency = player
            .attributes
            .value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID)?
            as f32;
        let factor = f64::from(
            crate::entity::living::body_movement_efficiency_speed_factor(efficiency, base),
        );
        let player = self.levels.player_mut(id)?;
        player.set_delta_movement(player.delta_movement().multiply(factor, 1.0, factor));
        Some(())
    }

    pub(super) fn finish_player_floating_query(
        &self,
        id: ConnectionId,
        movement: &MovePlayer,
        probe: &mut PlayerMoveProbe,
    ) -> Option<()> {
        let player = self.levels.player(id)?;
        if !super::player_packets::player_move_is_accepted(player, probe) {
            return Some(());
        }
        let target = movement
            .position
            .map(clamped_target)
            .unwrap_or(probe.packet_start.unwrap_or(player.position()));
        let mut candidate = *probe;
        candidate.no_blocks_around = true;
        if candidate.client_is_floating(
            player,
            target.y - player.last_good.y,
            player.shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING,
            ),
        ) {
            let bounds = player_pose_bounding_box(target, player.pose_id, player.pose_scale())
                .ok()?
                .inflate(0.0625, 0.0625, 0.0625)
                .expand_towards(Vec3d {
                    x: 0.0,
                    y: -0.55,
                    z: 0.0,
                });
            probe.no_blocks_around =
                no_blocks_around_checked(&self.level_by_id(&player.dimension).chunks, bounds)?;
        }
        Some(())
    }

    pub(super) fn finish_player_collision_queries(
        &self,
        connection_id: ConnectionId,
        movement: &MovePlayer,
        probe: &mut PlayerMoveProbe,
    ) -> Option<()> {
        let player = self.levels.player(connection_id)?;
        let start = probe.packet_start.unwrap_or(player.position());
        let target = movement.position.map(clamped_target).unwrap_or(start);
        let old_box = player_pose_bounding_box(start, player.pose_id, player.pose_scale()).ok()?;
        let level = self.level_by_id(&player.dimension);
        let resolved = probe.resolved_position.unwrap_or(player.position());
        let moved_distance = probe.moved_distance;
        let moved_wrongly = super::player_packets::player_moved_wrongly(player, moved_distance);
        let old_position_clear = if moved_wrongly {
            self.player_no_collision_at(connection_id, resolved, old_box)?
        } else {
            false
        };
        let target_box = old_box.moved(subtract(target, start));
        let collides_with_new_geometry =
            if player.game_type == GameType::Spectator || moved_wrongly && old_position_clear {
                false
            } else {
                let epsilon = -f64::from(1.0e-5_f32);
                let query = target_box.inflate(epsilon, epsilon, epsilon);
                let old_shape = BoxShape::from_box(old_box.inflate(epsilon, epsilon, epsilon))?;
                let entities = self
                    .player_collision_snapshot_in(&player.dimension, connection_id, query)
                    .entity_boxes;
                let mut new_entity = false;
                for bounds in entities {
                    if !BoxShape::from_box(bounds)?.intersects(&old_shape)? {
                        new_entity = true;
                        break;
                    }
                }
                new_entity
                    || !visit_block_collision_shapes_checked_with_scratch(
                        query,
                        |pos| moving_piston_at(&level.chunks, pos),
                        |pos, out| {
                            push_player_block_collision_shape_with_context(
                                &level.chunks,
                                player,
                                pos,
                                old_box.min_y,
                                true,
                                out,
                            )
                        },
                        |_, shape| shape.intersects(&old_shape),
                    )?
            };
        probe.old_position_clear = old_position_clear;
        probe.collides_with_new_geometry = collides_with_new_geometry;
        Some(())
    }
}
