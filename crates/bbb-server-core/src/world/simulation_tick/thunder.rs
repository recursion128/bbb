//! `ServerLevel.tickThunder` for one spawning chunk, moved out of the tick
//! facade with no change to what it does.

use super::*;
use crate::world::loot_context::LootLevelFacts;
use crate::world::simulation_level::LevelFrame;
use crate::world::LevelId;

impl ServerWorld {
    pub(super) fn tick_thunder_for_chunk(
        &mut self,
        dimension: &LevelId,
        chunk_x: i32,
        chunk_z: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        let raining = self.level_by_id(dimension).weather.is_raining();
        let thundering = self.level_by_id(dimension).weather.is_thundering();
        if !raining || !thundering {
            return;
        }
        // Roll `1/100_000` (`ServerLevel.java:534`) before building the full `LevelCtx` frame.
        if self.level_by_id(dimension).random.next_int(100_000) != 0 {
            return;
        }
        let conditions = crate::entity::thunder::ThunderConditions {
            raining,
            thundering,
            spawn_mobs: self.game_rules.spawn_mobs(),
        };
        let players = self.nearby_players_by_key(dimension);
        let moon = self.cold.dimension_sky_lights.spawn_context(
            self.cold
                .level_dimension_types
                .get(dimension)
                .map(String::as_str),
            &self.registries,
            &self.world_clocks,
        );
        let difficulty = self.admin.settings().difficulty;
        let mut missing_difficulty = false;
        let mut leftover = PendingEntitySpawns::default();
        let drop_tables = crate::entity::block_drops::BlockDropTables {
            tables: self
                .cold
                .config
                .datapacks
                .as_deref()
                .map(|packs| &packs.loot),
            recipes: None,
            // `Level.random` draws this level's lightning, whose damage
            // path never reaches a loot table it would read a damage
            // source for; the source view stays the one the level builds.
            damage_type_tags: None,
        };
        let entity_nbt = self.entity_nbt_read_snapshot();
        let originals =
            crate::world::natural_spawn::chunk_generation::OriginalMobWorldInputs::from_parts(
                &self.cold.config,
                &self.cold.mob_spawns,
                (
                    &self.registries,
                    &self.cold.dimension_sky_lights,
                    &self.world_clocks,
                    &self.cold.level_dimension_types,
                ),
                &self.admin,
                (self.game_rules, self.cold.seed, self.game_time, *dimension),
            );
        let block_interaction_policy = self.block_interaction_policy(dimension);
        let (ticked_level, other_levels) = self.levels.split_mut(dimension).expect("ticked level");
        let prev_handling = ticked_level.block_events.set_handling_tick(true);
        let mut loot_level_facts = LootLevelFacts::absent();
        loot_level_facts.scoreboard = Some(self.scoreboard.board());
        loot_level_facts.other_levels = Some(other_levels);
        let (mut level, rest) = LevelCtx::new(
            ticked_level,
            LevelFrame {
                block_interaction_policy: &block_interaction_policy,
                full_preparation:
                    crate::world::chunk_lifecycle::full_preparation::FullChunkPreparationInputs {
                        originals,
                        entity_nbt: &entity_nbt,
                    },
                game_time: self.game_time,
                game_rules: self.game_rules,
                feature_flags: self.feature_flags,
                drop_tables,
                loot_level_facts,
                structure_services: None,
                players: &players,
                item_spawns: &mut leftover,
            },
        );
        crate::entity::thunder::tick_thunder_after_roll(
            rest.entities,
            &mut level,
            chunk_x,
            chunk_z,
            conditions,
            |level, position| {
                let result = moon.overworld_clock_time().and_then(|clock| {
                    moon.effective_difficulty(level.chunks, position, difficulty, clock)
                });
                missing_difficulty |= result.is_none();
                result
            },
        );
        drop(level);
        self.level_mut_by_id(dimension)
            .block_events
            .set_handling_tick(prev_handling);
        if missing_difficulty {
            events.push(WorldEvent::ConsoleMessage(
                bbb_protocol::RichTextComponent::plain(
                    "thunder: missing positional difficulty context",
                ),
            ));
        }
        // The frame the thunder tick ran on is over.
        self.post_pending_game_events(dimension);
        self.apply_pending_spawns(dimension, leftover);
    }
}

#[cfg(test)]
impl ServerWorld {
    /// One chunk's thunder roll outside the tick's chunk walk, published to
    /// the trackers the way the tick's phase boundary publishes it.
    pub(crate) fn tick_thunder_for_test(
        &mut self,
        dimension: Java26BuiltinDimension,
        chunk: (i32, i32),
    ) -> Vec<WorldEvent> {
        let dimension = dimension.into();
        let mut events = Vec::new();
        self.tick_thunder_for_chunk(&dimension, chunk.0, chunk.1, &mut events);
        self.apply_level_entity_lifecycle_by_key(&dimension, &mut events);
        events
    }
}
