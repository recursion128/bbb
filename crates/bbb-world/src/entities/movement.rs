use bbb_protocol::packets::{
    EntityMove as ProtocolEntityMove, EntityPositionSync as ProtocolEntityPositionSync,
    TeleportEntity as ProtocolTeleportEntity, Vec3d as ProtocolVec3d, PLAYER_RELATIVE_DELTA_X,
    PLAYER_RELATIVE_DELTA_Y, PLAYER_RELATIVE_DELTA_Z, PLAYER_RELATIVE_ROTATE_DELTA,
    PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y, PLAYER_RELATIVE_Y_ROT,
    PLAYER_RELATIVE_Z,
};

use crate::WorldStore;

use super::EntityVec3;

impl WorldStore {
    pub fn apply_entity_position_sync(&mut self, packet: ProtocolEntityPositionSync) -> bool {
        self.counters.entity_position_syncs_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.position = entity_vec3(packet.position);
        entity.position_base = entity_vec3(packet.position);
        entity.delta_movement = entity_vec3(packet.delta_movement);
        entity.y_rot = packet.y_rot;
        entity.x_rot = packet.x_rot;
        entity.on_ground = Some(packet.on_ground);
        self.counters.entity_position_syncs_applied += 1;
        true
    }

    pub fn apply_entity_move(&mut self, packet: ProtocolEntityMove) -> bool {
        self.counters.entity_moves_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        if packet.delta_x != 0 || packet.delta_y != 0 || packet.delta_z != 0 {
            let position = decode_entity_delta_position(
                entity.position_base,
                packet.delta_x,
                packet.delta_y,
                packet.delta_z,
            );
            entity.position = position;
            entity.position_base = position;
        }
        if let Some(y_rot) = packet.y_rot {
            entity.y_rot = y_rot;
        }
        if let Some(x_rot) = packet.x_rot {
            entity.x_rot = x_rot;
        }
        entity.on_ground = Some(packet.on_ground);
        self.counters.entity_moves_applied += 1;
        true
    }

    pub fn apply_teleport_entity(&mut self, packet: ProtocolTeleportEntity) -> bool {
        self.counters.entity_teleports_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        let absolute = entity_absolute_move_rotation(
            entity.position,
            entity.delta_movement,
            entity.y_rot,
            entity.x_rot,
            packet.position,
            packet.delta_movement,
            packet.y_rot,
            packet.x_rot,
            packet.relatives_mask,
        );
        entity.position = absolute.position;
        entity.delta_movement = absolute.delta_movement;
        entity.y_rot = absolute.y_rot;
        entity.x_rot = absolute.x_rot;
        entity.on_ground = Some(packet.on_ground);
        self.counters.entity_teleports_applied += 1;
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct EntityMoveRotation {
    pub(super) position: EntityVec3,
    pub(super) delta_movement: EntityVec3,
    pub(super) y_rot: f32,
    pub(super) x_rot: f32,
}

pub(super) fn entity_vec3(vec: ProtocolVec3d) -> EntityVec3 {
    EntityVec3 {
        x: vec.x,
        y: vec.y,
        z: vec.z,
    }
}

pub(super) fn entity_distance_squared(a: EntityVec3, b: EntityVec3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    dx * dx + dy * dy + dz * dz
}

pub(super) fn decode_entity_delta_position(
    base: EntityVec3,
    xa: i16,
    ya: i16,
    za: i16,
) -> EntityVec3 {
    if xa == 0 && ya == 0 && za == 0 {
        return base;
    }

    EntityVec3 {
        x: decode_entity_delta_axis(base.x, xa),
        y: decode_entity_delta_axis(base.y, ya),
        z: decode_entity_delta_axis(base.z, za),
    }
}

fn decode_entity_delta_axis(base: f64, delta: i16) -> f64 {
    if delta == 0 {
        base
    } else {
        java_round_to_i64(base * 4096.0).saturating_add(i64::from(delta)) as f64 / 4096.0
    }
}

fn java_round_to_i64(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

pub(super) fn entity_absolute_move_rotation(
    current_position: EntityVec3,
    current_delta_movement: EntityVec3,
    current_y_rot: f32,
    current_x_rot: f32,
    change_position: ProtocolVec3d,
    change_delta_movement: ProtocolVec3d,
    change_y_rot: f32,
    change_x_rot: f32,
    relatives_mask: i32,
) -> EntityMoveRotation {
    let position = EntityVec3 {
        x: absolute_or_relative_f64(
            current_position.x,
            change_position.x,
            relatives_mask,
            PLAYER_RELATIVE_X,
        ),
        y: absolute_or_relative_f64(
            current_position.y,
            change_position.y,
            relatives_mask,
            PLAYER_RELATIVE_Y,
        ),
        z: absolute_or_relative_f64(
            current_position.z,
            change_position.z,
            relatives_mask,
            PLAYER_RELATIVE_Z,
        ),
    };
    let y_rot = absolute_or_relative_f32(
        current_y_rot,
        change_y_rot,
        relatives_mask,
        PLAYER_RELATIVE_Y_ROT,
    );
    let x_rot = absolute_or_relative_f32(
        current_x_rot,
        change_x_rot,
        relatives_mask,
        PLAYER_RELATIVE_X_ROT,
    )
    .clamp(-90.0, 90.0);

    let rotated_delta = if relatives_mask & PLAYER_RELATIVE_ROTATE_DELTA != 0 {
        rotate_entity_delta(
            current_delta_movement,
            current_y_rot - y_rot,
            current_x_rot - x_rot,
        )
    } else {
        current_delta_movement
    };
    let delta_movement = EntityVec3 {
        x: absolute_or_relative_f64(
            rotated_delta.x,
            change_delta_movement.x,
            relatives_mask,
            PLAYER_RELATIVE_DELTA_X,
        ),
        y: absolute_or_relative_f64(
            rotated_delta.y,
            change_delta_movement.y,
            relatives_mask,
            PLAYER_RELATIVE_DELTA_Y,
        ),
        z: absolute_or_relative_f64(
            rotated_delta.z,
            change_delta_movement.z,
            relatives_mask,
            PLAYER_RELATIVE_DELTA_Z,
        ),
    };

    EntityMoveRotation {
        position,
        delta_movement,
        y_rot,
        x_rot,
    }
}

fn absolute_or_relative_f64(current: f64, change: f64, mask: i32, relative_bit: i32) -> f64 {
    if mask & relative_bit != 0 {
        current + change
    } else {
        change
    }
}

fn absolute_or_relative_f32(current: f32, change: f32, mask: i32, relative_bit: i32) -> f32 {
    if mask & relative_bit != 0 {
        current + change
    } else {
        change
    }
}

fn rotate_entity_delta(delta: EntityVec3, y_rot_degrees: f32, x_rot_degrees: f32) -> EntityVec3 {
    let x_rad = f64::from(x_rot_degrees).to_radians();
    let y_rad = f64::from(y_rot_degrees).to_radians();
    let cos_x = x_rad.cos();
    let sin_x = x_rad.sin();
    let after_x = EntityVec3 {
        x: delta.x,
        y: delta.y * cos_x + delta.z * sin_x,
        z: delta.z * cos_x - delta.y * sin_x,
    };
    let cos_y = y_rad.cos();
    let sin_y = y_rad.sin();
    EntityVec3 {
        x: after_x.x * cos_y + after_x.z * sin_y,
        y: after_x.y,
        z: after_x.z * cos_y - after_x.x * sin_y,
    }
}
