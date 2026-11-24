//! The level reads `SpawnPlacements` predicates take, shared by natural,
//! chunk-generation, spawner and event spawns.

use std::cell::Cell;

use crate::random::GameRandom;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_datapack::dimension_type::{
    builtin_dimension_type_runtime_facts, dimension_type_runtime_facts, DimensionTypeRuntimeFacts,
};
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_worldgen::value_providers::{sample_int, IntProvider};

use super::environment_attributes::SpawnEnvironmentContext;
use super::natural_spawn::{
    block_resource_id, block_state, fuzzy_biome_resource_id, tag_member, valid_spawn_floor,
    world_surface_height, BIOME_REGISTRY,
};
use crate::chunk::ChunkStore;

#[derive(Clone)]
pub(super) struct SpawnDimensionFacts {
    pub(super) monster_spawn_light_level: IntProvider,
    pub(super) monster_spawn_block_light_limit: u8,
    pub(super) ambient_light: f32,
    pub(super) sky_darken: u8,
    pub(super) thundering: bool,
}

pub(super) fn spawn_dimension_facts(
    datapacks: Option<&DatapackStartup>,
    dimension_type_id: Option<&str>,
    thundering: bool,
) -> Option<SpawnDimensionFacts> {
    let dimension_type_id = dimension_type_id?;
    let facts = match datapacks {
        Some(startup) => {
            dimension_type_runtime_facts(&startup.registries, dimension_type_id).ok()?
        }
        None => builtin_dimension_type_runtime_facts(dimension_type_id).ok()?,
    };
    spawn_dimension_facts_of(&facts, thundering)
}

pub(super) fn spawn_dimension_facts_of(
    facts: &DimensionTypeRuntimeFacts,
    thundering: bool,
) -> Option<SpawnDimensionFacts> {
    Some(SpawnDimensionFacts {
        monster_spawn_light_level: facts.monster_spawn_light_level.clone(),
        monster_spawn_block_light_limit: u8::try_from(facts.monster_spawn_block_light_limit)
            .ok()?,
        ambient_light: facts.ambient_light,
        sky_darken: 0,
        thundering,
    })
}

/// A probe that read an unavailable fact sets `missing`; predicate booleans may
/// be negated, so callers that must not guess reject such a probe.
pub(crate) struct SpawnPredicateLevel<'a> {
    pub(super) chunks: &'a ChunkStore,
    pub(super) registries: &'a crate::registry_sync::SynchronizedRegistries,
    pub(super) biome_zoom_seed: i64,
    pub(super) world_seed: i64,
    pub(super) difficulty: Difficulty,
    pub(super) spawn_dimension: Option<&'a SpawnDimensionFacts>,
    pub(super) spawn_environment: &'a SpawnEnvironmentContext<'a>,
    /// Feet positions of the alive, non-spectator players.
    pub(super) players: &'a [Vec3d],
    pub(super) generation_region: bool,
    pub(super) missing: Cell<bool>,
}

impl SpawnPredicateLevel<'_> {
    pub(super) fn missing(&self) -> bool {
        self.missing.get()
    }

    fn observe<T>(&self, value: Option<T>) -> Option<T> {
        if value.is_none() {
            self.missing.set(true);
        }
        value
    }

    fn state(&self, position: BlockPos) -> Option<i32> {
        self.observe(block_state(self.chunks, position))
    }

    pub(crate) fn is_peaceful(&self) -> bool {
        self.difficulty == Difficulty::Peaceful
    }

    pub(crate) fn sea_level(&self) -> i32 {
        self.chunks.sea_level()
    }

    pub(crate) fn max_y_exclusive(&self) -> i32 {
        self.chunks.min_y() + i32::try_from(self.chunks.section_count()).unwrap_or(0) * 16
    }

    pub(crate) fn sky_darken(&self) -> u8 {
        self.spawn_dimension.map_or(0, |facts| facts.sky_darken)
    }

    pub(crate) fn is_thundering(&self) -> bool {
        self.spawn_dimension.is_some_and(|facts| facts.thundering)
    }

    pub(crate) fn block_is(&self, position: BlockPos, resource_id: &str) -> bool {
        self.state(position).and_then(block_resource_id) == Some(resource_id)
    }

    pub(crate) fn block_matches_tag(&self, position: BlockPos, tag: &str) -> bool {
        self.state(position)
            .is_some_and(|state| tag_member(self.chunks, tag, state))
    }

    pub(crate) fn biome_matches_tag(&self, position: BlockPos, tag: &str) -> bool {
        self.observe(fuzzy_biome_resource_id(
            self.registries,
            self.chunks,
            self.biome_zoom_seed,
            position,
        ))
        .is_some_and(|resource_id| {
            self.chunks
                .registry_tags()
                .is_some_and(|tags| tags.contains(BIOME_REGISTRY, tag, resource_id))
        })
    }

    pub(crate) fn fluid_is_water(&self, position: BlockPos) -> bool {
        self.state(position).is_some_and(|state| {
            block_state_fluid_state(state).family() == Some(FluidFamily::Water)
        })
    }

    pub(crate) fn fluid_is_lava(&self, position: BlockPos) -> bool {
        self.state(position)
            .is_some_and(|state| block_state_fluid_state(state).family() == Some(FluidFamily::Lava))
    }

    pub(crate) fn has_liquid(&self, position: BlockPos) -> bool {
        self.state(position)
            .is_none_or(|state| !block_state_fluid_state(state).is_empty())
    }

    pub(crate) fn light_dampening(&self, position: BlockPos) -> u8 {
        self.state(position)
            .and_then(|state| {
                u8::try_from(
                    crate::block_behavior::support::light_dampening_with_registry(
                        state,
                        self.chunks.content_registry(),
                    ),
                )
                .ok()
            })
            .unwrap_or(u8::MAX)
    }

    pub(crate) fn sky_light(&self, position: BlockPos) -> u8 {
        let _ = self.state(position);
        self.chunks
            .sky_light_level(position.x, position.y, position.z)
    }

    pub(crate) fn block_light(&self, position: BlockPos) -> u8 {
        let _ = self.state(position);
        self.chunks
            .block_light_level(position.x, position.y, position.z)
    }

    pub(crate) fn world_surface_height(&self, position: BlockPos) -> i32 {
        self.observe(world_surface_height(self.chunks, position))
            .unwrap_or(self.chunks.min_y())
    }

    pub(crate) fn mob_spawn_floor_is_valid(&self, position: BlockPos, entity_type_id: i32) -> bool {
        let _ = self.state(position);
        valid_spawn_floor(self.chunks, position, entity_type_id)
    }

    pub(crate) fn has_nearby_alive_player(&self, position: BlockPos, radius: f64) -> bool {
        let (x, y, z) = (
            f64::from(position.x) + 0.5,
            f64::from(position.y) + 0.5,
            f64::from(position.z) + 0.5,
        );
        let radius_squared = radius * radius;
        self.players.iter().any(|player| {
            (player.x - x).mul_add(
                player.x - x,
                (player.y - y).mul_add(player.y - y, (player.z - z) * (player.z - z)),
            ) < radius_squared
        })
    }

    pub(crate) fn monster_spawn_block_light_limit(&self) -> Option<u8> {
        self.spawn_dimension
            .map(|facts| facts.monster_spawn_block_light_limit)
    }

    pub(crate) fn sample_monster_spawn_light_test(&self, random: &GameRandom) -> Option<u8> {
        let provider = &self.spawn_dimension?.monster_spawn_light_level;
        u8::try_from(sample_int(provider, &mut &*random)).ok()
    }

    pub(crate) fn surface_slime_spawn_chance(&self, position: BlockPos) -> Option<f32> {
        // WorldGenRegion.java:459-460; EnvironmentAttributes.java:144-145.
        if self.generation_region {
            return Some(0.0);
        }
        self.observe(
            self.spawn_environment
                .surface_slime_spawn_chance(self.chunks, position),
        )
    }

    pub(crate) fn is_slime_chunk(&self, chunk_x: i32, chunk_z: i32) -> bool {
        is_slime_chunk(self.world_seed, chunk_x, chunk_z)
    }
}

fn is_slime_chunk(seed: i64, x: i32, z: i32) -> bool {
    let mut h = (seed as u64)
        ^ (i64::from(x) as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (i64::from(z) as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    h.is_multiple_of(10)
}

#[cfg(test)]
mod tests {
    #[test]
    fn slime_chunks_are_deterministic_and_roughly_one_in_ten() {
        let count = (-16..16)
            .flat_map(|x| (-16..16).map(move |z| (x, z)))
            .filter(|&(x, z)| super::is_slime_chunk(12345, x, z))
            .count();
        assert_eq!(
            super::is_slime_chunk(12345, -3, 7),
            super::is_slime_chunk(12345, -3, 7)
        );
        assert!(
            (70..=135).contains(&count),
            "expected roughly 1/10 of 1024 chunks to be slime chunks, found {count}"
        );
        let first_grid: Vec<_> = (-4..4)
            .flat_map(|x| (-4..4).map(move |z| super::is_slime_chunk(12345, x, z)))
            .collect();
        let other_grid: Vec<_> = (-4..4)
            .flat_map(|x| (-4..4).map(move |z| super::is_slime_chunk(67890, x, z)))
            .collect();
        assert_ne!(first_grid, other_grid);
    }
}
