use super::*;
use crate::block_entity::sculk_shrieker::{
    state_with_shrieking, LEVEL_EVENT_PARTICLES_SCULK_SHRIEK, SHRIEKING_TICKS, UPDATE_CLIENTS,
};
use crate::entity::level::SculkShriekerStepSource;
use crate::entity::warden_spawn_tracker::{
    WardenSpawnTracker, PLAYER_SEARCH_RADIUS, WARNING_CHECK_DIAMETER,
};
use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SCULK_SHRIEKER_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID;
use bbb_protocol::game_types::GameType;

impl LevelCtx<'_, '_> {
    pub(super) fn sculk_shrieker_step(
        &mut self,
        pos: BlockPos,
        source: SculkShriekerStepSource,
    ) -> Option<()> {
        if self
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::sculk_shrieker)
            .is_none()
        {
            return (self.chunks.block_entities().loaned_type(pos)
                != Some(VANILLA_BLOCK_ENTITY_TYPE_SCULK_SHRIEKER_ID))
            .then_some(());
        }
        crate::block_entity::sculk_shrieker::SculkShriekerBlockEntity::try_shriek_in_place(
            pos,
            self,
            source.player.uuid,
            Some(source.player.entity_id),
            source.warden_nearby,
        )
    }

    pub(crate) fn set_sculk_shrieker_warning_level(&mut self, pos: BlockPos, warning_level: i32) {
        if let Some(shrieker) = self
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::sculk_shrieker_mut)
        {
            shrieker.set_warning_level(warning_level);
        }
    }

    pub(crate) fn sculk_shrieker_state(&self, pos: BlockPos) -> Option<i32> {
        self.chunks.get_block_state(pos.x, pos.y, pos.z)
    }

    pub(crate) fn sculk_shrieker_can_respond(&self) -> Option<bool> {
        if !self.game_rules.spawn_wardens {
            return Some(false);
        }
        Some(self.sculk_warning.as_ref()?.difficulty != Difficulty::Peaceful)
    }

    pub(in crate::world) fn sculk_shrieker_warden_nearby(&self, pos: BlockPos) -> Option<bool> {
        let context = self.structure_entities.as_ref()?;
        let origin = crate::block_entity::game_event::block_pos_center(pos);
        let bounds = bbb_server_types::geometry::Aabb::of_size(
            origin,
            WARNING_CHECK_DIAMETER,
            WARNING_CHECK_DIAMETER,
            WARNING_CHECK_DIAMETER,
        );
        Some(
            context
                .store
                .ids_in_box_sections(bounds)
                .into_iter()
                .any(|id| {
                    context.store.get(id).is_some_and(|entity| {
                        entity.entity_type_id() == VANILLA_ENTITY_TYPE_WARDEN_ID
                            && entity.is_alive()
                            && entity.bounding_box().intersects(bounds)
                    })
                }),
        )
    }

    pub(crate) fn try_sculk_shrieker_warning(
        &mut self,
        pos: BlockPos,
        player: Uuid,
        warden_nearby: bool,
    ) -> Option<i32> {
        if warden_nearby {
            return None;
        }
        let origin = crate::block_entity::game_event::block_pos_center(pos);
        let store = &mut *self.structure_entities.as_mut()?.store;
        let mut players: Vec<Uuid> = store
            .players()
            .filter(|session| {
                !session.avatar_removed
                    && session.game_type != GameType::Spectator
                    && !session.damage.is_dead_or_dying()
                    && session.damage.health > 0.0
            })
            .filter(|session| {
                let dx = session.position().x - origin.x;
                let dy = session.position().y - origin.y;
                let dz = session.position().z - origin.z;
                dx * dx + dy * dy + dz * dz < PLAYER_SEARCH_RADIUS * PLAYER_SEARCH_RADIUS
            })
            .map(|session| session.profile.uuid)
            .collect();
        if !players.contains(&player) {
            players.push(player);
        }
        let warning = WardenSpawnTracker::warning_from_trackers(players.iter().map(|uuid| {
            store
                .player_by_uuid(*uuid)
                .map(|session| session.cold.warden_tracker)
                .unwrap_or_default()
        }))?;
        for session in store
            .players_mut()
            .filter(|session| players.contains(&session.profile.uuid))
        {
            session.cold.warden_tracker = warning;
        }
        Some(warning.warning_level())
    }

    pub(crate) fn shriek(&mut self, pos: BlockPos, state: i32, source_entity_id: Option<i32>) {
        if let Some(shrieking) = state_with_shrieking(state, true) {
            self.set_block(pos, shrieking, UPDATE_CLIENTS, UPDATE_LIMIT);
        }
        if let Some(block) = BlockTickType::of_block_state(self.block_state(pos)) {
            self.schedule_block_tick(pos, block, SHRIEKING_TICKS, TickPriority::Normal);
        }
        self.event_sink
            .level_event(LEVEL_EVENT_PARTICLES_SCULK_SHRIEK, pos, 0);
        self.event_sink.game_event_with_source(
            VanillaGameEvent::SHRIEK,
            pos,
            source_entity_id,
            None,
        );
    }
}
