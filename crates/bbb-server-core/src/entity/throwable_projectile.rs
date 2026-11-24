//! `ThrowableProjectile.tick` shared by the stored throwable item projectiles.

use crate::level_event::{PendingHurtEntity, VanillaGameEvent};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::EntityEvent;
use bbb_server_types::fluid::block_state_fluid_state;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::hurting_projectile::{self, ProjectileState};
use crate::entity::level::fluid_state_tag_membership;
use crate::entity::living::SynchedValue;
use crate::entity::projectile_action::{PendingProjectileAction, PendingProjectileHurt};
use crate::entity::projectile_hit::{
    hit_result_on_move_vector, HitResult, ProjectileCandidate, ProjectileDeflection,
    ProjectileTarget,
};
use crate::entity::store::EntityStore;
use crate::entity::{apply_gravity, Entity, EntityCommand, EntityData};

/// `ThrowableProjectile.applyInertia` (`ThrowableProjectile.java:67-92`).
const DRY_INERTIA: f32 = 0.99;
const WATER_INERTIA: f32 = 0.8;

/// `ThrowableItemProjectile.DATA_ITEM_STACK` (`ThrowableItemProjectile.java:17-19`).
const ITEM_STACK_DATA_ID: u8 = 8;

/// Item and owner state shared by throwable item projectiles.
#[derive(Debug, Clone, PartialEq)]
pub struct ThrowableItemData {
    pub(super) item: RuntimeItemStack,
    pub(crate) projectile: ProjectileState,
}

impl ThrowableItemData {
    pub fn new(item: RuntimeItemStack) -> Self {
        Self::from_saved(item, ProjectileState::default())
    }

    pub fn from_saved(item: RuntimeItemStack, projectile: ProjectileState) -> Self {
        Self { item, projectile }
    }

    pub const fn item(&self) -> &RuntimeItemStack {
        &self.item
    }

    pub const fn projectile(&self) -> &ProjectileState {
        &self.projectile
    }
}

/// `ThrowableItemProjectile` synched rows after the `Entity` rows.
pub(crate) fn synched_values(entity: &Entity, data: &ThrowableItemData) -> Vec<SynchedValue> {
    use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID;
    use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};

    let mut rows = crate::entity::living::entity_synched_values(entity);
    let stack = data.item().summary().clone();
    let is_default = stack.item_id.is_none();
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: ITEM_STACK_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(stack),
        },
        is_default,
    });
    rows
}

/// `ThrowableProjectile.tick` up to `hitTargetOrDeflectSelf`: `Err` refuses the
/// tick, `Ok(Some(hit))` is a non-deflected hit the subtype's `onHit` acts on.
pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Result<Option<HitResult>, ()> {
    let data = match &entity.data {
        EntityData::Egg(data)
        | EntityData::Snowball(data)
        | EntityData::EnderPearl(data)
        | EntityData::ExperienceBottle(data)
        | EntityData::SplashPotion(data)
        | EntityData::LingeringPotion(data) => data,
        _ => unreachable!("shared throwable tick requires a throwable item projectile"),
    };
    let mut state = data.projectile;
    let hit = tick_state(entity, store, level, &mut state);
    let data = match &mut entity.data {
        EntityData::Egg(data)
        | EntityData::Snowball(data)
        | EntityData::EnderPearl(data)
        | EntityData::ExperienceBottle(data)
        | EntityData::SplashPotion(data)
        | EntityData::LingeringPotion(data) => data,
        _ => unreachable!("shared throwable tick preserves the projectile family"),
    };
    // Deflection may replace the owner in the entity while tick_state runs.
    data.projectile.left_owner = state.left_owner();
    data.projectile.has_been_shot = state.has_been_shot();
    hit
}

fn tick_state(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    state: &mut ProjectileState,
) -> Result<Option<HitResult>, ()> {
    let owner = state.owner();
    let left_owner = state.left_owner();
    let owner_vehicle = owner_vehicle(owner, left_owner, store, level);
    apply_gravity(entity);
    let inertia = if entity.was_touching_water() {
        WATER_INERTIA
    } else {
        DRY_INERTIA
    };
    entity.set_delta_movement(entity.delta_movement().scale(f64::from(inertia)));
    let hit = hit_result_on_move_vector(entity, store, &*level, |candidate, store, level| {
        candidate.is_alive_and_pickable_in_level(level)
            && !rides_with_owner(candidate, owner_vehicle, store)
    })
    .ok_or(())?;
    let location = match hit {
        HitResult::Miss => entity.position().plus(entity.delta_movement()),
        HitResult::Block { location, .. } | HitResult::Entity { location, .. } => location,
    };
    let movement = entity.delta_movement();
    let horizontal = movement.horizontal_distance_sqr().sqrt();
    entity.set_rot(
        (movement.x.atan2(movement.z).to_degrees() as f32).rem_euclid(360.0),
        (-movement.y.atan2(horizontal).to_degrees() as f32).rem_euclid(360.0),
    );
    entity.set_pos(location);
    let destination_water = fluid_state_tag_membership(
        level,
        block_state_fluid_state(level.block_state(entity.block_position())),
        "minecraft:water",
    );
    store.refile_detached(entity);
    if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store)) {
        return Err(());
    }

    if !state.has_been_shot() {
        level.game_event_at(
            VanillaGameEvent::PROJECTILE_SHOOT,
            entity.position(),
            owner_entity_id(owner, store, level),
        );
        state.mark_shot();
    }
    if !state.left_owner() {
        state.set_left_owner(outside_owner_collision_range(entity, owner, store, level));
    }
    if !crate::entity::damage::base_tick_from_environment(entity, store, level) {
        return Err(());
    }
    let redirect_action = match hit {
        HitResult::Entity {
            target,
            deflection: ProjectileDeflection::None,
            ..
        } => hurting_projectile::projectile_deflection_action(target, owner, store, &*level)
            .map_err(|_| ())?,
        _ => None,
    };
    entity.set_was_touching_water(destination_water);

    if hit == HitResult::Miss || !entity.is_alive() {
        return Ok(None);
    }
    if let HitResult::Entity {
        target, deflection, ..
    } = hit
    {
        if hurting_projectile::apply_target_deflection(entity, store, target, deflection) {
            return Ok(None);
        }
    }
    if let Some(action) = redirect_action {
        store.queue_command(EntityCommand::Projectile(action));
    }
    match hit {
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
            let state = level.block_state(pos);
            level.game_event_with_state(
                VanillaGameEvent::PROJECTILE_LAND,
                pos,
                Some(entity.id()),
                Some(state),
            );
        }
        HitResult::Entity { location, .. } => level.game_event_at(
            VanillaGameEvent::PROJECTILE_LAND,
            location,
            Some(entity.id()),
        ),
        HitResult::Miss => unreachable!("a miss returned above"),
    }
    Ok(Some(hit))
}

#[derive(Clone, Copy)]
pub(crate) struct OwnerVehicle {
    root: i32,
    has_passengers: bool,
}

pub(crate) fn owner_vehicle(
    owner: Option<Uuid>,
    left_owner: bool,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<OwnerVehicle> {
    if left_owner {
        return None;
    }
    let root = store.root_vehicle(owner_entity_id(owner, store, level)?);
    Some(OwnerVehicle {
        root,
        has_passengers: !store.passengers(root).is_empty(),
    })
}

/// `Projectile.canHitEntity`'s owner clause (`Projectile.java:322-323`):
/// `owner.isPassengerOfSameVehicle(entity)`.
pub(crate) fn rides_with_owner(
    candidate: ProjectileCandidate<'_>,
    owner: Option<OwnerVehicle>,
    store: &EntityStore,
) -> bool {
    owner.is_some_and(|owner| {
        let id = candidate.target().entity_id();
        id == owner.root || (owner.has_passengers && store.root_vehicle(id) == owner.root)
    })
}

pub(crate) fn owner_entity_id(
    owner: Option<Uuid>,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<i32> {
    let owner = owner?;
    level
        .player(owner)
        .map(|player| player.entity_id)
        .or_else(|| store.entity_id_for_uuid(owner))
}

/// `Projectile.isOutsideOwnerCollisionRange` (`Projectile.java:117-128`): no
/// pickable body of the owner's vehicle stack is near the projectile.
pub(crate) fn outside_owner_collision_range(
    entity: &Entity,
    owner: Option<Uuid>,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> bool {
    let Some(owner_id) = owner_entity_id(owner, store, level) else {
        return true;
    };
    let area = entity
        .bounding_box()
        .expand_towards(entity.delta_movement())
        .inflate(1.0, 1.0, 1.0);
    let mut next = Some(store.root_vehicle(owner_id));
    let mut pending = Vec::new();
    while let Some(id) = next {
        let outside = match store.get(id) {
            Some(body) => !body.is_pickable() || !body.bounding_box().intersects(area),
            None => level
                .players()
                .iter()
                .find(|player| player.entity_id == id)
                .is_none_or(|player| player.spectator || !player.bounding_box().intersects(area)),
        };
        if !outside {
            return false;
        }
        pending.extend_from_slice(store.passengers(id));
        next = pending.pop();
    }
    true
}

pub(crate) const fn pending_target(target: ProjectileTarget) -> PendingHurtEntity {
    match target {
        ProjectileTarget::Stored { entity_id } => PendingHurtEntity::Stored { entity_id },
        ProjectileTarget::Player { uuid, .. } => PendingHurtEntity::Player { uuid },
    }
}

/// The living owner a projectile damage source names as its causing entity.
pub(crate) fn pending_owner(
    owner: Option<Uuid>,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<PendingHurtEntity> {
    let owner = owner?;
    if level.player(owner).is_some() {
        return Some(PendingHurtEntity::Player { uuid: owner });
    }
    let entity_id = store.entity_id_for_uuid(owner)?;
    store
        .get(entity_id)
        .is_some_and(|entity| entity.living().is_some())
        .then_some(PendingHurtEntity::Stored { entity_id })
}

pub(crate) struct ThrowableHurt {
    pub(crate) target: PendingHurtEntity,
    pub(crate) damage_type: &'static str,
    pub(crate) amount: f32,
    pub(crate) causing: Option<PendingHurtEntity>,
    pub(crate) direct_resource_id: &'static str,
}

/// Queues `entity.hurt(damageSource(projectile, owner), amount)` for a world
/// that still holds the projectile as the direct entity.
pub(crate) fn queue_hurt(
    projectile: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    hurt: ThrowableHurt,
) {
    store.queue_command(EntityCommand::Projectile(PendingProjectileAction::Hurt(
        PendingProjectileHurt {
            target: hurt.target,
            damage_type: hurt.damage_type,
            amount: hurt.amount,
            causing: hurt.causing,
            direct: Some(PendingHurtEntity::Stored {
                entity_id: projectile.id(),
            }),
            direct_resource_id: hurt.direct_resource_id,
            registry_tags: level.capture_registry_tags(),
        },
    )));
}

pub(crate) fn broadcast_entity_event(projectile: &Entity, store: &mut EntityStore, event_id: i8) {
    store.broadcast_to_tracking(
        projectile.id(),
        crate::connection::ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id: projectile.id(),
            event_id,
        }),
    );
}

/// `discard()` after an entity hit: the queued hurt still reads the projectile
/// as its direct entity, so removal is ordered after it.
pub(crate) fn discard_after_queued_hit(
    projectile: &mut Entity,
    store: &mut EntityStore,
    queued: bool,
) {
    if queued || store.has_pending_player_hurt_entity_from(projectile.id()) {
        store.queue_command(EntityCommand::Projectile(
            PendingProjectileAction::Discard {
                entity_id: projectile.id(),
            },
        ));
    } else {
        projectile.discard();
    }
}
