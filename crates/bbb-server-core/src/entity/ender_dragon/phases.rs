//! `EnderDragonPhase` and the server half of each `DragonPhaseInstance`
//! (`net/minecraft/world/entity/boss/enderdragon/phases`).

use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::{mth_cos, mth_sin, Vec3Ops};

use super::path::{DragonNodes, Node};
use super::{EnderDragonState, PhaseScratch};
use crate::entity::area_effect_cloud::{spawn_area_effect_cloud, AreaEffectCloudData};
use crate::entity::hurting_projectile::dragon_fireball::{
    dragon_breath_particle, spawn_dragon_fireball,
};
use crate::entity::living;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::{Entity, EntityStore, NearbyPlayer};
use crate::world::simulation_level::LevelCtx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DragonPhase {
    HoldingPattern,
    StrafePlayer,
    LandingApproach,
    Landing,
    Takeoff,
    SittingFlaming,
    SittingScanning,
    SittingAttacking,
    ChargingPlayer,
    Dying,
    Hover,
}

impl DragonPhase {
    /// `EnderDragonPhase`'s registration order (`EnderDragonPhase.java:9-19`).
    const ALL: [Self; 11] = [
        Self::HoldingPattern,
        Self::StrafePlayer,
        Self::LandingApproach,
        Self::Landing,
        Self::Takeoff,
        Self::SittingFlaming,
        Self::SittingScanning,
        Self::SittingAttacking,
        Self::ChargingPlayer,
        Self::Dying,
        Self::Hover,
    ];

    pub(crate) fn id(self) -> i32 {
        Self::ALL
            .iter()
            .position(|phase| *phase == self)
            .expect("every phase is registered") as i32
    }

    /// `EnderDragonPhase.getById`: an unknown id holds the pattern.
    pub(crate) fn from_id(id: i32) -> Self {
        usize::try_from(id)
            .ok()
            .and_then(|index| Self::ALL.get(index))
            .copied()
            .unwrap_or(Self::HoldingPattern)
    }

    pub(crate) const fn is_sitting(self) -> bool {
        matches!(
            self,
            Self::SittingFlaming | Self::SittingScanning | Self::SittingAttacking | Self::Hover
        )
    }

    pub(super) const fn fly_speed(self) -> f32 {
        match self {
            Self::ChargingPlayer | Self::Dying => 3.0,
            Self::Landing => 1.5,
            Self::Hover => 1.0,
            _ => 0.6,
        }
    }

    /// `getTurnSpeed`, which `DragonLandingPhase` overrides.
    pub(super) fn turn_speed(self, delta: Vec3d) -> f32 {
        let rot_speed = (delta.x * delta.x + delta.z * delta.z).sqrt() as f32 + 1.0;
        let dist = rot_speed.min(40.0);
        if self == Self::Landing {
            dist / rot_speed
        } else {
            0.7 / dist / rot_speed
        }
    }

    /// `AbstractDragonSittingPhase.onHurt`: a perched dragon shrugs off
    /// arrows and wind charges.
    pub(super) fn on_hurt(self, damage: f32, direct_is_arrow_or_wind_charge: bool) -> f32 {
        let sitting = matches!(
            self,
            Self::SittingFlaming | Self::SittingScanning | Self::SittingAttacking
        );
        if sitting && direct_is_arrow_or_wind_charge {
            0.0
        } else {
            damage
        }
    }
}

/// `EnderDragonPhaseManager.setPhase`: `end` the old phase, `begin` the new.
pub(super) fn set_phase(
    state: &mut EnderDragonState,
    entity: &mut Entity,
    store: Option<&mut EntityStore>,
    phase: DragonPhase,
) {
    if state.phase == phase {
        return;
    }
    // `DragonSittingFlamingPhase.end` discards its breath cloud.
    if let (Some(cloud), Some(store)) = (state.scratch.flame_cloud.take(), store) {
        store.remove(cloud, crate::entity::RemovalReason::Discarded);
    }
    state.phase = phase;
    entity.mark_synched_dirty(super::PHASE_DATA_ID);
    state.scratch = PhaseScratch {
        first_tick: phase == DragonPhase::Takeoff,
        ..PhaseScratch::default()
    };
    if phase == DragonPhase::SittingFlaming {
        state.flame_count += 1;
    }
}

/// `TargetingConditions.forCombat` against a player as `Player.canBeSeenAsEnemy`
/// answers it.
pub(super) fn can_attack(player: &NearbyPlayer) -> bool {
    !player.spectator && !player.instabuild && !player.invulnerable && !player.dead_or_dying
}

/// `ServerLevel.getNearestPlayer(conditions, dragon, x, y, z)`.
fn nearest_player(
    level: &LevelCtx<'_, '_>,
    to: Vec3d,
    accept: impl Fn(&NearbyPlayer) -> bool,
) -> Option<NearbyPlayer> {
    level
        .players()
        .iter()
        .filter(|player| can_attack(player) && accept(player))
        .min_by(|a, b| {
            a.position
                .distance_to_sqr(to)
                .total_cmp(&b.position.distance_to_sqr(to))
        })
        .cloned()
}

/// `getHeightmapPos(MOTION_BLOCKING_NO_LEAVES, EndPodiumFeature.getLocation(origin))`.
fn egg_position(level: &LevelCtx<'_, '_>) -> BlockPos {
    let y = level
        .motion_blocking_no_leaves_height(0, 0)
        .unwrap_or_else(|| level.min_y());
    BlockPos { x: 0, y, z: 0 }
}

fn distance_to_center_sqr(pos: BlockPos, to: Vec3d) -> f64 {
    let d = |a: i32, b: f64| f64::from(a) + 0.5 - b;
    let (x, y, z) = (d(pos.x, to.x), d(pos.y, to.y), d(pos.z, to.z));
    x * x + y * y + z * z
}

fn bottom_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y),
        z: f64::from(pos.z) + 0.5,
    }
}

fn crystals_alive(state: &EnderDragonState) -> bool {
    state.fight.is_some_and(|fight| fight.alive_crystals > 0)
}

fn nodes<'a>(state: &'a mut EnderDragonState, level: &LevelCtx<'_, '_>) -> &'a DragonNodes {
    state.nodes.get_or_insert_with(|| DragonNodes::build(level))
}

fn closest_node(state: &mut EnderDragonState, level: &LevelCtx<'_, '_>, at: Vec3d) -> usize {
    let alive = crystals_alive(state);
    nodes(state, level).closest(at.x, at.y, at.z, alive)
}

fn find_path(
    state: &mut EnderDragonState,
    level: &LevelCtx<'_, '_>,
    from: usize,
    to: usize,
    final_node: Option<Node>,
) {
    let alive = crystals_alive(state);
    state.scratch.path = nodes(state, level).find_path(from, to, final_node, alive);
}

/// The ring a holding or strafing dragon circles next: the outer ring while
/// the fight has crystals, the inner one without.
fn ring_target(target: usize, outer_ring: bool) -> usize {
    let target = target as i32;
    let ring = if outer_ring {
        target.rem_euclid(12)
    } else {
        ((target - 12) & 7) + 12
    };
    ring as usize
}

/// `navigateToNextPathNode`: the next node, raised by up to 20 blocks.
fn navigate(state: &mut EnderDragonState, store: &EntityStore) {
    let Some(path) = state.scratch.path.as_mut() else {
        return;
    };
    if path.is_done() {
        return;
    }
    let [x, y, z] = path.next_node();
    path.advance();
    state.scratch.target_location = Some(Vec3d {
        x: f64::from(x),
        y: f64::from(y) + f64::from(store.random().next_float() * 20.0),
        z: f64::from(z),
    });
}

fn path_done(state: &EnderDragonState) -> bool {
    state
        .scratch
        .path
        .as_ref()
        .is_none_or(super::path::DragonPath::is_done)
}

fn needs_new_target(state: &EnderDragonState, entity: &Entity) -> bool {
    let dist = state
        .scratch
        .target_location
        .map_or(0.0, |target| target.distance_to_sqr(entity.position()));
    dist < 100.0 || dist > 22500.0
}

/// One `DragonPhaseInstance.doServerTick`.
pub(super) fn server_tick(
    state: &mut EnderDragonState,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    match state.phase {
        DragonPhase::HoldingPattern => {
            if needs_new_target(state, entity) {
                holding_find_new_target(state, entity, store, level);
            }
        }
        DragonPhase::StrafePlayer => strafe_tick(state, entity, store, level),
        DragonPhase::LandingApproach => {
            if needs_new_target(state, entity) {
                landing_approach_find_new_target(state, entity, store, level);
            }
        }
        DragonPhase::Landing => {
            let target = *state
                .scratch
                .target_location
                .get_or_insert_with(|| bottom_center(egg_position(level)));
            if target.distance_to_sqr(entity.position()) < 1.0 {
                state.flame_count = 0;
                set_phase(state, entity, Some(store), DragonPhase::SittingScanning);
            }
        }
        DragonPhase::Takeoff => takeoff_tick(state, entity, store, level),
        DragonPhase::SittingScanning => scanning_tick(state, entity, store, level),
        DragonPhase::SittingAttacking => {
            state.scratch.ticks += 1;
            if state.scratch.ticks > 40 {
                set_phase(state, entity, Some(store), DragonPhase::SittingFlaming);
            }
        }
        DragonPhase::SittingFlaming => flaming_tick(state, entity, store, level),
        DragonPhase::ChargingPlayer => {
            if state.scratch.target_location.is_none() {
                set_phase(state, entity, Some(store), DragonPhase::HoldingPattern);
            } else if state.scratch.ticks > 0 {
                state.scratch.ticks += 1;
                if state.scratch.ticks > 10 {
                    set_phase(state, entity, Some(store), DragonPhase::HoldingPattern);
                }
            } else if needs_new_target(state, entity) {
                state.scratch.ticks += 1;
            }
        }
        DragonPhase::Dying => {
            state.scratch.ticks += 1;
            let target = *state.scratch.target_location.get_or_insert_with(|| {
                let y = level.motion_blocking_height(0, 0).unwrap_or(level.min_y());
                bottom_center(BlockPos { x: 0, y, z: 0 })
            });
            let dist = target.distance_to_sqr(entity.position());
            let health = if (100.0..=22500.0).contains(&dist) {
                1.0
            } else {
                0.0
            };
            living::set_health(entity, health);
        }
        DragonPhase::Hover => {
            if state.scratch.target_location.is_none() {
                state.scratch.target_location = Some(entity.position());
            }
        }
    }
}

/// `DragonHoldingPatternPhase.findNewTarget`.
fn holding_find_new_target(
    state: &mut EnderDragonState,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    if state
        .scratch
        .path
        .as_ref()
        .is_some_and(super::path::DragonPath::is_done)
    {
        let egg = egg_position(level);
        let crystals = state.fight.map_or(0, |fight| fight.alive_crystals);
        if store.random().next_int(crystals + 3) == 0 {
            set_phase(state, entity, Some(store), DragonPhase::LandingApproach);
            return;
        }
        let player = nearest_player(level, bottom_center(egg), |_| true);
        let dist_sqr = player.as_ref().map_or(64.0, |player| {
            distance_to_center_sqr(egg, player.position) / 512.0
        });
        if let Some(player) = player {
            if store.random().next_int((dist_sqr + 2.0) as i32) == 0
                || store.random().next_int(crystals + 2) == 0
            {
                strafe_player(state, entity, store, level, &player);
                return;
            }
        }
    }
    if path_done(state) {
        let current = closest_node(state, level, entity.position());
        let mut target = current;
        if store.random().next_int(8) == 0 {
            state.holding_clockwise = !state.holding_clockwise;
            target += 6;
        }
        let target = if state.holding_clockwise {
            target + 1
        } else {
            target.wrapping_sub(1)
        };
        let target = ring_target(target, state.fight.is_some());
        find_path(state, level, current, target, None);
        if let Some(path) = state.scratch.path.as_mut() {
            path.advance();
        }
    }
    navigate(state, store);
}

/// `DragonHoldingPatternPhase.strafePlayer` with `DragonStrafePlayerPhase.setTarget`.
pub(super) fn strafe_player(
    state: &mut EnderDragonState,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    player: &NearbyPlayer,
) {
    set_phase(state, entity, Some(store), DragonPhase::StrafePlayer);
    state.scratch.strafe_target = Some(player.uuid);
    let current = closest_node(state, level, entity.position());
    let target = closest_node(state, level, player.position);
    let final_x = player.position.x.floor() as i32;
    let final_z = player.position.z.floor() as i32;
    let xd = f64::from(final_x) - entity.position().x;
    let zd = f64::from(final_z) - entity.position().z;
    let height = (f64::from(0.4_f32) + (xd * xd + zd * zd).sqrt() / 80.0 - 1.0).min(10.0);
    let final_y = (player.position.y + height).floor() as i32;
    find_path(
        state,
        level,
        current,
        target,
        Some([final_x, final_y, final_z]),
    );
    if let Some(path) = state.scratch.path.as_mut() {
        path.advance();
        navigate(state, store);
    }
}

/// `DragonStrafePlayerPhase.doServerTick`.
fn strafe_tick(
    state: &mut EnderDragonState,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let Some(target) = state
        .scratch
        .strafe_target
        .and_then(|uuid| level.player(uuid))
    else {
        set_phase(state, entity, Some(store), DragonPhase::HoldingPattern);
        return;
    };
    let position = entity.position();
    if state
        .scratch
        .path
        .as_ref()
        .is_some_and(super::path::DragonPath::is_done)
    {
        let (xd, zd) = (
            target.position.x - position.x,
            target.position.z - position.z,
        );
        let height = (f64::from(0.4_f32) + (xd * xd + zd * zd).sqrt() / 80.0 - 1.0).min(10.0);
        state.scratch.target_location = Some(Vec3d {
            y: target.position.y + height,
            ..target.position
        });
    }
    if needs_new_target(state, entity) {
        strafe_find_new_target(state, entity, store, level);
    }
    if target.position.distance_to_sqr(position) >= 4096.0 {
        state.scratch.fireball_charge = (state.scratch.fireball_charge - 1).max(0);
        return;
    }
    let eye = Vec3d {
        y: position.y + f64::from(entity.eye_height()),
        ..position
    };
    let target_eye = Vec3d {
        y: target.position.y + target.eye_height,
        ..target.position
    };
    let sees = eye.distance_to_sqr(target_eye) <= 128.0 * 128.0
        && crate::entity::projectile_hit::collision_clip_misses(level, eye, target_eye);
    if !sees {
        state.scratch.fireball_charge = (state.scratch.fireball_charge - 1).max(0);
        return;
    }
    state.scratch.fireball_charge += 1;
    let aim = Vec3d {
        x: target.position.x - position.x,
        y: 0.0,
        z: target.position.z - position.z,
    }
    .normalize();
    let angle = facing_angle(entity, aim);
    if state.scratch.fireball_charge < 5 || !(0.0..10.0).contains(&angle) {
        return;
    }
    let view = view_vector(entity.x_rot(), entity.y_rot());
    let head = state.part(super::DragonPart::Head).position();
    let start = Vec3d {
        x: head.x - view.x,
        y: head.y + 0.5 + 0.5,
        z: head.z - view.z,
    };
    let direction = Vec3d {
        x: target.position.x - start.x,
        y: target.position.y + f64::from(target.dimensions[1]) * 0.5 - start.y,
        z: target.position.z - start.z,
    };
    if !entity.is_silent() {
        level.level_event(1017, entity.block_position(), 0);
    }
    let _ = spawn_dragon_fireball(
        store,
        start,
        ProjectileShot::new(direction.normalize().scale(
            crate::entity::hurting_projectile::HurtingProjectileState::INITIAL_ACCELERATION_POWER,
        )),
        entity.uuid(),
    );
    state.scratch.fireball_charge = 0;
    if let Some(path) = state.scratch.path.as_mut() {
        while !path.is_done() {
            path.advance();
        }
    }
    set_phase(state, entity, Some(store), DragonPhase::HoldingPattern);
}

/// `DragonStrafePlayerPhase.findNewTarget`.
fn strafe_find_new_target(
    state: &mut EnderDragonState,
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    if path_done(state) {
        let current = closest_node(state, level, entity.position());
        let mut target = current;
        if store.random().next_int(8) == 0 {
            state.strafe_clockwise = !state.strafe_clockwise;
            target += 6;
        }
        let target = if state.strafe_clockwise {
            target + 1
        } else {
            target.wrapping_sub(1)
        };
        let target = ring_target(target, crystals_alive(state));
        find_path(state, level, current, target, None);
        if let Some(path) = state.scratch.path.as_mut() {
            path.advance();
        }
    }
    navigate(state, store);
}

/// The angle between the dragon's heading and `aim`, plus the half degree the
/// strafe and scanning phases add.
fn facing_angle(entity: &Entity, aim: Vec3d) -> f32 {
    let radians = f64::from(entity.y_rot() * (std::f32::consts::PI / 180.0));
    let dir = Vec3d {
        x: f64::from(mth_sin(radians)),
        y: 0.0,
        z: f64::from(-mth_cos(radians)),
    }
    .normalize();
    let dot = (dir.x * aim.x + dir.y * aim.y + dir.z * aim.z) as f32;
    (f64::from(dot).clamp(-1.0, 1.0).acos() * 180.0 / std::f64::consts::PI) as f32 + 0.5
}

/// `Entity.calculateViewVector`.
fn view_vector(x_rot: f32, y_rot: f32) -> Vec3d {
    let pitch = f64::from(x_rot * (std::f32::consts::PI / 180.0));
    let yaw = f64::from(-y_rot * (std::f32::consts::PI / 180.0));
    let (h, i) = (mth_cos(yaw), mth_sin(yaw));
    let (j, k) = (mth_cos(pitch), mth_sin(pitch));
    Vec3d {
        x: f64::from(i * j),
        y: f64::from(-k),
        z: f64::from(h * j),
    }
}

/// `EnderDragon.getHeadLookVector` for the landing and takeoff phases.
fn head_look_vector(entity: &Entity, level: &LevelCtx<'_, '_>) -> Vec3d {
    let egg = egg_position(level);
    let dist = ((distance_to_center_sqr(egg, entity.position()).sqrt() as f32) / 4.0).max(1.0);
    let y_offset = 6.0 / dist;
    view_vector(-y_offset * 1.5 * 5.0, entity.y_rot())
}

/// `DragonLandingApproachPhase.findNewTarget`.
fn landing_approach_find_new_target(
    state: &mut EnderDragonState,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    if path_done(state) {
        let current = closest_node(state, level, entity.position());
        let egg = egg_position(level);
        let player = nearest_player(level, bottom_center(egg), |_| true);
        let target = match player {
            Some(player) => {
                let aim = Vec3d {
                    y: 0.0,
                    ..player.position
                }
                .normalize();
                closest_node(
                    state,
                    level,
                    Vec3d {
                        x: -aim.x * 40.0,
                        y: 105.0,
                        z: -aim.z * 40.0,
                    },
                )
            }
            None => closest_node(
                state,
                level,
                Vec3d {
                    x: 40.0,
                    y: f64::from(egg.y),
                    z: 0.0,
                },
            ),
        };
        find_path(state, level, current, target, Some([egg.x, egg.y, egg.z]));
        if let Some(path) = state.scratch.path.as_mut() {
            path.advance();
        }
    }
    navigate(state, store);
    if state
        .scratch
        .path
        .as_ref()
        .is_some_and(super::path::DragonPath::is_done)
    {
        set_phase(state, entity, Some(store), DragonPhase::Landing);
    }
}

/// `DragonTakeoffPhase.doServerTick`.
fn takeoff_tick(
    state: &mut EnderDragonState,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    if !state.scratch.first_tick && state.scratch.path.is_some() {
        let egg = egg_position(level);
        if distance_to_center_sqr(egg, entity.position()) >= 100.0 {
            set_phase(state, entity, Some(store), DragonPhase::HoldingPattern);
        }
        return;
    }
    state.scratch.first_tick = false;
    let current = closest_node(state, level, entity.position());
    let look = head_look_vector(entity, level);
    let target = closest_node(
        state,
        level,
        Vec3d {
            x: -look.x * 40.0,
            y: 105.0,
            z: -look.z * 40.0,
        },
    );
    let target = ring_target(target, crystals_alive(state));
    find_path(state, level, current, target, None);
    if let Some(path) = state.scratch.path.as_mut() {
        path.advance();
    }
    navigate(state, store);
}

/// `DragonSittingScanningPhase.doServerTick`.
fn scanning_tick(
    state: &mut EnderDragonState,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    state.scratch.ticks += 1;
    let position = entity.position();
    let near = nearest_player(level, position, |player| {
        player.position.distance_to_sqr(position) <= 20.0 * 20.0
            && (player.position.y - position.y).abs() <= 10.0
    });
    if let Some(target) = near {
        if state.scratch.ticks > 25 {
            set_phase(state, entity, Some(store), DragonPhase::SittingAttacking);
            return;
        }
        let aim = Vec3d {
            x: target.position.x - position.x,
            y: 0.0,
            z: target.position.z - position.z,
        }
        .normalize();
        let angle = facing_angle(entity, aim);
        if !(0.0..=10.0).contains(&angle) {
            let head = state.part(super::DragonPart::Head).position();
            let (xd, zd) = (target.position.x - head.x, target.position.z - head.z);
            let wanted = f64::from(living::wrap_degrees(
                (180.0 - xd.atan2(zd) * 180.0 / std::f64::consts::PI) as f32 - entity.y_rot(),
            ))
            .clamp(-100.0, 100.0);
            state.y_rot_a *= 0.8;
            let rot_speed = (xd * xd + zd * zd).sqrt() as f32 + 1.0;
            let dist = rot_speed.min(40.0);
            state.y_rot_a += wanted as f32 * (0.7 / dist / rot_speed);
            entity.set_rot(entity.y_rot() + state.y_rot_a, entity.x_rot());
        }
    } else if state.scratch.ticks >= 100 {
        let charge = nearest_player(level, position, |player| {
            player.position.distance_to_sqr(position) <= 150.0 * 150.0
        });
        set_phase(state, entity, Some(store), DragonPhase::Takeoff);
        if let Some(target) = charge {
            set_phase(state, entity, Some(store), DragonPhase::ChargingPlayer);
            state.scratch.target_location = Some(target.position);
        }
    }
}

/// `DragonSittingFlamingPhase.doServerTick`: the breath cloud at tick 10.
fn flaming_tick(
    state: &mut EnderDragonState,
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    state.scratch.ticks += 1;
    if state.scratch.ticks >= 200 {
        let next = if state.flame_count >= 4 {
            DragonPhase::Takeoff
        } else {
            DragonPhase::SittingScanning
        };
        set_phase(state, entity, Some(store), next);
        return;
    }
    if state.scratch.ticks != 10 {
        return;
    }
    let head = state.part(super::DragonPart::Head).position();
    let position = entity.position();
    let look = Vec3d {
        x: head.x - position.x,
        y: 0.0,
        z: head.z - position.z,
    }
    .normalize();
    let x = head.x + look.x * 5.0 / 2.0;
    let z = head.z + look.z * 5.0 / 2.0;
    let initial_y = head.y + 0.5;
    let mut y = initial_y;
    let block = |y: f64| BlockPos {
        x: x.floor() as i32,
        y: y.floor() as i32,
        z: z.floor() as i32,
    };
    while block_state_is_air(level.block_state(block(y))) {
        y -= 1.0;
        if y < 0.0 {
            y = initial_y;
            break;
        }
    }
    let y = y.floor() + 1.0;
    let cloud = AreaEffectCloudData::new()
        .with_radius(5.0)
        .with_duration(200)
        .with_potion_duration_scale(0.25)
        .with_owner(Some(entity.uuid()))
        .with_custom_particle(Some(dragon_breath_particle()))
        .with_potion_contents(Some(instant_damage()));
    state.scratch.flame_cloud = spawn_area_effect_cloud(store, Vec3d { x, y, z }, cloud);
}

/// `new MobEffectInstance(MobEffects.INSTANT_DAMAGE)` as potion contents.
fn instant_damage() -> bbb_protocol::packets::PotionContentsSummary {
    bbb_protocol::packets::PotionContentsSummary {
        custom_effects: vec![bbb_protocol::packets::MobEffectInstanceSummary {
            effect_id: bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_INSTANT_DAMAGE_ID,
            amplifier: 0,
            duration: 1,
            ambient: false,
            show_particles: true,
            show_icon: true,
            hidden_effect: None,
        }],
        ..Default::default()
    }
}
