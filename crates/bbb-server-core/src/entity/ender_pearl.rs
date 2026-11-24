//! Shared throwable movement and the world-owned ender pearl impact.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_PEARL_ID;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::hurting_projectile::ProjectileState;
use crate::entity::projectile_hit::HitResult;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::store::EntityStore;
use crate::entity::throwable_projectile;
use crate::entity::{Entity, EntityCommand, EntityData};

/// `ThrowableProjectile.getDefaultGravity()` (`ThrowableProjectile.java:106-108`).
pub const DEFAULT_GRAVITY: f64 = 0.03;

pub use super::throwable_projectile::ThrowableItemData as EnderPearlData;

pub fn spawn_ender_pearl(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    item: RuntimeItemStack,
    owner: Option<uuid::Uuid>,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let data = EnderPearlData::from_saved(item, ProjectileState::new(owner));
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_ENDER_PEARL_ID,
        position,
        EntityData::EnderPearl(data),
        |entity| shot.install(entity),
    )
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let hit = throwable_projectile::tick(entity, store, level);
    let Ok(hit) = hit else {
        return false;
    };
    if let Some(hit) = hit {
        store.queue_command(EntityCommand::EnderPearlHit {
            entity_id: entity.id(),
            target: match hit {
                HitResult::Entity { target, .. } => {
                    Some(throwable_projectile::pending_target(target))
                }
                HitResult::Block { .. } => None,
                HitResult::Miss => unreachable!("shared throwable tick returns real hits"),
            },
        });
    }
    true
}
