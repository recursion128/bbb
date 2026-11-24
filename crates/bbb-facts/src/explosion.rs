//! Shared explosion ray exposure, bounding box, damage, and knockback math.

use crate::collision::{
    clip_ray_with, vec3_distance_sqr, vec3_normalize, CollisionBounds, CollisionBox,
};

/// `ServerExplosion.LARGE_EXPLOSION_RADIUS` (`ServerExplosion.java:42`).
pub const LARGE_EXPLOSION_RADIUS: f32 = 2.0;

/// `WaterFluid.getExplosionResistance` (`WaterFluid.java:122-124`) / `LavaFluid` (`LavaFluid.java:226-228`).
pub const FLUID_EXPLOSION_RESISTANCE: f32 = 100.0;

#[inline]
fn lerp(alpha: f64, p0: f64, p1: f64) -> f64 {
    p0 + alpha * (p1 - p0)
}

/// Entity collection AABB for `ServerExplosion.hurtEntities` (`ServerExplosion.java:173-182`).
///
/// Returns `None` when `radius < 1.0E-5F` (`:173`).
pub fn hurt_entities_box(center: [f64; 3], radius: f32) -> Option<CollisionBox> {
    if !radius.is_finite()
        || radius < 1.0e-5
        || !center[0].is_finite()
        || !center[1].is_finite()
        || !center[2].is_finite()
    {
        return None;
    }
    let double_radius = f64::from(radius * 2.0);
    Some(CollisionBox {
        min_x: f64::from((center[0] - double_radius - 1.0).floor() as i32),
        min_y: f64::from((center[1] - double_radius - 1.0).floor() as i32),
        min_z: f64::from((center[2] - double_radius - 1.0).floor() as i32),
        max_x: f64::from((center[0] + double_radius + 1.0).floor() as i32),
        max_y: f64::from((center[1] + double_radius + 1.0).floor() as i32),
        max_z: f64::from((center[2] + double_radius + 1.0).floor() as i32),
    })
}

/// Tests whether an unblocked line of sight reaches `to` from `from` through block-local shapes.
pub fn clip_misses_with<B, I>(
    from: [f64; 3],
    to: [f64; 3],
    mut boxes_at: impl FnMut([i32; 3]) -> I,
) -> bool
where
    B: CollisionBounds,
    I: AsRef<[B]>,
{
    matches!(
        clip_ray_with(from, to, |pos| Some((boxes_at(pos), None::<CollisionBox>))),
        Some(None)
    )
}

/// `ServerExplosion.getSeenPercent` (`ServerExplosion.java:88-105`).
pub fn get_seen_percent<M, B, I>(
    center: [f64; 3],
    bounds: M,
    mut boxes_at: impl FnMut([i32; 3]) -> I,
) -> f32
where
    M: CollisionBounds,
    B: CollisionBounds,
    I: AsRef<[B]>,
{
    let xs = 1.0 / ((bounds.max_x() - bounds.min_x()) * 2.0 + 1.0);
    let ys = 1.0 / ((bounds.max_y() - bounds.min_y()) * 2.0 + 1.0);
    let zs = 1.0 / ((bounds.max_z() - bounds.min_z()) * 2.0 + 1.0);
    if !(xs > 0.0 && xs.is_finite() && ys > 0.0 && ys.is_finite() && zs > 0.0 && zs.is_finite()) {
        return 0.0;
    }
    let x_offset = (1.0 - (1.0 / xs).floor() * xs) / 2.0;
    let z_offset = (1.0 - (1.0 / zs).floor() * zs) / 2.0;
    if !x_offset.is_finite() || !z_offset.is_finite() {
        return 0.0;
    }
    let mut hits: i32 = 0;
    let mut count: i32 = 0;
    let mut xx = 0.0;
    while xx <= 1.0 {
        let mut yy = 0.0;
        while yy <= 1.0 {
            let mut zz = 0.0;
            while zz <= 1.0 {
                let from = [
                    lerp(xx, bounds.min_x(), bounds.max_x()) + x_offset,
                    lerp(yy, bounds.min_y(), bounds.max_y()),
                    lerp(zz, bounds.min_z(), bounds.max_z()) + z_offset,
                ];
                if clip_misses_with(from, center, &mut boxes_at) {
                    hits += 1;
                }
                count += 1;
                zz += zs;
            }
            yy += ys;
        }
        xx += xs;
    }
    hits as f32 / count as f32
}

/// Normalized distance ratio `Math.sqrt(entity.distanceToSqr(center)) / (radius * 2)` (`ServerExplosion.java:184`).
#[inline]
pub fn entity_distance_ratio(center: [f64; 3], entity_position: [f64; 3], radius: f32) -> f64 {
    vec3_distance_sqr(entity_position, center).sqrt() / f64::from(radius * 2.0)
}

/// `ExplosionDamageCalculator.getEntityDamageAmount` (`ExplosionDamageCalculator.java:31-37`).
#[inline]
pub fn entity_damage_amount(
    radius: f32,
    center: [f64; 3],
    entity_position: [f64; 3],
    exposure: f32,
) -> f32 {
    let double_radius = radius * 2.0;
    let dist = vec3_distance_sqr(entity_position, center).sqrt() / f64::from(double_radius);
    let impact = (1.0 - dist) * f64::from(exposure);
    ((impact * impact + impact) / 2.0 * 7.0 * f64::from(double_radius) + 1.0) as f32
}

/// `ServerExplosion.hurtEntities` knockback vector (`ServerExplosion.java:187, 198-199`).
#[inline]
pub fn explosion_knockback(
    center: [f64; 3],
    entity_origin: [f64; 3],
    dist_ratio: f64,
    exposure: f32,
    knockback_multiplier: f32,
    knockback_resistance: f64,
) -> [f64; 3] {
    let direction = vec3_normalize([
        entity_origin[0] - center[0],
        entity_origin[1] - center[1],
        entity_origin[2] - center[2],
    ]);
    let power = (1.0 - dist_ratio)
        * f64::from(exposure)
        * f64::from(knockback_multiplier)
        * (1.0 - knockback_resistance);
    [
        direction[0] * power,
        direction[1] * power,
        direction[2] * power,
    ]
}

/// Inputs for [`compute_explosion_entity_hit`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExplosionHitInput<M> {
    pub center: [f64; 3],
    pub radius: f32,
    pub entity_position: [f64; 3],
    pub entity_origin: [f64; 3],
    pub bounding_box: M,
    pub knockback_resistance: f64,
    pub damages_entities: bool,
    pub knockback_multiplier: f32,
}

/// One entity's exposure, damage, and knockback from `ServerExplosion.hurtEntities` (`ServerExplosion.java:184-199`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExplosionEntityHit {
    pub dist: f64,
    pub exposure: f32,
    pub damage: f32,
    pub knockback: [f64; 3],
}

pub fn compute_explosion_entity_hit<M, B, I>(
    input: ExplosionHitInput<M>,
    boxes_at: impl FnMut([i32; 3]) -> I,
) -> Option<ExplosionEntityHit>
where
    M: CollisionBounds,
    B: CollisionBounds,
    I: AsRef<[B]>,
{
    if !input.radius.is_finite() || input.radius <= 0.0 {
        return None;
    }
    let dist = entity_distance_ratio(input.center, input.entity_position, input.radius);
    if !dist.is_finite() || dist > 1.0 {
        return None;
    }
    let exposure = if !input.damages_entities && input.knockback_multiplier == 0.0 {
        0.0
    } else {
        get_seen_percent(input.center, input.bounding_box, boxes_at)
    };
    let damage = if input.damages_entities {
        entity_damage_amount(input.radius, input.center, input.entity_position, exposure)
    } else {
        0.0
    };
    let knockback = explosion_knockback(
        input.center,
        input.entity_origin,
        dist,
        exposure,
        input.knockback_multiplier,
        input.knockback_resistance,
    );
    Some(ExplosionEntityHit {
        dist,
        exposure,
        damage,
        knockback,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hurt_entities_box_and_seen_percent_respect_occlusion() {
        assert!(hurt_entities_box([0.5, 64.0, 0.5], 0.0).is_none());
        let search = hurt_entities_box([0.5, 64.0, 0.5], 4.0).expect("radius 4 box");
        assert_eq!(search.min_x, -9.0);
        assert_eq!(search.max_x, 9.0);

        let target = CollisionBox::from_arrays([2.2, 64.0, 0.2], [2.8, 65.8, 0.8]);
        assert_eq!(
            get_seen_percent([0.5, 64.5, 0.5], target, |_| Vec::<CollisionBox>::new()),
            1.0
        );
        let blocked = get_seen_percent([0.5, 64.5, 0.5], target, |pos| {
            if pos[0] == 1 && (63..=66).contains(&pos[1]) && (-1..=2).contains(&pos[2]) {
                vec![CollisionBox::UNIT_CUBE]
            } else {
                Vec::new()
            }
        });
        assert_eq!(blocked, 0.0);
    }

    #[test]
    fn explosion_damage_and_knockback_scale_with_distance_and_resistance() {
        let hit = compute_explosion_entity_hit(
            ExplosionHitInput {
                center: [0.5, 64.0, 0.5],
                radius: 4.0,
                entity_position: [4.5, 64.0, 0.5],
                entity_origin: [4.5, 65.0, 0.5],
                bounding_box: CollisionBox::from_arrays([4.2, 64.0, 0.2], [4.8, 65.8, 0.8]),
                knockback_resistance: 0.25,
                damages_entities: true,
                knockback_multiplier: 1.0,
            },
            |_| Vec::<CollisionBox>::new(),
        )
        .expect("within double radius");
        assert!((hit.dist - 0.5).abs() < 1e-12);
        assert_eq!(hit.exposure, 1.0);
        assert!(
            (hit.damage - entity_damage_amount(4.0, [0.5, 64.0, 0.5], [4.5, 64.0, 0.5], 1.0)).abs()
                < 1e-6
        );
        assert!(hit.knockback[0] > 0.0 && hit.knockback[1] > 0.0);
        assert_eq!(hit.knockback[2], 0.0);

        // Degenerate zero-radius and infinite AABB bounds must not hang or emit NaN.
        assert!(compute_explosion_entity_hit(
            ExplosionHitInput {
                center: [0.5, 64.0, 0.5],
                radius: 0.0,
                entity_position: [0.5, 64.0, 0.5],
                entity_origin: [0.5, 64.0, 0.5],
                bounding_box: CollisionBox::UNIT_CUBE,
                knockback_resistance: 0.0,
                damages_entities: true,
                knockback_multiplier: 1.0,
            },
            |_| Vec::<CollisionBox>::new(),
        )
        .is_none());
        let huge = CollisionBox::from_arrays([f64::NEG_INFINITY; 3], [f64::INFINITY; 3]);
        assert_eq!(
            get_seen_percent([0.0, 0.0, 0.0], huge, |_| Vec::<CollisionBox>::new()),
            0.0
        );
    }
}
