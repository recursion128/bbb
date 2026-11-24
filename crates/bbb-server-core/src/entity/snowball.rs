//! `Snowball`: a `ThrowableItemProjectile` that hurts what it hits (only a
//! blaze takes damage) and breaks.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_SNOWBALL_ID};
use bbb_protocol::packets::Vec3d;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::hurting_projectile::ProjectileState;
use crate::entity::projectile_hit::HitResult;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::store::EntityStore;
use crate::entity::throwable_projectile;
use crate::entity::{Entity, EntityData};

/// `ThrowableProjectile.getDefaultGravity()` (`ThrowableProjectile.java:106-108`).
pub const DEFAULT_GRAVITY: f64 = 0.03;

pub use super::throwable_projectile::ThrowableItemData as SnowballData;

pub fn spawn_snowball(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    item: RuntimeItemStack,
    owner: Option<uuid::Uuid>,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let data = SnowballData::from_saved(item, ProjectileState::new(owner));
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_SNOWBALL_ID,
        position,
        EntityData::Snowball(data),
        |entity| shot.install(entity),
    )
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let EntityData::Snowball(data) = &entity.data else {
        unreachable!("snowball tick requires its projectile family");
    };
    let owner = data.projectile.owner;
    let hit = throwable_projectile::tick(entity, store, level);
    let Ok(hit) = hit else {
        return false;
    };
    let Some(hit) = hit else {
        return true;
    };
    // `Snowball.onHitEntity` (`Snowball.java:53-58`).
    let hurt = if let HitResult::Entity { target, .. } = hit {
        let is_blaze = store
            .get(target.entity_id())
            .is_some_and(|target| target.entity_type_id() == VANILLA_ENTITY_TYPE_BLAZE_ID);
        let causing = throwable_projectile::pending_owner(owner, store, level);
        throwable_projectile::queue_hurt(
            entity,
            store,
            level,
            throwable_projectile::ThrowableHurt {
                target: throwable_projectile::pending_target(target),
                damage_type: "minecraft:thrown",
                amount: if is_blaze { 3.0 } else { 0.0 },
                causing,
                direct_resource_id: "minecraft:snowball",
            },
        );
        true
    } else {
        false
    };
    // `Snowball.onHit` (`Snowball.java:61-67`).
    throwable_projectile::broadcast_entity_event(entity, store, 3);
    throwable_projectile::discard_after_queued_hit(entity, store, hurt);
    true
}
