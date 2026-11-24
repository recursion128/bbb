use crate::chunk::ChunkStore;
#[cfg(test)]
use crate::world::LevelId;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::{PlayerRotation, Vec3d};
use bbb_server_datapack::debug_generator::DebugLevelSource;
use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};
use bbb_server_types::{Result, ServerWorldError};

use crate::registry_sync::SynchronizedRegistries;

use super::{
    spawn::SpawnFacts,
    world_clock::{strict_next_noon, OVERWORLD_CLOCK, WORLD_CLOCK_REGISTRY},
    ServerLevel, ServerWorld, WorldSpawn, BIOME_REGISTRY,
};

const PLAINS_BIOME: &str = "minecraft:plains";
const ADVANCE_WEATHER_RULE: &str = "minecraft:advance_weather";

pub(super) fn debug_chunks(
    source: DebugLevelSource,
    registries: &SynchronizedRegistries,
    facts: SpawnFacts,
) -> Result<ChunkStore> {
    let biome_registry_len = registries.registry_len(BIOME_REGISTRY).ok_or_else(|| {
        ServerWorldError::InvalidConfiguration(
            "the synchronized catalog contains no biome registry".to_string(),
        )
    })?;
    let plains_id = registries
        .require_registry_element_index(BIOME_REGISTRY, PLAINS_BIOME)
        .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))?;
    Ok(ChunkStore::new_debug_with_biome_registry_and_skylight(
        source,
        plains_id,
        biome_registry_len,
        facts.has_skylight,
    ))
}

pub(super) fn install_global_debug_view(
    levels: &mut std::collections::BTreeMap<Java26LevelKey, ServerLevel>,
) {
    let overworld = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let Some(source) = levels
        .get(&overworld)
        .and_then(|level| level.chunks.debug_generator())
        .cloned()
    else {
        return;
    };
    for level in levels.values_mut() {
        level.chunks.install_debug_view(source.clone());
    }
}

impl ServerWorld {
    pub fn is_debug(&self) -> bool {
        self.level(Java26BuiltinDimension::Overworld)
            .chunks
            .is_debug()
    }

    pub(super) fn finish_debug_dimension_initialization(
        &mut self,
        initialized: bool,
    ) -> Result<()> {
        if self.is_debug() && !initialized {
            self.cold.default_spawn = WorldSpawn {
                dimension: Java26BuiltinDimension::Overworld.into(),
                position: Vec3d {
                    x: 0.5,
                    y: 80.0,
                    z: 0.5,
                },
                rotation: PlayerRotation {
                    y_rot: 0.0,
                    x_rot: 0.0,
                },
            };
            self.admin.set_difficulty(Difficulty::Peaceful);
            self.cold.difficulty_locked = true;
            if !self
                .admin
                .set_game_rule(ADVANCE_WEATHER_RULE, GameRuleValue::Boolean(false))
            {
                return Err(ServerWorldError::InvalidConfiguration(format!(
                    "debug world has no {ADVANCE_WEATHER_RULE} game rule"
                )));
            }
            let clock_index = self
                .registries
                .require_registry_element_index(WORLD_CLOCK_REGISTRY, OVERWORLD_CLOCK)
                .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))?;
            let clock_index = usize::try_from(clock_index).map_err(|_| {
                ServerWorldError::InvalidConfiguration(
                    "Overworld clock index is outside usize".to_string(),
                )
            })?;
            let total_ticks = self.world_clock_total_ticks(clock_index).ok_or_else(|| {
                ServerWorldError::InvalidConfiguration(
                    "Overworld clock index is outside the synchronized registry".to_string(),
                )
            })?;
            self.world_clocks
                .move_to(clock_index, strict_next_noon(total_ticks))?;
            self.cold.default_game_type = GameType::Spectator;
            self.admin.set_game_mode(GameType::Spectator);
        }
        let forced = self.level_difficulty(self.cold.config.difficulty);
        self.admin.set_difficulty(forced);
        Ok(())
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn debug_initialization_facts_for_test(
        &self,
    ) -> (
        Vec3d,
        PlayerRotation,
        Difficulty,
        bool,
        Option<GameRuleValue>,
        GameType,
        Option<i64>,
    ) {
        let clock = self
            .registries
            .registry_element_index(WORLD_CLOCK_REGISTRY, OVERWORLD_CLOCK)
            .and_then(|index| usize::try_from(index).ok())
            .and_then(|index| self.world_clock_total_ticks(index));
        (
            self.cold.default_spawn.position,
            self.cold.default_spawn.rotation,
            self.admin.settings().difficulty,
            self.cold.difficulty_locked,
            self.admin.game_rule_value(ADVANCE_WEATHER_RULE),
            self.cold.default_game_type,
            clock,
        )
    }

    pub(crate) fn debug_level_shape_for_test(&self, dimension: &LevelId) -> (i32, usize) {
        let chunks = &self.level_by_id(dimension).chunks;
        (chunks.min_y(), chunks.section_count())
    }

    pub(crate) fn debug_level_spawn_height_for_test(&self, dimension: &LevelId) -> i32 {
        self.level_by_id(dimension)
            .spawn_facts
            .generator_spawn_height
    }

    pub(crate) fn debug_generated_block_state_for_test(
        &mut self,
        dimension: &LevelId,
        x: i32,
        y: i32,
        z: i32,
    ) -> Option<i32> {
        let chunks = &mut self.level_mut_by_id(dimension).chunks;
        chunks.load_column(x >> 4, z >> 4);
        chunks.generated_block_state(x, y, z)
    }

    pub(crate) fn prime_debug_suppression_for_test(&mut self) {
        use crate::level_ticks::ScheduleOutcome;
        use bbb_protocol::block_state_types::vanilla_block_default_state_id;
        use bbb_protocol::block_types::VANILLA_BLOCK_BARRIER_ID;
        use bbb_protocol::packets::BlockPos;
        use bbb_protocol::registry_data_types::VanillaFluidIdentity;
        use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, ScheduledTick};

        let dimension = LevelId::from(Java26BuiltinDimension::Overworld);
        self.register_chunk_tick_containers_by_key(&dimension, (0, 0));
        let trigger_tick = self.game_time() + 1;
        let pos = BlockPos { x: 1, y: 60, z: 1 };
        let barrier = vanilla_block_default_state_id(VANILLA_BLOCK_BARRIER_ID)
            .expect("the built-in barrier has a default state");
        let block = BlockTickType::of_block_state(barrier).expect("barrier tick type");
        let fluid = FluidTickType::of_fluid(VanillaFluidIdentity::Water);
        let level = self.level_mut_by_id(&dimension);
        assert_eq!(
            level
                .chunks
                .schedule_tick(ScheduledTick::new(block, pos, trigger_tick, 0)),
            ScheduleOutcome::Scheduled
        );
        assert_eq!(
            level
                .chunks
                .schedule_tick(ScheduledTick::new(fluid, pos, trigger_tick, 1)),
            ScheduleOutcome::Scheduled
        );
        level.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
    }

    pub(crate) fn debug_suppression_facts_for_test(&self) -> (usize, usize, usize, Option<i64>) {
        use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType};

        let level = self.level(Java26BuiltinDimension::Overworld);
        (
            level.chunks.scheduled_tick_keys::<BlockTickType>().count(),
            level.chunks.scheduled_tick_keys::<FluidTickType>().count(),
            level.changed_block_broadcasts.len(),
            level.chunks.inhabited_time(0, 0),
        )
    }
}
