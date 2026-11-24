use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_block_type_id,
};
use bbb_protocol::block_types::{VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_POWDER_SNOW_ID};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::{
    collide_with_boxes, collide_with_step, visit_block_collisions_with_scratch, Aabb, Vec3Ops,
};
use std::cell::Cell;

use crate::entity::store::EntityStore;
use crate::entity::supporting_block::{
    set_on_ground_with_movement, set_on_ground_with_movement_checked,
};
use crate::entity::{inside_blocks, mth_equal, Entity, MoverType};

mod emission;
pub use emission::{
    body_movement_effects, body_swim_sound_volume, BodyCrystalSoundState, BodyMovementEffect,
    BodyMovementEmissionState, BodyMovementFacts,
};

mod fall;
mod landing;
#[cfg(test)]
pub(crate) use fall::check_fall_damage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementEmission {
    None,
    Sounds,
    Events,
    All,
}

impl MovementEmission {
    /// `MovementEmission.emitsAnything()` (`Entity.java:4055-4057`).
    #[must_use]
    pub const fn emits_anything(self) -> bool {
        self.emits_events() || self.emits_sounds()
    }

    /// `MovementEmission.emitsEvents()`.
    #[must_use]
    pub const fn emits_events(self) -> bool {
        matches!(self, Self::Events | Self::All)
    }

    /// `MovementEmission.emitsSounds()`.
    #[must_use]
    pub const fn emits_sounds(self) -> bool {
        matches!(self, Self::Sounds | Self::All)
    }
}

///
/// The transcribed spine is: collide the requested delta, commit the clipped
/// movement, derive the three collision flags from where the clip differs from
/// the request, resolve the supporting block, account fall distance, zero the
/// blocked axes, run the movement emission's step sound (`:804-839`), and
/// finally scale the horizontal delta by the block speed factor.
///
/// Not transcribed, with the reason each one has no owner here:
///
/// * `maybeBackOffFromEdge` (`:732`) -- `Entity`'s own implementation returns
///   the delta unchanged (`:1031-1033`); only `Player` overrides it.
/// * `isHorizontalCollisionMinor` (`:764`) -- `Entity`'s own implementation
///   returns false (`:946-948`).
pub(crate) fn move_entity(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    mover: MoverType,
    delta: Vec3d,
) -> Option<f64> {
    move_entity_inner(
        entity,
        Some(store),
        level,
        mover,
        delta,
        None,
        |_, _, _, _| Ok::<(), std::convert::Infallible>(()),
    )
    .unwrap_or_else(|never| match never {})
}

pub(crate) use move_entity_with_ground_contact as move_living_entity;

pub(crate) fn move_entity_with_ground_contact(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    mover: MoverType,
    delta: Vec3d,
) -> Result<(), ()> {
    move_entity_inner(
        entity,
        Some(store),
        level,
        mover,
        delta,
        Some(()),
        |entity, store, level, distance| {
            landing::fall_on(entity, store.ok_or(())?, level, distance).ok_or(())
        },
    )
    .map(|_| ())
}

/// The rest of `ServerGamePacketListenerImpl.handleMoveVehicle` once the
/// rider's client placed its vehicle and its ground state
/// (`ServerGamePacketListenerImpl.java:470-475`, `:502-503`): the step
/// emission of `move(MoverType.PLAYER, ..)` and the fall check over the
/// client's delta.
pub(crate) fn finish_client_vehicle_move(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    delta: Vec3d,
) -> Result<(), ()> {
    if entity.living().is_some() && crate::entity::living::on_climbable(entity, level).ok_or(())? {
        entity.reset_fall_distance();
    }
    let effect_pos = entity.on_pos_legacy();
    let effect_state = level.block_state(effect_pos);
    let emission = entity.movement_emission();
    if emission.emits_anything() {
        apply_movement_emission(
            entity,
            store,
            level,
            emission,
            delta,
            effect_pos,
            effect_state,
        );
    }
    fall::check_with(
        entity,
        Some(store),
        level,
        delta.y,
        (effect_pos, effect_state),
        Some(()),
        |entity, store, level, distance| {
            landing::fall_on(entity, store.ok_or(())?, level, distance).ok_or(())
        },
    )
    .map(|_| ())
}

pub fn limit_body_piston_movement(
    piston_deltas: &mut [f64; 3],
    piston_deltas_game_time: &mut i64,
    delta: Vec3d,
    game_time: i64,
) -> Vec3d {
    if delta.length_sqr() <= 1.0e-7 {
        return delta;
    }
    if game_time != *piston_deltas_game_time {
        *piston_deltas = [0.0; 3];
        *piston_deltas_game_time = game_time;
    }
    let (axis, amount) = if delta.x != 0.0 {
        (0, delta.x)
    } else if delta.y != 0.0 {
        (1, delta.y)
    } else if delta.z != 0.0 {
        (2, delta.z)
    } else {
        return Vec3d::default();
    };
    let total = (amount + piston_deltas[axis]).clamp(-0.51, 0.51);
    let movement = total - piston_deltas[axis];
    piston_deltas[axis] = total;
    if movement.abs() <= f64::from(1.0e-5_f32) {
        return Vec3d::default();
    }
    match axis {
        0 => Vec3d {
            x: movement,
            y: 0.0,
            z: 0.0,
        },
        1 => Vec3d {
            x: 0.0,
            y: movement,
            z: 0.0,
        },
        _ => Vec3d {
            x: 0.0,
            y: 0.0,
            z: movement,
        },
    }
}

fn move_entity_inner<E>(
    entity: &mut Entity,
    mut store: Option<&mut EntityStore>,
    level: &mut LevelCtx<'_, '_>,
    mover: MoverType,
    mut delta: Vec3d,
    collision_error: Option<E>,
    mut on_fall: impl FnMut(
        &mut Entity,
        Option<&mut EntityStore>,
        &mut LevelCtx<'_, '_>,
        f64,
    ) -> Result<(), E>,
) -> Result<Option<f64>, E> {
    if entity.no_physics {
        entity.set_pos(entity.position.plus(delta));
        if let Some(store) = store.as_deref_mut() {
            store.refile_detached(entity);
        }
        entity.set_horizontal_collision(false);
        entity.vertical_collision = false;
        entity.vertical_collision_below = false;
        entity.minor_horizontal_collision = false;
        return Ok(None);
    }
    if mover == MoverType::Piston {
        delta = limit_body_piston_movement(
            &mut entity.piston_deltas,
            &mut entity.piston_deltas_game_time,
            delta,
            level.game_time(),
        );
        if delta == Vec3d::default() {
            return Ok(None);
        }
    }
    if entity.stuck_speed_multiplier.length_sqr() > 1.0e-7 {
        if mover != MoverType::Piston {
            let stuck = entity.stuck_speed_multiplier;
            delta = delta.multiply(stuck.x, stuck.y, stuck.z);
        }
        entity.stuck_speed_multiplier = Vec3d::default();
        entity.set_delta_movement(Vec3d::default());
    }
    let bounding_box = entity.bounding_box;
    let unavailable = Cell::new(false);
    let checked = collision_error.is_some();
    let collect = |query: Aabb| {
        let mut boxes = Vec::new();
        let _ = visit_block_collisions_with_scratch(
            query,
            |pos| {
                level.is_moving_piston_at(pos).unwrap_or_else(|| {
                    unavailable.set(true);
                    !checked
                })
            },
            |pos, out| {
                let start = out.len();
                if level
                    .push_block_collision_boxes_for_entity(entity, pos, out)
                    .is_none()
                {
                    out.truncate(start);
                    unavailable.set(true);
                    level.push_block_collision_boxes(pos, out);
                }
            },
            |_, shape| {
                boxes.push(shape);
                true
            },
        );
        // `Entity.collectColliders` (`Entity.java:1149-1152`).
        if let Some(border) = level.world_border() {
            boxes.extend(border.collision_boxes_at(entity.position(), query, level.game_time()));
        }
        if let Some(store) = store.as_deref() {
            // `AbstractBoat.canVehicleCollide` (`AbstractBoat.java:113-115`), shared by
            // `AbstractMinecart.canCollideWith` (`AbstractMinecart.java:163-165`).
            let vehicle = crate::entity::minecart::is_minecart(entity.data())
                || crate::entity::boat::is_boat(entity.data());
            let bodies = crate::entity::store::EntityBodyReadView::new(
                store,
                Some(entity),
                level.detached_bodies(),
            );
            let area = query.inflate(1.0e-7, 1.0e-7, 1.0e-7);
            let ids = if vehicle {
                bodies.ids_in_box_sections_with_players(area)
            } else {
                bodies.ids_in_box_sections(area)
            };
            for id in ids {
                if id == entity.id() {
                    continue;
                }
                let other_box = if let Some(other) = bodies.get(id) {
                    let collides = other.can_be_collided_with()
                        || vehicle
                            && crate::entity::push::pushable_short_of_climbing(other)
                                != Some(false);
                    if other.removal_reason().is_some() || !collides {
                        continue;
                    }
                    other.bounding_box()
                } else if let Some(player) = store.player(id) {
                    // Only a vehicle reaches here: a player is never `canBeCollidedWith`.
                    if !player.is_alive() || player.is_spectator() {
                        continue;
                    }
                    player.body().bounding_box()
                } else {
                    continue;
                };
                if !other_box.intersects(area) {
                    continue;
                }
                match bodies.same_vehicle(entity.id(), id) {
                    Some(false) => boxes.push(other_box),
                    Some(true) => {}
                    None => unavailable.set(true),
                }
            }
        }
        boxes
    };
    let movement = if delta.length_sqr() == 0.0 {
        delta
    } else {
        let clipped = collide_with_boxes(
            delta,
            bounding_box,
            &collect(bounding_box.expand_towards(delta)),
        );
        // `Entity.collide` (`Entity.java:1083-1101`).
        collide_with_step(
            bounding_box,
            delta,
            clipped,
            entity.on_ground(),
            entity
                .living()
                .map_or(0.0, crate::entity::living::max_up_step),
            |query| Some(collect(query)),
        )
        .expect("collision collection always succeeds")
    };
    if unavailable.get() {
        if let Some(error) = collision_error {
            return Err(error);
        }
    }

    // `:734-750`: the position only moves when the clip is meaningful, or when
    // the clip took away almost nothing.
    let movement_length = movement.length_sqr();
    if movement_length > 1.0e-7 || delta.length_sqr() - movement_length < 1.0e-7 {
        let from = entity.position;
        if entity.fall_distance != 0.0 && movement_length >= 1.0 {
            let distance = movement_length.sqrt().min(8.0);
            let check_to = from.plus(movement.normalize().scale(distance));
            match level.clip_fall_resetting(from, check_to) {
                Some(true) => entity.reset_fall_distance(),
                Some(false) => {}
                None => {
                    if let Some(error) = collision_error {
                        return Err(error);
                    }
                }
            }
        }
        let to = from.plus(movement);
        entity.record_movement(inside_blocks::EntityMovement::axis_dependent(
            from, to, delta,
        ));
        entity.set_pos(to);
        if let Some(store) = store.as_deref_mut() {
            store.refile_detached(entity);
        }
    }

    let x_collision = !mth_equal(delta.x, movement.x);
    let z_collision = !mth_equal(delta.z, movement.z);
    entity.set_horizontal_collision(x_collision || z_collision);
    entity.vertical_collision = delta.y != movement.y;
    entity.vertical_collision_below = entity.vertical_collision && delta.y < 0.0;
    let vertical_collision_below = entity.vertical_collision_below;
    let horizontal_collision = entity.horizontal_collision;
    let collision_error = match collision_error {
        Some(error) => {
            if set_on_ground_with_movement_checked(
                entity,
                level,
                vertical_collision_below,
                horizontal_collision,
                movement,
            )
            .is_none()
            {
                return Err(error);
            }
            Some(error)
        }
        None => {
            set_on_ground_with_movement(
                entity,
                level,
                vertical_collision_below,
                horizontal_collision,
                movement,
            );
            None
        }
    };
    entity.minor_horizontal_collision = false;

    // `:769-770`, read before the fall-damage check and reused by the emission.
    let effect_pos = entity.on_pos_legacy();
    let effect_state = level.block_state(effect_pos);

    let fall_on = fall::check_with(
        entity,
        store.as_deref_mut(),
        level,
        movement.y,
        (effect_pos, effect_state),
        collision_error,
        |entity, store, level, distance| on_fall(entity, store, level, distance),
    )?;

    if entity.removal_reason.is_some() {
        return Ok(fall_on);
    }

    // `:778-781`: a blocked axis loses its stored velocity outright.
    if entity.horizontal_collision {
        entity.delta_movement = Vec3d {
            x: if x_collision {
                0.0
            } else {
                entity.delta_movement.x
            },
            y: entity.delta_movement.y,
            z: if z_collision {
                0.0
            } else {
                entity.delta_movement.z
            },
        };
    }

    if delta.y != movement.y {
        entity.delta_movement = crate::block_behavior::ground_contact::movement_after_fall_on(
            effect_state,
            entity.living().is_some(),
            entity.get_shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            ),
            entity.delta_movement,
        )
        // The infallible move API retains its ordinary response for unknown states.
        .unwrap_or_else(|| entity.delta_movement.multiply(1.0, 0.0, 1.0));
    }

    // `:790-795`. `isPassenger()` is `Entity.vehicle != null`.
    let emission = entity.movement_emission();
    if emission.emits_anything() {
        if let Some(store) = store {
            if store.vehicle(entity.id()).is_none() {
                apply_movement_emission(
                    entity,
                    store,
                    level,
                    emission,
                    movement,
                    effect_pos,
                    effect_state,
                );
            }
        }
    }

    // `:797-798`.
    let speed_factor = f64::from(block_speed_factor(entity, level));
    entity.delta_movement = entity
        .delta_movement
        .multiply(speed_factor, 1.0, speed_factor);
    Ok(fall_on)
}

fn apply_movement_emission(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    emission: MovementEmission,
    delta: Vec3d,
    pos: BlockPos,
    state: i32,
) {
    emission::apply(entity, store, level, emission, delta, pos, state);
}

fn is_state_climbable(level: &LevelCtx<'_, '_>, block_state: i32) -> bool {
    if vanilla_block_state_block_type_id(block_state) == Some(VANILLA_BLOCK_POWDER_SNOW_ID) {
        return true;
    }
    crate::entity::level::block_state_tag_membership(level, block_state, "minecraft:climbable")
}

/// `PistonMovingBlockEntity.moveEntityByPiston` (`:192-198`): a real
/// `Entity.move(MoverType.PISTON, ..)` (`:195`) whose recorder entry is then
/// dropped (`:197`), with the block effects between the two positions run
/// straight after it (`:196`).
///
/// Being a real `Entity.move`, it runs the movement emission like any other
/// move -- hence the store the step sound queues on.
///
/// `no_clip` is the `NOCLIP` direction the call sets (`:193`, `:374-377`), so
/// the moving piston doing the pushing contributes only its head shape to this
/// one move instead of stopping the entity against itself.
#[cfg(test)]
pub(crate) fn move_entity_by_piston(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    delta: Vec3d,
    no_clip: bbb_protocol::packets::Direction,
) -> bool {
    let Some(previous) = begin_piston_move(entity, store, level, delta, no_clip) else {
        return false;
    };
    let resolved = crate::entity::apply_nonliving_inside_block_effects_between(
        entity,
        level,
        Some(&mut *store),
        previous,
        entity.position(),
    );
    finish_piston_contact(entity, level);
    resolved
}

fn begin_piston_move(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    delta: Vec3d,
    no_clip: bbb_protocol::packets::Direction,
) -> Option<Vec3d> {
    let previous = entity.position();
    level.set_piston_no_clip(Some(no_clip));
    match crate::entity::hanging::move_by_piston(entity, store, level, delta) {
        Some(false) => {
            level.set_piston_no_clip(None);
            return None;
        }
        Some(true) => {}
        None => {
            if move_entity_with_ground_contact(entity, store, level, MoverType::Piston, delta)
                .is_err()
            {
                level.set_piston_no_clip(None);
                return None;
            }
        }
    }
    Some(previous)
}

fn finish_piston_contact(entity: &mut Entity, level: &mut LevelCtx<'_, '_>) {
    entity.movement_this_tick.pop_back();
    level.set_piston_no_clip(None);
}

pub struct PendingPistonMove {
    id: i32,
    uuid: uuid::Uuid,
    inside: inside_blocks::NonlivingInsidePause,
}

impl PendingPistonMove {
    pub fn entity_id(&self) -> i32 {
        self.id
    }
    pub fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }
    pub fn hurt(&self) -> &crate::entity::projectile_action::PendingProjectileHurt {
        self.inside.hurt()
    }
    pub fn complete_hurt(&mut self, succeeded: bool) {
        self.inside.complete_hurt(succeeded);
    }

    pub(crate) fn resume(
        self,
        store: &mut EntityStore,
        level: &mut LevelCtx<'_, '_>,
    ) -> Option<std::ops::ControlFlow<Self, ()>> {
        let Self { id, uuid, inside } = self;
        if store.get(id).is_none_or(|body| body.uuid() != uuid) {
            // The hurt removed the body; nothing is left to move.
            level.set_piston_no_clip(None);
            return Some(std::ops::ControlFlow::Continue(()));
        }
        let flow = store.with_entity_mut(id, |body, store| {
            let flow = inside.resume(body, store, level);
            if !matches!(flow, Some(std::ops::ControlFlow::Break(_))) {
                finish_piston_contact(body, level);
            }
            flow
        })??;
        Some(flow.map_break(|inside| Self { id, uuid, inside }))
    }
}

pub(crate) fn move_entity_by_piston_pausing(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    delta: Vec3d,
    no_clip: bbb_protocol::packets::Direction,
) -> Result<Option<PendingPistonMove>, ()> {
    let previous = begin_piston_move(entity, store, level, delta, no_clip).ok_or(())?;
    let movements = vec![inside_blocks::EntityMovement::direct(
        previous,
        entity.position(),
    )];
    let inside =
        match inside_blocks::start_nonliving_inside_pausing(entity, store, level, movements) {
            Ok(inside) => inside,
            Err(()) => {
                finish_piston_contact(entity, level);
                return Err(());
            }
        };
    if let Some(inside) = inside {
        return Ok(Some(PendingPistonMove {
            id: entity.id(),
            uuid: entity.uuid(),
            inside,
        }));
    }
    finish_piston_contact(entity, level);
    Ok(None)
}

///
/// It is not the factor of one block: the block the entity stands *in*
/// answers, but only when it slows the entity down at all. A factor of exactly
/// `1.0F` there hands the question to
/// [`Entity::block_pos_below_that_affects_my_movement`] (`:1025`), which is
/// what makes soul sand slow an entity walking *on* it rather than only one
/// buried in it. Water and bubble columns are the exception (`:1024`): they
/// answer for themselves and never fall through, so a soul-sand floor under
/// water does not slow a swimmer.
fn block_speed_factor(entity: &Entity, level: &LevelCtx<'_, '_>) -> f32 {
    let base = entity_block_speed_factor(entity, level);
    match entity.living() {
        Some(living) => crate::entity::living::living_block_speed_factor(living, base),
        None => base,
    }
}

fn entity_block_speed_factor(entity: &Entity, level: &LevelCtx<'_, '_>) -> f32 {
    let here = level.block_speed_factor(entity.block_position);
    let state = level.block_state(entity.block_position);
    // The infallible Native adapter treats unknown identity as non-water, retaining its factor.
    let state = if vanilla_block_state_block_type_id(state).is_some() {
        state
    } else {
        vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).expect("the generated air state")
    };
    crate::movement_speed::select_block_speed_factor(state, here, || {
        Some(level.block_speed_factor(entity.block_pos_below_that_affects_my_movement()))
    })
    .expect("the normalized state and infallible below factor are available")
}
