//! `Entity.teleport` as a command performs it on a stored entity.

use bbb_protocol::entity_anchors::EntityAnchor;
use bbb_protocol::packets::movement::{PlayerPositionState, PlayerPositionUpdate};
use bbb_protocol::packets::{
    Vec3d, PLAYER_RELATIVE_X, PLAYER_RELATIVE_X_ROT, PLAYER_RELATIVE_Y, PLAYER_RELATIVE_Y_ROT,
    PLAYER_RELATIVE_Z,
};

use super::EntityStore;
use crate::entity::living::wrap_degrees;
use crate::entity::projectile_hit::mth_atan2;
use crate::entity::Entity;

impl Entity {
    /// `Entity.teleportSetPosition` (`Entity.java:3079-3089`), whose absolute
    /// destination is `PositionMoveRotation.calculateAbsolute`
    /// (`PositionMoveRotation.java:40-63`) -- the same computation
    /// [`PlayerPositionUpdate::apply_to_state`] carries for a session.
    ///
    /// `setOldRot` (`:1745-1748`) has no subject here: this runtime keeps no
    /// previous rotation for a stored entity.
    pub fn teleport_set_position(&mut self, destination: PlayerPositionState, relatives_mask: i32) {
        let absolute = PlayerPositionUpdate {
            id: 0,
            position: destination.position,
            delta_movement: destination.delta_movement,
            y_rot: destination.y_rot,
            x_rot: destination.x_rot,
            relatives_mask,
        }
        .apply_to_state(PlayerPositionState {
            position: self.position(),
            delta_movement: self.delta_movement(),
            y_rot: self.y_rot(),
            x_rot: self.x_rot(),
        });
        // `setPosRaw` (`:3081`) and the `reapplyPosition` that follows it
        // (`:3085`) are together what this crate's `set_pos` is, except for the
        // block-attached entities whose `setPos` writes the attachment instead.
        if !crate::entity::hanging::set_attached_pos(self, absolute.position) {
            self.set_pos(absolute.position);
        }
        self.set_rot(absolute.y_rot, absolute.x_rot);
        self.set_y_head_rot(absolute.y_rot);
        self.set_old_pos_and_rot();
        self.set_delta_movement(absolute.delta_movement);
        self.clear_movement_this_tick();
    }

    /// `Entity.lookAt` (`Entity.java:3570-3581`) over
    /// `EntityAnchorArgument.Anchor.apply` (`EntityAnchorArgument.java:61-62`).
    ///
    /// `xRotO`/`yRotO` (`:3579-3580`) have no subject here for the same reason
    /// [`Self::teleport_set_position`]'s `setOldRot` has none.
    pub fn look_at(&mut self, from_anchor: EntityAnchor, target: Vec3d) {
        let position = self.position();
        let from = match from_anchor {
            EntityAnchor::Feet => position,
            EntityAnchor::Eyes => Vec3d {
                y: position.y + f64::from(self.eye_height()),
                ..position
            },
        };
        let xd = target.x - from.x;
        let yd = target.y - from.y;
        let zd = target.z - from.z;
        let sd = (xd * xd + zd * zd).sqrt();
        // `radians * 180.0F / (float)Math.PI` associates left, so the radians
        // are scaled before the float-rounded pi divides them.
        let degrees = |radians: f64| radians * 180.0 / f64::from(std::f32::consts::PI);
        let x_rot = wrap_degrees(-degrees(mth_atan2(yd, sd)) as f32);
        let y_rot = wrap_degrees(degrees(mth_atan2(zd, xd)) as f32 - 90.0);
        self.set_rot(y_rot, x_rot);
        self.set_y_head_rot(y_rot);
    }
}

impl EntityStore {
    pub(crate) fn unride_body(&mut self, entity: &mut Entity) {
        if self.stop_invalid_riding(entity.id()) {
            entity.set_boarding_cooldown(60);
        }
        self.eject_body_passengers_reverse(entity);
    }

    pub fn stop_stored_riding(&mut self, passenger_id: i32) -> bool {
        self.get(passenger_id).is_some() && self.stop_invalid_riding(passenger_id)
    }

    /// `Entity.teleport` (`Entity.java:2971-2983`) and the
    /// `teleportSameDimension` arm it picks (`:2985-3000`), which is all of
    /// `Entity.teleportTo` (`:3246-3258`) once a command has computed the
    /// transition. Returns whether the entity was there to move.
    pub fn teleport_stored_entity(
        &mut self,
        id: i32,
        destination: PlayerPositionState,
        relatives_mask: i32,
    ) -> bool {
        self.teleport_stored(id, destination, relatives_mask, false)
    }

    fn teleport_stored(
        &mut self,
        id: i32,
        destination: PlayerPositionState,
        relatives_mask: i32,
        as_passenger: bool,
    ) -> bool {
        let Some(entity) = self.get(id) else {
            return false;
        };
        if entity.removal_reason().is_some() {
            return false;
        }
        let (position, y_rot, x_rot) = (entity.position(), entity.y_rot(), entity.x_rot());
        if !as_passenger {
            self.stop_invalid_riding(id);
        }
        // `calculatePassengerTransition` (`:3047-3058`).
        let relative = |flag: i32| relatives_mask & flag != 0;
        for passenger in self.passengers(id).to_vec() {
            let Some(rider) = self.get(passenger) else {
                continue;
            };
            let offset = |flag: i32, value: f64| if relative(flag) { 0.0 } else { value };
            let turn = |flag: i32, value: f32| if relative(flag) { 0.0 } else { value };
            let at = rider.position();
            let rider_destination = PlayerPositionState {
                position: Vec3d {
                    x: destination.position.x + offset(PLAYER_RELATIVE_X, at.x - position.x),
                    y: destination.position.y + offset(PLAYER_RELATIVE_Y, at.y - position.y),
                    z: destination.position.z + offset(PLAYER_RELATIVE_Z, at.z - position.z),
                },
                y_rot: destination.y_rot + turn(PLAYER_RELATIVE_Y_ROT, rider.y_rot() - y_rot),
                x_rot: destination.x_rot + turn(PLAYER_RELATIVE_X_ROT, rider.x_rot() - x_rot),
                ..destination
            };
            self.teleport_stored(passenger, rider_destination, relatives_mask, true);
        }
        let Some(entity) = self.get_mut(id) else {
            return false;
        };
        entity.teleport_set_position(destination, relatives_mask);
        self.refile(id);
        true
    }
}
