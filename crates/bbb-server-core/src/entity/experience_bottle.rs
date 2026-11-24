//! `ThrownExperienceBottle`: a `ThrowableItemProjectile` that breaks into
//! experience orbs.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID;
use bbb_protocol::packets::Vec3d;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::hurting_projectile::ProjectileState;
use crate::entity::projectile_hit::HitResult;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::store::EntityStore;
use crate::entity::throwable_projectile;
use crate::entity::{Entity, EntityData};

/// `ThrownExperienceBottle.getDefaultGravity()` (`ThrownExperienceBottle.java:34-36`).
pub const DEFAULT_GRAVITY: f64 = 0.07;

pub use super::throwable_projectile::ThrowableItemData as ExperienceBottleData;

pub fn spawn_experience_bottle(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    item: RuntimeItemStack,
    owner: Option<uuid::Uuid>,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let data = ExperienceBottleData::from_saved(item, ProjectileState::new(owner));
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID,
        position,
        EntityData::ExperienceBottle(data),
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
    let Some(hit) = hit else {
        return true;
    };
    // `ThrownExperienceBottle.onHit` (`ThrownExperienceBottle.java:39-53`).
    level.level_event(2002, entity.block_position(), -13083194);
    let amount = 3 + store.random().next_int(5) + store.random().next_int(5);
    let (location, rough) = match hit {
        HitResult::Block {
            location,
            direction,
            ..
        } => (location, direction_normal(direction)),
        HitResult::Entity { location, .. } => (location, entity.delta_movement().scale(-1.0)),
        HitResult::Miss => (entity.position(), Vec3d::default()),
    };
    let orbs = crate::entity::award_experience(store, level.level_random(), location, amount);
    // `ExperienceOrb(level, pos, roughly, value)` (`ExperienceOrb.java:59-65`):
    // the orb leaves half its 0.5 size along the rough direction, moving away.
    if rough.length_sqr() > 0.0 {
        let offset = rough.normalize().scale(0.25);
        for id in orbs {
            if let Some(orb) = store.get_mut(id) {
                let movement = orb.delta_movement();
                if rough.x * movement.x + rough.y * movement.y + rough.z * movement.z < 0.0 {
                    orb.set_delta_movement(movement.scale(-1.0));
                }
                orb.set_pos(location.plus(offset));
                store.refile(id);
            }
        }
    }
    entity.discard();
    true
}

const fn direction_normal(direction: bbb_protocol::packets::Direction) -> Vec3d {
    use bbb_protocol::packets::Direction;
    let (x, y, z) = match direction {
        Direction::Down => (0.0, -1.0, 0.0),
        Direction::Up => (0.0, 1.0, 0.0),
        Direction::North => (0.0, 0.0, -1.0),
        Direction::South => (0.0, 0.0, 1.0),
        Direction::West => (-1.0, 0.0, 0.0),
        Direction::East => (1.0, 0.0, 0.0),
    };
    Vec3d { x, y, z }
}
