use serde::{Deserialize, Serialize};

use crate::{codec::Encoder, ids};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec3d {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerPositionUpdate {
    pub id: i32,
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub relatives_mask: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayerPositionState {
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerRotationUpdate {
    pub y_rot: f32,
    pub relative_y: bool,
    pub x_rot: f32,
    pub relative_x: bool,
}

impl PlayerPositionUpdate {
    pub fn apply_to_state(self, current: PlayerPositionState) -> PlayerPositionState {
        let mut current_delta = current.delta_movement;
        let position = Vec3d {
            x: absolute_or_relative(
                current.position.x,
                self.position.x,
                self.relatives_mask,
                PLAYER_RELATIVE_X,
            ),
            y: absolute_or_relative(
                current.position.y,
                self.position.y,
                self.relatives_mask,
                PLAYER_RELATIVE_Y,
            ),
            z: absolute_or_relative(
                current.position.z,
                self.position.z,
                self.relatives_mask,
                PLAYER_RELATIVE_Z,
            ),
        };
        let y_rot = absolute_or_relative_f32(
            current.y_rot,
            self.y_rot,
            self.relatives_mask,
            PLAYER_RELATIVE_Y_ROT,
        );
        let x_rot = absolute_or_relative_f32(
            current.x_rot,
            self.x_rot,
            self.relatives_mask,
            PLAYER_RELATIVE_X_ROT,
        )
        .clamp(-90.0, 90.0);
        if self.relatives_mask & PLAYER_RELATIVE_ROTATE_DELTA != 0 {
            current_delta =
                rotate_delta_movement(current_delta, current.y_rot - y_rot, current.x_rot - x_rot);
        }
        let delta_movement = Vec3d {
            x: absolute_or_relative(
                current_delta.x,
                self.delta_movement.x,
                self.relatives_mask,
                PLAYER_RELATIVE_DELTA_X,
            ),
            y: absolute_or_relative(
                current_delta.y,
                self.delta_movement.y,
                self.relatives_mask,
                PLAYER_RELATIVE_DELTA_Y,
            ),
            z: absolute_or_relative(
                current_delta.z,
                self.delta_movement.z,
                self.relatives_mask,
                PLAYER_RELATIVE_DELTA_Z,
            ),
        };

        PlayerPositionState {
            position,
            delta_movement,
            y_rot,
            x_rot,
        }
    }
}

impl PlayerRotationUpdate {
    pub fn apply_to_state(self, current: PlayerPositionState) -> PlayerPositionState {
        PlayerPositionState {
            position: current.position,
            delta_movement: current.delta_movement,
            y_rot: if self.relative_y {
                current.y_rot + self.y_rot
            } else {
                self.y_rot
            },
            x_rot: (if self.relative_x {
                current.x_rot + self.x_rot
            } else {
                self.x_rot
            })
            .clamp(-90.0, 90.0),
        }
    }
}

pub const PLAYER_RELATIVE_X: i32 = 1 << 0;
pub const PLAYER_RELATIVE_Y: i32 = 1 << 1;
pub const PLAYER_RELATIVE_Z: i32 = 1 << 2;
pub const PLAYER_RELATIVE_Y_ROT: i32 = 1 << 3;
pub const PLAYER_RELATIVE_X_ROT: i32 = 1 << 4;
pub const PLAYER_RELATIVE_DELTA_X: i32 = 1 << 5;
pub const PLAYER_RELATIVE_DELTA_Y: i32 = 1 << 6;
pub const PLAYER_RELATIVE_DELTA_Z: i32 = 1 << 7;
pub const PLAYER_RELATIVE_ROTATE_DELTA: i32 = 1 << 8;

pub fn encode_play_accept_teleportation(id: i32) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(id);
    (
        ids::play::SERVERBOUND_ACCEPT_TELEPORTATION,
        out.into_inner(),
    )
}

pub fn encode_play_move_player_pos_rot(
    x: f64,
    y: f64,
    z: f64,
    y_rot: f32,
    x_rot: f32,
    on_ground: bool,
    horizontal_collision: bool,
) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_f64(x);
    out.write_f64(y);
    out.write_f64(z);
    out.write_f32(y_rot);
    out.write_f32(x_rot);
    out.write_u8(super::pack_move_flags(on_ground, horizontal_collision));
    (ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT, out.into_inner())
}

fn absolute_or_relative(current: f64, change: f64, mask: i32, relative_bit: i32) -> f64 {
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

fn rotate_delta_movement(delta: Vec3d, y_rot_degrees: f32, x_rot_degrees: f32) -> Vec3d {
    let x_rad = f64::from(x_rot_degrees).to_radians();
    let y_rad = f64::from(y_rot_degrees).to_radians();
    let cos_x = x_rad.cos();
    let sin_x = x_rad.sin();
    let after_x = Vec3d {
        x: delta.x,
        y: delta.y * cos_x + delta.z * sin_x,
        z: delta.z * cos_x - delta.y * sin_x,
    };
    let cos_y = y_rad.cos();
    let sin_y = y_rad.sin();
    Vec3d {
        x: after_x.x * cos_y + after_x.z * sin_y,
        y: after_x.y,
        z: after_x.z * cos_y - after_x.x * sin_y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        codec::{Decoder, Encoder},
        ids,
        packets::{decode_play_clientbound, PlayClientbound},
    };

    #[test]
    fn decodes_player_position_and_encodes_ack_pair() {
        let mut payload = Encoder::new();
        payload.write_var_i32(77);
        payload.write_f64(1.0);
        payload.write_f64(64.0);
        payload.write_f64(-2.0);
        payload.write_f64(0.0);
        payload.write_f64(0.1);
        payload.write_f64(0.0);
        payload.write_f32(180.0);
        payload.write_f32(15.0);
        payload.write_i32(0);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_PLAYER_POSITION,
            &payload.into_inner(),
        )
        .unwrap();
        let PlayClientbound::PlayerPosition(update) = packet else {
            panic!("wrong packet");
        };
        assert_eq!(update.id, 77);
        assert_eq!(update.position.y, 64.0);

        let (id, ack) = encode_play_accept_teleportation(update.id);
        assert_eq!(id, ids::play::SERVERBOUND_ACCEPT_TELEPORTATION);
        assert_eq!(Decoder::new(&ack).read_var_i32().unwrap(), 77);

        let (id, pos) = encode_play_move_player_pos_rot(
            update.position.x,
            update.position.y,
            update.position.z,
            update.y_rot,
            update.x_rot,
            false,
            false,
        );
        assert_eq!(id, ids::play::SERVERBOUND_MOVE_PLAYER_POS_ROT);
        assert_eq!(pos.len(), 33);
    }

    #[test]
    fn decodes_player_rotation_packet() {
        let mut payload = Encoder::new();
        payload.write_f32(15.0);
        payload.write_bool(true);
        payload.write_f32(-120.0);
        payload.write_bool(false);

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_PLAYER_ROTATION,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::PlayerRotation(PlayerRotationUpdate {
                y_rot: 15.0,
                relative_y: true,
                x_rot: -120.0,
                relative_x: false,
            })
        );
    }

    #[test]
    fn player_position_update_applies_relative_state() {
        let current = PlayerPositionState {
            position: Vec3d {
                x: 10.0,
                y: 64.0,
                z: -5.0,
            },
            delta_movement: Vec3d {
                x: 0.125,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 90.0,
            x_rot: 15.0,
        };
        let change = PlayerPositionUpdate {
            id: 2,
            position: Vec3d {
                x: 1.5,
                y: -2.0,
                z: 7.0,
            },
            delta_movement: Vec3d {
                x: 0.25,
                y: 0.5,
                z: 0.75,
            },
            y_rot: 20.0,
            x_rot: -120.0,
            relatives_mask: PLAYER_RELATIVE_X
                | PLAYER_RELATIVE_Y_ROT
                | PLAYER_RELATIVE_X_ROT
                | PLAYER_RELATIVE_DELTA_X,
        };

        let state = change.apply_to_state(current);

        assert_eq!(
            state.position,
            Vec3d {
                x: 11.5,
                y: -2.0,
                z: 7.0,
            }
        );
        assert_eq!(
            state.delta_movement,
            Vec3d {
                x: 0.375,
                y: 0.5,
                z: 0.75,
            }
        );
        assert_eq!(state.y_rot, 110.0);
        assert_eq!(state.x_rot, -90.0);
    }

    #[test]
    fn player_rotation_update_applies_relative_state() {
        let current = PlayerPositionState {
            position: Vec3d {
                x: 10.0,
                y: 64.0,
                z: -5.0,
            },
            delta_movement: Vec3d {
                x: 0.125,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 90.0,
            x_rot: 15.0,
        };
        let update = PlayerRotationUpdate {
            y_rot: 20.0,
            relative_y: true,
            x_rot: -120.0,
            relative_x: false,
        };

        let state = update.apply_to_state(current);

        assert_eq!(state.position, current.position);
        assert_eq!(state.delta_movement, current.delta_movement);
        assert_eq!(state.y_rot, 110.0);
        assert_eq!(state.x_rot, -90.0);
    }
}
