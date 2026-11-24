//! The room checks an item that places an entity runs before the level takes
//! it: block collisions under a box and the entities already inside it.

use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::geometry::{collect_block_collisions_with_moving_pistons, Aabb};

use super::simulation_level::pending_spawns::PendingEntitySpawns;
use super::simulation_level::LevelCtx;
use super::{LevelId, ServerWorld};
use crate::entity::store::EntityStore;
use crate::entity::Entity;

/// The block collision boxes `CollisionGetter.getBlockCollisions` finds in
/// `query`, moving pistons included.
pub(super) fn block_collisions(
    world: &mut ServerWorld,
    dimension: impl Into<LevelId>,
    query: Aabb,
) -> Vec<(BlockPos, Aabb)> {
    let dimension = dimension.into();
    let world = std::cell::RefCell::new(world);
    collect_block_collisions_with_moving_pistons(
        query,
        |pos| {
            let mut world = world.borrow_mut();
            let _ = world.block_state(dimension, pos);
            super::movement_collision::moving_piston_at(&world.level_by_id(&dimension).chunks, pos)
                .unwrap_or(true)
        },
        |pos| {
            let mut world = world.borrow_mut();
            let state = world
                .block_state(dimension, pos)
                .unwrap_or_else(|| world.level_by_id(&dimension).chunks.air_state_id());
            super::movement_collision::static_collision_shape_with_live_piston(
                &world.level_by_id(&dimension).chunks,
                pos,
                state,
            )
        },
    )
}

/// Whether a block collision box intersects `area` (`noBlockCollision`).
pub(super) fn has_block_collision(
    world: &mut ServerWorld,
    dimension: impl Into<LevelId>,
    area: Aabb,
) -> bool {
    block_collisions(world, dimension, area)
        .into_iter()
        .any(|(_, shape)| shape.intersects(area))
}

/// `EntityGetter.getEntities(null, area)` is non-empty for the stored entities
/// `stored` accepts, or any non-spectator player (`NO_SPECTATORS`,
/// `EntityGetter.java:29-30`).
pub(super) fn any_entity_in(
    world: &ServerWorld,
    dimension: impl Into<LevelId>,
    area: Aabb,
    stored: impl Fn(&Entity) -> bool,
) -> bool {
    let dimension = dimension.into();
    let entities = &world.level_by_id(&dimension).entities;
    let stored_hit = entities.first_entity_in_box(area, stored).is_some();
    stored_hit
        || world.levels.players().any(|player| {
            player.dimension == dimension
                && player.game_type != GameType::Spectator
                && player
                    .bounding_box()
                    .is_ok_and(|box_| box_.intersects(area))
        })
}

impl ServerWorld {
    /// Runs `body` over one level and its entity store, for a check that reads
    /// both before an entity is added.
    pub(super) fn with_level_and_entities<R>(
        &mut self,
        dimension: impl Into<LevelId>,
        body: impl FnOnce(&mut LevelCtx<'_, '_>, &mut EntityStore) -> R,
    ) -> R {
        let dimension = dimension.into();
        let players = self.nearby_players_by_key(&dimension);
        let mut leftover = PendingEntitySpawns::default();
        let answer = self.with_level_ctx(&dimension, &players, &mut leftover, |level, rest| {
            body(level, rest.entities)
        });
        self.apply_pending_spawns(&dimension, leftover);
        answer
    }
}
