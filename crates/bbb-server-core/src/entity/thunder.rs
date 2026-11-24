//! Chunk lightning strikes and skeleton horse spawning.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Aabb;

use crate::entity::{Entity, EntityStore};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThunderConditions {
    pub raining: bool,
    pub thundering: bool,
    pub spawn_mobs: bool,
}

/// What one thunder roll filed in the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThunderStrike {
    pub position: BlockPos,
    pub bolt_entity_id: Option<i32>,
    pub trap_horse_entity_id: Option<i32>,
}

///
/// The level random is drawn exactly where vanilla draws it, including on the
/// branches that then decline: the `1/100000` roll (`:534`) happens on every
/// raining-and-thundering chunk, the block position (`:535`) and the target
/// pick (`:618`) are taken before `isRainingAt` (`:536`) can reject the strike,
/// and the trap roll (`:539`) is taken before the lightning-rod test beside it
/// (`:540`) can reject the trap.
/// Runs the post-roll body of `ServerLevel.tickThunder` (`ServerLevel.java:529-549`)
/// after the caller has already checked `raining && thundering` and won
/// `next_int(100_000) == 0`.
pub(crate) fn tick_thunder_after_roll(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    chunk_x: i32,
    chunk_z: i32,
    conditions: ThunderConditions,
    mut difficulty_at: impl FnMut(&LevelCtx<'_, '_>, BlockPos) -> Option<f32>,
) -> Option<ThunderStrike> {
    // `:530-531`, `:535`.
    let min_x = chunk_x << 4;
    let min_z = chunk_z << 4;
    let random_pos =
        crate::random_ticks::random_block_pos(level.level_random(), min_x, 0, min_z, 15);
    let pos = find_lightning_target_around(store, level, random_pos)?;
    // `:536`.
    if !level.is_raining_at(pos) {
        return None;
    }
    let effective_difficulty = difficulty_at(level, pos)?;
    let is_trap = conditions.spawn_mobs
        && level.level_random().next_double() < f64::from(effective_difficulty) * 0.01
        && !crate::block_behavior::lightning_rod::is_lightning_rod(level.block_state(BlockPos {
            x: pos.x,
            y: pos.y - 1,
            z: pos.z,
        }));
    let trap_horse_entity_id = if is_trap {
        spawn_trap_horse(store, pos)
    } else {
        None
    };
    // `:551-556`; `Vec3.atBottomCenterOf` centres on the block's floor.
    let bolt_entity_id = crate::entity::lightning_bolt::spawn_lightning_bolt(
        store,
        Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y),
            z: f64::from(pos.z) + 0.5,
        },
        is_trap,
        None,
    );
    Some(ThunderStrike {
        position: pos,
        bolt_entity_id,
        trap_horse_entity_id,
    })
}

fn spawn_trap_horse(store: &mut EntityStore, pos: BlockPos) -> Option<i32> {
    let id = store.spawn_general_living(
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
        Vec3d {
            x: f64::from(pos.x),
            y: f64::from(pos.y),
            z: f64::from(pos.z),
        },
    )?;
    let entity = store.get_mut(id).expect("the horse was just spawned");
    if !crate::entity::skeleton_horse::set_state(
        entity,
        crate::entity::skeleton_horse::SkeletonHorseTrapState::trap(),
    ) {
        debug_assert!(false, "a freshly spawned skeleton horse carries trap state");
    }
    Some(id)
}

fn find_lightning_target_around(
    store: &EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
) -> Option<BlockPos> {
    let mut center = BlockPos {
        x: pos.x,
        y: level.motion_blocking_height(pos.x, pos.z)?,
        z: pos.z,
    };
    if let Some(rod_target) = level.find_lightning_rod(center) {
        return Some(rod_target);
    }
    let top = level.max_y() + 1;
    let search = Aabb {
        min_x: f64::from(center.x),
        min_y: f64::from(center.y.min(top)),
        min_z: f64::from(center.z),
        max_x: f64::from(center.x + 1),
        max_y: f64::from(center.y.max(top) + 1),
        max_z: f64::from(center.z + 1),
    }
    .inflate(3.0, 3.0, 3.0);
    let targets = lightning_targets(store, level, search);
    // `:617-619`.
    if !targets.is_empty() {
        let bound =
            i32::try_from(targets.len()).expect("a search box holds fewer than 2^31 targets");
        return targets
            .get(level.level_random().next_int(bound) as usize)
            .copied();
    }
    // `:621-623`; an empty column answers `getMinY() - 1`.
    if center.y == level.min_y() - 1 {
        center.y += 2;
    }
    Some(center)
}

///
/// Sessions are `LivingEntity` too and vanilla's selector does not exclude
/// spectators, so both halves are gathered. Vanilla's list order is its entity
/// section iteration order, which this store does not reproduce; the index the
/// level random picks (`:618`) is therefore drawn over the same membership in a
/// different order.
fn lightning_targets(store: &EntityStore, level: &LevelCtx<'_, '_>, search: Aabb) -> Vec<BlockPos> {
    let mut targets: Vec<BlockPos> = store
        .ids_in_box_sections(search)
        .into_iter()
        .filter_map(|id| store.get(id))
        .filter(|entity| {
            entity
                .living()
                .is_some_and(|living| entity.is_alive() && living.health() > 0.0)
                && entity.bounding_box().intersects(search)
                && level.can_see_sky(entity.block_position())
        })
        .map(Entity::block_position)
        .collect();
    targets.extend(
        level
            .players_in_box(search)
            .filter(|player| !player.dead_or_dying)
            .map(|player| BlockPos {
                x: player.position.x.floor() as i32,
                y: player.position.y.floor() as i32,
                z: player.position.z.floor() as i32,
            })
            .filter(|pos| level.can_see_sky(*pos)),
    );
    targets
}

#[cfg(test)]
mod tests;
