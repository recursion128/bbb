//! `LargeFireball` leaf state; hit effects are owned by the fireball lane.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FIREBALL_ID;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::hurting_projectile::HurtingProjectileState;
use crate::entity::item;
use crate::entity::living::SynchedValue;
use crate::entity::projectile_action::{
    PendingProjectileAction, PendingProjectileHurt, ProjectileLandOrigin,
};
use crate::entity::projectile_hit::HitResult;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::throwable_projectile::pending_target;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    PendingExplosion,
};
use crate::level_event::PendingHurtEntity;

#[derive(Debug, Clone, PartialEq)]
pub struct LargeFireballData {
    projectile: HurtingProjectileState,
    item: RuntimeItemStack,
    explosion_power: i8,
}

pub fn synched_values(entity: &Entity, data: &LargeFireballData) -> Vec<SynchedValue> {
    synched_values_for_item(entity, data.item())
}

pub(super) fn synched_values_for_item(
    entity: &Entity,
    item: &RuntimeItemStack,
) -> Vec<SynchedValue> {
    use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID;
    use bbb_protocol::entity_synched_data::VANILLA_FIREBALL_ITEM_DATA_ID;
    use bbb_protocol::item_types::VANILLA_ITEM_FIRE_CHARGE_ID;
    use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};

    let mut rows = crate::entity::living::entity_synched_values(entity);
    let stack = item.summary().clone();
    let is_default = stack.item_id == Some(VANILLA_ITEM_FIRE_CHARGE_ID)
        && stack.count == 1
        && stack.component_patch.is_empty();
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_FIREBALL_ITEM_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(stack),
        },
        is_default,
    });
    rows
}

pub(crate) fn default_item() -> RuntimeItemStack {
    item::plain_stack("minecraft:fire_charge", 1)
        .expect("the built-in fire charge has a canonical stack")
}

impl LargeFireballData {
    pub(crate) fn replace_slot_item(&mut self, item: RuntimeItemStack) {
        self.item = item;
    }

    pub const fn new(
        projectile: HurtingProjectileState,
        item: RuntimeItemStack,
        explosion_power: i8,
    ) -> Self {
        Self {
            projectile,
            item,
            explosion_power,
        }
    }

    pub const fn projectile(&self) -> &HurtingProjectileState {
        &self.projectile
    }

    pub fn projectile_mut(&mut self) -> &mut HurtingProjectileState {
        &mut self.projectile
    }

    pub const fn item(&self) -> &RuntimeItemStack {
        &self.item
    }

    pub const fn explosion_power(&self) -> i8 {
        self.explosion_power
    }
}

pub fn spawn_large_fireball(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    owner: Uuid,
    explosion_power: i8,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let item = default_item();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_FIREBALL_ID,
        position,
        EntityData::LargeFireball(LargeFireballData::new(
            HurtingProjectileState::new(
                Some(owner),
                HurtingProjectileState::INITIAL_ACCELERATION_POWER,
            ),
            item,
            explosion_power,
        )),
        |entity| shot.install(entity),
    )
}

pub(super) fn on_hit(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    owner: Option<Uuid>,
    hit: HitResult,
    explosion_power: i8,
) -> bool {
    if let HitResult::Entity { target, .. } = hit {
        let causing = pending_owner(owner, store, level);
        store.queue_command(crate::entity::EntityCommand::Projectile(
            PendingProjectileAction::Hurt(PendingProjectileHurt {
                target: pending_target(target),
                damage_type: if causing.is_some() {
                    "minecraft:fireball"
                } else {
                    "minecraft:unattributed_fireball"
                },
                amount: 6.0,
                causing,
                direct: Some(PendingHurtEntity::Stored {
                    entity_id: entity.id(),
                }),
                direct_resource_id: "minecraft:fireball",
                registry_tags: level.capture_registry_tags(),
            }),
        ));
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
        radius: f32::from(explosion_power),
        fire: level.mob_griefing(),
        interaction: ExplosionInteraction::Mob,
        damage_source: None,
        damage_profile: ExplosionDamageProfile::Default,
        presentation: ExplosionPresentation::Default,
    }));
    store.queue_command(crate::entity::EntityCommand::Projectile(
        PendingProjectileAction::Discard {
            entity_id: entity.id(),
        },
    ));
    true
}

fn pending_owner(
    owner: Option<uuid::Uuid>,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<PendingHurtEntity> {
    let owner = owner?;
    if level.player(owner).is_some() {
        Some(PendingHurtEntity::Player { uuid: owner })
    } else {
        store
            .entity_id_for_uuid(owner)
            .map(|entity_id| PendingHurtEntity::Stored { entity_id })
    }
}
