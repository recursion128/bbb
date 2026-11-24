//! Server chunk lifecycle adapters for the shared block and sky light engine.

use std::collections::BTreeSet;

use rustc_hash::{FxHashMap, FxHashSet};

use super::lighting::{initialize_light_window_coordinates, noise_initialized, ColumnLightSources};
use super::{Chunk, ChunkStore};
use bbb_protocol::chunk_light::light_engine::{
    light_facts, light_facts_with_registry, LightEngine, LightFacts,
};
use bbb_protocol::chunk_light::light_storage::ChunkLightStorage;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_server_types::chunk_data::{
    ChunkColumn, ChunkGenerationState, ChunkInitializeLightState, SectionBlocks,
};
use bbb_server_types::save::{SaveError, SaveResult};

type Pos = (i32, i32, i32);

/// The blocks a light pass may read; any other chunk reads as bedrock
/// (`LightEngine.getState`).
pub(crate) enum LightBlocks<'a> {
    /// One radius-one generation window around `center`.
    Window {
        center: (i32, i32),
        columns: [Option<&'a ChunkColumn>; 9],
    },
    /// Loaded columns whose blocks are final for lighting
    /// (`ServerChunkCache.getChunkForLighting`).
    Readable(&'a FxHashMap<(i32, i32), Chunk>),
}

impl<'a> LightBlocks<'a> {
    fn column(&self, chunk: (i32, i32)) -> Option<&'a ChunkColumn> {
        match self {
            Self::Window { center, columns } => {
                let dx = chunk.0.checked_sub(center.0)?;
                let dz = chunk.1.checked_sub(center.1)?;
                if !(-1..=1).contains(&dx) || !(-1..=1).contains(&dz) {
                    return None;
                }
                columns[((dz + 1) * 3 + (dx + 1)) as usize]
            }
            Self::Readable(chunks) => chunks
                .get(&chunk)
                .filter(|loaded| {
                    loaded.generation.as_ref().is_some_and(|state| {
                        matches!(state, ChunkGenerationState::Full { .. })
                            || noise_initialized(state).is_some()
                    })
                })
                .map(|loaded| &loaded.column),
        }
    }

    fn block_reader(&self) -> impl FnMut(Pos) -> Option<i32> + '_ {
        let mut cached = None;
        move |pos: Pos| {
            let chunk = (pos.0 >> 4, pos.2 >> 4);
            let column = match cached {
                Some((key, column)) if key == chunk => column,
                _ => {
                    let column = self.column(chunk);
                    cached = Some((chunk, column));
                    column
                }
            };
            column.and_then(|column| column.get_block_state(pos.0 & 15, pos.1, pos.2 & 15))
        }
    }
}

#[inline]
fn resolve_light_facts(state: i32, registry: Option<&ContentRegistry>) -> Option<LightFacts> {
    match registry {
        Some(registry) => light_facts_with_registry(state, registry),
        None => light_facts(state),
    }
}

fn new_light_engine<'a, B, H>(
    storage: &'a mut ChunkLightStorage,
    block_at: B,
    has_column: H,
    air_state: i32,
    registry: Option<&'a ContentRegistry>,
) -> bbb_protocol::chunk_light::LightResult<LightEngine<'a, B, H>>
where
    B: FnMut(Pos) -> Option<i32>,
    H: Fn((i32, i32)) -> bool,
{
    match registry {
        Some(registry) => {
            LightEngine::new_with_registry(storage, block_at, has_column, air_state, registry)
        }
        None => LightEngine::new(storage, block_at, has_column, air_state),
    }
}

/// `BlockLightEngine.propagateLightSources` (`BlockLightEngine.java:117-126`).
fn propagate_block_sources(
    engine: &mut LightEngine<'_, impl FnMut(Pos) -> Option<i32>, impl Fn((i32, i32)) -> bool>,
    blocks: &LightBlocks<'_>,
    chunk: (i32, i32),
    registry: Option<&ContentRegistry>,
) -> SaveResult<()> {
    let Some(column) = blocks.column(chunk) else {
        return Ok(());
    };
    let (base_x, base_z) = (chunk.0 * 16, chunk.1 * 16);
    for section_index in 0..column.section_count() {
        let section_y = column.min_y() + section_index as i32 * 16;
        let blocks = column.section(section_index);
        // ChunkAccess.java:320-344: skip sections whose palette has no light source.
        let palette = match blocks {
            SectionBlocks::Single(state) => std::slice::from_ref(state),
            SectionBlocks::Indirect { palette, .. } => palette.as_slice(),
        };
        let mut may_emit = false;
        for &state in palette {
            may_emit |= resolve_light_facts(state, registry)
                .ok_or_else(|| {
                    invalid_light(chunk, format!("block state {state} has no light facts"))
                })?
                .emission
                != 0;
        }
        if !may_emit {
            continue;
        }
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    let facts = resolve_light_facts(blocks.get(x, y, z), registry)
                        .ok_or_else(|| invalid_light(chunk, "block state has no light facts"))?;
                    if facts.emission != 0 {
                        engine.queue_block_source(
                            (base_x + x as i32, section_y + y as i32, base_z + z as i32),
                            facts,
                        );
                    }
                }
            }
        }
    }
    Ok(engine.run()?)
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub struct StagedLightChunk {
    pub center: (i32, i32),
    pub initialized: ChunkInitializeLightState,

    pub storage: ChunkLightStorage,
}

#[cfg(test)]
impl StagedLightChunk {
    pub fn storage(&self) -> &ChunkLightStorage {
        &self.storage
    }
}

type LightWindowSources = (
    ChunkInitializeLightState,
    Vec<((i32, i32), ColumnLightSources)>,
);

impl ChunkStore {
    /// Validates the radius-one LIGHT window of a noise center.
    /// `ChunkPyramid.java:36-39`.
    fn light_window_sources(&self, center: (i32, i32)) -> SaveResult<LightWindowSources> {
        let window = initialize_light_window_coordinates(center)?;
        let mut sources = Vec::with_capacity(9);
        let mut initialized = None;
        for chunk in window {
            validate_shell_coordinate(center, chunk)?;
            let state = match self.generation(chunk) {
                Some(state @ ChunkGenerationState::InitializeLight(_)) => {
                    noise_initialized(state).expect("matched INITIALIZE_LIGHT")
                }
                Some(state @ ChunkGenerationState::Light(light))
                    if chunk != center || !light.is_light_correct() =>
                {
                    noise_initialized(state).expect("matched LIGHT")
                }
                Some(state @ ChunkGenerationState::Spawn(_))
                | Some(state @ ChunkGenerationState::NoiseFull(_))
                    if chunk != center =>
                {
                    noise_initialized(state).expect("matched post-LIGHT noise status")
                }
                Some(ChunkGenerationState::Light(_)) => {
                    return Err(invalid_light(center, "the center is already correct LIGHT"))
                }
                Some(_) => {
                    return Err(invalid_light(
                        center,
                        format!("dependency {chunk:?} is below INITIALIZE_LIGHT"),
                    ))
                }
                None => {
                    return Err(invalid_light(
                        center,
                        format!("dependency {chunk:?} has no generation state"),
                    ))
                }
            };
            if !self.chunks.contains_key(&chunk) {
                return Err(invalid_light(
                    center,
                    format!("dependency {chunk:?} has no loaded column"),
                ));
            }
            for &section_y in state.non_empty_sections() {
                validate_section_y(center, section_y)?;
            }
            sources.push((chunk, ColumnLightSources::from_noise(state)));
            if chunk == center {
                initialized = Some(state.clone());
            }
        }
        Ok((
            initialized.expect("the LIGHT window contains its center"),
            sources,
        ))
    }

    /// Propagates one noise center's sources over its window into `storage`
    /// (`ThreadedLevelLightEngine.lightChunk`, `ThreadedLevelLightEngine.java:164-180`).
    pub(super) fn light_noise_center(
        &self,
        center: (i32, i32),
        storage: &mut ChunkLightStorage,
    ) -> SaveResult<ChunkInitializeLightState> {
        let (initialized, sources) = self.light_window_sources(center)?;
        let mut columns = [None; 9];
        for (chunk, source) in &sources {
            for &section_y in source.non_empty_sections() {
                storage.register_non_empty_section((chunk.0, section_y, chunk.1));
            }
            storage.set_column_sources(*chunk, source.lowest_sky_source_y());
            let dx = chunk.0 - center.0;
            let dz = chunk.1 - center.1;
            columns[((dz + 1) * 3 + (dx + 1)) as usize] = Some(&self.loaded(*chunk).column);
        }
        storage.enable_column(center);
        let registry = self.custom_block_registry();
        let blocks = LightBlocks::Window { center, columns };
        let mut engine = new_light_engine(
            storage,
            blocks.block_reader(),
            |chunk| blocks.column(chunk).is_some(),
            self.air_state_id(),
            registry,
        )?;
        propagate_block_sources(&mut engine, &blocks, center, registry)?;
        if self.has_skylight {
            engine.set_sky(true);
            engine.propagate_sky_sources(center)?;
        }
        Ok(initialized)
    }

    #[cfg(test)]
    pub fn stage_light_chunk(
        &self,
        center: (i32, i32),
        existing: &ChunkLightStorage,
    ) -> SaveResult<StagedLightChunk> {
        let mut storage = existing.clone();
        let initialized = self.light_noise_center(center, &mut storage)?;
        Ok(StagedLightChunk {
            center,
            initialized,

            storage,
        })
    }

    /// Lights ordinary columns that just reached FULL from their own sources,
    /// and joins them and `restored` saved-light columns to lit neighbours.
    pub(super) fn light_new_columns(
        &mut self,
        chunks: &BTreeSet<(i32, i32)>,
        restored: &BTreeSet<(i32, i32)>,
    ) -> SaveResult<()> {
        if chunks.is_empty() && restored.is_empty() {
            return Ok(());
        }
        let mut sources = Vec::with_capacity(chunks.len());
        for &chunk in chunks {
            sources.push((chunk, self.stage_column_light_sources(chunk)?));
        }
        for (chunk, source) in &sources {
            for &section_y in source.non_empty_sections() {
                validate_section_y(*chunk, section_y)?;
                self.light_storage
                    .register_non_empty_section((chunk.0, section_y, chunk.1));
            }
            self.light_storage
                .set_column_sources(*chunk, source.lowest_sky_source_y());
        }
        for &chunk in chunks {
            self.light_storage.enable_column(chunk);
        }
        let joined = chunks.union(restored).copied().collect::<BTreeSet<_>>();
        let registry = self.content_registry.as_deref();
        let blocks = LightBlocks::Readable(&self.chunks);
        let mut engine = new_light_engine(
            &mut self.light_storage,
            blocks.block_reader(),
            |chunk| blocks.column(chunk).is_some(),
            self.generated.air_state_id(),
            registry,
        )?;
        for sky in [false, true] {
            if sky && !self.has_skylight {
                break;
            }
            engine.set_sky(sky);
            for &chunk in chunks {
                if sky {
                    engine.propagate_sky_sources(chunk)?;
                } else {
                    propagate_block_sources(&mut engine, &blocks, chunk, registry)?;
                }
            }
            for &chunk in chunks {
                engine.exchange_faces(chunk, chunks);
            }
            for &chunk in restored {
                engine.exchange_faces(chunk, &joined);
            }
            engine.run()?;
        }
        Ok(())
    }

    /// Re-evaluates light around changed blocks of lit columns
    /// (`LevelLightEngine.checkBlock`, `LevelChunk.java:289-302`).
    pub(super) fn relight_changed_blocks(
        &mut self,
        changes: &crate::chunk::PendingLightChangeMap,
    ) -> SaveResult<()> {
        let mut checks = Vec::new();
        let mut empty_sections_to_unregister = Vec::new();
        for (&chunk, positions) in changes {
            if self.light_storage.column_sources(chunk).is_none() {
                continue;
            }
            let Some(column) = self.chunks.get(&chunk).map(|loaded| &loaded.column) else {
                continue;
            };
            let min_section = column.min_y() >> 4;
            let mut sections = FxHashSet::default();
            let mut source_columns = FxHashSet::default();
            for &pos in positions {
                sections.insert(pos.1 >> 4);
                source_columns.insert((pos.0 & 15, pos.2 & 15));
                checks.push(pos);
            }
            // LevelChunk.java:289-293; LayerLightSectionStorage.java:212-261:
            // register newly non-empty sections before lighting, and defer
            // unregistering emptied sections until after light updates drain.
            for section_y in sections {
                let Some(index) = usize::try_from(section_y - min_section)
                    .ok()
                    .filter(|&index| index < column.section_count())
                else {
                    continue;
                };
                let section = (chunk.0, section_y, chunk.1);
                if super::lighting::section_has_only_air(column, index) {
                    empty_sections_to_unregister.push(section);
                } else {
                    self.light_storage.register_non_empty_section(section);
                }
            }
            if self.has_skylight && !source_columns.is_empty() {
                let empty_sections = (0..column.section_count())
                    .map(|index| super::lighting::section_has_only_air(column, index))
                    .collect::<Vec<_>>();
                let registry = self.content_registry.as_deref();
                let mut lowest = Vec::with_capacity(source_columns.len());
                for (x, z) in source_columns {
                    lowest.push((
                        x,
                        z,
                        super::lighting::lowest_sky_source_y_with_empty_sections(
                            column,
                            &empty_sections,
                            x,
                            z,
                            registry,
                        )?,
                    ));
                }
                if let Some(sources) = self.light_storage.column_sources_mut(chunk) {
                    for (x, z, y) in lowest {
                        sources[(x + z * 16) as usize] = y;
                    }
                }
            }
        }
        if !checks.is_empty() {
            let registry = self.content_registry.as_deref();
            let blocks = LightBlocks::Readable(&self.chunks);
            let mut engine = new_light_engine(
                &mut self.light_storage,
                blocks.block_reader(),
                |chunk| blocks.column(chunk).is_some(),
                self.generated.air_state_id(),
                registry,
            )?;
            for &pos in &checks {
                engine.check_block(pos)?;
            }
            engine.run()?;
            if self.has_skylight {
                engine.set_sky(true);
                for &pos in &checks {
                    engine.check_sky(pos);
                }
                engine.run()?;
            }
        }
        for section in empty_sections_to_unregister {
            self.light_storage.unregister_non_empty_section(section);
        }
        Ok(())
    }
}

fn validate_shell_coordinate(center: (i32, i32), chunk: (i32, i32)) -> SaveResult<()> {
    chunk
        .0
        .checked_sub(1)
        .and_then(|_| chunk.0.checked_add(1))
        .and_then(|_| chunk.0.checked_mul(16))
        .ok_or_else(|| invalid_light(center, format!("dependency {chunk:?} x shell overflows")))?;
    chunk
        .1
        .checked_sub(1)
        .and_then(|_| chunk.1.checked_add(1))
        .and_then(|_| chunk.1.checked_mul(16))
        .ok_or_else(|| invalid_light(center, format!("dependency {chunk:?} z shell overflows")))?;
    Ok(())
}

fn validate_section_y(center: (i32, i32), section_y: i32) -> SaveResult<()> {
    section_y
        .checked_sub(1)
        .and_then(|_| section_y.checked_add(1))
        .and_then(|_| section_y.checked_mul(16))
        .ok_or_else(|| invalid_light(center, "light section y shell overflows"))?;
    Ok(())
}

fn invalid_light(center: (i32, i32), detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!("LIGHT chunk [{},{}]: {}", center.0, center.1, detail.into()),
    }
}

#[cfg(test)]
pub mod tests;
