//! Client leash render-state extraction.
//!
//! Mirrors vanilla `EntityRenderer.extractRenderState`'s leash branch: a leashed entity
//! carries one `LeashState` (or four for a quad connection) with the rotated attach offset,
//! world-space start/end points, per-endpoint block/sky light, and the slack flag the rope
//! geometry uses for its sag curve. The store gathers per-entity inputs; everything here is
//! pure math against the vanilla formulas.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_GHAST_ID,
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_LEASH_KNOT_ID, VANILLA_ENTITY_TYPE_OCELOT_ID,
    VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
    VANILLA_ENTITY_TYPE_RABBIT_ID, VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
};

use super::{is_vanilla_boat_type, EntityVec3};

/// One rope's worth of vanilla `EntityRenderState.LeashState`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityLeashRenderState {
    /// The leashed entity's rotated attach offset (`getLeashOffset().yRot(-bodyRot)`).
    pub offset: [f64; 3],
    /// World-space rope start (leashed entity position plus offset).
    pub start: [f64; 3],
    /// World-space rope end (`getRopeHoldPosition` of the holder, or the quad holder point).
    pub end: [f64; 3],
    pub start_block_light: u8,
    pub end_block_light: u8,
    pub start_sky_light: u8,
    pub end_sky_light: u8,
    /// Vanilla defaults the sag curve on; quad connections pull the rope taut.
    pub slack: bool,
}

/// The per-endpoint inputs the store gathers for either side of a leash.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LeashEndpointInputs {
    pub(crate) entity_type_id: i32,
    pub(crate) position: EntityVec3,
    /// Degrees; vanilla `getPreciseBodyRotation(partialTicks)`.
    pub(crate) body_y_rot: f32,
    /// Degrees; the player rope-hold's reversed half lerp is already applied.
    pub(crate) rope_hold_x_rot: f32,
    pub(crate) bounds_width: f32,
    pub(crate) bounds_height: f32,
    pub(crate) eye_height: f32,
    /// Vanilla `EntityRenderer.getBlockLightLevel` reads 15 for a burning entity.
    pub(crate) on_fire: bool,
    pub(crate) fall_flying_or_spin: bool,
    pub(crate) visually_swimming: bool,
    pub(crate) crouching: bool,
    pub(crate) main_arm_right: bool,
    pub(crate) delta_movement: EntityVec3,
}

fn vec_y_rot(vec: [f64; 3], radians: f32) -> [f64; 3] {
    let cos = f64::from(radians.cos());
    let sin = f64::from(radians.sin());
    [
        vec[0] * cos + vec[2] * sin,
        vec[1],
        vec[2] * cos - vec[0] * sin,
    ]
}

fn vec_x_rot(vec: [f64; 3], radians: f32) -> [f64; 3] {
    let cos = f64::from(radians.cos());
    let sin = f64::from(radians.sin());
    [
        vec[0],
        vec[1] * cos + vec[2] * sin,
        vec[2] * cos - vec[1] * sin,
    ]
}

fn vec_z_rot(vec: [f64; 3], radians: f32) -> [f64; 3] {
    let cos = f64::from(radians.cos());
    let sin = f64::from(radians.sin());
    [
        vec[0] * cos + vec[1] * sin,
        vec[1] * cos - vec[0] * sin,
        vec[2],
    ]
}

fn vec_add(position: EntityVec3, vec: [f64; 3]) -> [f64; 3] {
    [
        position.x + vec[0],
        position.y + vec[1],
        position.z + vec[2],
    ]
}

/// Vanilla `Leashable.getLeashOffset` with every subclass override.
fn leash_offset(inputs: &LeashEndpointInputs) -> [f64; 3] {
    let eye = f64::from(inputs.eye_height);
    let width = f64::from(inputs.bounds_width);
    match inputs.entity_type_id {
        VANILLA_ENTITY_TYPE_ALLAY_ID => [0.0, eye * 0.6, width * 0.1],
        VANILLA_ENTITY_TYPE_BEE_ID => [0.0, eye * 0.5, width * 0.2],
        VANILLA_ENTITY_TYPE_OCELOT_ID | VANILLA_ENTITY_TYPE_PARROT_ID => {
            [0.0, eye * 0.5, width * 0.4]
        }
        VANILLA_ENTITY_TYPE_PIG_ID
        | VANILLA_ENTITY_TYPE_WOLF_ID
        | VANILLA_ENTITY_TYPE_STRIDER_ID
        | VANILLA_ENTITY_TYPE_RABBIT_ID => [0.0, eye * 0.6, width * 0.4],
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => [0.0, eye * 0.875, width * 0.4],
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => [0.0; 3],
        _ => [0.0, eye, width * 0.4],
    }
}

/// Vanilla `Leashable.createQuadLeashOffsets(entity, frontOffset, frontBack, leftRight, height)`.
fn quad_leash_offsets(
    inputs: &LeashEndpointInputs,
    front_offset: f64,
    front_back: f64,
    left_right: f64,
    height: f64,
) -> [[f64; 3]; 4] {
    let width = f64::from(inputs.bounds_width);
    let front_offset = front_offset * width;
    let front_back = front_back * width;
    let left_right = left_right * width;
    let height = height * f64::from(inputs.bounds_height);
    [
        [-left_right, height, front_back + front_offset],
        [-left_right, height, -front_back + front_offset],
        [left_right, height, -front_back + front_offset],
        [left_right, height, front_back + front_offset],
    ]
}

/// Vanilla `Entity.getRopeHoldPosition` with the Player and LeashKnot overrides.
fn rope_hold_position(holder: &LeashEndpointInputs) -> [f64; 3] {
    match holder.entity_type_id {
        VANILLA_ENTITY_TYPE_LEASH_KNOT_ID => vec_add(holder.position, [0.0, 0.2, 0.0]),
        VANILLA_ENTITY_TYPE_PLAYER_ID => {
            let x_off = 0.22 * if holder.main_arm_right { -1.0 } else { 1.0 };
            let x_rot = holder.rope_hold_x_rot.to_radians();
            let y_rot = holder.body_y_rot.to_radians();
            let offset = if holder.fall_flying_or_spin {
                let movement = holder.delta_movement;
                // Vanilla feeds `getViewVector(partialTicks)` (head yaw, full-lerp pitch)
                // here; the body yaw and rope-hold pitch stand in for the gliding edge
                // case, whose roll sign they reproduce for straight flight.
                let look = look_vector(holder.rope_hold_x_rot, holder.body_y_rot);
                let speed_sq = movement.x * movement.x + movement.z * movement.z;
                let look_sq = look[0] * look[0] + look[2] * look[2];
                let z_rot = if speed_sq > 0.0 && look_sq > 0.0 {
                    let dot =
                        (movement.x * look[0] + movement.z * look[2]) / (speed_sq * look_sq).sqrt();
                    let sign = movement.x * look[2] - movement.z * look[0];
                    (sign.signum() * dot.clamp(-1.0, 1.0).acos()) as f32
                } else {
                    0.0
                };
                vec_y_rot(
                    vec_x_rot(vec_z_rot([x_off, -0.11, 0.85], -z_rot), -x_rot),
                    -y_rot,
                )
            } else if holder.visually_swimming {
                vec_y_rot(vec_x_rot([x_off, 0.2, -0.15], -x_rot), -y_rot)
            } else {
                let y_off = f64::from(holder.bounds_height) - 1.0;
                let z_off = if holder.crouching { -0.2 } else { 0.07 };
                vec_y_rot([x_off, y_off, z_off], -y_rot)
            };
            vec_add(holder.position, offset)
        }
        _ => vec_add(
            holder.position,
            [0.0, f64::from(holder.eye_height) * 0.7, 0.0],
        ),
    }
}

/// Vanilla `Entity.getViewVector` from pitch/yaw degrees.
fn look_vector(x_rot: f32, y_rot: f32) -> [f64; 3] {
    let pitch = x_rot.to_radians();
    let yaw = -y_rot.to_radians();
    let cos_pitch = f64::from(pitch.cos());
    [
        f64::from(yaw.sin()) * cos_pitch,
        f64::from(-pitch.sin()),
        f64::from(yaw.cos()) * cos_pitch,
    ]
}

/// Whether vanilla renders this pairing as a four-rope quad connection
/// (`roper.supportQuadLeashAsHolder() && leashable.supportQuadLeash()`).
fn is_quad_connection(leashed_type_id: i32, holder_type_id: i32) -> bool {
    (holder_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        || holder_type_id == VANILLA_ENTITY_TYPE_GHAST_ID)
        && is_vanilla_boat_type(leashed_type_id)
}

/// Assembles the leash rope states without lights; the store fills the light
/// fields from the endpoint eye positions afterwards.
pub(crate) fn leash_render_states(
    leashed: &LeashEndpointInputs,
    holder: &LeashEndpointInputs,
) -> Vec<EntityLeashRenderState> {
    let leashed_body_radians = leashed.body_y_rot.to_radians();
    if is_quad_connection(leashed.entity_type_id, holder.entity_type_id) {
        let holder_body_radians = holder.body_y_rot.to_radians();
        // Boats declare `createQuadLeashOffsets(0, 0.64, 0.382, 0.88)`; the holder side is
        // the `Entity.getQuadLeashHolderOffsets` default `(0, 0.5, 0.5, 0)`.
        let leashed_points = quad_leash_offsets(leashed, 0.0, 0.64, 0.382, 0.88);
        let holder_points = quad_leash_offsets(holder, 0.0, 0.5, 0.5, 0.0);
        leashed_points
            .iter()
            .zip(holder_points.iter())
            .map(|(leashed_point, holder_point)| {
                let offset = vec_y_rot(*leashed_point, -leashed_body_radians);
                EntityLeashRenderState {
                    offset,
                    start: vec_add(leashed.position, offset),
                    end: vec_add(
                        holder.position,
                        vec_y_rot(*holder_point, -holder_body_radians),
                    ),
                    start_block_light: 0,
                    end_block_light: 0,
                    start_sky_light: 0,
                    end_sky_light: 0,
                    slack: false,
                }
            })
            .collect()
    } else {
        let offset = vec_y_rot(leash_offset(leashed), -leashed_body_radians);
        vec![EntityLeashRenderState {
            offset,
            start: vec_add(leashed.position, offset),
            end: rope_hold_position(holder),
            start_block_light: 0,
            end_block_light: 0,
            start_sky_light: 0,
            end_sky_light: 0,
            slack: true,
        }]
    }
}
