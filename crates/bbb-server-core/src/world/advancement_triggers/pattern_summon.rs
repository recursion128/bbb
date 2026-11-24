use crate::world::{LevelId, ServerWorld, WorldEvent};

impl ServerWorld {
    // CarvedPumpkinBlock.java:107-116; WitherSkullBlock.java:72-74.
    pub(in crate::world) fn award_pattern_summon(
        &mut self,
        dimension: &LevelId,
        entity_id: i32,
        range: f64,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(entity) = self.level_by_id(dimension).entities.get(entity_id) else {
            return;
        };
        let bounds = entity.bounding_box().inflate(range, range, range);
        let players: Vec<_> = self
            .levels
            .player_entries()
            .filter(|(_, player)| {
                &player.dimension == dimension
                    && !player.avatar_removed
                    && bbb_server_types::interaction::player_pose_bounding_box(
                        player.position(),
                        player.pose_id,
                        player.pose_scale(),
                    )
                    .is_ok_and(|body| body.intersects(bounds))
            })
            .map(|(&id, _)| id)
            .collect();
        for player in players {
            self.trigger_summoned_entity(player, *dimension, entity_id, events);
        }
    }
}
