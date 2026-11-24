//! Shared `WindCharge` and `BreezeWindCharge` leaf state.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
};
use bbb_protocol::packets::Vec3d;
use bbb_server_types::directions::step_vec;
use uuid::Uuid;

use crate::entity::hurting_projectile::HurtingProjectileState;
use crate::entity::projectile_action::{
    PendingProjectileAction, PendingProjectileHurt, ProjectileLandOrigin,
};
use crate::entity::projectile_hit::{HitResult, ProjectileCandidate};
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::throwable_projectile::{pending_owner, pending_target};
use crate::entity::{Entity, EntityData, EntityStore};
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    PendingExplosion,
};
use crate::level_event::PendingHurtEntity;
use bbb_server_types::geometry::Vec3Ops;

#[derive(Debug, Clone, PartialEq)]
pub struct WindChargeData {
    projectile: HurtingProjectileState,
    no_deflect_ticks: i32,
}

impl WindChargeData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.no_deflect_ticks = previous.no_deflect_ticks;
    }

    pub const fn new(projectile: HurtingProjectileState, no_deflect_ticks: i32) -> Self {
        Self {
            projectile,
            no_deflect_ticks,
        }
    }

    pub const fn projectile(&self) -> &HurtingProjectileState {
        &self.projectile
    }

    pub fn projectile_mut(&mut self) -> &mut HurtingProjectileState {
        &mut self.projectile
    }

    pub(super) fn tick_no_deflect(&mut self) {
        self.no_deflect_ticks = (self.no_deflect_ticks - 1).max(0);
    }

    pub const fn no_deflect_ticks(&self) -> i32 {
        self.no_deflect_ticks
    }
}

pub fn spawn_wind_charge(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    owner: Option<Uuid>,
) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        position,
        EntityData::WindCharge(WindChargeData::new(
            HurtingProjectileState::new(owner, 0.0),
            5,
        )),
        |entity| shot.install(entity),
    )
}

pub fn spawn_breeze_wind_charge(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    owner: Uuid,
) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_BREEZE_WIND_CHARGE_ID,
        position,
        EntityData::BreezeWindCharge(WindChargeData::new(
            HurtingProjectileState::new(Some(owner), 0.0),
            0,
        )),
        |entity| shot.install(entity),
    )
}

pub(super) fn on_above_world(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    owner: Option<Uuid>,
    breeze: bool,
) {
    queue_explosion(entity, store, level, owner, breeze, entity.position());
    store.queue_command(crate::entity::EntityCommand::Projectile(
        PendingProjectileAction::Discard {
            entity_id: entity.id(),
        },
    ));
}

pub(super) fn on_hit(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    owner: Option<Uuid>,
    hit: HitResult,
    breeze: bool,
) -> bool {
    match hit {
        HitResult::Entity {
            target, location, ..
        } => {
            store.queue_command(crate::entity::EntityCommand::Projectile(
                PendingProjectileAction::Hurt(PendingProjectileHurt {
                    target: pending_target(target),
                    damage_type: "minecraft:wind_charge",
                    amount: 1.0,
                    causing: pending_owner(owner, store, level),
                    direct: Some(PendingHurtEntity::Stored {
                        entity_id: entity.id(),
                    }),
                    direct_resource_id: if breeze {
                        "minecraft:breeze_wind_charge"
                    } else {
                        "minecraft:wind_charge"
                    },
                    registry_tags: level.capture_registry_tags(),
                }),
            ));
            queue_explosion(entity, store, level, owner, breeze, location);
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
            queue_explosion(
                entity,
                store,
                level,
                owner,
                breeze,
                location.plus(step_vec(direction).scale(0.25)),
            );
            store.queue_command(crate::entity::EntityCommand::Projectile(
                PendingProjectileAction::Discard {
                    entity_id: entity.id(),
                },
            ));
            store.queue_command(crate::entity::EntityCommand::Projectile(
                PendingProjectileAction::Land {
                    origin: ProjectileLandOrigin::Block(pos),
                    source_entity_id: entity.id(),
                },
            ));
        }
        HitResult::Miss => unreachable!("the shared tick returns only non-miss hits"),
    }
    if matches!(hit, HitResult::Entity { .. }) {
        store.queue_command(crate::entity::EntityCommand::Projectile(
            PendingProjectileAction::Discard {
                entity_id: entity.id(),
            },
        ));
    }
    true
}

fn queue_explosion(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    owner: Option<uuid::Uuid>,
    breeze: bool,
    center: Vec3d,
) {
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
            wind_charge: true,
        }),
        center,
        radius: if breeze { 3.0 } else { 1.2 },
        fire: false,
        interaction: ExplosionInteraction::Trigger,
        damage_source: None,
        damage_profile: ExplosionDamageProfile::wind_charge(false, if breeze { 1.0 } else { 1.22 }),
        presentation: ExplosionPresentation::wind_charge(if breeze {
            "minecraft:entity.breeze.wind_burst"
        } else {
            "minecraft:entity.wind_charge.wind_burst"
        }),
    }));
}

pub(super) fn can_hit(candidate: ProjectileCandidate<'_>, level: &LevelCtx<'_, '_>) -> bool {
    if !candidate.is_alive_and_hurting_projectile_target_in_level(level) {
        return false;
    }
    if let ProjectileCandidate::Stored(entity) = candidate {
        if matches!(
            entity.data(),
            EntityData::WindCharge(_) | EntityData::BreezeWindCharge(_) | EntityData::EndCrystal(_)
        ) {
            return false;
        }
    }
    true
}
