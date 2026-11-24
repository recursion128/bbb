use super::*;

pub(super) fn drain_ground_navigation_updates(
    entities: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    let positions = level.chunks.drain_navigation_updates();
    if !positions.is_empty() {
        entities.recompute_ground_paths_affected_by(level, &positions);
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn move_witch_ground_to_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
        target: BlockPos,
        reach_range: i32,
        speed_modifier: f64,
    ) -> bool {
        let block_interaction_policy = self.block_interaction_policy(&dimension.into());
        let mut leftover = PendingEntitySpawns::default();
        self.with_stored_entity_level(
            &dimension.into(),
            &block_interaction_policy,
            None,
            &[],
            &mut leftover,
            |entities, level, _| {
                entities.move_witch_ground_to(entity_id, level, target, reach_range, speed_modifier)
            },
        )
    }

    pub(crate) fn ground_path_for_test(
        &self,
        dimension: Java26BuiltinDimension,
        entity_id: i32,
    ) -> Option<crate::entity::GroundPath> {
        self.level(dimension)
            .entities
            .ground_path(entity_id)
            .cloned()
    }
}

#[cfg(test)]
mod patrol_tests;
