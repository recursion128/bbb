//! Fresh-player spawn search over authoritative FULL columns.

use crate::chunk::ChunkStore;
use bbb_protocol::block_state_types::{
    vanilla_block_state_support_facts, VanillaBlockSupportDirection,
};
use bbb_protocol::entity_types::{vanilla_entity_type_dimensions, VANILLA_ENTITY_TYPE_PLAYER_ID};
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
    VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_datapack::dimension_type::{
    builtin_dimension_type_runtime_facts, dimension_type_runtime_facts,
};
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_types::chunk_data::SECTION_SIDE;
use bbb_server_types::fluid::block_state_fluid_state;
use bbb_server_types::geometry::{collect_block_collisions_with_moving_pistons, Aabb};
use bbb_server_types::save::{SaveError, SaveResult};
use bbb_server_types::ServerWorldError;

use super::{ServerLevelSource, ServerWorld, WorldSpawn};

const ABSOLUTE_MAX_ATTEMPTS: i64 = 1_024;
const DEFAULT_RESPAWN_RADIUS: i32 = 10;
const RESPAWN_RADIUS_RULE: &str = "minecraft:respawn_radius";

#[derive(Clone, Copy)]
pub(super) struct SpawnFacts {
    pub(super) has_skylight: bool,
    pub(super) has_ceiling: bool,
    pub(super) generator_spawn_height: i32,
    /// `DimensionType.ambientLight` (`DimensionType.java:40`), which reaches
    /// this level's `getLightLevelDependentMagicValue` rather than its spawn.
    pub(super) ambient_light: f32,
    /// `DimensionType.logicalHeight` (`DimensionType.java:38`).
    pub(super) logical_height: i32,
    /// `DimensionType.hasEnderDragonFight` (`ServerLevel.java:296`).
    pub(super) has_ender_dragon_fight: bool,
}

pub(super) fn dimension_spawn_facts(
    datapacks: Option<&DatapackStartup>,
    element_id: &str,
    source: &ServerLevelSource,
) -> bbb_server_types::Result<SpawnFacts> {
    let facts = match datapacks {
        Some(datapacks) => dimension_type_runtime_facts(&datapacks.registries, element_id),
        None => builtin_dimension_type_runtime_facts(element_id),
    }
    .map_err(|error| ServerWorldError::InvalidConfiguration(error.to_string()))?;
    let generator_spawn_height = match source {
        ServerLevelSource::Flat(source) => source.spawn_height(),
        ServerLevelSource::Debug(source) => source.spawn_height(),
        // `ChunkGenerator.getSpawnHeight` is 64
        // (`ChunkGenerator.java:427-429`); NoiseBasedChunkGenerator does not override it.
        ServerLevelSource::Noise { .. } => 64,
    };
    Ok(SpawnFacts {
        has_skylight: facts.shape.has_skylight,
        has_ceiling: facts.shape.has_ceiling,
        generator_spawn_height,
        ambient_light: facts.ambient_light,
        logical_height: facts.shape.logical_height,
        has_ender_dragon_fight: facts.shape.has_ender_dragon_fight,
    })
}

pub(super) fn flat_chunks(
    source: bbb_server_datapack::flat_generator::FlatLevelSource,
    biome_id: i32,
    biome_registry_len: usize,
    facts: SpawnFacts,
) -> ChunkStore {
    ChunkStore::new_with_biome_registry_and_skylight(
        source,
        biome_id,
        biome_registry_len,
        facts.has_skylight,
    )
}

impl ServerWorld {
    pub(super) fn find_default_spawn(&mut self) -> SaveResult<WorldSpawn> {
        let spawn = self.search_default_spawn()?;
        // Whoever searches after startup has read the area it kept.
        for level in self.levels.values_mut() {
            level.chunk_lifecycle.resident_spawn_column = None;
        }
        Ok(spawn)
    }

    /// `PlayerSpawnFinder.findSpawn` (`PlayerSpawnFinder.java:50-85`).
    fn search_default_spawn(&mut self) -> SaveResult<WorldSpawn> {
        let suggestion = self.cold.default_spawn.clone();
        self.level_mut_by_id(&suggestion.dimension)
            .chunks
            .flush_live_relights()?;
        let suggested = BlockPos {
            x: suggestion.position.x.floor() as i32,
            y: suggestion.position.y.floor() as i32,
            z: suggestion.position.z.floor() as i32,
        };
        let (has_skylight, has_ceiling, generator_spawn_height, world_border) = {
            let level = self.level_by_id(&suggestion.dimension);
            (
                level.spawn_facts.has_skylight,
                level.spawn_facts.has_ceiling,
                level.spawn_facts.generator_spawn_height,
                level.world_border,
            )
        };
        if !has_skylight
            || self.cold.default_game_type == bbb_protocol::game_types::GameType::Adventure
        {
            self.install_full_chunk_for_read(
                &suggestion.dimension,
                (suggested.x.div_euclid(16), suggested.z.div_euclid(16)),
            )?;
            let chunks = &self.level_by_id(&suggestion.dimension).chunks;
            return Ok(WorldSpawn {
                position: fixup_spawn_height(chunks, suggested),
                ..suggestion
            });
        }
        let mut radius = match self.admin.game_rule_value(RESPAWN_RADIUS_RULE) {
            Some(GameRuleValue::Integer(radius)) => radius.max(0),
            Some(GameRuleValue::Boolean(_)) | None => DEFAULT_RESPAWN_RADIUS,
        };
        let distance_to_border = world_border
            .distance_to_border_at(
                f64::from(suggested.x),
                f64::from(suggested.z),
                self.game_time,
            )
            .floor() as i32;
        if distance_to_border < radius {
            radius = distance_to_border;
        }
        if distance_to_border <= 1 {
            radius = 1;
        }
        let side = i64::from(radius)
            .checked_mul(2)
            .and_then(|diameter| diameter.checked_add(1))
            .ok_or_else(|| invalid_spawn_search("respawn radius overflows"))?;
        let candidates = side
            .checked_mul(side)
            .map(|count| count.min(ABSOLUTE_MAX_ATTEMPTS))
            .ok_or_else(|| invalid_spawn_search("respawn candidate count overflows"))?;
        let coprime = if candidates <= 16 { candidates - 1 } else { 17 };

        for candidate_index in 0..candidates {
            // Vanilla starts at thread-local entropy. This deterministic world
            // pins that otherwise unobservable offset to zero.
            let value = (coprime * candidate_index) % candidates;
            let delta_x = value % side;
            let delta_z = value / side;
            let x = i64::from(suggested.x)
                .checked_add(delta_x - i64::from(radius))
                .and_then(|value| i32::try_from(value).ok())
                .ok_or_else(|| invalid_spawn_search("candidate x is outside Java int"))?;
            let z = i64::from(suggested.z)
                .checked_add(delta_z - i64::from(radius))
                .and_then(|value| i32::try_from(value).ok())
                .ok_or_else(|| invalid_spawn_search("candidate z is outside Java int"))?;
            self.install_full_chunk_for_read(&suggestion.dimension, (x >> 4, z >> 4))?;
            let chunks = &self.level_by_id(&suggestion.dimension).chunks;
            if let Some(pos) =
                overworld_respawn_pos(chunks, x, z, has_ceiling, generator_spawn_height)?
            {
                if no_collision_no_liquid(chunks, pos) {
                    return Ok(WorldSpawn {
                        position: bottom_center(pos),
                        ..suggestion
                    });
                }
            }
        }

        self.install_full_chunk_for_read(
            &suggestion.dimension,
            (suggested.x.div_euclid(16), suggested.z.div_euclid(16)),
        )?;
        let chunks = &self.level_by_id(&suggestion.dimension).chunks;
        Ok(WorldSpawn {
            position: fixup_spawn_height(chunks, suggested),
            ..suggestion
        })
    }
}

impl ServerWorld {
    /// `MinecraftServer.setInitialSpawn` (`MinecraftServer.java:480-532`) for a
    /// level that was never initialized: start from the climate spawn target,
    /// then take the first valid column of the 11x11-chunk spiral around it.
    pub(super) fn set_initial_spawn(&mut self) -> SaveResult<()> {
        let dimension =
            super::LevelId::from(bbb_server_types::save::Java26BuiltinDimension::Overworld);
        let (origin, has_ceiling, generator_spawn_height) = {
            let level = self.level_by_id(&dimension);
            (
                level.chunks.climate_spawn_position(),
                level.spawn_facts.has_ceiling,
                level.spawn_facts.generator_spawn_height,
            )
        };
        let spawn_chunk = (origin[0].div_euclid(16), origin[1].div_euclid(16));
        let mut height = generator_spawn_height;
        if height < self.level_by_id(&dimension).chunks.min_y() {
            self.install_full_chunks_by_key(&dimension, &[spawn_chunk])?;
            height = self
                .level_by_id(&dimension)
                .chunks
                .final_heightmap_first_available(
                    spawn_chunk.0,
                    spawn_chunk.1,
                    VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
                    8,
                    8,
                )
                .ok_or_else(|| invalid_spawn_search("FULL column has no final heightmap"))?;
        }
        let mut spawn = BlockPos {
            x: spawn_chunk.0 * 16 + 8,
            y: height,
            z: spawn_chunk.1 * 16 + 8,
        };
        let (mut x, mut z, mut dx, mut dz) = (0_i32, 0_i32, 0_i32, -1_i32);
        for _ in 0..11 * 11 {
            if (-5..=5).contains(&x) && (-5..=5).contains(&z) {
                let chunk = (spawn_chunk.0 + x, spawn_chunk.1 + z);
                if let Some(pos) =
                    self.spawn_pos_in_chunk(&dimension, chunk, has_ceiling, generator_spawn_height)?
                {
                    spawn = pos;
                    break;
                }
            }
            if x == z || (x < 0 && x == -z) || (x > 0 && x == 1 - z) {
                (dx, dz) = (-dz, dx);
            }
            x += dx;
            z += dz;
        }
        self.cold.default_spawn = WorldSpawn {
            dimension,
            position: Vec3d {
                x: f64::from(spawn.x) + 0.5,
                y: f64::from(spawn.y),
                z: f64::from(spawn.z) + 0.5,
            },
            rotation: bbb_protocol::packets::PlayerRotation {
                y_rot: 0.0,
                x_rot: 0.0,
            },
        };
        Ok(())
    }

    /// `PlayerSpawnFinder.getSpawnPosInChunk` (`PlayerSpawnFinder.java:178-193`).
    fn spawn_pos_in_chunk(
        &mut self,
        dimension: &super::LevelId,
        chunk: (i32, i32),
        has_ceiling: bool,
        generator_spawn_height: i32,
    ) -> SaveResult<Option<BlockPos>> {
        self.install_full_chunks_by_key(dimension, &[chunk])?;
        let chunks = &self.level_by_id(dimension).chunks;
        for x in chunk.0 * 16..chunk.0 * 16 + 16 {
            for z in chunk.1 * 16..chunk.1 * 16 + 16 {
                if let Some(pos) =
                    overworld_respawn_pos(chunks, x, z, has_ceiling, generator_spawn_height)?
                {
                    return Ok(Some(pos));
                }
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn set_test_default_spawn(
        &mut self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
        position: Vec3d,
    ) {
        self.cold.default_spawn.dimension = dimension.into();
        self.cold.default_spawn.position = position;
    }

    /// Pins `respawn_radius`, whose square `PlayerSpawnFinder.findSpawn`
    /// searches around the spawn suggestion (`PlayerSpawnFinder.java:52-62`);
    /// zero is the suggestion itself.
    pub(crate) fn set_test_respawn_radius(&mut self, radius: i32) {
        assert!(self
            .admin
            .set_game_rule(RESPAWN_RADIUS_RULE, GameRuleValue::Integer(radius)));
    }

    pub(crate) fn level_spawn_facts(
        &self,
        dimension: bbb_server_types::save::Java26BuiltinDimension,
    ) -> (bool, bool, Option<i32>) {
        let level = self.level(dimension);
        (
            level.spawn_facts.has_skylight,
            level.spawn_facts.has_ceiling,
            Some(level.spawn_facts.generator_spawn_height),
        )
    }
}

/// `PlayerSpawnFinder.getOverworldRespawnPos` (`PlayerSpawnFinder.java:148-175`).
fn overworld_respawn_pos(
    chunks: &ChunkStore,
    x: i32,
    z: i32,
    has_ceiling: bool,
    generator_spawn_height: i32,
) -> SaveResult<Option<BlockPos>> {
    let cx = x.div_euclid(16);
    let cz = z.div_euclid(16);
    let local_x = x.rem_euclid(16);
    let local_z = z.rem_euclid(16);
    // `ChunkAccess.getHeight` returns `heightmap.getFirstAvailable - 1`
    // (`PlayerSpawnFinder.java:151-159`).
    let height = |kind| {
        chunks
            .final_heightmap_first_available(cx, cz, kind, local_x, local_z)
            .map(|first_available| first_available - 1)
            .ok_or_else(|| invalid_spawn_search("FULL column has no final heightmap"))
    };
    let top_y = if has_ceiling {
        generator_spawn_height
    } else {
        height(VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID)?
    };
    if top_y < chunks.min_y() {
        return Ok(None);
    }
    let surface = height(VANILLA_HEIGHTMAP_WORLD_SURFACE_ID)?;
    let ocean_floor = height(VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID)?;
    if surface <= top_y && surface > ocean_floor {
        return Ok(None);
    }

    let max_y_exclusive = chunks.min_y().saturating_add(
        i32::try_from(chunks.section_count())
            .unwrap_or(i32::MAX)
            .saturating_mul(i32::try_from(SECTION_SIDE).unwrap_or(i32::MAX)),
    );
    for y in (chunks.min_y()..=top_y.saturating_add(1)).rev() {
        if y >= max_y_exclusive {
            continue;
        }
        let pos = BlockPos { x, y, z };
        let state = chunks
            .get_block_state(x, y, z)
            .ok_or_else(|| invalid_spawn_search("spawn scan left its FULL column"))?;
        if !block_state_fluid_state(state).is_empty() {
            break;
        }
        let face_full = vanilla_block_state_support_facts(state)
            .map(|facts| facts.collision_face_full(VanillaBlockSupportDirection::Up))
            .or_else(|| {
                chunks
                    .content_registry()
                    .block_by_state_id(state)
                    .map(|(block, _)| block.full_cube_collision)
            })
            .ok_or_else(|| invalid_spawn_search("spawn scan block has no support facts"))?;
        if face_full {
            return Ok(y.checked_add(1).map(|y| BlockPos { y, ..pos }));
        }
    }
    Ok(None)
}

/// `PlayerSpawnFinder.fixupSpawnHeight` (`PlayerSpawnFinder.java:89-103`).
fn fixup_spawn_height(chunks: &ChunkStore, mut pos: BlockPos) -> Vec3d {
    // `LevelHeightAccessor.getMaxY` is the last block inside the world, one
    // below `minY + height` (`LevelHeightAccessor.java:11-13`).
    let max_y = chunks
        .min_y()
        .saturating_add(
            i32::try_from(chunks.section_count())
                .unwrap_or(i32::MAX)
                .saturating_mul(i32::try_from(SECTION_SIDE).unwrap_or(i32::MAX)),
        )
        .saturating_sub(1);
    while !no_collision_no_liquid(chunks, pos) && pos.y < max_y {
        pos.y += 1;
    }
    pos.y = pos.y.saturating_sub(1);
    while no_collision_no_liquid(chunks, pos) && pos.y > chunks.min_y() {
        pos.y -= 1;
    }
    pos.y = pos.y.saturating_add(1);
    bottom_center(pos)
}

/// `PlayerSpawnFinder.noCollisionNoLiquid` (`PlayerSpawnFinder.java:106-108`).
fn no_collision_no_liquid(chunks: &ChunkStore, pos: BlockPos) -> bool {
    let position = bottom_center(pos);
    let [width, height] = vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_PLAYER_ID)
        .expect("the generated entity catalog contains player dimensions");
    let bounds = Aabb::of_entity(position, width, height);
    if !collect_block_collisions_with_moving_pistons(
        bounds,
        |probe| super::movement_collision::moving_piston_at(chunks, probe).unwrap_or(true),
        |probe| {
            let Some(state) = chunks.get_block_state(probe.x, probe.y, probe.z) else {
                return vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })];
            };
            super::movement_collision::static_collision_shape_with_live_piston(chunks, probe, state)
        },
    )
    .is_empty()
    {
        return false;
    }
    (pos.y..=pos.y.saturating_add(1)).all(|y| {
        chunks
            .get_block_state(pos.x, y, pos.z)
            .is_some_and(|state| block_state_fluid_state(state).is_empty())
    })
}

fn bottom_center(pos: BlockPos) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y),
        z: f64::from(pos.z) + 0.5,
    }
}

fn invalid_spawn_search(detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("player spawn search: {}", detail.into()),
    }
}
