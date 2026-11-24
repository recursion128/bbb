//! `TeleportRandomlyConsumeEffect.apply` (`TeleportRandomlyConsumeEffect.java:41-79`)
//! and the `LivingEntity.randomTeleport` it runs (`LivingEntity.java:3601-3645`)
//! for a session.

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::packets::{
    BlockPos, EntityEvent, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
    PLAYER_RELATIVE_DELTA_X, PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_DELTA_Z,
    PLAYER_RELATIVE_ROTATE_DELTA, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y_ROT,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_server_types::fluid::block_state_blocks_motion;
use bbb_server_types::geometry::visit_block_collisions_with_scratch;
use bbb_server_types::interaction::player_pose_bounding_box;

use crate::world::external_player_ride_events::DismountLanding;
use crate::world::level_blocks::stored_block_state;
use crate::world::movement_collision::{
    border_collision_boxes, moving_piston_at, push_block_collision_shape,
};
use crate::world::natural_spawn::box_contains_liquid;
use crate::world::nether_portal::dimension_shape;
use crate::world::{ServerWorld, TeleportTransition, WorldEvent};

/// `EntityEvent.TELEPORT` (`EntityEvent.java:42`).
const ENTITY_EVENT_TELEPORT: i8 = 46;

/// `Relative.union(Relative.DELTA, Relative.ROTATION)` (`ServerPlayer.java:1669`).
const TELEPORT_TO_RELATIVES: i32 = PLAYER_RELATIVE_DELTA_X
    | PLAYER_RELATIVE_DELTA_Y
    | PLAYER_RELATIVE_DELTA_Z
    | PLAYER_RELATIVE_ROTATE_DELTA
    | PLAYER_RELATIVE_Y_ROT
    | PLAYER_RELATIVE_X_ROT;

impl ServerWorld {
    pub(in crate::world) fn teleport_randomly(
        &mut self,
        connection_id: ConnectionId,
        diameter: f32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        self.teleport_randomly_checked(connection_id, diameter, events)
            .unwrap_or(false)
    }

    pub(super) fn teleport_randomly_checked(
        &mut self,
        connection_id: ConnectionId,
        diameter: f32,
        events: &mut Vec<WorldEvent>,
    ) -> Result<bool, String> {
        if !diameter.is_finite() || diameter < 0.0 {
            return Ok(false);
        }
        let dimension = self.levels[connection_id].dimension;
        let shape = dimension_shape(self, &dimension).map_err(|error| error.to_string())?;
        let diameter = f64::from(diameter);
        let min_y = f64::from(shape.min_y);
        let max_y = f64::from(shape.min_y + shape.logical_height - 1);
        for _ in 0..16 {
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("the consuming session is connected");
            let position = player.position();
            let x = position.x + (self.random.next_double() - 0.5) * diameter;
            let raw_y = position.y + (self.random.next_double() - 0.5) * diameter;
            let z = position.z + (self.random.next_double() - 0.5) * diameter;
            if !x.is_finite() || !raw_y.is_finite() || !z.is_finite() {
                continue;
            }
            let y = raw_y.clamp(min_y, max_y);
            let dismounted = self
                .stop_external_player_ride(
                    &dimension,
                    connection_id,
                    DismountLanding::Search,
                    events,
                )
                .ok_or_else(|| "random teleport could not publish player dismount".to_owned())?;
            if dismounted {
                self.apply_level_entity_lifecycle_by_key(&dimension, events);
            }
            let old_position = self.levels[connection_id].position();
            if self.random_teleport_player(connection_id, Vec3d { x, y, z }, events) {
                let player = self.levels.player_at(connection_id);
                let position = player.position();
                self.player_game_event_at(
                    connection_id,
                    VanillaGameEvent::TELEPORT,
                    old_position,
                    None,
                );
                if let Some(registry_id) =
                    vanilla_sound_event_registry_id("minecraft:item.chorus_fruit.teleport")
                {
                    let sound = SoundEvent {
                        sound: SoundEventHolder::Reference { registry_id },
                        source: SoundSource::Players,
                        position,
                        volume: 1.0,
                        pitch: 1.0,
                        seed: self.random.next_long(),
                    };
                    self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
                }
                self.reset_player_fall_distance(connection_id, events);
                self.levels[connection_id].reset_fall_impulse();
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `LivingEntity.randomTeleport(x, y, z, true)` through
    /// `ServerPlayer.teleportTo(double, double, double)` (`ServerPlayer.java:1669-1671`),
    /// which tells the client of every move, the one back included.
    fn random_teleport_player(
        &mut self,
        connection_id: ConnectionId,
        target: Vec3d,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        if !target.x.is_finite() || !target.y.is_finite() || !target.z.is_finite() {
            return false;
        }
        let player = self.levels.player_at(connection_id);
        let dimension = player.dimension;
        let origin = player.position();
        let chunks = &self.level_by_id(&dimension).chunks;
        let min_y = chunks.min_y();
        let mut y = target.y;
        let mut pos = BlockPos {
            x: target.x.floor() as i32,
            y: y.floor() as i32,
            z: target.z.floor() as i32,
        };
        let mut ok = false;
        if chunks.is_ticking_full(pos.x >> 4, pos.z >> 4) {
            let mut landed = false;
            while !landed && pos.y > min_y {
                let below = BlockPos {
                    y: pos.y - 1,
                    ..pos
                };
                if block_state_blocks_motion(stored_block_state(chunks, below)) {
                    landed = true;
                } else {
                    y -= 1.0;
                    pos = below;
                }
            }
            if landed {
                self.teleport_session_to(connection_id, Vec3d { y, ..target }, events);
                ok = self.player_box_is_free(connection_id);
            }
        }
        if !ok {
            self.teleport_session_to(connection_id, origin, events);
            return false;
        }
        let entity_id = self.levels[connection_id].entity_id();
        self.send_to_player_tracking_and_self(
            connection_id,
            entity_id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id,
                event_id: ENTITY_EVENT_TELEPORT,
            }),
            events,
        );
        true
    }

    fn teleport_session_to(
        &mut self,
        connection_id: ConnectionId,
        position: Vec3d,
        events: &mut Vec<WorldEvent>,
    ) {
        let transition = TeleportTransition {
            new_level: self.levels[connection_id].dimension,
            position,
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: TELEPORT_TO_RELATIVES,
        };
        self.teleport_set_position(connection_id, &transition, events);
        // Reset the movement baseline (`ServerGamePacketListenerImpl.java:371-378`)
        // after a server-initiated chorus-fruit teleport (`ServerPlayer.java:1669-1671`).
        let player = &mut self.levels[connection_id];
        let reset = player.position();
        player.first_good = reset;
        player.last_good = reset;
    }

    /// `level.noCollision(this) && !level.containsAnyLiquid(this.getBoundingBox())`
    /// (`LivingEntity.java:3625`).
    fn player_box_is_free(&self, connection_id: ConnectionId) -> bool {
        let player = self.levels.player_at(connection_id);
        let Ok(bounds) =
            player_pose_bounding_box(player.position(), player.pose_id, player.pose_scale())
        else {
            return false;
        };
        let dimension = player.dimension;
        let chunks = &self.level_by_id(&dimension).chunks;
        if box_contains_liquid(chunks, bounds) {
            return false;
        }
        let mut block_hit = false;
        let _ = visit_block_collisions_with_scratch(
            bounds,
            |pos| moving_piston_at(chunks, pos).unwrap_or(true),
            |pos, out| push_block_collision_shape(chunks, pos, None, out),
            |_, shape| {
                if shape.intersects(bounds) {
                    block_hit = true;
                    false
                } else {
                    true
                }
            },
        );
        if block_hit {
            return false;
        }
        let snapshot = self.player_collision_snapshot_in(&dimension, connection_id, bounds);
        let entity_or_border_hit = snapshot
            .entity_boxes
            .iter()
            .chain(&snapshot.player_boxes)
            .copied()
            .chain(border_collision_boxes(snapshot.border, snapshot.game_time))
            .any(|shape| shape.intersects(bounds));
        !entity_or_border_hit
    }
}
