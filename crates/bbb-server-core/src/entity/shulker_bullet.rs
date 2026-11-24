//! Stored `ShulkerBullet` homing, collision, hit and persistence state.

use crate::level_event::{PendingHurtEntity, VanillaGameEvent};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID;
use bbb_protocol::packets::{
    BlockPos, Direction, LevelParticles, ParticlePayload, SoundSource, Vec3d,
};
use bbb_protocol::particle_types::{
    VANILLA_PARTICLE_TYPE_CRIT_ID, VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
};
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::{Axis, Vec3Ops};
use uuid::Uuid;

use crate::entity::hurting_projectile::{self, outside_owner_collision_range, ProjectileState};
use crate::entity::level::NearbyPlayer;
use crate::entity::projectile_action::{
    PendingProjectileAction, PendingProjectileHurt, PendingProjectileMobEffect,
    ProjectileLandOrigin,
};
use crate::entity::projectile_hit::{
    hit_result_on_move_vector, mth_atan2, HitResult, ProjectileTarget,
};
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

pub const DEFAULT_GRAVITY: f64 = 0.04;
const SPEED: f64 = 0.15;

pub(crate) fn check_despawn(entity: &mut Entity, difficulty: Option<Difficulty>) {
    if difficulty == Some(Difficulty::Peaceful) {
        entity.discard();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShulkerBulletTarget {
    pub uuid: Uuid,
    pub position: Vec3d,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShulkerBulletData {
    projectile: ProjectileState,
    target: Option<Uuid>,
    current_direction: Option<Direction>,
    flight_steps: i32,
    target_delta: Vec3d,
}

impl ShulkerBulletData {
    pub(crate) const fn from_saved(
        projectile: ProjectileState,
        target: Option<Uuid>,
        current_direction: Option<Direction>,
        flight_steps: i32,
        target_delta: Vec3d,
    ) -> Self {
        Self {
            projectile,
            target,
            current_direction,
            flight_steps,
            target_delta,
        }
    }

    pub const fn projectile(&self) -> &ProjectileState {
        &self.projectile
    }

    pub(crate) fn projectile_mut(&mut self) -> &mut ProjectileState {
        &mut self.projectile
    }

    pub const fn target(&self) -> Option<Uuid> {
        self.target
    }

    pub const fn current_direction(&self) -> Option<Direction> {
        self.current_direction
    }

    pub const fn flight_steps(&self) -> i32 {
        self.flight_steps
    }

    pub const fn target_delta(&self) -> Vec3d {
        self.target_delta
    }
}

#[cfg(test)]
pub(crate) fn spawn_shulker_bullet(
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    owner_entity_id: i32,
    target: ShulkerBulletTarget,
    invalid_start_axis: Option<Axis>,
) -> Option<i32> {
    let owner = store.get(owner_entity_id)?.clone();
    spawn_shulker_bullet_from(store, level, &owner, target, invalid_start_axis)
}

/// `new ShulkerBullet(level, owner, target, invalidStartAxis)`
/// (`ShulkerBullet.java:50-58`) for an owner that may be detached from `store`.
pub(crate) fn spawn_shulker_bullet_from(
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    owner: &Entity,
    target: ShulkerBulletTarget,
    invalid_start_axis: Option<Axis>,
) -> Option<i32> {
    if !target.position.x.is_finite()
        || !target.position.y.is_finite()
        || !target.position.z.is_finite()
        || !target.height.is_finite()
        || target.height < 0.0
    {
        return None;
    }
    owner.living()?;
    let owner_uuid = owner.uuid();
    let position = owner.bounding_box().center();
    let uuid = store.next_uuid();
    let mut data = ShulkerBulletData {
        projectile: ProjectileState::new(Some(owner_uuid)),
        target: Some(target.uuid),
        current_direction: Some(Direction::Up),
        flight_steps: 0,
        target_delta: zero_vector(),
    };
    select_next_move_direction(
        position,
        containing(position),
        invalid_start_axis,
        Some(target),
        &mut data,
        store,
        level,
    );
    store.restore(
        uuid,
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        position,
        EntityData::ShulkerBullet(data),
    )
}

#[derive(Debug, Clone, Copy)]
struct ResolvedTarget {
    uuid: Uuid,
    position: Vec3d,
    height: f32,
    alive: bool,
    spectator: bool,
}

impl From<NearbyPlayer> for ResolvedTarget {
    fn from(player: NearbyPlayer) -> Self {
        Self {
            uuid: player.uuid,
            position: player.position,
            height: player.dimensions[1],
            alive: !player.dead_or_dying,
            spectator: player.spectator,
        }
    }
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if !projectile_tick(entity, store, level) {
        return false;
    }

    let EntityData::ShulkerBullet(data) = entity.data() else {
        return true;
    };
    let target = data
        .target
        .and_then(|uuid| resolve_target(uuid, store, &*level));
    let left_owner = data.projectile.left_owner()
        || outside_owner_collision_range(entity, store, &*level, &data.projectile);
    let owner = data.projectile.owner();
    let owner_vehicle =
        crate::entity::throwable_projectile::owner_vehicle(owner, left_owner, store, level);
    let movement = entity.delta_movement();
    let gravity = entity.gravity();
    let EntityData::ShulkerBullet(data) = &mut entity.data else {
        return true;
    };
    if data.target.is_some() && target.is_none() {
        data.target = None;
    }
    let movement = if target.is_none_or(|target| !target.alive || target.spectator) {
        movement.plus(Vec3d {
            x: 0.0,
            y: -gravity,
            z: 0.0,
        })
    } else {
        data.target_delta = Vec3d {
            x: (data.target_delta.x * 1.025).clamp(-1.0, 1.0),
            y: (data.target_delta.y * 1.025).clamp(-1.0, 1.0),
            z: (data.target_delta.z * 1.025).clamp(-1.0, 1.0),
        };
        movement.plus(data.target_delta.plus(movement.scale(-1.0)).scale(0.2))
    };
    entity.set_delta_movement(movement);

    let hit = hit_result_on_move_vector(entity, store, &*level, |candidate, store, level| {
        candidate.is_alive_and_hurting_projectile_target_in_level(level)
            && !crate::entity::throwable_projectile::rides_with_owner(
                candidate,
                owner_vehicle,
                store,
            )
    });
    let Some(hit) = hit else {
        return false;
    };
    entity.set_pos(entity.position().plus(movement));
    store.refile_detached(entity);
    if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store)) {
        return false;
    }
    if entity
        .portal_process()
        .is_some_and(|portal| portal.is_inside_this_tick())
        && !crate::entity::handle_portal(entity, level)
    {
        return false;
    }
    if hit != HitResult::Miss && entity.is_alive() {
        let redirect_action = match hit {
            HitResult::Entity {
                target,
                deflection: crate::entity::projectile_hit::ProjectileDeflection::None,
                ..
            } => {
                let Ok(action) =
                    hurting_projectile::projectile_deflection_action(target, owner, store, &*level)
                else {
                    return false;
                };
                action
            }
            _ => None,
        };
        let deflected = matches!(
            hit,
            HitResult::Entity {
                target,
                deflection,
                ..
            } if hurting_projectile::apply_target_deflection(
                entity,
                store,
                target,
                deflection,
            )
        );
        if !deflected {
            if let Some(action) = redirect_action {
                store.queue_command(crate::entity::EntityCommand::Projectile(action));
            }
            if let EntityData::ShulkerBullet(data) = entity.data() {
                on_hit(entity, store, level, data, hit);
            }
        }
    }
    rotate_towards_movement(entity);

    if let Some(target) = target {
        let position = entity.position();
        let current = entity.block_position();
        let EntityData::ShulkerBullet(data) = &mut entity.data else {
            return true;
        };
        if data.flight_steps > 0 {
            data.flight_steps -= 1;
            if data.flight_steps == 0 {
                select_next_move_direction(
                    position,
                    current,
                    data.current_direction.map(direction_axis),
                    Some(target_snapshot(target)),
                    data,
                    store,
                    &*level,
                );
            }
        }
        if let Some(direction) = data.current_direction {
            let axis = direction_axis(direction);
            let ahead = relative(current, direction);
            let target_pos = containing(target.position);
            let reached_axis = match axis {
                Axis::X => current.x == target_pos.x,
                Axis::Y => current.y == target_pos.y,
                Axis::Z => current.z == target_pos.z,
            };
            // `Level.loadedAndEntityCanStandOn` (`ShulkerBullet.java:238`): a full top face.
            let ahead_state = level.block_state(ahead);
            let blocked =
                bbb_protocol::block_state_types::vanilla_block_state_support_facts(ahead_state)
                    .filter(|facts| !facts.dynamic_shape())
                    .map(|facts| {
                        facts.collision_face_full(
                            bbb_protocol::block_state_types::VanillaBlockSupportDirection::Up,
                        )
                    })
                    .unwrap_or_else(|| {
                        bbb_server_types::geometry::collision_face_full(
                            &level.block_collision_boxes(ahead),
                            bbb_protocol::packets::Direction::Up,
                        )
                        .unwrap_or(false)
                    });
            if blocked || reached_axis {
                select_next_move_direction(
                    position,
                    current,
                    Some(axis),
                    Some(target_snapshot(target)),
                    data,
                    store,
                    &*level,
                );
            }
        }
    }
    true
}

fn projectile_tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let EntityData::ShulkerBullet(data) = entity.data() else {
        return true;
    };
    if !data.projectile.has_been_shot() {
        let owner_id = data
            .projectile
            .owner()
            .and_then(|owner| level.player(owner).map(|player| player.entity_id))
            .or_else(|| {
                data.projectile
                    .owner()
                    .and_then(|owner| store.entity_id_for_uuid(owner))
            });
        level.game_event_at(
            VanillaGameEvent::PROJECTILE_SHOOT,
            entity.position(),
            owner_id,
        );
        if let EntityData::ShulkerBullet(data) = &mut entity.data {
            data.projectile.mark_shot();
        }
    }
    let EntityData::ShulkerBullet(data) = entity.data() else {
        return true;
    };
    if !data.projectile.left_owner() {
        let left_owner = outside_owner_collision_range(entity, store, level, &data.projectile);
        if let EntityData::ShulkerBullet(data) = &mut entity.data {
            data.projectile.set_left_owner(left_owner);
        }
    }
    crate::entity::damage::base_tick_from_environment(entity, store, level)
}

fn resolve_target(
    uuid: Uuid,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<ResolvedTarget> {
    if let Some(player) = level.player(uuid) {
        return Some(player.into());
    }
    let entity = store
        .entity_id_for_uuid(uuid)
        .and_then(|id| store.get(id))?;
    Some(ResolvedTarget {
        uuid,
        position: entity.position(),
        height: entity.bb_height(),
        alive: entity.is_alive(),
        spectator: false,
    })
}

fn target_snapshot(target: ResolvedTarget) -> ShulkerBulletTarget {
    ShulkerBulletTarget {
        uuid: target.uuid,
        position: target.position,
        height: target.height,
    }
}

#[allow(clippy::too_many_arguments)]
fn select_next_move_direction(
    position: Vec3d,
    current: BlockPos,
    avoid_axis: Option<Axis>,
    target: Option<ShulkerBulletTarget>,
    data: &mut ShulkerBulletData,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    let (target_pos, y_offset) = target.map_or(
        (
            BlockPos {
                x: current.x,
                y: current.y - 1,
                z: current.z,
            },
            0.5,
        ),
        |target| {
            let y_offset = f64::from(target.height) * 0.5;
            (
                containing(target.position.plus(Vec3d {
                    x: 0.0,
                    y: y_offset,
                    z: 0.0,
                })),
                y_offset,
            )
        },
    );
    let mut target_point = Vec3d {
        x: f64::from(target_pos.x) + 0.5,
        y: f64::from(target_pos.y) + y_offset,
        z: f64::from(target_pos.z) + 0.5,
    };
    let target_center = Vec3d {
        x: f64::from(target_pos.x) + 0.5,
        y: f64::from(target_pos.y) + 0.5,
        z: f64::from(target_pos.z) + 0.5,
    };
    let mut selection = None;
    if target_center.distance_to_sqr(position) >= 4.0 {
        let mut options = Vec::with_capacity(3);
        if avoid_axis != Some(Axis::X) {
            if current.x < target_pos.x && is_empty(level, relative(current, Direction::East)) {
                options.push(Direction::East);
            } else if current.x > target_pos.x
                && is_empty(level, relative(current, Direction::West))
            {
                options.push(Direction::West);
            }
        }
        if avoid_axis != Some(Axis::Y) {
            if current.y < target_pos.y && is_empty(level, relative(current, Direction::Up)) {
                options.push(Direction::Up);
            } else if current.y > target_pos.y
                && is_empty(level, relative(current, Direction::Down))
            {
                options.push(Direction::Down);
            }
        }
        if avoid_axis != Some(Axis::Z) {
            if current.z < target_pos.z && is_empty(level, relative(current, Direction::South)) {
                options.push(Direction::South);
            } else if current.z > target_pos.z
                && is_empty(level, relative(current, Direction::North))
            {
                options.push(Direction::North);
            }
        }
        let mut selected = random_direction(store);
        if options.is_empty() {
            for _ in 0..5 {
                if is_empty(level, relative(current, selected)) {
                    break;
                }
                selected = random_direction(store);
            }
        } else {
            selected = options[store.random().next_int(options.len() as i32) as usize];
        }
        selection = Some(selected);
        let (x, y, z) = direction_step(selected);
        target_point = position.plus(Vec3d {
            x: f64::from(x),
            y: f64::from(y),
            z: f64::from(z),
        });
    }
    data.current_direction = selection;
    let delta = target_point.plus(position.scale(-1.0));
    data.target_delta = if delta.length_sqr() == 0.0 {
        zero_vector()
    } else {
        delta.normalize().scale(SPEED)
    };
    data.flight_steps = 10 + store.random().next_int(5) * 10;
}

fn on_hit(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    data: &ShulkerBulletData,
    hit: HitResult,
) {
    match hit {
        HitResult::Entity {
            target, location, ..
        } => {
            queue_entity_hit(entity, store, level, data, target);
            store.queue_command(crate::entity::EntityCommand::Projectile(
                PendingProjectileAction::Land {
                    origin: ProjectileLandOrigin::Entity(location),
                    source_entity_id: entity.id(),
                },
            ));
        }
        HitResult::Block {
            location,
            pos,
            direction,
            ..
        } => {
            crate::block_behavior::projectile_hit::on_projectile_hit(
                level,
                store,
                entity,
                crate::block_behavior::projectile_hit::ProjectileBlockHit {
                    location,
                    pos,
                    direction,
                },
            );
            store.queue_command(crate::entity::EntityCommand::Projectile(
                PendingProjectileAction::Land {
                    origin: ProjectileLandOrigin::Block(pos),
                    source_entity_id: entity.id(),
                },
            ));
            send_particles(
                store,
                entity.position(),
                VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
                2,
            );
            play_sound(store, entity, "minecraft:entity.shulker_bullet.hit");
        }
        HitResult::Miss => {}
    }
    store.queue_command(crate::entity::EntityCommand::Projectile(
        PendingProjectileAction::Destroy {
            entity_id: entity.id(),
            position: entity.position(),
        },
    ));
}

fn queue_entity_hit(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    data: &ShulkerBulletData,
    target: ProjectileTarget,
) {
    let (hurt, effect) = pending_entity_hit(entity, data, target, store, level);
    store.queue_command(crate::entity::EntityCommand::Projectile(
        PendingProjectileAction::HurtThenMobEffect { hurt, effect },
    ));
}

fn pending_entity_hit(
    entity: &Entity,
    data: &ShulkerBulletData,
    target: ProjectileTarget,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> (PendingProjectileHurt, PendingProjectileMobEffect) {
    let target = crate::entity::throwable_projectile::pending_target(target);
    let hurt = PendingProjectileHurt {
        target,
        damage_type: "minecraft:mob_projectile",
        amount: 4.0,
        causing: crate::entity::throwable_projectile::pending_owner(
            data.projectile.owner(),
            store,
            level,
        ),
        direct: Some(PendingHurtEntity::Stored {
            entity_id: entity.id(),
        }),
        direct_resource_id: "minecraft:shulker_bullet",
        registry_tags: level.capture_registry_tags(),
    };
    (
        hurt,
        PendingProjectileMobEffect {
            resource_id: "minecraft:levitation",
            amplifier: 0,
            easy_duration_ticks: 200,
            normal_duration_ticks: 200,
            hard_duration_ticks: 200,
            source_owner: data.projectile.owner(),
            projectile_entity_id: entity.id(),
        },
    )
}

pub(crate) fn hurt_server(entity: &mut Entity, store: &mut EntityStore) {
    play_sound(store, entity, "minecraft:entity.shulker_bullet.hurt");
    send_particles(store, entity.position(), VANILLA_PARTICLE_TYPE_CRIT_ID, 15);
    entity.discard();
}

fn send_particles(store: &mut EntityStore, position: Vec3d, particle_type_id: i32, count: i32) {
    store.send_particles(LevelParticles {
        override_limiter: false,
        always_show: false,
        position,
        offset: Vec3d {
            x: 0.2,
            y: 0.2,
            z: 0.2,
        },
        max_speed: 0.0,
        count,
        particle: ParticlePayload {
            particle_type_id,
            raw_options: Vec::new(),
        },
    });
}

fn play_sound(store: &mut EntityStore, entity: &Entity, resource_id: &str) {
    if entity.is_silent() {
        return;
    }
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: resource_id.to_string(),
        source: SoundSource::Hostile,
        volume: 1.0,
        pitch: 1.0,
    });
}

fn rotate_towards_movement(entity: &mut Entity) {
    let movement = entity.delta_movement();
    if movement.length_sqr() == 0.0 {
        return;
    }
    let horizontal = movement.horizontal_distance_sqr().sqrt();
    let y_rot = mth_atan2(movement.z, movement.x) as f32 * 180.0 / std::f32::consts::PI + 90.0;
    let x_rot = mth_atan2(horizontal, movement.y) as f32 * 180.0 / std::f32::consts::PI - 90.0;
    entity.set_rot(
        lerp_rotation(entity.y_rot(), y_rot),
        lerp_rotation(entity.x_rot(), x_rot),
    );
}

fn lerp_rotation(previous: f32, current: f32) -> f32 {
    if !current.is_finite() {
        return 0.0;
    }
    if !previous.is_finite() {
        return current;
    }
    let delta =
        ((f64::from(current) - f64::from(previous) + 180.0).rem_euclid(360.0) - 180.0) as f32;
    (current - delta) + 0.5 * delta
}

fn is_empty(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    block_state_is_air(level.block_state(pos))
}

fn random_direction(store: &mut EntityStore) -> Direction {
    match store.random().next_int(6) {
        0 => Direction::Down,
        1 => Direction::Up,
        2 => Direction::North,
        3 => Direction::South,
        4 => Direction::West,
        _ => Direction::East,
    }
}

pub(crate) const fn direction_axis(direction: Direction) -> Axis {
    match direction {
        Direction::Down | Direction::Up => Axis::Y,
        Direction::North | Direction::South => Axis::Z,
        Direction::West | Direction::East => Axis::X,
    }
}

fn direction_step(direction: Direction) -> (i32, i32, i32) {
    match direction {
        Direction::Down => (0, -1, 0),
        Direction::Up => (0, 1, 0),
        Direction::North => (0, 0, -1),
        Direction::South => (0, 0, 1),
        Direction::West => (-1, 0, 0),
        Direction::East => (1, 0, 0),
    }
}

fn relative(pos: BlockPos, direction: Direction) -> BlockPos {
    let (x, y, z) = direction_step(direction);
    BlockPos {
        x: pos.x + x,
        y: pos.y + y,
        z: pos.z + z,
    }
}

fn containing(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

const fn zero_vector() -> Vec3d {
    Vec3d {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::lerp_rotation;

    #[test]
    fn lerp_rotation_handles_huge_and_non_finite_angles_in_o1() {
        let large = lerp_rotation(1.0e10_f32, 45.0);
        assert!(large.is_finite());
        assert_eq!(lerp_rotation(f32::INFINITY, 90.0), 90.0);
        assert_eq!(lerp_rotation(f32::NEG_INFINITY, -30.0), -30.0);
        assert_eq!(lerp_rotation(f32::NAN, 12.0), 12.0);
        assert_eq!(lerp_rotation(10.0, f32::NAN), 0.0);
        // Wraps across 180/-180 seam cleanly.
        let wrapped = lerp_rotation(170.0, -170.0);
        assert!((wrapped - (-180.0)).abs() < 1e-4 || (wrapped - 180.0).abs() < 1e-4);
    }
}
