//! `FireworkRocketEntity`: a `Projectile` that accelerates upward and explodes
//! when its lifetime expires, it hits an entity, or it hits a block while
//! carrying explosions. A rocket attached to a gliding player rides along
//! with it; the boost itself is the client's to apply to its own player.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_synched_data::{
    VANILLA_FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID, VANILLA_FIREWORK_ROCKET_ITEM_DATA_ID,
    VANILLA_FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::hurting_projectile::{self, ProjectileState};
use crate::entity::living::SynchedValue;
use crate::entity::projectile_hit::{
    collision_clip_misses, hit_result_on_move_vector, HitResult, ProjectileCandidate,
};
use crate::entity::store::EntityStore;
use crate::entity::throwable_projectile;
use crate::entity::{move_entity, Entity, EntityData, MoverType};
use crate::level_event::{PendingHurtEntity, VanillaGameEvent};
use bbb_server_types::geometry::{Aabb, Axis, Vec3Ops};

pub fn default_lifetime_for_item(item: &RuntimeItemStack) -> i32 {
    let flight_count = 1 + fireworks_flight_duration(item);
    10 * flight_count
}

/// The `FireworkRocketEntity` and `Projectile` fields this slice carries.
#[derive(Debug, Clone, PartialEq)]
pub struct FireworkRocketData {
    item: RuntimeItemStack,
    life: i32,
    lifetime: i32,
    shot_at_angle: bool,
    attached_to_target: Option<i32>,
    pub(crate) projectile: ProjectileState,
}

impl FireworkRocketData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.attached_to_target = previous.attached_to_target;
    }

    pub fn new(item: RuntimeItemStack) -> Self {
        let lifetime = default_lifetime_for_item(&item);
        Self::from_saved(item, 0, lifetime, false, ProjectileState::default())
    }

    pub fn from_saved(
        item: RuntimeItemStack,
        life: i32,
        lifetime: i32,
        shot_at_angle: bool,
        projectile: ProjectileState,
    ) -> Self {
        Self {
            item,
            life,
            lifetime,
            shot_at_angle,
            attached_to_target: None,
            projectile,
        }
    }

    pub const fn item(&self) -> &RuntimeItemStack {
        &self.item
    }

    pub const fn life(&self) -> i32 {
        self.life
    }

    pub const fn lifetime(&self) -> i32 {
        self.lifetime
    }

    pub const fn shot_at_angle(&self) -> bool {
        self.shot_at_angle
    }

    pub const fn attached_to_target(&self) -> Option<i32> {
        self.attached_to_target
    }

    pub const fn projectile(&self) -> &ProjectileState {
        &self.projectile
    }

    pub fn has_explosions(&self) -> bool {
        explosion_count(&self.item) > 0
    }
}

fn spawn_firework_rocket_with(
    store: &mut EntityStore,
    position: Vec3d,
    delta_movement: Vec3d,
    data: FireworkRocketData,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let y_rot = (delta_movement.x.atan2(delta_movement.z) * 180.0 / std::f64::consts::PI) as f32;
    let horizontal =
        (delta_movement.x * delta_movement.x + delta_movement.z * delta_movement.z).sqrt();
    let x_rot = (delta_movement.y.atan2(horizontal) * 180.0 / std::f64::consts::PI) as f32;
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        position,
        EntityData::FireworkRocket(data),
        |entity| {
            entity.set_delta_movement(delta_movement);
            entity.set_rot(y_rot, x_rot);
            entity.set_needs_sync(true);
        },
    )
}

pub fn spawn_firework_rocket(
    store: &mut EntityStore,
    position: Vec3d,
    delta_movement: Vec3d,
    item: RuntimeItemStack,
) -> Option<i32> {
    spawn_firework_rocket_with(
        store,
        position,
        delta_movement,
        FireworkRocketData::new(item),
    )
}

/// `new FireworkRocketEntity(level, owner, x, y, z, stack)`
/// (`FireworkRocketEntity.java:55-73`), attached to `stuck_to` when given
/// (`:76-80`).
pub(crate) fn spawn_launched_firework(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    owner: uuid::Uuid,
    stuck_to: Option<i32>,
) -> Option<i32> {
    let random = store.random();
    let delta_movement = Vec3d {
        x: random.triangle(0.0, 0.002297),
        y: 0.05,
        z: random.triangle(0.0, 0.002297),
    };
    let extra_life = random.next_int(6) + random.next_int(7);
    let mut data = FireworkRocketData::new(item);
    data.lifetime += extra_life;
    data.projectile.owner = Some(owner);
    data.attached_to_target = stuck_to;
    spawn_firework_rocket_with(store, position, delta_movement, data)
}

/// `new FireworkRocketEntity(level, stack, owner, x, y, z, true)`
/// (`FireworkRocketEntity.java:86-91`) carrying a crossbow's `shot`.
pub(crate) fn spawn_shot_firework(
    store: &mut EntityStore,
    position: Vec3d,
    item: RuntimeItemStack,
    owner: Option<uuid::Uuid>,
    shot: crate::entity::projectile_shoot::ProjectileShot,
    initialize: impl FnOnce(&mut Entity),
) -> Option<i32> {
    let random = store.random();
    let _ = (
        random.triangle(0.0, 0.002297),
        random.triangle(0.0, 0.002297),
    );
    let extra_life = random.next_int(6) + random.next_int(7);
    let mut data = FireworkRocketData::new(item);
    data.lifetime += extra_life;
    data.projectile.owner = owner;
    data.shot_at_angle = true;
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        position,
        EntityData::FireworkRocket(data),
        |entity| {
            shot.install(entity);
            initialize(entity);
        },
    )
}

/// The position and movement of the entity a rocket is attached to
/// (`FireworkRocketEntity.java:116-142`), with no hand offset.
fn attached_body(
    attached: i32,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<(Vec3d, Vec3d)> {
    if let Some(entity) = store.get(attached) {
        return entity
            .living()
            .is_some()
            .then(|| (entity.position(), entity.delta_movement()));
    }
    level
        .players()
        .iter()
        .find(|player| player.entity_id == attached)
        .map(|player| (player.position, player.delta_movement))
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let owner = state_mut(entity).projectile.owner;
    // `FireworkRocketEntity.tick` starts with `Projectile.tick` (`:111-113`,
    // `Projectile.java:99-103`).
    if !state_mut(entity).projectile.has_been_shot {
        let owner = throwable_projectile::owner_entity_id(owner, store, level);
        level.game_event_at(VanillaGameEvent::PROJECTILE_SHOOT, entity.position(), owner);
        state_mut(entity).projectile.has_been_shot = true;
    }
    if !state_mut(entity).projectile.left_owner {
        let left_owner =
            throwable_projectile::outside_owner_collision_range(entity, owner, store, level);
        state_mut(entity).projectile.left_owner = left_owner;
    }
    if !crate::entity::damage::base_tick_from_environment(entity, store, level) {
        return false;
    }

    let owner_vehicle = throwable_projectile::owner_vehicle(
        owner,
        state_mut(entity).projectile.left_owner,
        store,
        level,
    );
    let can_hit =
        |candidate: ProjectileCandidate<'_>, store: &EntityStore, level: &LevelCtx<'_, '_>| {
            candidate.is_alive_and_pickable_in_level(level)
                && !throwable_projectile::rides_with_owner(candidate, owner_vehicle, store)
        };
    // A boosting rocket follows its body and still hit-tests along its
    // movement (`FireworkRocketEntity.java:116-142`).
    let (hit, movement) = if let Some(attached) = state_mut(entity).attached_to_target {
        if let Some((position, movement)) = attached_body(attached, store, level) {
            entity.set_pos(position);
            entity.set_delta_movement(movement);
        }
        let Some(hit) = hit_result_on_move_vector(entity, store, &*level, can_hit) else {
            return false;
        };
        (hit, entity.delta_movement())
    } else {
        if !state_mut(entity).shot_at_angle {
            let horizontal_accel = if entity.horizontal_collision() {
                1.0
            } else {
                1.15
            };
            let movement = entity.delta_movement();
            entity.set_delta_movement(Vec3d {
                x: movement.x * horizontal_accel,
                y: movement.y + 0.04,
                z: movement.z * horizontal_accel,
            });
        }

        let movement = entity.delta_movement();
        let Some(hit) = hit_result_on_move_vector(entity, store, &*level, can_hit) else {
            return false;
        };
        move_entity(entity, store, level, MoverType::Self_, movement);
        if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store)) {
            return false;
        }
        entity.set_delta_movement(movement);
        (hit, movement)
    };

    if entity.is_alive() {
        // `Projectile.onHit` (`Projectile.java:296-306`) lands after the
        // subtype's hit, even once the rocket exploded.
        let mut land = None;
        let explodes = match hit {
            // `FireworkRocketEntity.onHitEntity` (`:198-204`).
            HitResult::Entity {
                target,
                deflection,
                location,
            } => {
                let hits =
                    !hurting_projectile::apply_target_deflection(entity, store, target, deflection);
                if hits {
                    // `Projectile.onHit` (`Projectile.java:295-297`).
                    if let Ok(Some(action)) = hurting_projectile::projectile_deflection_action(
                        target, owner, store, &*level,
                    ) {
                        store.queue_command(crate::entity::EntityCommand::Projectile(action));
                    }
                    land = Some((location, None));
                }
                hits
            }
            // `FireworkRocketEntity.onHitBlock` (`:206-215`).
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
                land = Some((location, Some(pos)));
                state_mut(entity).has_explosions()
            }
            HitResult::Miss => false,
        };
        if explodes {
            explode(entity, store, level);
        }
        match land {
            Some((_, Some(pos))) => {
                let state = level.block_state(pos);
                level.game_event_with_state(
                    VanillaGameEvent::PROJECTILE_LAND,
                    pos,
                    Some(entity.id()),
                    Some(state),
                );
            }
            Some((location, None)) => level.game_event_at(
                VanillaGameEvent::PROJECTILE_LAND,
                location,
                Some(entity.id()),
            ),
            None => {}
        }
        if explodes {
            return true;
        }
    }

    // `Projectile.updateRotation` (`Projectile.java:326-343`).
    let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
    let target_y_rot = (movement.x.atan2(movement.z) * 180.0 / std::f64::consts::PI) as f32;
    let target_x_rot = (movement.y.atan2(horizontal) * 180.0 / std::f64::consts::PI) as f32;
    entity.set_rot(
        lerp_rotation(entity.y_rot(), target_y_rot),
        lerp_rotation(entity.x_rot(), target_x_rot),
    );

    // `FireworkRocketEntity.tick` (`:168-170`).
    if state_mut(entity).life == 0 && !entity.is_silent() {
        store.play_entity_sound(crate::entity::store::PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.firework_rocket.launch".to_owned(),
            source: bbb_protocol::packets::SoundSource::Ambient,
            volume: 3.0,
            pitch: 1.0,
        });
    }
    let data = state_mut(entity);
    data.life += 1;
    if data.life > data.lifetime && entity.is_alive() {
        explode(entity, store, level);
    }
    true
}

fn state_mut(entity: &mut Entity) -> &mut FireworkRocketData {
    match &mut entity.data {
        EntityData::FireworkRocket(data) => data,
        _ => unreachable!("firework rocket ticks require firework rocket state"),
    }
}

/// `Projectile.lerpRotation` (`Projectile.java:333-343`).
fn lerp_rotation(previous: f32, current: f32) -> f32 {
    if !previous.is_finite() || !current.is_finite() {
        return current;
    }
    let delta = (current - previous + 180.0).rem_euclid(360.0) - 180.0;
    let wrapped_previous = current - delta;
    wrapped_previous + 0.2_f32 * (current - wrapped_previous)
}

/// `FireworkRocketEntity.explode` (`:191-196`). The discard is queued after
/// the hurts so the rocket stays their direct entity.
fn explode(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    let data = state_mut(entity);
    let (owner, attached, explosions) = (
        data.projectile.owner,
        data.attached_to_target,
        explosion_count(&data.item),
    );
    throwable_projectile::broadcast_entity_event(entity, store, 17);
    let owner_id = throwable_projectile::owner_entity_id(owner, store, level);
    level.game_event_at(VanillaGameEvent::EXPLODE, entity.position(), owner_id);
    let hurt = deal_explosion_damage(entity, store, level, owner, attached, explosions);
    throwable_projectile::discard_after_queued_hit(entity, store, hurt);
}

/// `FireworkRocketEntity.dealExplosionDamage` (`:221-256`).
fn deal_explosion_damage(
    rocket: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    owner: Option<uuid::Uuid>,
    attached: Option<i32>,
    explosions: usize,
) -> bool {
    if explosions == 0 {
        return false;
    }
    let damage = 5.0 + explosions as f32 * 2.0;
    let causing = throwable_projectile::pending_owner(owner, store, level);
    let mut queued = false;
    let mut hurt = |store: &mut EntityStore, target: PendingHurtEntity, amount: f32| {
        queued = true;
        throwable_projectile::queue_hurt(
            rocket,
            store,
            level,
            throwable_projectile::ThrowableHurt {
                target,
                damage_type: "minecraft:fireworks",
                amount,
                causing: causing.clone(),
                direct_resource_id: "minecraft:firework_rocket",
            },
        );
    };
    if let Some(attached) = attached {
        if let Some(target) = pending_entity(attached, store, level) {
            hurt(store, target, damage);
        }
    }
    let origin = rocket.position();
    let area = rocket.bounding_box().inflate(5.0, 5.0, 5.0);
    let mut targets: Vec<(i32, PendingHurtEntity, Vec3d, f64)> = store
        .ids_in_box_sections(area)
        .into_iter()
        .filter_map(|id| store.get(id))
        .filter(|target| {
            target
                .living()
                .is_some_and(|living| !living.is_dead_or_dying())
                && target.bounding_box().intersects(area)
        })
        .map(|target| {
            (
                target.id(),
                PendingHurtEntity::Stored {
                    entity_id: target.id(),
                },
                target.position(),
                target.bounding_box().max(Axis::Y) - target.bounding_box().min(Axis::Y),
            )
        })
        .collect();
    targets.extend(
        level
            .players()
            .iter()
            .filter(|player| {
                !player.spectator
                    && Aabb::of_entity(player.position, player.dimensions[0], player.dimensions[1])
                        .intersects(area)
            })
            .map(|player| {
                (
                    player.entity_id,
                    PendingHurtEntity::Player { uuid: player.uuid },
                    player.position,
                    f64::from(player.dimensions[1]),
                )
            }),
    );
    for (entity_id, target, position, height) in targets {
        if attached == Some(entity_id) {
            continue;
        }
        let distance_sqr = origin.distance_to_sqr(position);
        if distance_sqr > 25.0 {
            continue;
        }
        let can_see = (0..2).any(|step| {
            let to = Vec3d {
                y: position.y + height * 0.5 * f64::from(step),
                ..position
            };
            collision_clip_misses(level, origin, to)
        });
        if can_see {
            let amount = damage * ((5.0 - distance_sqr.sqrt()) / 5.0).sqrt() as f32;
            hurt(store, target, amount);
        }
    }
    queued
}

/// The hurt target for a synched entity id: a stored entity or a player.
fn pending_entity(
    entity_id: i32,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<PendingHurtEntity> {
    if store.get(entity_id).is_some() {
        return Some(PendingHurtEntity::Stored { entity_id });
    }
    level
        .players()
        .iter()
        .find(|player| player.entity_id == entity_id)
        .map(|player| PendingHurtEntity::Player { uuid: player.uuid })
}

pub fn synched_values(entity: &Entity, data: &FireworkRocketData) -> Vec<SynchedValue> {
    use bbb_protocol::entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
    };
    use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};

    let mut rows = crate::entity::living::entity_synched_values(entity);
    let stack = data.item().summary().clone();
    let is_default = stack.item_id.is_none();
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_FIREWORK_ROCKET_ITEM_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(stack),
        },
        is_default,
    });
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_FIREWORK_ROCKET_ATTACHED_TO_TARGET_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
            value: EntityDataValueKind::OptionalUnsignedInt(data.attached_to_target()),
        },
        is_default: data.attached_to_target().is_none(),
    });
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_FIREWORK_ROCKET_SHOT_AT_ANGLE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(data.shot_at_angle()),
        },
        is_default: !data.shot_at_angle(),
    });
    rows
}

fn fireworks_component(
    item: &RuntimeItemStack,
) -> Option<&bbb_protocol::packets::DataComponentPatchSummary> {
    bbb_server_types::item_stack::static_item_component(
        item.summary(),
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
    )
    .ok()
    .flatten()
}

fn fireworks_flight_duration(item: &RuntimeItemStack) -> i32 {
    fireworks_component(item)
        .and_then(|component| component.fireworks_flight_duration)
        .unwrap_or(0)
}

fn explosion_count(item: &RuntimeItemStack) -> usize {
    fireworks_component(item)
        .map(|component| component.fireworks_explosions.len())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::item::plain_stack;
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    #[test]
    fn default_lifetime_follows_flight_duration() {
        let plain = plain_stack("minecraft:firework_rocket", 1).expect("firework rocket stack");
        assert_eq!(default_lifetime_for_item(&plain), 20);

        let with_flight = bbb_server_types::item_stack::RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::string("minecraft:firework_rocket"),
                },
                NbtSummaryEntry {
                    name: "count".into(),
                    value: NbtSummaryValue::Int(1),
                },
                NbtSummaryEntry {
                    name: "components".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "minecraft:fireworks".into(),
                        value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                            name: "flight_duration".into(),
                            value: NbtSummaryValue::Int(2),
                        }]),
                    }]),
                },
            ]),
            &bbb_registry_codec::UnavailableHolderResolver,
        )
        .expect("firework stack with flight duration");
        assert_eq!(default_lifetime_for_item(&with_flight), 30);
    }
}
