//! Shared `Projectile` and `AbstractHurtingProjectile` state and tick order.

use crate::world::simulation_level::LevelCtx;
pub mod dragon_fireball;
pub mod fireball;
pub mod small_fireball;
pub mod wind_charge;
pub mod wither_skull;

use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};
use uuid::Uuid;

use crate::entity::level::{builtin_registry_tag_membership, fluid_state_tag_membership};
use crate::entity::projectile_action::PendingProjectileAction;
use crate::entity::projectile_hit::{
    hit_result_on_move_vector, mth_atan2, HitResult, ProjectileCandidate, ProjectileDeflection,
    ProjectileTarget,
};
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{base_tick, Entity, EntityData};
use crate::level_event::VanillaGameEvent;
use bbb_server_types::fluid::block_state_fluid_state;
use bbb_server_types::geometry::{mth_cos, mth_sin, Vec3Ops};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProjectileState {
    pub(super) owner: Option<Uuid>,
    pub(super) left_owner: bool,
    pub(super) has_been_shot: bool,
}

impl ProjectileState {
    pub const fn new(owner: Option<Uuid>) -> Self {
        Self {
            owner,
            left_owner: false,
            has_been_shot: false,
        }
    }

    pub const fn from_saved(owner: Option<Uuid>, left_owner: bool, has_been_shot: bool) -> Self {
        Self {
            owner,
            left_owner,
            has_been_shot,
        }
    }

    pub const fn owner(&self) -> Option<Uuid> {
        self.owner
    }

    pub(crate) fn set_owner(&mut self, owner: Option<Uuid>) {
        self.owner = owner;
    }

    pub const fn left_owner(&self) -> bool {
        self.left_owner
    }

    pub const fn has_been_shot(&self) -> bool {
        self.has_been_shot
    }

    pub(crate) fn mark_shot(&mut self) {
        self.has_been_shot = true;
    }

    pub(crate) fn set_left_owner(&mut self, left_owner: bool) {
        self.left_owner = left_owner;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HurtingProjectileState {
    projectile: ProjectileState,
    acceleration_power: f64,
}

impl HurtingProjectileState {
    pub const INITIAL_ACCELERATION_POWER: f64 = 0.1;

    pub const fn new(owner: Option<Uuid>, acceleration_power: f64) -> Self {
        Self {
            projectile: ProjectileState::new(owner),
            acceleration_power,
        }
    }

    pub const fn from_saved(projectile: ProjectileState, acceleration_power: f64) -> Self {
        Self {
            projectile,
            acceleration_power,
        }
    }

    pub const fn projectile(&self) -> &ProjectileState {
        &self.projectile
    }

    pub const fn owner(&self) -> Option<Uuid> {
        self.projectile.owner()
    }

    pub const fn acceleration_power(&self) -> f64 {
        self.acceleration_power
    }

    pub fn set_acceleration_power(&mut self, acceleration_power: f64) {
        self.acceleration_power = acceleration_power;
    }

    fn deflect_by_attack(&mut self, owner: Option<Uuid>) {
        self.projectile.owner = owner;
        self.acceleration_power = Self::INITIAL_ACCELERATION_POWER;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HurtingProjectileSpec {
    pub dry_inertia: f32,
    pub liquid_inertia: f32,
    pub burns: bool,
}

pub const FIREBALL_SPEC: HurtingProjectileSpec = HurtingProjectileSpec {
    dry_inertia: 0.95,
    liquid_inertia: 0.8,
    burns: true,
};

pub const WIND_CHARGE_SPEC: HurtingProjectileSpec = HurtingProjectileSpec {
    dry_inertia: 1.0,
    liquid_inertia: 1.0,
    burns: false,
};

pub fn deflect_by_player_attack(entity: &mut Entity, owner: Uuid, y_rot: f32, x_rot: f32) -> bool {
    deflect_by_attack(entity, Some(owner), Some(view_vector(y_rot, x_rot)))
}

fn view_vector(y_rot: f32, x_rot: f32) -> Vec3d {
    let to_radians = std::f32::consts::PI / 180.0;
    let real_x_rot = x_rot * to_radians;
    let real_y_rot = -y_rot * to_radians;
    let y_cos = mth_cos(f64::from(real_y_rot));
    let y_sin = mth_sin(f64::from(real_y_rot));
    let x_cos = mth_cos(f64::from(real_x_rot));
    let x_sin = mth_sin(f64::from(real_x_rot));
    Vec3d {
        x: f64::from(y_sin * x_cos),
        y: f64::from(-x_sin),
        z: f64::from(y_cos * x_cos),
    }
}

pub fn can_deflect_by_player_attack(entity: &Entity) -> bool {
    can_deflect_by_player_attack_if_member(
        entity,
        builtin_registry_tag_membership(
            "minecraft:entity_type",
            "minecraft:redirectable_projectile",
            entity.data().entity_resource_id(),
        ),
    )
}

#[cfg(test)]
pub(crate) fn can_deflect_by_player_attack_in_level(
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
) -> bool {
    can_deflect_by_player_attack_with_registry_tag_view(entity, &level.capture_registry_tags())
}

pub fn can_deflect_by_player_attack_with_registry_tag_view(
    entity: &Entity,
    registry_tags: &crate::entity::EntityRegistryTagView,
) -> bool {
    can_deflect_by_player_attack_if_member(
        entity,
        registry_tags.contains(
            "minecraft:entity_type",
            "minecraft:redirectable_projectile",
            entity.data().entity_resource_id(),
        ),
    )
}

fn can_deflect_by_player_attack_if_member(entity: &Entity, membership: bool) -> bool {
    membership
        && entity.is_projectile_class()
        && !matches!(entity.data(), crate::entity::EntityData::WindCharge(data) if data.no_deflect_ticks() > 0)
}

fn deflect_by_attack(entity: &mut Entity, owner: Option<Uuid>, direction: Option<Vec3d>) -> bool {
    match &mut entity.data {
        crate::entity::EntityData::Arrow(data) => data.projectile.set_owner(owner),
        crate::entity::EntityData::SpectralArrow(data) => data.projectile.set_owner(owner),
        crate::entity::EntityData::Trident(data) => data.projectile.set_owner(owner),
        crate::entity::EntityData::FishingBobber(data) => data.set_owner(owner),
        crate::entity::EntityData::Snowball(data) => data.projectile.set_owner(owner),
        crate::entity::EntityData::Egg(data) => data.projectile.set_owner(owner),
        crate::entity::EntityData::EnderPearl(data) => data.projectile.set_owner(owner),
        crate::entity::EntityData::ExperienceBottle(data) => data.projectile.set_owner(owner),
        crate::entity::EntityData::FireworkRocket(data) => data.projectile.set_owner(owner),
        crate::entity::EntityData::LlamaSpit(data) => data.set_owner(owner),
        crate::entity::EntityData::LargeFireball(data) => {
            data.projectile_mut().deflect_by_attack(owner)
        }
        crate::entity::EntityData::SmallFireball(data) => {
            data.projectile_mut().deflect_by_attack(owner)
        }
        crate::entity::EntityData::DragonFireball(data) => {
            data.projectile_mut().deflect_by_attack(owner)
        }
        crate::entity::EntityData::WitherSkull(data) => {
            data.projectile_mut().deflect_by_attack(owner)
        }
        crate::entity::EntityData::WindCharge(data) if data.no_deflect_ticks() == 0 => {
            data.projectile_mut().deflect_by_attack(owner);
        }
        crate::entity::EntityData::BreezeWindCharge(data) => {
            data.projectile_mut().deflect_by_attack(owner);
        }
        crate::entity::EntityData::SplashPotion(data)
        | crate::entity::EntityData::LingeringPotion(data) => {
            data.projectile.set_owner(owner);
        }
        crate::entity::EntityData::ShulkerBullet(data) => data.projectile_mut().set_owner(owner),
        _ => return false,
    }
    if let Some(direction) = direction {
        entity.set_delta_movement(direction);
        entity.set_needs_sync(true);
    }
    true
}

pub fn apply_projectile_deflection(
    entity: &mut Entity,
    new_owner: Option<Uuid>,
    direction: Option<Vec3d>,
) -> bool {
    deflect_by_attack(entity, new_owner, direction)
}

pub(crate) fn apply_target_deflection(
    entity: &mut Entity,
    store: &mut EntityStore,
    target: ProjectileTarget,
    deflection: ProjectileDeflection,
) -> bool {
    if deflection == ProjectileDeflection::None {
        return false;
    }
    if entity.last_deflected_by() == Some(target) {
        return true;
    }
    if deflection == ProjectileDeflection::BreezeReverse {
        let position = match target {
            ProjectileTarget::Stored { entity_id } => store
                .get(entity_id)
                .map_or(entity.position(), Entity::position),
            ProjectileTarget::Player { .. } => entity.position(),
        };
        store.play_entity_sound(PendingEntitySound {
            position,
            resource_id: "minecraft:entity.breeze.deflect".to_string(),
            source: SoundSource::Hostile,
            volume: 1.0,
            pitch: 1.0,
        });
    }
    let rotation = 170.0 + store.random().next_float() * 20.0;
    entity.set_delta_movement(entity.delta_movement().scale(-0.5));
    entity.set_rot(entity.y_rot() + rotation, entity.x_rot());
    entity.set_needs_sync(true);
    entity.set_last_deflected_by(target);
    true
}

pub fn set_owner_from_explosion(entity: &mut Entity, owner: Option<Uuid>) -> bool {
    let state = match &mut entity.data {
        crate::entity::EntityData::LargeFireball(data) => data.projectile_mut(),
        crate::entity::EntityData::WindCharge(data)
        | crate::entity::EntityData::BreezeWindCharge(data) => data.projectile_mut(),
        _ => return false,
    };
    state.projectile.owner = owner;
    true
}

pub(crate) fn projectile_deflection_action(
    target: ProjectileTarget,
    new_owner: Option<Uuid>,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Result<Option<PendingProjectileAction>, ()> {
    let ProjectileTarget::Stored { entity_id } = target else {
        return Ok(None);
    };
    let Some(target) = store.get(entity_id) else {
        return Ok(None);
    };
    let membership = level.registry_tag_membership(
        "minecraft:entity_type",
        "minecraft:redirectable_projectile",
        target.data().entity_resource_id(),
    );
    if !can_deflect_by_player_attack_if_member(target, membership) {
        return Ok(None);
    }
    let direction = new_owner.and_then(|owner| {
        level
            .player(owner)
            .map(|player| view_vector(player.y_rot, player.x_rot))
            .or_else(|| {
                store
                    .entity_id_for_uuid(owner)
                    .and_then(|owner_id| store.get(owner_id))
                    .map(|owner| view_vector(owner.y_rot(), owner.x_rot()))
            })
    });
    Ok(Some(PendingProjectileAction::Deflect {
        target_entity_id: entity_id,
        new_owner,
        direction,
    }))
}

fn state_mut(entity: &mut Entity) -> &mut HurtingProjectileState {
    match &mut entity.data {
        EntityData::LargeFireball(data) => data.projectile_mut(),
        EntityData::SmallFireball(data) => data.projectile_mut(),
        EntityData::DragonFireball(data) => data.projectile_mut(),
        EntityData::WitherSkull(data) => data.projectile_mut(),
        EntityData::WindCharge(data) | EntityData::BreezeWindCharge(data) => data.projectile_mut(),
        _ => unreachable!("hurting projectile ticks require hurting projectile state"),
    }
}

pub(crate) fn tick_entity(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let mut state = *state_mut(entity);
    let owner = state.owner();
    let wind = matches!(
        entity.data(),
        EntityData::WindCharge(_) | EntityData::BreezeWindCharge(_)
    );
    let hit = if wind && entity.position().y.floor() as i32 > level.max_y() + 30 {
        wind_charge::on_above_world(
            entity,
            store,
            level,
            owner,
            matches!(entity.data(), EntityData::BreezeWindCharge(_)),
        );
        Ok(None)
    } else {
        let spec = match entity.data() {
            EntityData::LargeFireball(_) | EntityData::SmallFireball(_) => FIREBALL_SPEC,
            EntityData::DragonFireball(_) => HurtingProjectileSpec {
                burns: false,
                ..FIREBALL_SPEC
            },
            EntityData::WitherSkull(data) => HurtingProjectileSpec {
                dry_inertia: if data.is_dangerous() { 0.73 } else { 0.95 },
                burns: false,
                ..FIREBALL_SPEC
            },
            EntityData::WindCharge(_) | EntityData::BreezeWindCharge(_) => WIND_CHARGE_SPEC,
            _ => unreachable!("hurting projectile ticks require hurting projectile state"),
        };
        tick(
            entity,
            store,
            level,
            &mut state,
            spec,
            |candidate, level| {
                if wind {
                    wind_charge::can_hit(candidate, level)
                } else {
                    candidate.is_alive_and_hurting_projectile_target_in_level(level)
                }
            },
        )
    };
    *state_mut(entity) = state;
    let Ok(hit) = hit else {
        return false;
    };
    if let EntityData::WindCharge(data) | EntityData::BreezeWindCharge(data) = &mut entity.data {
        data.tick_no_deflect();
    }
    let Some(hit) = hit else {
        return true;
    };
    match entity.data() {
        EntityData::LargeFireball(data) => {
            let explosion_power = data.explosion_power();
            fireball::on_hit(entity, store, level, owner, hit, explosion_power)
        }
        EntityData::SmallFireball(_) => small_fireball::on_hit(entity, store, level, owner, hit),
        EntityData::DragonFireball(_) => dragon_fireball::on_hit(entity, store, level, owner, hit),
        EntityData::WitherSkull(data) => {
            let dangerous = data.is_dangerous();
            wither_skull::on_hit(entity, store, level, owner, hit, dangerous)
        }
        EntityData::WindCharge(_) => wind_charge::on_hit(entity, store, level, owner, hit, false),
        EntityData::BreezeWindCharge(_) => {
            wind_charge::on_hit(entity, store, level, owner, hit, true)
        }
        _ => unreachable!("hurting projectile hits require hurting projectile state"),
    }
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    state: &mut HurtingProjectileState,
    spec: HurtingProjectileSpec,
    matching: impl Fn(ProjectileCandidate<'_>, &LevelCtx<'_, '_>) -> bool,
) -> Result<Option<HitResult>, ()> {
    let movement = entity.delta_movement();
    let inertia = if entity.was_touching_water() {
        spec.liquid_inertia
    } else {
        spec.dry_inertia
    };
    entity.set_delta_movement(
        movement
            .plus(movement.normalize().scale(state.acceleration_power))
            .scale(f64::from(inertia)),
    );

    let owner_removed = state.owner().is_some_and(|owner| {
        store
            .entity_id_for_uuid(owner)
            .and_then(|id| store.get(id))
            .is_some_and(|owner| !owner.is_alive())
    });
    if owner_removed || !level.has_ticking_column_at(entity.block_position()) {
        entity.discard();
        return Ok(None);
    }

    let owner_vehicle = crate::entity::throwable_projectile::owner_vehicle(
        state.owner(),
        state.projectile.left_owner(),
        store,
        level,
    );
    let hit = hit_result_on_move_vector(entity, store, &*level, |candidate, store, level| {
        matching(candidate, level)
            && !crate::entity::throwable_projectile::rides_with_owner(
                candidate,
                owner_vehicle,
                store,
            )
    })
    .ok_or(())?;
    let position = match hit {
        HitResult::Miss => entity.position().plus(entity.delta_movement()),
        HitResult::Block { location, .. } | HitResult::Entity { location, .. } => location,
    };
    let water_membership = fluid_state_tag_membership(
        level,
        block_state_fluid_state(level.block_state(block_pos(position))),
        "minecraft:water",
    );
    rotate_towards_movement(entity);
    entity.set_pos(position);
    store.refile_detached(entity);
    if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store)) {
        return Err(());
    }

    if !state.projectile.has_been_shot {
        let owner_id = state
            .owner()
            .and_then(|owner| level.player(owner).map(|player| player.entity_id))
            .or_else(|| {
                state
                    .owner()
                    .and_then(|owner| store.entity_id_for_uuid(owner))
            });
        level.game_event_at(
            VanillaGameEvent::PROJECTILE_SHOOT,
            entity.position(),
            owner_id,
        );
        state.projectile.mark_shot();
    }
    if !state.projectile.left_owner {
        state.projectile.left_owner =
            outside_owner_collision_range(entity, store, level, &state.projectile);
    }
    if !base_tick(entity, store, level) {
        return Err(());
    }
    let redirect_action = match hit {
        HitResult::Entity {
            target,
            deflection: ProjectileDeflection::None,
            ..
        } => projectile_deflection_action(target, state.owner(), store, &*level)?,
        _ => None,
    };
    if spec.burns {
        entity.ignite_for_seconds(1.0);
    }
    entity.set_was_touching_water(water_membership);
    if let HitResult::Entity {
        target, deflection, ..
    } = hit
    {
        let first_target_deflection =
            deflection != ProjectileDeflection::None && entity.last_deflected_by() != Some(target);
        if apply_target_deflection(entity, store, target, deflection) {
            if first_target_deflection {
                state.acceleration_power *= 0.5;
            }
            return Ok(None);
        }
    }
    if let Some(action) = redirect_action {
        store.queue_command(crate::entity::EntityCommand::Projectile(action));
    }
    if let HitResult::Block {
        location,
        pos,
        direction,
        ..
    } = hit
    {
        if entity.is_alive() {
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
        }
    }
    Ok((hit != HitResult::Miss && entity.is_alive()).then_some(hit))
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
    (current - delta) + 0.2 * delta
}

pub(crate) fn outside_owner_collision_range(
    entity: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
    state: &ProjectileState,
) -> bool {
    crate::entity::throwable_projectile::outside_owner_collision_range(
        entity,
        state.owner(),
        store,
        level,
    )
}

pub fn block_pos(location: Vec3d) -> BlockPos {
    BlockPos {
        x: location.x.floor() as i32,
        y: location.y.floor() as i32,
        z: location.z.floor() as i32,
    }
}
