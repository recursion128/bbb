use super::generation_queue::{Commit, COMMIT_BUDGET};
use super::{ChunkLifecycle, FullChunkStatus};
use crate::chunk_sender::ChunkKey;
use crate::entity::EntityVisibility;
use crate::world::LevelId;
use crate::world::ServerWorld;
use bbb_protocol::packets::BlockPos;
use bbb_server_persistence::Java26ChunkStatus;
use bbb_server_types::save::SaveResult;
use std::sync::Arc;

/// The reach of a FULL column's generation, which the resident spawn column
/// keeps loaded with it.
const SPAWN_RESIDENT_RADIUS: u32 = 3;

impl ServerWorld {
    pub(in crate::world) fn refresh_loading_chunks(
        &mut self,
        dimension: &LevelId,
    ) -> SaveResult<()> {
        self.refresh_loading_chunks_waiting(dimension, !self.cold.config.background_generation)
    }

    pub(in crate::world) fn refresh_loading_chunks_waiting(
        &mut self,
        dimension: &LevelId,
        wait: bool,
    ) -> SaveResult<()> {
        let view = self.admin.settings().view_distance;
        let level = self.level_by_id(dimension);
        let mut sources = level
            .saved_chunk_tickets
            .iter()
            .map(|ticket| (ticket.chunk, ticket.chunk, ticket.level))
            .chain(
                level
                    .chunk_lifecycle
                    .ender_pearl_sources()
                    .map(|(chunk, level)| (chunk, chunk, level)),
            )
            .collect::<Vec<_>>();
        if let Some((chunk, ticket_level)) = level
            .dragon_fight
            .as_ref()
            .and_then(|fight| fight.chunk_ticket())
        {
            sources.push((chunk, chunk, ticket_level));
        }
        // DistanceManager.java:257-284: server view distance files PLAYER_LOADING at level 31.
        for (x, z) in level.player_chunk_membership.centres() {
            sources.push(((x - view, z - view), (x + view, z + view), 31));
        }
        let level = self.level_mut_by_id(dimension);
        let sources_changed = level.chunk_lifecycle.rebuild(sources);
        let settled_key = (
            level.chunk_lifecycle.resident_spawn_column,
            level.chunks.column_count(),
        );
        if !wait
            && !sources_changed
            && level.chunk_lifecycle.generation.is_empty()
            && level.chunk_lifecycle.settled == Some(settled_key)
        {
            return Ok(());
        }
        // A world that waits for its generation waits for its reads too.
        if wait {
            self.read_pending_columns_now(dimension);
        }
        let level = self.level_by_id(dimension);
        let desired = Arc::clone(&level.chunk_lifecycle.loading_levels);
        let mut known = level.chunks.loaded_chunks().collect::<Vec<_>>();
        known.extend(level.chunk_lifecycle.confirmed.keys().copied());
        known.sort_unstable();
        known.dedup();
        for chunk in known {
            let next = FullChunkStatus::from_level(desired.get(&chunk).copied().unwrap_or(45));
            if next < self.level_by_id(dimension).chunk_lifecycle.status(chunk) {
                self.confirm_chunk_status(dimension, chunk, next);
            }
            let level = self.level_mut_by_id(dimension);
            if !desired.contains_key(&chunk)
                && !level.chunks.is_old_generation(chunk)
                && !level
                    .chunk_lifecycle
                    .resident_spawn_column
                    .is_some_and(|(x, z)| {
                        chunk.0.abs_diff(x) <= SPAWN_RESIDENT_RADIUS
                            && chunk.1.abs_diff(z) <= SPAWN_RESIDENT_RADIUS
                    })
            {
                level.chunk_lifecycle.column_unloads.insert(chunk);
            }
        }
        let level = self.level_by_id(dimension);
        // Dependencies must observe the saved column before any generator
        // publishes its replacement; a read settles within a tick either way.
        let evicted = &level.chunk_lifecycle.evicted_columns;
        let has_desired_evicted = if desired.len() <= evicted.len() {
            desired.keys().any(|chunk| evicted.contains(chunk))
        } else {
            evicted.iter().any(|chunk| desired.contains_key(chunk))
        };
        if !has_desired_evicted
            && !(level.chunks.has_stored_unread()
                && desired
                    .keys()
                    .any(|&chunk| level.chunks.is_stored_unread(chunk)))
        {
            let generation = desired
                .iter()
                .filter_map(|(&chunk, &level)| {
                    ChunkLifecycle::generation_status(level).map(|status| (chunk, status))
                })
                .collect::<Vec<_>>();
            self.queue_generation(dimension, generation);
            let commit = if wait {
                Commit::Wait(usize::MAX)
            } else {
                Commit::Until(std::time::Instant::now() + COMMIT_BUDGET)
            };
            self.commit_generation(dimension, commit)?;
        }
        let mut accessible = desired
            .iter()
            .filter_map(|(&chunk, &ticket_level)| {
                let next = FullChunkStatus::from_level(ticket_level);
                (next != FullChunkStatus::Inaccessible).then_some((chunk, next))
            })
            .collect::<Vec<_>>();
        accessible.sort_unstable_by_key(|&(chunk, _)| chunk);
        for (chunk, next) in accessible {
            if self.level_by_id(dimension).chunk_lifecycle.status(chunk) < FullChunkStatus::Full
                && self.accessible_chunk_prepared(dimension, chunk)
            {
                self.register_chunk_tick_containers_by_key(dimension, chunk);
                self.confirm_chunk_status(dimension, chunk, FullChunkStatus::Full);
            }
            // ChunkMap.java:677-710: postprocessing precedes BLOCK_TICKING confirmation.
            if next >= FullChunkStatus::BlockTicking
                && self.level_by_id(dimension).chunk_lifecycle.status(chunk)
                    == FullChunkStatus::Full
                && self.full_chunk_range_prepared(dimension, chunk, 1)
            {
                self.postprocess_chunk(dimension, chunk);
                self.confirm_chunk_status(dimension, chunk, FullChunkStatus::BlockTicking);
            }
            // ChunkMap.java:361-364: ENTITY_TICKING requires radius two FULL.
            if next == FullChunkStatus::EntityTicking
                && self.level_by_id(dimension).chunk_lifecycle.status(chunk)
                    == FullChunkStatus::BlockTicking
                && self.full_chunk_range_prepared(dimension, chunk, 2)
            {
                self.confirm_chunk_status(dimension, chunk, FullChunkStatus::EntityTicking);
            }
        }
        if wait {
            self.read_pending_entities_now(dimension);
        }
        let level = self.level_mut_by_id(dimension);
        let all_settled = level.chunk_lifecycle.generation.is_empty()
            && desired.iter().all(|(&chunk, &ticket_level)| {
                ChunkLifecycle::generation_status(ticket_level).is_none_or(|required| {
                    level
                        .chunks
                        .generation_status(chunk.0, chunk.1)
                        .is_some_and(|status| status >= required)
                }) && level.chunk_lifecycle.status(chunk)
                    == FullChunkStatus::from_level(ticket_level)
            });
        level.chunk_lifecycle.settled = all_settled.then_some((
            level.chunk_lifecycle.resident_spawn_column,
            level.chunks.column_count(),
        ));
        Ok(())
    }

    fn accessible_chunk_prepared(&self, dimension: &LevelId, (x, z): ChunkKey) -> bool {
        let chunks = &self.level_by_id(dimension).chunks;
        (-1_i32..=1).all(|dx| {
            (-1_i32..=1).all(|dz| {
                let required = if dx == 0 && dz == 0 {
                    Java26ChunkStatus::Full
                } else {
                    Java26ChunkStatus::InitializeLight
                };
                chunks
                    .generation_status(x + dx, z + dz)
                    .is_some_and(|status| status >= required)
            })
        })
    }

    fn full_chunk_range_prepared(
        &self,
        dimension: &LevelId,
        (x, z): ChunkKey,
        radius: i32,
    ) -> bool {
        let chunks = &self.level_by_id(dimension).chunks;
        (-radius..=radius)
            .all(|dx| (-radius..=radius).all(|dz| chunks.is_ticking_full(x + dx, z + dz)))
    }

    fn confirm_chunk_status(
        &mut self,
        dimension: &LevelId,
        chunk: ChunkKey,
        status: FullChunkStatus,
    ) {
        let level = self.level_mut_by_id(dimension);
        if level.chunk_lifecycle.status(chunk) == status {
            return;
        }
        level.chunk_lifecycle.confirmed.insert(chunk, status);
        let visibility = match status {
            FullChunkStatus::Inaccessible => EntityVisibility::Hidden,
            FullChunkStatus::Full | FullChunkStatus::BlockTicking => EntityVisibility::Tracked,
            FullChunkStatus::EntityTicking => EntityVisibility::Ticking,
        };
        level.entities.apply_chunk_visibility(chunk, visibility);
        if visibility == EntityVisibility::Hidden {
            level.chunk_lifecycle.entity_unloads.insert(chunk);
        } else {
            level.chunk_lifecycle.entity_unloads.remove(&chunk);
            level.chunk_lifecycle.request_entities(chunk);
        }
    }

    // LevelChunk.java:562-590: use the original state for the second arm after the fluid tick.
    fn postprocess_chunk(&mut self, dimension: &LevelId, (x, z): ChunkKey) {
        let positions = self
            .level_by_id(dimension)
            .chunks
            .postprocessing_positions(x, z);
        self.with_level(dimension, |level| {
            for (x, y, z) in positions {
                let pos = BlockPos { x, y, z };
                let state = level.block_state(pos);
                let fluid = bbb_server_types::fluid::block_state_fluid_state(state);
                if fluid.family().is_some() {
                    crate::fluid::tick_fluid(level, pos, fluid.tick_type());
                }
                if crate::fluid::liquid_block_family(state).is_none() {
                    let updated = crate::world::simulation_level::update_from_neighbour_shapes(
                        level, state, pos,
                    );
                    if updated != state {
                        level.set_block(pos, updated, 276, crate::world::UPDATE_LIMIT);
                    }
                }
            }
        });
        self.level_mut_by_id(dimension)
            .chunks
            .clear_postprocessing_positions(x, z);
    }
}
