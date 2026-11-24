//! `ThrownEgg`: a `ThrowableItemProjectile` that hurts what it hits for no
//! damage and sometimes hatches baby chickens where it breaks.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_EGG_ID};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::{Aabb, Vec3Ops};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::hurting_projectile::ProjectileState;
use crate::entity::projectile_hit::HitResult;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::store::EntityStore;
use crate::entity::throwable_projectile;
use crate::entity::{Entity, EntityData};

/// `ThrowableProjectile.getDefaultGravity()` (`ThrowableProjectile.java:106-108`).
pub const DEFAULT_GRAVITY: f64 = 0.03;

pub use super::throwable_projectile::ThrowableItemData as EggData;

pub fn spawn_egg(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    item: RuntimeItemStack,
    owner: Option<uuid::Uuid>,
) -> Option<i32> {
    let uuid = store.next_uuid();
    let data = EggData::from_saved(item, ProjectileState::new(owner));
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_EGG_ID,
        position,
        EntityData::Egg(data),
        |entity| shot.install(entity),
    )
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let EntityData::Egg(data) = &entity.data else {
        unreachable!("egg tick requires its projectile family");
    };
    let owner = data.projectile.owner;
    let hit = throwable_projectile::tick(entity, store, level);
    let Ok(hit) = hit else {
        return false;
    };
    let Some(hit) = hit else {
        return true;
    };
    // `ThrownEgg.onHitEntity` (`ThrownEgg.java:59-62`).
    let hurt = if let HitResult::Entity { target, .. } = hit {
        let causing = throwable_projectile::pending_owner(owner, store, level);
        throwable_projectile::queue_hurt(
            entity,
            store,
            level,
            throwable_projectile::ThrowableHurt {
                target: throwable_projectile::pending_target(target),
                damage_type: "minecraft:thrown",
                amount: 0.0,
                causing,
                direct_resource_id: "minecraft:egg",
            },
        );
        true
    } else {
        false
    };
    // `ThrownEgg.onHit` (`ThrownEgg.java:65-92`).
    if store.random().next_int(8) == 0 {
        let EntityData::Egg(data) = &entity.data else {
            unreachable!("shared throwable tick preserves the egg family");
        };
        let item = &data.item;
        let count = if store.random().next_int(32) == 0 {
            4
        } else {
            1
        };
        for _ in 0..count {
            // `fudgePositionAfterSizeChange(ZERO_SIZED_DIMENSIONS)` (`ThrownEgg.java:80`).
            let Some(center) = free_chick_center(level, entity.position()) else {
                break;
            };
            let feet = Vec3d {
                y: center.y - f64::from(BABY_CHICKEN_HEIGHT) / 2.0,
                ..center
            };
            if !hatch_chick(entity, store, item, feet) {
                return false;
            }
        }
    }
    throwable_projectile::broadcast_entity_event(entity, store, 3);
    throwable_projectile::discard_after_queued_hit(entity, store, hurt);
    true
}

/// `ThrownEgg.java:75-84`: a triggered baby chicken at the egg, with the
/// variant the egg item carries.
fn hatch_chick(
    egg: &Entity,
    store: &mut EntityStore,
    item: &RuntimeItemStack,
    position: Vec3d,
) -> bool {
    let Some(id) = store.spawn_general_living(VANILLA_ENTITY_TYPE_CHICKEN_ID, position) else {
        return false;
    };
    let Some(chicken) = store.get_mut(id) else {
        return false;
    };
    chicken.set_rot(egg.y_rot(), 0.0);
    let variant = chicken_variant(item);
    crate::entity::set_spawner_age(chicken, -24000)
        && variant.is_none_or(|(resource_id, holder_id)| {
            crate::entity::set_spawner_registry_value(
                chicken,
                "minecraft:chicken_variant",
                resource_id,
                holder_id,
            )
        })
}

/// `Chicken.BABY_DIMENSIONS` (`Chicken.java:56`).
const BABY_CHICKEN_WIDTH: f32 = 0.3;
const BABY_CHICKEN_HEIGHT: f32 = 0.4;

/// `Entity.fudgePositionAfterSizeChange` from zero-sized dimensions
/// (`Entity.java:3324-3335`) over `CollisionGetter.findFreePosition`
/// (`CollisionGetter.java:155-171`): the point nearest the egg, within the
/// grown size, where a baby chicken's centre clears every block collision box.
fn free_chick_center(level: &LevelCtx<'_, '_>, egg: Vec3d) -> Option<Vec3d> {
    let (width, height) = (
        f64::from(BABY_CHICKEN_WIDTH),
        f64::from(BABY_CHICKEN_HEIGHT),
    );
    let allowed = Aabb::of_size(egg, width + 1.0e-6, height + 1.0e-6, width + 1.0e-6);
    let search = allowed.inflate(width, height, width);
    let mut blocked = Vec::new();
    for x in search.min_x.floor() as i32 - 1..=search.max_x.floor() as i32 + 1 {
        for y in search.min_y.floor() as i32 - 1..=search.max_y.floor() as i32 + 1 {
            for z in search.min_z.floor() as i32 - 1..=search.max_z.floor() as i32 + 1 {
                let offset = |shape: Aabb| Aabb {
                    min_x: shape.min_x + f64::from(x),
                    min_y: shape.min_y + f64::from(y),
                    min_z: shape.min_z + f64::from(z),
                    max_x: shape.max_x + f64::from(x),
                    max_y: shape.max_y + f64::from(y),
                    max_z: shape.max_z + f64::from(z),
                };
                blocked.extend(
                    level
                        .block_collision_boxes(BlockPos { x, y, z })
                        .into_iter()
                        .map(offset)
                        .filter(|shape| shape.intersects(search))
                        .map(|shape| shape.inflate(width / 2.0, height / 2.0, width / 2.0)),
                );
            }
        }
    }
    let cuts = |min: f64, max: f64, edges: &dyn Fn(&Aabb) -> [f64; 2]| {
        let mut cuts = vec![min, max];
        cuts.extend(
            blocked
                .iter()
                .flat_map(edges)
                .filter(|cut| *cut > min && *cut < max),
        );
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        cuts
    };
    let xs = cuts(allowed.min_x, allowed.max_x, &|b| [b.min_x, b.max_x]);
    let ys = cuts(allowed.min_y, allowed.max_y, &|b| [b.min_y, b.max_y]);
    let zs = cuts(allowed.min_z, allowed.max_z, &|b| [b.min_z, b.max_z]);
    let mut best: Option<(f64, Vec3d)> = None;
    for x in xs.windows(2) {
        for y in ys.windows(2) {
            for z in zs.windows(2) {
                let mid = Vec3d {
                    x: (x[0] + x[1]) / 2.0,
                    y: (y[0] + y[1]) / 2.0,
                    z: (z[0] + z[1]) / 2.0,
                };
                if blocked.iter().any(|b| {
                    b.min_x < mid.x
                        && mid.x < b.max_x
                        && b.min_y < mid.y
                        && mid.y < b.max_y
                        && b.min_z < mid.z
                        && mid.z < b.max_z
                }) {
                    continue;
                }
                let point = Vec3d {
                    x: egg.x.clamp(x[0], x[1]),
                    y: egg.y.clamp(y[0], y[1]),
                    z: egg.z.clamp(z[0], z[1]),
                };
                let distance = point.distance_to_sqr(egg);
                if best.is_none_or(|(nearest, _)| distance < nearest) {
                    best = Some((distance, point));
                }
            }
        }
    }
    best.map(|(_, point)| point)
}

/// The built-in `minecraft:chicken_variant` entries in holder order.
const CHICKEN_VARIANTS: [&str; 3] = ["minecraft:cold", "minecraft:temperate", "minecraft:warm"];

/// The egg item's `minecraft:chicken_variant`: a patched holder, else the
/// item default (`Items.java:1368-1376`).
fn chicken_variant(item: &RuntimeItemStack) -> Option<(&'static str, i32)> {
    let holder_id =
        item.summary()
            .component_patch
            .chicken_variant_id
            .or(match item.item_resource_id()? {
                "minecraft:blue_egg" => Some(0),
                "minecraft:egg" => Some(1),
                "minecraft:brown_egg" => Some(2),
                _ => None,
            })?;
    let resource_id = CHICKEN_VARIANTS.get(usize::try_from(holder_id).ok()?)?;
    Some((resource_id, holder_id))
}
