use super::test_fixture::CommandTestWorld;
use bbb_server_persistence::Java26WeatherData;
use bbb_server_types::interaction::WorldBorder;
use bbb_server_types::save::Java26LevelKey;

impl CommandTestWorld {
    pub(crate) fn with_tick_samples(
        mut self,
        samples: &[i64; crate::world::TICK_STATS_SPAN],
    ) -> Self {
        self.world.cold.tick_times_nanos = *samples;
        self.world.cold.aggregated_tick_times_nanos = samples.iter().sum();
        self.world.tick = crate::world::TICK_STATS_SPAN as u64;
        self
    }

    pub(crate) fn with_border(mut self, level: Java26LevelKey, border: WorldBorder) -> Self {
        self.world
            .level_mut_by_id(&self.world.require_level_id(&level))
            .world_border = border;
        self
    }

    pub(crate) fn border(&self, level: Java26LevelKey) -> WorldBorder {
        self.world
            .level_by_id(&self.world.require_level_id(&level))
            .world_border
    }

    pub(crate) fn weather_data(&self) -> Java26WeatherData {
        self.world.weather_data()
    }
    pub(crate) fn forced_chunks(
        &self,
        level: Java26LevelKey,
    ) -> Vec<bbb_protocol::packets::ChunkPos> {
        self.world
            .forced_chunks_by_key(&self.world.require_level_id(&level))
    }

    pub(crate) fn random_sequence_keys(&self) -> Vec<String> {
        self.world
            .cold
            .random_sequences
            .borrow()
            .entries()
            .map(|(key, _)| key.to_string())
            .collect()
    }

    pub(crate) fn random_sequence_defaults(&self) -> (i32, bool, bool) {
        (
            self.world.cold.random_sequences.borrow().salt(),
            self.world
                .cold
                .random_sequences
                .borrow()
                .include_world_seed(),
            self.world
                .cold
                .random_sequences
                .borrow()
                .include_sequence_id(),
        )
    }
    pub(crate) fn biome_name(&self, pos: bbb_protocol::packets::BlockPos) -> Option<String> {
        let id = self
            .world
            .level_by_id(&self.world.require_level_id(&self.origin.level))
            .chunks
            .biome_id_at(pos.x, pos.y, pos.z)?;
        self.world
            .registries
            .registry_element_id("minecraft:worldgen/biome", id)
            .map(str::to_string)
    }

    pub(crate) fn block_state_at(&self, pos: bbb_protocol::packets::BlockPos) -> Option<i32> {
        self.world
            .level_by_id(&self.world.require_level_id(&self.origin.level))
            .chunks
            .get_block_state(pos.x, pos.y, pos.z)
    }
}
