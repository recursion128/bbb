use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Aabb;

use crate::entity::{EntityStore, ExternalPlayerCanRide, ExternalPlayerVehiclePlacement};

const HORIZONTAL_LIMIT: f64 = 30_000_000.0;
const VERTICAL_LIMIT: f64 = 20_000_000.0;
const TOO_QUICK_DISTANCE: f64 = 100.0;
const MOVED_WRONGLY_DISTANCE: f64 = 0.0625;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalPlayerBoatAdmissionFacts {
    pub can_ride: ExternalPlayerCanRide,
    pub secondary_use_active: bool,
    pub eye_in_water: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExternalPlayerBoatControllerSnapshot {
    pub player_entity_id: i32,
    pub vehicle_entity_id: i32,
    pub root_vehicle_entity_id: i32,
    pub vehicle_position: Vec3d,
    pub vehicle_delta_movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub vertical_collision_below: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExternalPlayerBoatMoveRequest {
    pub player_entity_id: i32,
    pub expected_vehicle_entity_id: i32,
    pub first_good_position: Vec3d,
    pub last_good_position: Vec3d,
    pub target_position: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
    pub singleplayer_owner: bool,
    pub player_scale: f32,
    pub player_pose_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExternalPlayerBoatCollisionProbe {
    pub vehicle_entity_id: i32,
    pub old_position: Vec3d,
    pub old_bounding_box: Aabb,
    pub requested_delta: Vec3d,
    pub target_position: Vec3d,
    pub on_ground: bool,
    pub max_up_step: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExternalPlayerBoatCollisionResolution {
    pub resolved_position: Vec3d,
    pub horizontal_collision: bool,
    pub old_position_clear: bool,
    pub collides_with_new_geometry: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalPlayerBoatCorrectionReason {
    TooQuick,
    MovedWrongly,
    NewCollision,
}

#[derive(Debug, PartialEq)]
#[must_use = "accepted external-player Boat movement must be finalized against its level"]
pub enum ExternalPlayerBoatMoveOutcome {
    /// `containsInvalidValues`: the listener disconnects the player.
    InvalidValues,
    /// The player does not control its root vehicle, which is not `lastVehicle`.
    Ignored,
    Correction {
        vehicle_entity_id: i32,
        position: Vec3d,
        y_rot: f32,
        x_rot: f32,
        reason: ExternalPlayerBoatCorrectionReason,
    },
    Moved {
        controller: Box<ExternalPlayerBoatControllerSnapshot>,
        placement: ExternalPlayerVehiclePlacement,
        old_position: Vec3d,
        new_position: Vec3d,
        client_delta: Vec3d,
        attempted_delta: Vec3d,
        on_ground: bool,
        horizontal_collision: bool,
        vehicle_was_supported: bool,
        finalization: ExternalPlayerBoatMoveFinalization,
        y_rot: f32,
        x_rot: f32,
    },
}

#[derive(Debug, PartialEq)]
pub struct ExternalPlayerBoatMoveFinalization {
    vehicle_entity_id: i32,
    client_delta: Vec3d,
    on_ground: bool,
    horizontal_collision: bool,
}

impl EntityStore {
    pub fn admit_external_player_to_boat(
        &mut self,
        player_entity_id: i32,
        vehicle_entity_id: i32,
        facts: ExternalPlayerBoatAdmissionFacts,
    ) -> bool {
        if facts.secondary_use_active
            || facts.can_ride != ExternalPlayerCanRide::Allowed
            || facts.eye_in_water
        {
            return false;
        }
        let Some(boat) = self
            .get(vehicle_entity_id)
            .and_then(|entity| crate::entity::boat::boat(entity.data()))
        else {
            return false;
        };
        if boat.out_of_control_ticks() >= crate::entity::boat::TIME_TO_EJECT {
            return false;
        }
        self.attach_external_player(player_entity_id, vehicle_entity_id, facts.can_ride)
    }

    /// The vehicle `vehicle_entity_id` the caller proved `player_entity_id`
    /// controls (`Entity.getControllingPassenger`).
    pub fn external_player_vehicle_controller(
        &self,
        player_entity_id: i32,
        vehicle_entity_id: i32,
    ) -> Option<ExternalPlayerBoatControllerSnapshot> {
        if self.external_player_vehicle(player_entity_id) != Some(vehicle_entity_id) {
            return None;
        }
        let root_vehicle_entity_id = self.external_player_root_vehicle(player_entity_id)?;
        let vehicle = self.get(vehicle_entity_id)?;
        Some(ExternalPlayerBoatControllerSnapshot {
            player_entity_id,
            vehicle_entity_id,
            root_vehicle_entity_id,
            vehicle_position: vehicle.position(),
            vehicle_delta_movement: vehicle.delta_movement(),
            y_rot: vehicle.y_rot(),
            x_rot: vehicle.x_rot(),
            vertical_collision_below: vehicle.vertical_collision_below(),
        })
    }

    pub fn move_external_player_controlled_boat<F>(
        &mut self,
        request: ExternalPlayerBoatMoveRequest,
        resolve_collision: F,
    ) -> ExternalPlayerBoatMoveOutcome
    where
        F: FnOnce(ExternalPlayerBoatCollisionProbe) -> ExternalPlayerBoatCollisionResolution,
    {
        if contains_invalid_values(request) {
            return ExternalPlayerBoatMoveOutcome::InvalidValues;
        }
        let Some(controller) = self
            .external_player_vehicle_controller(
                request.player_entity_id,
                request.expected_vehicle_entity_id,
            )
            .filter(|controller| controller.root_vehicle_entity_id == controller.vehicle_entity_id)
        else {
            return ExternalPlayerBoatMoveOutcome::Ignored;
        };

        let target_position = Vec3d {
            x: request
                .target_position
                .x
                .clamp(-HORIZONTAL_LIMIT, HORIZONTAL_LIMIT),
            y: request
                .target_position
                .y
                .clamp(-VERTICAL_LIMIT, VERTICAL_LIMIT),
            z: request
                .target_position
                .z
                .clamp(-HORIZONTAL_LIMIT, HORIZONTAL_LIMIT),
        };
        let y_rot = crate::entity::living::wrap_degrees(request.y_rot);
        let x_rot = crate::entity::living::wrap_degrees(request.x_rot);
        let old_position = controller.vehicle_position;
        let vehicle_was_supported = controller.vertical_collision_below;
        let correction = |reason, correction_y_rot, correction_x_rot| {
            ExternalPlayerBoatMoveOutcome::Correction {
                vehicle_entity_id: controller.vehicle_entity_id,
                position: old_position,
                y_rot: correction_y_rot,
                x_rot: correction_x_rot,
                reason,
            }
        };
        let from_first = subtract(target_position, request.first_good_position);
        let moved_distance = length_squared(from_first);
        let raw_expected = length_squared(controller.vehicle_delta_movement);
        let expected_distance = if raw_expected.is_finite() {
            raw_expected
        } else {
            0.0
        };
        if !moved_distance.is_finite()
            || (moved_distance - expected_distance > TOO_QUICK_DISTANCE
                && !request.singleplayer_owner)
        {
            return correction(
                ExternalPlayerBoatCorrectionReason::TooQuick,
                controller.y_rot,
                controller.x_rot,
            );
        }

        let requested_delta = subtract(target_position, request.last_good_position);
        // Hard anti-DoS cap on single-packet vehicle displacement and swept collision volume,
        // matching `MovePlayer`'s budget even for high-velocity or singleplayer-owner moves.
        const MAX_UNCHECKED_MOVE_AXIS_DELTA: f64 = 512.0;
        const MAX_SWEEP_COLLISION_BLOCKS: u64 = 65_536;
        let dx = requested_delta
            .x
            .abs()
            .max((target_position.x - old_position.x).abs());
        let dy = requested_delta
            .y
            .abs()
            .max((target_position.y - old_position.y).abs());
        let dz = requested_delta
            .z
            .abs()
            .max((target_position.z - old_position.z).abs());
        let swept_blocks = (dx.ceil() as u64 + 3)
            .saturating_mul(dy.ceil() as u64 + 3)
            .saturating_mul(dz.ceil() as u64 + 3);
        if !dx.is_finite()
            || !dy.is_finite()
            || !dz.is_finite()
            || dx > MAX_UNCHECKED_MOVE_AXIS_DELTA
            || dy > MAX_UNCHECKED_MOVE_AXIS_DELTA
            || dz > MAX_UNCHECKED_MOVE_AXIS_DELTA
            || swept_blocks > MAX_SWEEP_COLLISION_BLOCKS
        {
            return correction(
                ExternalPlayerBoatCorrectionReason::TooQuick,
                controller.y_rot,
                controller.x_rot,
            );
        }
        let vehicle = self
            .get(controller.vehicle_entity_id)
            .expect("controller vehicle");
        // Entity.java:3835-3837, LivingEntity.java:3910-3913: non-living vehicles
        // step 0.0F; any player-controlled LivingEntity steps at least 1.0F.
        let max_up_step = vehicle.living().map_or(0.0, |living| {
            crate::entity::living::max_up_step(living).max(1.0)
        });
        let resolution = resolve_collision(ExternalPlayerBoatCollisionProbe {
            vehicle_entity_id: controller.vehicle_entity_id,
            old_position,
            old_bounding_box: vehicle.bounding_box(),
            requested_delta,
            target_position,
            on_ground: vehicle.on_ground(),
            max_up_step,
        });
        let unapplied_delta = subtract(target_position, resolution.resolved_position);
        let moved_wrongly = unapplied_delta.x * unapplied_delta.x
            + unapplied_delta.z * unapplied_delta.z
            > MOVED_WRONGLY_DISTANCE;
        if moved_wrongly && resolution.old_position_clear {
            self.get_mut(controller.vehicle_entity_id)
                .expect("controller vehicle")
                .set_rot(y_rot, x_rot);
            return correction(
                ExternalPlayerBoatCorrectionReason::MovedWrongly,
                y_rot,
                x_rot,
            );
        }
        if resolution.collides_with_new_geometry {
            self.get_mut(controller.vehicle_entity_id)
                .expect("controller vehicle")
                .set_rot(y_rot, x_rot);
            return correction(
                ExternalPlayerBoatCorrectionReason::NewCollision,
                y_rot,
                x_rot,
            );
        }

        let vehicle = self
            .get_mut(controller.vehicle_entity_id)
            .expect("controller vehicle");
        vehicle.set_pos(target_position);
        vehicle.set_rot(y_rot, x_rot);
        self.refile(controller.vehicle_entity_id);
        let controller = self
            .external_player_vehicle_controller(
                request.player_entity_id,
                request.expected_vehicle_entity_id,
            )
            .expect("committed controller relation");
        let placement = self
            .external_player_vehicle_placement_with_rider(
                request.player_entity_id,
                y_rot,
                request.player_scale,
                request.player_pose_id,
            )
            .expect("committed controller placement");
        let client_delta = subtract(target_position, old_position);
        ExternalPlayerBoatMoveOutcome::Moved {
            controller: Box::new(controller),
            placement,
            old_position,
            new_position: target_position,
            client_delta,
            attempted_delta: requested_delta,
            on_ground: request.on_ground,
            horizontal_collision: resolution.horizontal_collision,
            vehicle_was_supported,
            finalization: ExternalPlayerBoatMoveFinalization {
                vehicle_entity_id: request.expected_vehicle_entity_id,
                client_delta,
                on_ground: request.on_ground,
                horizontal_collision: resolution.horizontal_collision,
            },
            y_rot,
            x_rot,
        }
    }

    pub(crate) fn finalize_external_player_boat_move(
        &mut self,
        finalization: ExternalPlayerBoatMoveFinalization,
        level: &mut LevelCtx<'_, '_>,
    ) {
        self.with_entity_mut(finalization.vehicle_entity_id, |vehicle, store| {
            crate::entity::set_on_ground_with_movement(
                vehicle,
                level,
                finalization.on_ground,
                finalization.horizontal_collision,
                finalization.client_delta,
            );
            if crate::entity::movement::finish_client_vehicle_move(
                vehicle,
                store,
                level,
                finalization.client_delta,
            )
            .is_err()
            {
                tracing::warn!("client vehicle move dependency unavailable");
            }
        });
    }

    pub fn apply_external_player_boat_paddles(
        &mut self,
        player_entity_id: i32,
        left: bool,
        right: bool,
    ) {
        if let Some(vehicle) = self
            .external_player_direct_controlled_boat(player_entity_id)
            .and_then(|vehicle_entity_id| self.get_mut(vehicle_entity_id))
        {
            crate::entity::boat::set_paddle_state(vehicle, left, right);
        }
    }
}

fn contains_invalid_values(request: ExternalPlayerBoatMoveRequest) -> bool {
    request.target_position.x.is_nan()
        || request.target_position.y.is_nan()
        || request.target_position.z.is_nan()
        || !request.y_rot.is_finite()
        || !request.x_rot.is_finite()
}

fn subtract(left: Vec3d, right: Vec3d) -> Vec3d {
    Vec3d {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

fn length_squared(value: Vec3d) -> f64 {
    value.x * value.x + value.y * value.y + value.z * value.z
}
