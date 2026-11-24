//! `ServerChunkCache.tickChunks`'s per-column random tick.

use super::*;
use crate::world::LevelId;

impl ServerWorld {
    #[cfg(test)]
    pub(crate) fn run_live_random_tick_sample_for_test(
        &mut self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
        pos: BlockPos,
        state: i32,
    ) {
        let dimension_id = super::LevelId::from(dimension);
        let infiniburn_blocks = self.level_infiniburn_blocks(&dimension_id);
        let players = self.nearby_players_by_key(&dimension_id);
        let difficulty = self.admin.settings().difficulty;
        let environment = fire::FireFacts {
            infiniburn_blocks: &infiniburn_blocks,
            players: &players,
            fire_spread_radius: self.game_rules.fire_spread_radius_around_player,
            difficulty_id: difficulty.id(),
            spawning_monsters: self.game_rules.spawning_monsters(difficulty),
            raining: self.level_by_id(&dimension_id).weather.is_raining(),
        };
        self.with_level(dimension, |level| {
            run_live_random_tick_sample(level, &environment, pos, state);
        });
    }

    /// `ServerLevel.tickChunk` (`ServerLevel.java:482-525`): the
    /// precipitation draws, then the section random ticks.
    pub(super) fn tick_random_ticks(
        &mut self,
        dimension: &LevelId,
        ticking: &[ChunkKey],
        events: &mut Vec<WorldEvent>,
    ) {
        let tick_speed = self.game_rules.random_tick_speed;
        if tick_speed <= 0 || ticking.is_empty() {
            return;
        }
        let infiniburn_blocks = self.level_infiniburn_blocks(dimension);
        let difficulty = self.admin.settings().difficulty;
        let difficulty_id = difficulty.id();
        let game_rules = self.game_rules;
        let spawning_monsters = game_rules.spawning_monsters(difficulty);
        // Only a push-up moves a player between columns.
        let mut players = self.nearby_players_by_key(dimension);
        let prev_handling = self
            .level_mut_by_id(dimension)
            .block_events
            .set_handling_tick(true);
        let mut chunk_index = 0;
        while chunk_index < ticking.len() {
            let mut leftover = PendingEntitySpawns::default();
            self.with_level_ctx(dimension, &players, &mut leftover, |slice, rest| {
                let environment = fire::FireFacts {
                    infiniburn_blocks: &infiniburn_blocks,
                    players: &players,
                    fire_spread_radius: game_rules.fire_spread_radius_around_player,
                    difficulty_id,
                    spawning_monsters,
                    raining: slice.weather.is_raining(),
                };
                while chunk_index < ticking.len() {
                    let (chunk_x, chunk_z) = ticking[chunk_index];
                    chunk_index += 1;
                    if slice.chunks.column(chunk_x, chunk_z).is_none() {
                        continue;
                    }
                    let mut sampler = RandomTickSampler::new(chunk_x, chunk_z, tick_speed);
                    // A level without biome climates can neither freeze nor
                    // receive precipitation, so it draws nothing.
                    for _ in 0..tick_speed {
                        if !slice.weather.has_biome_climates() {
                            break;
                        }
                        if slice.random.next_int(48) == 0 {
                            let x = chunk_x * 16 + slice.random.next_int(16);
                            let z = chunk_z * 16 + slice.random.next_int(16);
                            let mut level = slice.with_structure_entities(rest.entities, None);
                            super::precipitation::tick_precipitation(&mut level, x, z);
                        }
                    }
                    loop {
                        let next_sample = {
                            let Some(column) = slice.chunks.column(chunk_x, chunk_z) else {
                                break;
                            };
                            sampler.next_position(column, slice.random).map(|pos| {
                                let state = column
                                    .get_block_state(pos.x & 15, pos.y, pos.z & 15)
                                    .unwrap_or(0);
                                (pos, state)
                            })
                        };
                        let Some((pos, state)) = next_sample else {
                            break;
                        };
                        let mut level = slice.with_structure_entities(rest.entities, None);
                        run_live_random_tick_sample(&mut level, &environment, pos, state);
                    }
                    if slice.event_sink.has_pending_game_events() || slice.item_spawns.has_pending()
                    {
                        break;
                    }
                }
            });
            // Flush game events, push-ups, and spawns produced by this batch before resuming.
            self.post_pending_game_events(dimension);
            let push_ups = std::mem::take(&mut leftover.entity_push_ups);
            if !push_ups.is_empty() {
                for (old_state, new_state, pos) in push_ups {
                    let _ =
                        self.push_live_entities_up(dimension, old_state, new_state, pos, events);
                }
                players = self.nearby_players_by_key(dimension);
            }
            self.apply_pending_spawns(dimension, leftover);
        }
        self.level_mut_by_id(dimension)
            .block_events
            .set_handling_tick(prev_handling);
    }
}
