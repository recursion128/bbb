//! The random destination samplers a ground goal draws its walk target from
//! (`RandomPos.java`, `LandRandomPos.java`, `DefaultRandomPos.java`,
//! `GoalUtils.java`). Every entry answers `None` when the level cannot supply
//! a block, fluid, light or tag the sample reads, which refuses the whole draw
//! rather than weighing a position against a guess.

use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_path_facts;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_solid};

use crate::entity::level::{
    block_state_tag_membership, block_state_water_pathfindable, pathfinding_cost_from_light_levels,
};
use crate::entity::natural_spawn::mob_eligibility::{
    mob_walk_target_value, NaturalSpawnRulesProbe,
};

use super::navigation::malus::PathfindingMalus;
use super::navigation::{random_pos_stable_destination, walk_path_type};

// `RandomPos.RANDOM_POS_ATTEMPTS` (`RandomPos.java:16`).
const RANDOM_POS_ATTEMPTS: usize = 10;

/// Everything the samplers read off the mob rather than off the level
/// (`RandomPos.java`, `GoalUtils.java`, and the
/// `getWalkTargetValue` overrides that read the mob's own state).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RandomPosMob {
    pub(crate) position: Vec3d,
    pub(crate) entity_type_id: i32,
    /// `Mob.getHomePosition`/`getHomeRadius`, absent while `hasHome` is false
    /// (`Mob.java`).
    pub(crate) home: Option<(BlockPos, i32)>,
    pub(crate) malus: PathfindingMalus,
    pub(crate) turtle_going_home: bool,
    pub(crate) strider_is_in_lava: bool,
    pub(crate) hoglin_repellent: Option<BlockPos>,
}

impl RandomPosMob {
    pub(crate) fn of(entity: &crate::entity::Entity, malus: PathfindingMalus) -> Self {
        Self {
            position: entity.position(),
            entity_type_id: entity.entity_type_id(),
            home: entity.mob_home(),
            malus,
            turtle_going_home: match entity.data() {
                crate::entity::EntityData::GeneralLiving(data) => data
                    .turtle_retained()
                    .is_some_and(crate::entity::turtle::TurtleRetainedState::going_home),
                _ => false,
            },
            strider_is_in_lava: entity.is_in_lava(),
            hoglin_repellent: crate::entity::hoglin::nearest_repellent(entity),
        }
    }
}

// `LandRandomPos.getPos` (`LandRandomPos.java`).
pub(crate) fn land_random_pos(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    horizontal_dist: i32,
    vertical_dist: i32,
) -> Option<Option<Vec3d>> {
    land_random_pos_scored(level, random, mob, horizontal_dist, vertical_dist, |pos| {
        walk_target_value(level, mob, pos).map(f64::from)
    })
}

pub(super) fn land_random_pos_scored(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    horizontal_dist: i32,
    vertical_dist: i32,
    weight: impl FnMut(BlockPos) -> Option<f64>,
) -> Option<Option<Vec3d>> {
    let restrict = mob_restricted(mob, f64::from(horizontal_dist));
    generate_random_pos_scored(
        level,
        random,
        |level, random| {
            let direction = generate_random_direction(random, horizontal_dist, vertical_dist);
            match land_pos_toward_direction(
                level,
                random,
                mob,
                f64::from(horizontal_dist),
                restrict,
                direction,
            )? {
                Some(pos) => move_pos_up_out_of_solid(level, mob, pos),
                None => Some(None),
            }
        },
        weight,
    )
}

// `DefaultRandomPos.getPos` (`DefaultRandomPos.java`).
pub(super) fn default_random_pos(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    horizontal_dist: i32,
    vertical_dist: i32,
) -> Option<Option<Vec3d>> {
    let restrict = mob_restricted(mob, f64::from(horizontal_dist));
    generate_random_pos(level, random, mob, |level, random| {
        let direction = generate_random_direction(random, horizontal_dist, vertical_dist);
        default_pos_toward_direction(level, random, mob, horizontal_dist, restrict, direction)
    })
}

// `DefaultRandomPos.getPosTowards` (`DefaultRandomPos.java`).
pub(super) fn default_random_pos_towards(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    horizontal_dist: i32,
    vertical_dist: i32,
    towards: Vec3d,
    max_xz_radians: f64,
) -> Option<Option<Vec3d>> {
    let direction_x = towards.x - mob.position.x;
    let direction_z = towards.z - mob.position.z;
    let restrict = mob_restricted(mob, f64::from(horizontal_dist));
    generate_random_pos(level, random, mob, |level, random| {
        let Some(direction) = generate_random_direction_within_radians(
            random,
            0.0,
            f64::from(horizontal_dist),
            vertical_dist,
            0,
            direction_x,
            direction_z,
            max_xz_radians,
        ) else {
            return Some(None);
        };
        default_pos_toward_direction(level, random, mob, horizontal_dist, restrict, direction)
    })
}

// `DefaultRandomPos.getPosAway` (`DefaultRandomPos.java`).
pub(super) fn default_random_pos_away(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    horizontal_dist: i32,
    vertical_dist: i32,
    avoid: Vec3d,
) -> Option<Option<Vec3d>> {
    let towards = Vec3d {
        x: 2.0 * mob.position.x - avoid.x,
        y: mob.position.y,
        z: 2.0 * mob.position.z - avoid.z,
    };
    default_random_pos_towards(
        level,
        random,
        mob,
        horizontal_dist,
        vertical_dist,
        towards,
        f64::from(std::f32::consts::FRAC_PI_2),
    )
}

pub(super) fn land_random_pos_towards(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    horizontal_dist: i32,
    vertical_dist: i32,
    towards: Vec3d,
) -> Option<Option<Vec3d>> {
    land_random_pos_in_direction(
        level,
        random,
        mob,
        0.0,
        f64::from(horizontal_dist),
        vertical_dist,
        [towards.x - mob.position.x, towards.z - mob.position.z],
    )
}

// `LandRandomPos.getPosAway`.
pub(super) fn land_random_pos_away(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    min_horizontal_dist: f64,
    max_horizontal_dist: f64,
    vertical_dist: i32,
    avoid: Vec3d,
) -> Option<Option<Vec3d>> {
    let (mut x_dir, mut z_dir) = (mob.position.x - avoid.x, mob.position.z - avoid.z);
    if x_dir == 0.0 && z_dir == 0.0 && mob.position.y == avoid.y {
        x_dir = random.next_double() - 0.5;
        z_dir = random.next_double() - 0.5;
    }
    land_random_pos_in_direction(
        level,
        random,
        mob,
        min_horizontal_dist,
        max_horizontal_dist,
        vertical_dist,
        [x_dir, z_dir],
    )
}

fn land_random_pos_in_direction(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    min_horizontal_dist: f64,
    max_horizontal_dist: f64,
    vertical_dist: i32,
    [x_dir, z_dir]: [f64; 2],
) -> Option<Option<Vec3d>> {
    let restrict = mob_restricted(mob, max_horizontal_dist);
    generate_random_pos(level, random, mob, |level, random| {
        let Some(direction) = generate_random_direction_within_radians(
            random,
            min_horizontal_dist,
            max_horizontal_dist,
            vertical_dist,
            0,
            x_dir,
            z_dir,
            f64::from(std::f32::consts::FRAC_PI_2),
        ) else {
            return Some(None);
        };
        match land_pos_toward_direction(
            level,
            random,
            mob,
            max_horizontal_dist,
            restrict,
            direction,
        )? {
            Some(pos) => move_pos_up_out_of_solid(level, mob, pos),
            None => Some(None),
        }
    })
}

// `HoverRandomPos.getPos` (`HoverRandomPos.java`).
#[allow(clippy::too_many_arguments)]
pub(super) fn hover_random_pos(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    horizontal_dist: i32,
    vertical_dist: i32,
    x_dir: f64,
    z_dir: f64,
    max_xz_radians: f64,
    hover_max_height: i32,
    hover_min_height: i32,
) -> Option<Option<Vec3d>> {
    let restrict = mob_restricted(mob, f64::from(horizontal_dist));
    generate_random_pos(level, random, mob, |level, random| {
        let Some(direction) = generate_random_direction_within_radians(
            random,
            0.0,
            f64::from(horizontal_dist),
            vertical_dist,
            0,
            x_dir,
            z_dir,
            max_xz_radians,
        ) else {
            return Some(None);
        };
        let Some(pos) = land_pos_toward_direction(
            level,
            random,
            mob,
            f64::from(horizontal_dist),
            restrict,
            direction,
        )?
        else {
            return Some(None);
        };
        let above_solid_amount =
            random.next_int(hover_max_height - hover_min_height + 1) + hover_min_height;
        let pos = move_up_to_above_solid(level, pos, above_solid_amount)?;
        Some((!is_water(level, pos)? && !has_malus(level, mob, pos)?).then_some(pos))
    })
}

// `AirAndWaterRandomPos.getPos` over its `generateRandomPos`
// (`AirAndWaterRandomPos.java`).
#[allow(clippy::too_many_arguments)]
pub(super) fn air_and_water_random_pos(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    horizontal_dist: i32,
    vertical_dist: i32,
    flying_height: i32,
    x_dir: f64,
    z_dir: f64,
    max_xz_radians: f64,
) -> Option<Option<Vec3d>> {
    let restrict = mob_restricted(mob, f64::from(horizontal_dist));
    generate_random_pos(level, random, mob, |level, random| {
        let Some(direction) = generate_random_direction_within_radians(
            random,
            0.0,
            f64::from(horizontal_dist),
            vertical_dist,
            flying_height,
            x_dir,
            z_dir,
            max_xz_radians,
        ) else {
            return Some(None);
        };
        let pos = generate_random_pos_toward_direction(
            random,
            mob,
            f64::from(horizontal_dist),
            direction,
        );
        if is_outside_limits(level, pos) || is_restricted(restrict, mob, pos) {
            return Some(None);
        }
        let pos = move_up_out_of_solid(level, pos)?;
        Some((!has_malus(level, mob, pos)?).then_some(pos))
    })
}

// BehaviorUtils.getRandomSwimmablePos (BehaviorUtils.java) checks
// at most eleven independently sampled DefaultRandomPos results.
pub(super) fn random_swimmable_pos(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    horizontal_dist: i32,
    vertical_dist: i32,
) -> Option<Option<Vec3d>> {
    let mut target = default_random_pos(level, random, mob, horizontal_dist, vertical_dist)?;
    let mut retries = 0;
    while let Some(position) = target {
        let pos = containing_block(position);
        if !level.has_ticking_column_at(pos) {
            return None;
        }
        if block_state_water_pathfindable(level, level.block_state(pos))? || retries >= 10 {
            return Some(Some(position));
        }
        retries += 1;
        target = default_random_pos(level, random, mob, horizontal_dist, vertical_dist)?;
    }
    Some(None)
}

// RandomStroll.getTargetSwimPos (RandomStroll.java).
pub(super) fn swim_target_pos(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
) -> Option<Option<Vec3d>> {
    const TIERS: [(i32, i32); 6] = [(1, 1), (3, 3), (5, 5), (6, 5), (7, 7), (10, 7)];
    let mut fallback: Option<Vec3d> = None;
    for (horizontal, vertical) in TIERS {
        let target = match fallback {
            None => random_swimmable_pos(level, random, mob, horizontal, vertical)?,
            Some(previous) => {
                let dx = previous.x - mob.position.x;
                let dy = previous.y - mob.position.y;
                let dz = previous.z - mob.position.z;
                let length = (dx * dx + dy * dy + dz * dz).sqrt();
                let (unit_x, unit_y, unit_z) = if length < 1.0e-5_f32 as f64 {
                    (0.0, 0.0, 0.0)
                } else {
                    (dx / length, dy / length, dz / length)
                };
                Some(Vec3d {
                    x: mob.position.x + unit_x * f64::from(horizontal),
                    y: mob.position.y + unit_y * f64::from(vertical),
                    z: mob.position.z + unit_z * f64::from(horizontal),
                })
            }
        };
        let Some(position) = target else {
            return Some(fallback);
        };
        let pos = containing_block(position);
        if !level.has_ticking_column_at(pos) {
            return None;
        }
        let state = level.block_state(pos);
        if vanilla_block_state_path_facts(state).is_none() {
            level.content_registry().block_by_state_id(state)?;
        }
        let restrict = mob_restricted(mob, f64::from(horizontal));
        if block_state_fluid_state(state).is_empty() || is_restricted_vec3(restrict, mob, position)
        {
            return Some(fallback);
        }
        fallback = Some(position);
    }
    Some(fallback)
}

fn containing_block(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

// `RandomPos.generateRandomPos(PathfinderMob, Supplier)`
// (`RandomPos.java`): ten attempts, each weighed by the mob's own
// `getWalkTargetValue`, and the first strictly greatest one wins.
fn generate_random_pos(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    supplier: impl FnMut(&LevelCtx<'_, '_>, &GameRandom) -> Option<Option<BlockPos>>,
) -> Option<Option<Vec3d>> {
    generate_random_pos_scored(level, random, supplier, |pos| {
        walk_target_value(level, mob, pos).map(f64::from)
    })
}

fn generate_random_pos_scored(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mut supplier: impl FnMut(&LevelCtx<'_, '_>, &GameRandom) -> Option<Option<BlockPos>>,
    mut weight: impl FnMut(BlockPos) -> Option<f64>,
) -> Option<Option<Vec3d>> {
    let mut best_weight = f64::NEG_INFINITY;
    let mut best_pos = None;
    for _ in 0..RANDOM_POS_ATTEMPTS {
        let Some(pos) = supplier(level, random)? else {
            continue;
        };
        let value = weight(pos)?;
        if value > best_weight {
            best_weight = value;
            best_pos = Some(pos);
        }
    }
    Some(best_pos.map(bottom_center_of))
}

// `RandomPos.generateRandomDirection` (`RandomPos.java`).
fn generate_random_direction(
    random: &GameRandom,
    horizontal_dist: i32,
    vertical_dist: i32,
) -> BlockPos {
    let x = random.next_int(2 * horizontal_dist + 1) - horizontal_dist;
    let y = random.next_int(2 * vertical_dist + 1) - vertical_dist;
    let z = random.next_int(2 * horizontal_dist + 1) - horizontal_dist;
    BlockPos { x, y, z }
}

// `RandomPos.generateRandomDirectionWithinRadians` (`RandomPos.java`).
#[allow(clippy::too_many_arguments)]
fn generate_random_direction_within_radians(
    random: &GameRandom,
    min_horizontal_dist: f64,
    max_horizontal_dist: f64,
    vertical_dist: i32,
    flying_height: i32,
    x_dir: f64,
    z_dir: f64,
    max_xz_radians: f64,
) -> Option<BlockPos> {
    let center = crate::entity::projectile_hit::mth_atan2(z_dir, x_dir)
        - f64::from(std::f32::consts::FRAC_PI_2);
    let angle = center + f64::from(2.0_f32 * random.next_float() - 1.0_f32) * max_xz_radians;
    let distance = (min_horizontal_dist
        + random.next_double().sqrt() * (max_horizontal_dist - min_horizontal_dist))
        * f64::from(2.0_f32.sqrt());
    let x = -distance * angle.sin();
    let z = distance * angle.cos();
    if x.abs() > max_horizontal_dist || z.abs() > max_horizontal_dist {
        return None;
    }
    let y = random.next_int(2 * vertical_dist + 1) - vertical_dist + flying_height;
    Some(BlockPos {
        x: x.floor() as i32,
        y,
        z: z.floor() as i32,
    })
}

// `RandomPos.generateRandomPosTowardDirection` (`RandomPos.java`).
fn generate_random_pos_toward_direction(
    random: &GameRandom,
    mob: RandomPosMob,
    xz_dist: f64,
    direction: BlockPos,
) -> BlockPos {
    let mut x = f64::from(direction.x);
    let mut z = f64::from(direction.z);
    if let Some((center, _)) = mob.home.filter(|_| xz_dist > 1.0) {
        if mob.position.x > f64::from(center.x) {
            x -= random.next_double() * xz_dist / 2.0;
        } else {
            x += random.next_double() * xz_dist / 2.0;
        }
        if mob.position.z > f64::from(center.z) {
            z -= random.next_double() * xz_dist / 2.0;
        } else {
            z += random.next_double() * xz_dist / 2.0;
        }
    }
    BlockPos {
        x: (x + mob.position.x).floor() as i32,
        y: (f64::from(direction.y) + mob.position.y).floor() as i32,
        z: (z + mob.position.z).floor() as i32,
    }
}

// `LandRandomPos.generateRandomPosTowardDirection` (`LandRandomPos.java`).
fn land_pos_toward_direction(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    horizontal_dist: f64,
    restrict: bool,
    direction: BlockPos,
) -> Option<Option<BlockPos>> {
    let pos = generate_random_pos_toward_direction(random, mob, horizontal_dist, direction);
    Some(
        (!is_outside_limits(level, pos)
            && !is_restricted(restrict, mob, pos)
            && !is_not_stable(level, mob, pos)?)
        .then_some(pos),
    )
}

// `DefaultRandomPos.generateRandomPosTowardDirection`
// (`DefaultRandomPos.java`).
fn default_pos_toward_direction(
    level: &LevelCtx<'_, '_>,
    random: &GameRandom,
    mob: RandomPosMob,
    horizontal_dist: i32,
    restrict: bool,
    direction: BlockPos,
) -> Option<Option<BlockPos>> {
    let pos =
        generate_random_pos_toward_direction(random, mob, f64::from(horizontal_dist), direction);
    Some(
        (!is_outside_limits(level, pos)
            && !is_restricted(restrict, mob, pos)
            && !is_not_stable(level, mob, pos)?
            && !has_malus(level, mob, pos)?)
        .then_some(pos),
    )
}

// `LandRandomPos.movePosUpOutOfSolid` (`LandRandomPos.java`).
fn move_pos_up_out_of_solid(
    level: &LevelCtx<'_, '_>,
    mob: RandomPosMob,
    pos: BlockPos,
) -> Option<Option<BlockPos>> {
    let pos = move_up_out_of_solid(level, pos)?;
    Some((!is_water(level, pos)? && !has_malus(level, mob, pos)?).then_some(pos))
}

// `RandomPos.moveUpOutOfSolid` (`RandomPos.java`).
fn move_up_out_of_solid(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<BlockPos> {
    if !is_solid(level, pos)? {
        return Some(pos);
    }
    let max_y = level.max_y();
    let mut on_ground = BlockPos {
        y: pos.y + 1,
        ..pos
    };
    while on_ground.y <= max_y && is_solid(level, on_ground)? {
        on_ground.y += 1;
    }
    Some(on_ground)
}

// `RandomPos.moveUpToAboveSolid` (`RandomPos.java`); callers only pass
// a non-negative amount.
fn move_up_to_above_solid(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    above_solid_amount: i32,
) -> Option<BlockPos> {
    if !is_solid(level, pos)? {
        return Some(pos);
    }
    let max_y = level.max_y();
    let mut moved = BlockPos {
        y: pos.y + 1,
        ..pos
    };
    while moved.y <= max_y && is_solid(level, moved)? {
        moved.y += 1;
    }
    let first_non_solid_y = moved.y;
    while moved.y <= max_y && moved.y - first_non_solid_y < above_solid_amount {
        moved.y += 1;
        if is_solid(level, moved)? {
            moved.y -= 1;
            break;
        }
    }
    Some(moved)
}

// `GoalUtils.mobRestricted` (`GoalUtils.java`).
fn mob_restricted(mob: RandomPosMob, horizontal_dist: f64) -> bool {
    mob.home.is_some_and(|(center, radius)| {
        let distance = f64::from(radius) + horizontal_dist + 1.0;
        dist_to_center_sqr(center, mob.position) < distance * distance
    })
}

// `GoalUtils.isRestricted` over `Mob.isWithinHome(BlockPos)`
// (`GoalUtils.java`, `Mob.java`).
fn is_restricted(restrict: bool, mob: RandomPosMob, pos: BlockPos) -> bool {
    restrict
        && mob.home.is_some_and(|(center, radius)| {
            dist_to_low_corner_sqr(center, pos) >= f64::from(radius) * f64::from(radius)
        })
}

// GoalUtils.isRestricted(boolean, PathfinderMob, Vec3) (GoalUtils.java).
fn is_restricted_vec3(restrict: bool, mob: RandomPosMob, pos: Vec3d) -> bool {
    restrict
        && mob.home.is_some_and(|(center, radius)| {
            dist_to_center_sqr(center, pos) >= f64::from(radius) * f64::from(radius)
        })
}

// `GoalUtils.isOutsideLimits` over `LevelHeightAccessor.isOutsideBuildHeight`
// (`GoalUtils.java`, `LevelHeightAccessor.java`).
fn is_outside_limits(level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    pos.y < level.min_y() || pos.y > level.max_y()
}

// `GoalUtils.isNotStable` over `PathNavigation.isStableDestination`
// (`GoalUtils.java`, `PathNavigation.java`).
fn is_not_stable(level: &LevelCtx<'_, '_>, mob: RandomPosMob, pos: BlockPos) -> Option<bool> {
    Some(!random_pos_stable_destination(
        level,
        mob.entity_type_id,
        pos,
    )?)
}

// `GoalUtils.isSolid` (`GoalUtils.java`).
fn is_solid(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    level
        .has_ticking_column_at(pos)
        .then(|| block_state_is_solid(level.block_state(pos)))
}

// `GoalUtils.isWater` (`GoalUtils.java`).
fn is_water(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<bool> {
    if !level.has_ticking_column_at(pos) {
        return None;
    }
    Some(crate::entity::level::fluid_state_tag_membership(
        level,
        block_state_fluid_state(level.block_state(pos)),
        "minecraft:water",
    ))
}

// `GoalUtils.hasMalus` (`GoalUtils.java`).
fn has_malus(level: &LevelCtx<'_, '_>, mob: RandomPosMob, pos: BlockPos) -> Option<bool> {
    Some(mob.malus.get(walk_path_type(level, pos)?) != 0.0)
}

// `Vec3.atBottomCenterOf` (`Vec3.java`).
fn bottom_center_of(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y),
        z: f64::from(pos.z) + 0.5,
    }
}

// `Vec3i.distToCenterSqr` (`Vec3i.java`).
fn dist_to_center_sqr(pos: BlockPos, position: Vec3d) -> f64 {
    let dx = f64::from(pos.x) + 0.5 - position.x;
    let dy = f64::from(pos.y) + 0.5 - position.y;
    let dz = f64::from(pos.z) + 0.5 - position.z;
    dx * dx + dy * dy + dz * dz
}

// `Vec3i.distSqr(Vec3i)` over `distToLowCornerSqr` (`Vec3i.java`).
fn dist_to_low_corner_sqr(pos: BlockPos, other: BlockPos) -> f64 {
    let dx = f64::from(pos.x - other.x);
    let dy = f64::from(pos.y - other.y);
    let dz = f64::from(pos.z - other.z);
    dx * dx + dy * dy + dz * dz
}

// `PathfinderMob.getWalkTargetValue(BlockPos, LevelReader)`
// (`PathfinderMob.java`) and its overrides, fed from the level at `pos`.
pub(super) fn walk_target_value(
    level: &LevelCtx<'_, '_>,
    mob: RandomPosMob,
    pos: BlockPos,
) -> Option<f32> {
    let below = BlockPos {
        y: pos.y - 1,
        ..pos
    };
    if !level.has_ticking_column_at(pos) || !level.has_ticking_column_at(below) {
        return None;
    }
    let blocks = [level.block_state(pos), level.block_state(below)];
    let cost = pathfinding_cost_from_light_levels(level, pos)?;
    let air_at_and_below =
        if mob.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
            [
                blocks[0],
                blocks[1],
                level.block_state(BlockPos {
                    y: pos.y - 2,
                    ..pos
                }),
            ]
            .map(bbb_server_types::fluid::block_state_is_air)
        } else {
            [false; 3]
        };
    let probe = NaturalSpawnRulesProbe {
        pathfinding_cost_from_light_levels: move || cost,
        block_at: move |depth| blocks[depth],
        below_is_sand: |state| block_state_tag_membership(level, state, "minecraft:sand"),
        air_at_and_below,
        turtle_going_home: mob.turtle_going_home,
        strider_is_in_lava: mob.strider_is_in_lava,
        // `HoglinAi.isPosNearNearestRepellent` (`HoglinAi.java:165-168`).
        hoglin_near_repellent: mob.hoglin_repellent.is_some_and(|repellent| {
            let dx = f64::from(repellent.x - pos.x);
            let dy = f64::from(repellent.y - pos.y);
            let dz = f64::from(repellent.z - pos.z);
            dx * dx + dy * dy + dz * dz < 64.0
        }),
    };
    mob_walk_target_value(mob.entity_type_id, probe).ok()
}

#[cfg(test)]
pub(super) mod tests;
