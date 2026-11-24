//! `WitherSkull` state, metadata and ordered hit effects.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID;
use bbb_protocol::entity_synched_data::VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITHER_SKULL_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, Vec3d};
use uuid::Uuid;

use crate::entity::hurting_projectile::HurtingProjectileState;
use crate::entity::living::SynchedValue;
use crate::entity::projectile_action::{
    PendingProjectileAction, PendingProjectileHurt, PendingProjectileMobEffect,
    ProjectileLandOrigin,
};
use crate::entity::projectile_hit::{HitResult, ProjectileTarget};
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::throwable_projectile::{pending_owner, pending_target};
use crate::entity::{Entity, EntityData, EntityStore};
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    PendingExplosion,
};
use crate::level_event::PendingHurtEntity;

#[derive(Debug, Clone, PartialEq)]
pub struct WitherSkullData {
    projectile: HurtingProjectileState,
    dangerous: bool,
}

impl WitherSkullData {
    pub const fn new(projectile: HurtingProjectileState, dangerous: bool) -> Self {
        Self {
            projectile,
            dangerous,
        }
    }

    pub const fn projectile(&self) -> &HurtingProjectileState {
        &self.projectile
    }

    pub fn projectile_mut(&mut self) -> &mut HurtingProjectileState {
        &mut self.projectile
    }

    pub const fn is_dangerous(&self) -> bool {
        self.dangerous
    }
}

pub fn set_dangerous(entity: &mut Entity, dangerous: bool) {
    let EntityData::WitherSkull(data) = &mut entity.data else {
        return;
    };
    if data.dangerous != dangerous {
        data.dangerous = dangerous;
        entity.mark_synched_dirty(VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID);
    }
}

pub fn finish_load(entity: &mut Entity) {
    if matches!(entity.data(), EntityData::WitherSkull(data) if data.is_dangerous()) {
        entity.mark_synched_dirty(VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID);
    }
}

pub fn synched_values(entity: &Entity, data: &WitherSkullData) -> Vec<SynchedValue> {
    let mut rows = crate::entity::living::entity_synched_values(entity);
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(data.dangerous),
        },
        is_default: !data.dangerous,
    });
    rows
}

pub fn spawn_wither_skull(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    owner: Option<Uuid>,
    dangerous: bool,
) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        position,
        EntityData::WitherSkull(WitherSkullData::new(
            HurtingProjectileState::new(owner, HurtingProjectileState::INITIAL_ACCELERATION_POWER),
            dangerous,
        )),
        |entity| {
            shot.install(entity);
            if dangerous {
                entity.mark_synched_dirty(VANILLA_WITHER_SKULL_DANGEROUS_DATA_ID);
            }
        },
    )
}

pub(super) fn on_hit(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    owner: Option<Uuid>,
    hit: HitResult,
    dangerous: bool,
) -> bool {
    if let HitResult::Entity { target, .. } = hit {
        queue_hurt(entity, store, level, owner, target);
    }
    let origin = match hit {
        HitResult::Block { pos, .. } => ProjectileLandOrigin::Block(pos),
        HitResult::Entity { location, .. } => ProjectileLandOrigin::Entity(location),
        HitResult::Miss => unreachable!("the shared tick returns only non-miss hits"),
    };
    store.queue_command(crate::entity::EntityCommand::Projectile(
        PendingProjectileAction::Land {
            origin,
            source_entity_id: entity.id(),
        },
    ));
    let direct = crate::entity::damage::explosion_source_entity(entity);
    let indirect = owner.and_then(|owner| {
        crate::entity::damage::explosion_owner_source_entity(owner, store, level)
    });
    store.queue_command(crate::entity::EntityCommand::Explosion(PendingExplosion {
        source: Some(ExplosionSource {
            direct,
            indirect: indirect.clone(),
            causing: indirect,
            in_water: entity.was_touching_water(),
            wind_charge: false,
        }),
        center: entity.position(),
        radius: 1.0,
        fire: false,
        interaction: ExplosionInteraction::Mob,
        damage_source: None,
        damage_profile: if dangerous {
            ExplosionDamageProfile::DangerousWitherSkull
        } else {
            ExplosionDamageProfile::Default
        },
        presentation: ExplosionPresentation::Default,
    }));
    store.queue_command(crate::entity::EntityCommand::Projectile(
        PendingProjectileAction::Discard {
            entity_id: entity.id(),
        },
    ));
    true
}

fn queue_hurt(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    owner: Option<Uuid>,
    target: ProjectileTarget,
) {
    let causing = pending_owner(owner, store, level);
    let hurt = PendingProjectileHurt {
        target: pending_target(target),
        damage_type: if causing.is_some() {
            "minecraft:wither_skull"
        } else {
            "minecraft:magic"
        },
        amount: if causing.is_some() { 8.0 } else { 5.0 },
        causing: causing.clone(),
        direct: causing.as_ref().map(|_| PendingHurtEntity::Stored {
            entity_id: entity.id(),
        }),
        direct_resource_id: "minecraft:wither_skull",
        registry_tags: level.capture_registry_tags(),
    };
    if target_is_living(target, store, level) {
        store.queue_command(crate::entity::EntityCommand::Projectile(
            PendingProjectileAction::HurtThenMobEffect {
                hurt,
                effect: PendingProjectileMobEffect {
                    resource_id: "minecraft:wither",
                    amplifier: 1,
                    easy_duration_ticks: 0,
                    normal_duration_ticks: 200,
                    hard_duration_ticks: 800,
                    source_owner: owner,
                    projectile_entity_id: entity.id(),
                },
            },
        ));
    } else {
        store.queue_command(crate::entity::EntityCommand::Projectile(
            PendingProjectileAction::Hurt(hurt),
        ));
    }
}

fn target_is_living(
    target: ProjectileTarget,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> bool {
    match target {
        ProjectileTarget::Stored { entity_id } => store
            .get(entity_id)
            .is_some_and(|entity| entity.living().is_some()),
        ProjectileTarget::Player { uuid, .. } => level.player(uuid).is_some(),
    }
}
