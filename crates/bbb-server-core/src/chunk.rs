//! Authoritative paletted-section chunk column store.
//!
//! Block states follow vanilla 26.1 `PalettedContainer` semantics:
//! `net/minecraft/world/level/chunk/Strategy.java:33-48` (palette strategy),
//! `Strategy.java:69-71` (`(y << 4 | z) << 4 | x` index order), and
//! `net/minecraft/util/SimpleBitStorage.java:240-299` (LSB-first packing,
//! entries never straddle u64 boundaries, `64 / bits` entries per u64).
//!
//! Wire format matches `net/minecraft/world/level/chunk/LevelChunkSection.java:181-186`
//! (`short nonEmptyBlockCount`, `short fluidCount`, states container, biomes
//! container) and `PalettedContainer.Data.write`
//! (`net/minecraft/world/level/chunk/PalettedContainer.java:325-329`):
//! `u8 bits`, palette, then the data u64 array big-endian with NO length
//! prefix (`FriendlyByteBuf.writeFixedSizeLongArray`,
//! `net/minecraft/network/FriendlyByteBuf.java:353-357`); the reader derives
//! the array length from the bit count (`PalettedContainer.read`,
//! `PalettedContainer.java:156-168`). For `bits == 0` the palette is a single
//! varint value and no data words follow
//! (`SingleValuePalette.java:59-65`).
//!
//! Vanilla's global-palette wire form (`Strategy.java:44`): past 8 bits /
//! 256 distinct values the wire carries no palette list and the data words
//! hold global block-state ids at `ceillog2` of the global registry size
//! (15 bits for 26.1's 29873 states). This store emits and accepts exactly
//! that form; in-memory sections keep an indirect palette regardless (the
//! global form is a wire-size optimization only).
//!
//! Sections are never repacked or shrunk on write: once a section becomes
//! indirect it stays indirect, and bits only grow. Vanilla likewise repacks
//! only when reading saved data, not on in-memory edits.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

mod blending;
pub mod block_entities;
pub mod chunk_packet;
mod cold_columns;
mod early_generation;
pub mod features_orchestration;
mod fluid_shapes;
pub mod full;
pub mod generation_publication;
pub mod light_engine;
pub mod lighting;
mod live_updates;
mod loading;
mod saved_column_restore;
mod stored_columns;
mod structure_generation;
mod ticks;
mod unload;
pub use saved_column_restore::SavedColumnEffects;
pub(crate) use saved_column_restore::SavedColumnResources;
mod column_generation;
mod locate;
pub mod noise_generator;
mod ordinary_generation;
mod ordinary_lighting;
pub(crate) mod parallel_generation;
mod path_navigation;
pub mod poi;
pub mod random_ticks;
mod reloadable_resources;
pub mod restored_generation;
pub mod spawn;
pub mod structure_status;
pub(crate) mod upgrade_data;

#[cfg(test)]
mod debug_tests;
#[cfg(test)]
mod end_full_tests;
#[cfg(test)]
mod fresh_noise_full_tests;
#[cfg(test)]
pub mod initialize_light_restore_tests;
#[cfg(test)]
mod locate_tests;
#[cfg(test)]
mod nether_full_tests;
#[cfg(test)]
mod ordinary_heightmap_tests;
#[cfg(test)]
pub mod spawn_full_persist_tests;
#[cfg(test)]
pub mod spawn_tests;
use self::structure_status::NoiseStructureStatusCache;
use bbb_protocol::chunk_light::light_storage::ChunkLightStorage;
use bbb_protocol::chunk_light::SavedLightSection;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_server_types::chunk_data::feature_heightmaps::FeatureHeightmaps;
use bbb_server_types::chunk_data::ChunkPreCarversState;

use bbb_server_types::chunk_data::{
    ChunkColumn, ChunkGenerationState, ChunkLightState, SectionBlocks, SECTION_SIDE,
    VANILLA_26_1_BIOME_REGISTRY_LEN,
};

use bbb_protocol::NbtSummaryEntry;

use crate::block_entity::{BlockEntity, BlockEntityKey};
use crate::chunk::noise_generator::NoiseFinalDensityProgram;
use crate::fluid::SharedFluidShapeCache;
use crate::level_ticks::TickContainers;
use crate::registry_tags::RegistryTagSnapshot;
use bbb_server_datapack::debug_generator::DebugLevelSource;
use bbb_server_datapack::flat_generator::FlatLevelSource;
use bbb_server_persistence::{Java26BlendingData, Java26ChunkStatus};
use bbb_server_types::save::{SaveError, SaveResult};
use block_entities::LevelBlockEntities;

enum ChunkSource {
    Flat(FlatLevelSource),
    Debug(DebugLevelSource),
    NoiseCarvers(Arc<NoiseFinalDensityProgram>),
}

impl fmt::Debug for ChunkSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Flat(source) => f.debug_tuple("Flat").field(source).finish(),
            Self::Debug(source) => f.debug_tuple("Debug").field(source).finish(),
            Self::NoiseCarvers(program) => f
                .debug_struct("NoiseCarvers")
                .field("dimension_type", &program.dimension_type_id())
                .field("settings", &program.settings_id())
                .finish(),
        }
    }
}

/// World-wide store keyed by chunk (x, z), answering an absent column from the
/// generator the loaded save declares.
///
/// This is the `ServerChunkCache` seam: vanilla answers `Level.getChunk(x, z,
/// ChunkStatus.FULL)` out of the cache and, when nothing is there, runs the
/// `ChunkGenerator` the level stem names. Nothing here is a fixture any more —
/// the columns a client is sent, and the base every block edit is compared
/// against, both come out of the declared source.
pub(crate) type PendingLightChangeMap =
    rustc_hash::FxHashMap<(i32, i32), rustc_hash::FxHashSet<(i32, i32, i32)>>;

#[derive(Debug)]
pub struct ChunkStore {
    poi: poi::PoiManager,
    chunks: rustc_hash::FxHashMap<(i32, i32), Chunk>,
    cold_columns: rustc_hash::FxHashMap<(i32, i32), ChunkColumn>,
    saved_column_restoration: saved_column_restore::SavedColumnRestoration,
    cold_column_reads: cold_columns::ColdColumnReads,
    source: ChunkSource,
    ordinary_structures: Option<Arc<structure_generation::OrdinaryStructureProgram>>,
    debug_view: Option<DebugLevelSource>,
    /// Origin template for geometry and created-save publication; coordinate-
    /// specific sources keep loaded preimages in [`Chunk::generated`].
    generated: ChunkColumn,
    generation_blending_catalog_complete: bool,
    generation_blending_restore_in_progress: bool,
    /// `ChunkMap.java:188-196` creates the light engine from the live dimension type.
    has_skylight: bool,
    registry_tags: Option<Arc<RegistryTagSnapshot>>,
    /// Present only when session plugins register custom blocks, keeping vanilla
    /// worlds on the zero-overhead fast path.
    content_registry: Option<Arc<ContentRegistry>>,
    fluid_shape_cache: SharedFluidShapeCache,
    light_storage: ChunkLightStorage,
    ordinary_light_restoring: bool,
    /// True when any ordinary FULL column or saved-light column may need a pass
    /// in `flush_ordinary_lights`; cleared once all pending columns are lit.
    ordinary_lights_dirty: bool,
    structure_status: NoiseStructureStatusCache,
    /// Light checks queued by block writes until the next light barrier.
    pending_light_changes: PendingLightChangeMap,
    live_light_updates: rustc_hash::FxHashMap<(i32, i32), live_updates::DirtyLightSections>,
    next_whole_rewrite_epoch: u64,
    biome_id: i32,
    biome_registry_len: usize,
    path_navigation: path_navigation::PathNavigationState,
    /// Level-wide block-entity state; the objects live in their [`Chunk`].
    block_entities: LevelBlockEntities,
}

/// One loaded column and the state that is generated, saved and unloaded with it.
#[derive(Debug)]
pub(crate) struct Chunk {
    column: ChunkColumn,
    generation: Option<ChunkGenerationState>,
    pre_carvers: Option<ChunkPreCarversState>,
    /// `Some(Some(_))` is old noise generation (`ChunkAccess.isOldNoiseGeneration`).
    blending: Option<Option<Java26BlendingData>>,
    legacy_work: Option<restored_generation::PendingLegacyGenerationWork>,
    /// Ordinary FULL heightmaps, primed on first read and kept current by
    /// writes; boxed so chunks that are never sent do not carry them.
    heightmaps: std::sync::OnceLock<Box<FeatureHeightmaps>>,
    /// Shared `LevelChunkWithLight` payload reused across viewers until the
    /// column's blocks, biomes, block entities or light change.
    pub(crate) cached_packet:
        std::sync::OnceLock<std::sync::Arc<bbb_protocol::packets::LevelChunkWithLight>>,
    proto_entities: Vec<Vec<NbtSummaryEntry>>,
    published_worldgen_entities: Vec<Vec<NbtSummaryEntry>>,
    whole_rewrite_epoch: Option<u64>,
    save_dirty_epoch: Option<u64>,
    pending_initial_persistence: bool,
    /// Saved light was installed and is not yet joined to lit neighbours.
    saved_light: bool,
    storage_backed_generation: bool,
    storage_backed_full: bool,
    /// Relighting changes the staged payload, not the wrapped LevelChunk type.
    relighting_full: bool,
    /// Ordinary FULL light changed since the last save.
    derived_dirty: bool,
    /// The scheduler's containers once registered; until then the column parks them.
    ticks: Option<TickContainers>,
    /// `LevelChunk.blockEntities`, ordered as chunk packets and saves list them.
    block_entities: BTreeMap<BlockEntityKey, BlockEntity>,
    /// Saved block-entity compounds with no modeled runtime object (`pendingBlockEntities`).
    packed_block_entities: BTreeMap<BlockEntityKey, Vec<NbtSummaryEntry>>,
}

static NEXT_SAVE_DIRTY_EPOCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Chunk {
    fn new(column: ChunkColumn) -> Self {
        Self {
            column,
            generation: None,
            pre_carvers: None,
            blending: None,
            legacy_work: None,
            heightmaps: std::sync::OnceLock::new(),
            cached_packet: std::sync::OnceLock::new(),
            proto_entities: Vec::new(),
            published_worldgen_entities: Vec::new(),
            whole_rewrite_epoch: None,
            save_dirty_epoch: None,
            pending_initial_persistence: false,
            saved_light: false,
            storage_backed_generation: false,
            storage_backed_full: false,
            relighting_full: false,
            derived_dirty: false,
            ticks: None,
            block_entities: BTreeMap::new(),
            packed_block_entities: BTreeMap::new(),
        }
    }

    pub(crate) fn mark_save_dirty(&mut self) {
        self.save_dirty_epoch =
            Some(NEXT_SAVE_DIRTY_EPOCH.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    }

    fn column_mut(&mut self) -> &mut ChunkColumn {
        self.heightmaps.take();
        self.cached_packet.take();
        &mut self.column
    }

    fn set_column(&mut self, column: ChunkColumn) {
        self.heightmaps.take();
        self.cached_packet.take();
        self.column = column;
    }

    fn heightmaps(&self) -> &FeatureHeightmaps {
        self.heightmaps
            .get_or_init(|| Box::new(FeatureHeightmaps::prime(&self.column)))
    }

    /// The one cell write: keeps primed heightmaps current.
    fn set_block_state(&mut self, local_x: i32, y: i32, local_z: i32, state: i32) -> Option<i32> {
        let previous = self.column.set_block_state(local_x, y, local_z, state)?;
        if previous != state {
            self.cached_packet.take();
            if let Some(maps) = self.heightmaps.get_mut() {
                maps.update_after_set(&self.column, local_x, y, local_z, state);
            }
        }
        Some(previous)
    }

    fn is_durable(&self) -> bool {
        self.storage_backed_generation || self.whole_rewrite_epoch.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WholeRewriteIntent {
    pub chunk: (i32, i32),
    pub epoch: u64,
}

impl ChunkStore {
    /// Binds a store to the generator that fills it. Every other shape fact —
    /// the world floor, how many sections a column has, which state counts as
    /// air — comes out of what that generator produces, so no caller can
    /// configure a store into disagreeing with its own chunks.
    pub fn new(generator: FlatLevelSource, biome_id: i32) -> Self {
        Self::new_with_biome_registry(generator, biome_id, VANILLA_26_1_BIOME_REGISTRY_LEN)
    }

    pub fn new_with_biome_registry(
        generator: FlatLevelSource,
        biome_id: i32,
        biome_registry_len: usize,
    ) -> Self {
        Self::new_with_biome_registry_and_skylight(generator, biome_id, biome_registry_len, true)
    }

    pub fn new_with_biome_registry_and_skylight(
        generator: FlatLevelSource,
        biome_id: i32,
        biome_registry_len: usize,
        has_skylight: bool,
    ) -> Self {
        let generated = generator
            .generate_chunk_with_biome_registry(0, 0, biome_id, biome_registry_len)
            .into_column();
        Self {
            chunks: rustc_hash::FxHashMap::default(),
            cold_columns: rustc_hash::FxHashMap::default(),
            saved_column_restoration: Default::default(),
            cold_column_reads: Default::default(),
            source: ChunkSource::Flat(generator),
            ordinary_structures: None,
            debug_view: None,
            generated,
            generation_blending_catalog_complete: false,
            generation_blending_restore_in_progress: false,
            has_skylight,
            registry_tags: None,
            content_registry: None,
            fluid_shape_cache: SharedFluidShapeCache::default(),
            light_storage: ChunkLightStorage::default(),
            poi: poi::PoiManager::default(),
            ordinary_light_restoring: false,
            ordinary_lights_dirty: false,
            structure_status: NoiseStructureStatusCache::new(false),
            pending_light_changes: rustc_hash::FxHashMap::default(),
            live_light_updates: rustc_hash::FxHashMap::default(),
            next_whole_rewrite_epoch: 0,
            biome_id,
            biome_registry_len,
            path_navigation: path_navigation::PathNavigationState::default(),
            block_entities: LevelBlockEntities::default(),
        }
    }

    pub fn new_debug_with_biome_registry_and_skylight(
        source: DebugLevelSource,
        plains_id: i32,
        biome_registry_len: usize,
        has_skylight: bool,
    ) -> Self {
        let generated =
            source.generate_chunk_with_biome_registry(0, 0, plains_id, biome_registry_len);
        Self {
            chunks: rustc_hash::FxHashMap::default(),
            cold_columns: rustc_hash::FxHashMap::default(),
            saved_column_restoration: Default::default(),
            cold_column_reads: Default::default(),
            source: ChunkSource::Debug(source),
            ordinary_structures: None,
            debug_view: None,
            generated,
            generation_blending_catalog_complete: false,
            generation_blending_restore_in_progress: false,
            has_skylight,
            registry_tags: None,
            content_registry: None,
            fluid_shape_cache: SharedFluidShapeCache::default(),
            light_storage: ChunkLightStorage::default(),
            poi: poi::PoiManager::default(),
            ordinary_light_restoring: false,
            ordinary_lights_dirty: false,
            structure_status: NoiseStructureStatusCache::new(false),
            pending_light_changes: rustc_hash::FxHashMap::default(),
            live_light_updates: rustc_hash::FxHashMap::default(),
            next_whole_rewrite_epoch: 0,
            biome_id: plains_id,
            biome_registry_len,
            path_navigation: path_navigation::PathNavigationState::default(),
            block_entities: LevelBlockEntities::default(),
        }
    }

    pub fn new_overworld_carvers(program: Arc<NoiseFinalDensityProgram>) -> SaveResult<Self> {
        Self::new_overworld_carvers_with_structure_generation(program, true)
    }

    pub fn new_overworld_carvers_with_structure_generation(
        program: Arc<NoiseFinalDensityProgram>,
        generate_structures: bool,
    ) -> SaveResult<Self> {
        if program.dimension_type_id() != "minecraft:overworld"
            || program.settings_id() != "minecraft:overworld"
        {
            return Err(invalid_noise_source(
                &program,
                "the Overworld compatibility constructor requires the built-in Overworld profile",
            ));
        }
        Self::new_noise_carvers_with_structure_generation(program, generate_structures)
    }

    pub fn new_nether_carvers_with_structure_generation(
        program: Arc<NoiseFinalDensityProgram>,
        generate_structures: bool,
    ) -> SaveResult<Self> {
        if program.dimension_type_id() != "minecraft:the_nether"
            || program.settings_id() != "minecraft:nether"
        {
            return Err(invalid_noise_source(
                &program,
                "the Nether constructor requires the built-in Nether profile",
            ));
        }
        Self::new_noise_carvers_with_structure_generation(program, generate_structures)
    }

    pub fn new_end_carvers_with_structure_generation(
        program: Arc<NoiseFinalDensityProgram>,
        generate_structures: bool,
    ) -> SaveResult<Self> {
        if program.dimension_type_id() != "minecraft:the_end"
            || program.settings_id() != "minecraft:end"
        {
            return Err(invalid_noise_source(
                &program,
                "the End constructor requires the built-in End profile",
            ));
        }
        Self::new_noise_carvers_with_structure_generation(program, generate_structures)
    }

    pub fn new_noise_carvers_with_structure_generation(
        program: Arc<NoiseFinalDensityProgram>,
        generate_structures: bool,
    ) -> SaveResult<Self> {
        let has_skylight = program.dimension_type_shape().has_skylight;
        let generated = program.generate_fresh_carved_chunk(0, 0)?.into_column();
        Ok(Self {
            chunks: rustc_hash::FxHashMap::default(),
            cold_columns: rustc_hash::FxHashMap::default(),
            saved_column_restoration: Default::default(),
            cold_column_reads: Default::default(),
            source: ChunkSource::NoiseCarvers(program),
            ordinary_structures: None,
            debug_view: None,
            generated,
            generation_blending_catalog_complete: false,
            generation_blending_restore_in_progress: false,
            has_skylight,
            registry_tags: None,
            content_registry: None,
            fluid_shape_cache: SharedFluidShapeCache::default(),
            light_storage: ChunkLightStorage::default(),
            poi: poi::PoiManager::default(),
            ordinary_light_restoring: false,
            ordinary_lights_dirty: false,
            structure_status: NoiseStructureStatusCache::new(generate_structures),
            pending_light_changes: rustc_hash::FxHashMap::default(),
            live_light_updates: rustc_hash::FxHashMap::default(),
            next_whole_rewrite_epoch: 0,
            biome_id: 0,
            biome_registry_len: VANILLA_26_1_BIOME_REGISTRY_LEN,
            path_navigation: path_navigation::PathNavigationState::default(),
            block_entities: LevelBlockEntities::default(),
        })
    }

    pub fn min_y(&self) -> i32 {
        self.generated.min_y()
    }

    pub const fn has_skylight(&self) -> bool {
        self.has_skylight
    }

    pub fn block_light_level(&self, x: i32, y: i32, z: i32) -> u8 {
        if self.is_ordinary_source() && !self.is_loaded(x.div_euclid(16), z.div_euclid(16)) {
            return 0;
        }
        self.light_storage.block_level((x, y, z))
    }

    pub fn sky_light_level(&self, x: i32, y: i32, z: i32) -> u8 {
        if !self.has_skylight {
            return 0;
        }
        if self.is_ordinary_source() && !self.is_loaded(x.div_euclid(16), z.div_euclid(16)) {
            return 0;
        }
        self.light_storage.sky_level((x, y, z))
    }

    pub fn biome_id_at(&self, x: i32, y: i32, z: i32) -> Option<i32> {
        self.biome_id_at_quart(x.div_euclid(4), y.div_euclid(4), z.div_euclid(4))
    }

    pub fn biome_id_at_quart(&self, quart_x: i32, quart_y: i32, quart_z: i32) -> Option<i32> {
        self.biome_id_at_quart_in_chunk(
            quart_x.div_euclid(4),
            quart_z.div_euclid(4),
            quart_x,
            quart_y,
            quart_z,
        )
    }

    // LevelReader.java:57-60; ServerLevel.java:315-317.
    pub fn biome_id_at_quart_or_source(
        &self,
        quart_x: i32,
        quart_y: i32,
        quart_z: i32,
    ) -> Option<i32> {
        let chunk = (quart_x.div_euclid(4), quart_z.div_euclid(4));
        let has_biomes = self
            .chunk(chunk)
            .and_then(|chunk| chunk.pre_carvers.as_ref())
            .is_none_or(|state| {
                state
                    .status()
                    .is_at_least(&bbb_server_types::chunk_data::ChunkPreCarversStatus::Biomes)
            });
        if has_biomes && self.chunks.contains_key(&chunk) {
            return self.biome_id_at_quart(quart_x, quart_y, quart_z);
        }
        match &self.source {
            ChunkSource::Flat(_) | ChunkSource::Debug(_) => Some(self.biome_id),
            ChunkSource::NoiseCarvers(program) => {
                program.uncached_noise_biome(quart_x, quart_y, quart_z)
            }
        }
    }

    // Vanilla 26.1 NaturalSpawner.java:154-160, NaturalSpawner.java:201-202,
    // NaturalSpawner.java:492-515.
    pub fn biome_id_at_quart_in_chunk(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        quart_x: i32,
        quart_y: i32,
        quart_z: i32,
    ) -> Option<i32> {
        let column = self.column(chunk_x, chunk_z)?;
        // Vanilla 26.1 ChunkAccess.java:427-433.
        let min_quart_y = column.min_y().div_euclid(4);
        let quart_height = i32::try_from(column.section_count().checked_mul(4)?).ok()?;
        let max_quart_y = min_quart_y.checked_add(quart_height.checked_sub(1)?)?;
        column.get_noise_biome(
            quart_x.rem_euclid(4),
            quart_y.clamp(min_quart_y, max_quart_y),
            quart_z.rem_euclid(4),
        )
    }

    pub fn install_registry_tags(&mut self, registry_tags: Arc<RegistryTagSnapshot>) {
        self.clear_path_type_cache();
        self.registry_tags = Some(registry_tags);
    }

    pub fn install_content_registry(&mut self, content_registry: Arc<ContentRegistry>) {
        let has_custom = !content_registry.blocks().is_empty()
            || !content_registry.items().is_empty()
            || !content_registry.entities().is_empty()
            || !content_registry.particles().is_empty()
            || !content_registry.sound_events().is_empty();
        self.content_registry = has_custom.then_some(content_registry);
    }

    pub fn content_registry(&self) -> &ContentRegistry {
        self.content_registry
            .as_deref()
            .unwrap_or_else(|| ContentRegistry::vanilla())
    }

    pub(super) fn custom_block_registry(&self) -> Option<&ContentRegistry> {
        self.content_registry
            .as_deref()
            .filter(|registry| !registry.blocks().is_empty())
    }

    pub fn registry_tags(&self) -> Option<&RegistryTagSnapshot> {
        self.registry_tags.as_deref()
    }

    /// The loaded tags, or the built-in datapack's before any install.
    pub fn block_tags(&self) -> &RegistryTagSnapshot {
        self.registry_tags
            .as_deref()
            .unwrap_or_else(|| RegistryTagSnapshot::builtin())
    }

    pub fn registry_tags_shared(&self) -> Option<Arc<RegistryTagSnapshot>> {
        self.registry_tags.as_ref().map(Arc::clone)
    }

    pub fn section_count(&self) -> usize {
        self.generated.section_count()
    }

    pub fn sea_level(&self) -> i32 {
        match &self.source {
            ChunkSource::Flat(generator) => generator.sea_level(),
            ChunkSource::Debug(generator) => generator.sea_level(),
            ChunkSource::NoiseCarvers(program) => program.sea_level(),
        }
    }

    /// `RandomState.sampler().findSpawnPosition()`; flat and debug sources
    /// have no spawn target and answer the origin.
    pub(crate) fn climate_spawn_position(&self) -> [i32; 2] {
        match &self.source {
            ChunkSource::NoiseCarvers(program) => program.find_spawn_position(),
            ChunkSource::Flat(_) | ChunkSource::Debug(_) => [0, 0],
        }
    }

    pub fn air_state_id(&self) -> i32 {
        self.generated.air_state_id()
    }

    pub fn biome_id(&self) -> i32 {
        self.biome_id
    }

    /// The served flat generator, absent for every other source.
    pub fn flat_generator(&self) -> Option<&FlatLevelSource> {
        match &self.source {
            ChunkSource::Flat(generator) => Some(generator),
            ChunkSource::Debug(_) | ChunkSource::NoiseCarvers(_) => None,
        }
    }

    pub fn debug_generator(&self) -> Option<&DebugLevelSource> {
        match &self.source {
            ChunkSource::Debug(generator) => Some(generator),
            ChunkSource::Flat(_) | ChunkSource::NoiseCarvers(_) => None,
        }
    }

    pub fn install_debug_view(&mut self, source: DebugLevelSource) {
        self.debug_view = Some(source);
    }

    pub const fn is_debug(&self) -> bool {
        self.debug_view.is_some()
    }

    /// `LevelHeightAccessor.isInsideBuildHeight` (`LevelHeightAccessor.java:31-33`)
    /// for the dimension this store's generator is bound to.
    pub fn is_inside_build_height(&self, y: i32) -> bool {
        let height = self.section_count() as i64 * SECTION_SIDE as i64;
        let offset = i64::from(y) - i64::from(self.min_y());
        (0..height).contains(&offset)
    }

    /// A fresh column exactly as the generator produces it, with none of the
    /// world's edits. The created-save writer publishes this as the chunk a
    /// brand-new save is opened with.
    pub fn generated_column(&self) -> ChunkColumn {
        self.generated.clone()
    }

    /// What the generator puts at this world position, ignoring every edit the
    /// world has made since. `None` outside the build height.
    pub fn generated_block_state(&self, x: i32, y: i32, z: i32) -> Option<i32> {
        let (_, lx) = split_world_coord(x);
        let (_, lz) = split_world_coord(z);
        match &self.source {
            ChunkSource::Debug(source) => self
                .is_inside_build_height(y)
                .then(|| source.block_state_at(x, y, z)),
            ChunkSource::Flat(_) | ChunkSource::NoiseCarvers(_) => {
                self.generated.get_block_state(lx, y, lz)
            }
        }
    }

    fn chunk(&self, chunk: (i32, i32)) -> Option<&Chunk> {
        self.chunks.get(&chunk)
    }

    fn chunk_mut(&mut self, chunk: (i32, i32)) -> Option<&mut Chunk> {
        self.chunks.get_mut(&chunk)
    }

    /// A chunk the caller has already loaded.
    fn loaded(&self, chunk: (i32, i32)) -> &Chunk {
        &self.chunks[&chunk]
    }

    fn loaded_mut(&mut self, chunk: (i32, i32)) -> &mut Chunk {
        self.chunks.get_mut(&chunk).expect("the chunk is loaded")
    }

    /// Replaces a loaded chunk's column, or loads a fresh chunk around it.
    fn put_column(&mut self, chunk: (i32, i32), column: ChunkColumn) -> &mut Chunk {
        if self.is_ordinary_source() {
            self.ordinary_lights_dirty = true;
        }
        match self.chunks.entry(chunk) {
            std::collections::hash_map::Entry::Occupied(entry) => {
                let loaded = entry.into_mut();
                loaded.set_column(column);
                loaded
            }
            std::collections::hash_map::Entry::Vacant(entry) => entry.insert(Chunk::new(column)),
        }
    }

    /// Applies `update` when the chunk is loaded.
    fn update_chunk(&mut self, chunk: (i32, i32), update: impl FnOnce(&mut Chunk)) {
        if let Some(loaded) = self.chunks.get_mut(&chunk) {
            update(loaded);
        }
    }

    fn column_at(&self, chunk: (i32, i32)) -> Option<&ChunkColumn> {
        self.chunk(chunk).map(|loaded| &loaded.column)
    }

    fn generation(&self, chunk: (i32, i32)) -> Option<&ChunkGenerationState> {
        self.chunk(chunk)?.generation.as_ref()
    }

    fn generation_mut(&mut self, chunk: (i32, i32)) -> Option<&mut ChunkGenerationState> {
        self.chunk_mut(chunk)?.generation.as_mut()
    }

    /// The generation state of a chunk the caller knows has one.
    fn state(&self, chunk: (i32, i32)) -> &ChunkGenerationState {
        self.generation(chunk)
            .expect("the chunk has a generation state")
    }

    fn set_generation(&mut self, chunk: (i32, i32), state: ChunkGenerationState) {
        if matches!(state, ChunkGenerationState::Full { .. }) {
            self.ordinary_lights_dirty = true;
        }
        let loaded = self.loaded_mut(chunk);
        loaded.cached_packet.take();
        loaded.generation = Some(state);
    }

    pub(crate) fn get_or_try_init_packet(
        &self,
        chunk: (i32, i32),
        build: impl FnOnce() -> SaveResult<std::sync::Arc<bbb_protocol::packets::LevelChunkWithLight>>,
    ) -> SaveResult<std::sync::Arc<bbb_protocol::packets::LevelChunkWithLight>> {
        let Some(loaded) = self.chunk(chunk) else {
            return build();
        };
        if let Some(cached) = loaded.cached_packet.get() {
            return Ok(std::sync::Arc::clone(cached));
        }
        let packet = build()?;
        let _ = loaded.cached_packet.set(std::sync::Arc::clone(&packet));
        Ok(packet)
    }

    fn pre_carvers(&self, chunk: (i32, i32)) -> Option<&ChunkPreCarversState> {
        self.chunk(chunk)?.pre_carvers.as_ref()
    }

    fn chunk_flag(&self, chunk: (i32, i32), flag: impl FnOnce(&Chunk) -> bool) -> bool {
        self.chunk(chunk).is_some_and(flag)
    }

    fn is_storage_backed_generation(&self, chunk: (i32, i32)) -> bool {
        self.chunk_flag(chunk, |loaded| loaded.storage_backed_generation)
    }

    fn has_whole_rewrite(&self, chunk: (i32, i32)) -> bool {
        self.chunk_flag(chunk, |loaded| loaded.whole_rewrite_epoch.is_some())
    }

    /// Saved generation the store must rewrite whole when it changes.
    fn is_durable(&self, chunk: (i32, i32)) -> bool {
        self.chunk_flag(chunk, Chunk::is_durable)
    }

    fn durable_chunks(&self) -> BTreeSet<(i32, i32)> {
        self.chunks
            .iter()
            .filter(|(_, chunk)| chunk.is_durable())
            .map(|(&key, _)| key)
            .collect()
    }

    pub fn column(&self, cx: i32, cz: i32) -> Option<&ChunkColumn> {
        self.chunks.get(&(cx, cz)).map(|chunk| &chunk.column)
    }

    pub fn column_mut(&mut self, cx: i32, cz: i32) -> Option<&mut ChunkColumn> {
        let chunk = self.chunks.get_mut(&(cx, cz))?;
        chunk.storage_backed_full = false;
        Some(chunk.column_mut())
    }

    /// The column at (cx, cz), generating and keeping it when it is not loaded
    /// yet (`ServerChunkCache.getChunk` with `ChunkStatus.FULL`).
    pub fn load_column(&mut self, cx: i32, cz: i32) -> &ChunkColumn {
        self.try_load_column(cx, cz)
            .expect("direct FULL chunk generation is infallible")
    }

    pub fn load_column_mut(&mut self, cx: i32, cz: i32) -> &mut ChunkColumn {
        self.load_column(cx, cz);
        self.column_mut(cx, cz)
            .expect("the loaded column has a transaction revision")
    }

    pub fn try_load_column(&mut self, cx: i32, cz: i32) -> SaveResult<&ChunkColumn> {
        self.prepare_generation_admission((cx, cz))?;
        self.advance_pre_carvers_column((cx, cz))?;
        if self.is_ordinary_source()
            && !self.ordinary_light_restoring
            && (!matches!(
                self.generation((cx, cz)),
                Some(ChunkGenerationState::Full { .. })
            ) || self.ordinary_light_pending())
        {
            self.ensure_ordinary_full_columns(&[(cx, cz)])?;
            return Ok(&self.loaded((cx, cz)).column);
        }
        self.try_load_column_unlit(cx, cz)
    }

    pub fn try_ensure_served_full_column(&mut self, cx: i32, cz: i32) -> SaveResult<&ChunkColumn> {
        self.require_legacy_generation_work_completed((cx, cz))?;
        if self.is_ordinary_source() {
            self.ensure_ordinary_full_columns(&[(cx, cz)])?;
            Ok(&self.loaded((cx, cz)).column)
        } else {
            self.try_ensure_noise_full_column(cx, cz)
        }
    }

    /// Atomically prepares a network delivery batch at FULL.
    pub fn try_ensure_served_full_columns(&mut self, chunks: &[(i32, i32)]) -> SaveResult<()> {
        if self.is_ordinary_source() {
            self.ensure_ordinary_full_columns(chunks)
        } else {
            self.try_ensure_noise_full_columns(chunks)
        }
    }

    pub fn is_loaded(&self, cx: i32, cz: i32) -> bool {
        self.chunks.contains_key(&(cx, cz))
    }

    pub fn inhabited_time(&self, cx: i32, cz: i32) -> Option<i64> {
        self.column(cx, cz).map(ChunkColumn::inhabited_time)
    }

    pub fn increment_inhabited_time(&mut self, cx: i32, cz: i32, delta: i64) -> bool {
        let Some(chunk) = self.chunks.get_mut(&(cx, cz)) else {
            return false;
        };
        chunk.column.increment_inhabited_time(delta);
        true
    }

    /// Replaces whatever is loaded at (cx, cz); the restore path uses it to put
    /// a saved column in front of the generated one.
    pub fn insert_column(&mut self, cx: i32, cz: i32, column: ChunkColumn) {
        self.put_restored_column((cx, cz), column);
    }

    fn put_restored_column(&mut self, chunk: (i32, i32), column: ChunkColumn) -> &mut Chunk {
        self.authorize_saved_column_hydration(chunk);
        let loaded = self.put_column(chunk, column);
        loaded.storage_backed_generation = false;
        loaded.storage_backed_full = false;
        loaded.blending = None;
        loaded.legacy_work = None;
        loaded.pre_carvers = None;
        loaded
    }

    pub fn insert_restored_column(
        &mut self,
        cx: i32,
        cz: i32,
        column: ChunkColumn,
        state: ChunkGenerationState,
    ) {
        let loaded = self.put_restored_column((cx, cz), column);
        loaded.generation = Some(state);
        loaded.pending_initial_persistence = false;
    }

    pub fn generation_state(&self, cx: i32, cz: i32) -> Option<&ChunkGenerationState> {
        self.generation((cx, cz))
    }

    pub fn is_ticking_full(&self, cx: i32, cz: i32) -> bool {
        let Some(chunk) = self.chunk((cx, cz)) else {
            return false;
        };
        if chunk.legacy_work.is_some() {
            return false;
        }
        matches!(
            (&self.source, chunk.generation.as_ref()),
            (
                ChunkSource::Flat(_) | ChunkSource::Debug(_),
                Some(ChunkGenerationState::Full { .. })
            ) | (
                ChunkSource::NoiseCarvers(_),
                Some(ChunkGenerationState::NoiseFull(_))
            )
        )
    }

    /// Advances one center after its radius-one INITIALIZE_LIGHT dependencies.
    /// `ChunkPyramid.java:36-39`; `ThreadedLevelLightEngine.java:164-180`.
    pub fn light_chunk(&mut self, center: (i32, i32)) -> SaveResult<()> {
        let mut storage = std::mem::take(&mut self.light_storage);
        let lit = self.light_noise_center(center, &mut storage);
        self.light_storage = storage;
        let initialized = lit?;
        self.loaded_mut(center).generation = Some(ChunkGenerationState::Light(ChunkLightState {
            initialized,
            is_light_correct: true,
        }));
        self.advance_pending_retrogen_status(center, Java26ChunkStatus::Light);
        Ok(())
    }

    pub fn saved_light_sections(&self, chunk: (i32, i32)) -> Vec<SavedLightSection> {
        if !matches!(
            self.generation(chunk),
            Some(
                ChunkGenerationState::InitializeLight(_)
                    | ChunkGenerationState::Light(_)
                    | ChunkGenerationState::Spawn(_)
                    | ChunkGenerationState::NoiseFull(_)
                    | ChunkGenerationState::Full { .. }
            )
        ) {
            return Vec::new();
        }
        let mut sections = self.light_storage.saved_sections(chunk);
        if !self.has_skylight {
            for section in &mut sections {
                section.sky_light = None;
            }
            sections.retain(|section| section.block_light.is_some());
        }
        sections
    }

    pub fn validate_light_persistence(&self, chunk: (i32, i32)) -> SaveResult<()> {
        if self.chunk_flag(chunk, |chunk| {
            chunk.generation.is_some() || chunk.pre_carvers.is_some()
        }) {
            return Ok(());
        }
        Err(invalid_light_persistence(
            chunk,
            "loaded column has no generation status",
        ))
    }

    pub fn structure_component(
        &self,
        cx: i32,
        cz: i32,
    ) -> Option<&bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData> {
        self.structure_status.component(cx, cz)
    }

    pub fn structure_component_or_unloaded(
        &self,
        cx: i32,
        cz: i32,
    ) -> Option<&bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData> {
        self.structure_status.component_or_unloaded(cx, cz)
    }

    #[cfg(test)]
    pub fn structure_status_counts(&self) -> (usize, usize) {
        (
            self.structure_status.starts_ready_count(),
            self.structure_status.references_ready_count(),
        )
    }

    #[cfg(test)]
    pub fn structure_status_name(&self, cx: i32, cz: i32) -> &'static str {
        match self.structure_status.status(cx, cz) {
            structure_status::NoiseStructureTaskStatus::Empty => "empty",
            structure_status::NoiseStructureTaskStatus::StructureStarts => "structure_starts",
            structure_status::NoiseStructureTaskStatus::StructureReferences => {
                "structure_references"
            }
        }
    }

    pub fn pending_initial_persistence(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.chunks
            .iter()
            .filter(|(_, chunk)| chunk.pending_initial_persistence)
            .map(|(&key, _)| key)
    }

    pub fn is_pending_initial_persistence(&self, chunk: (i32, i32)) -> bool {
        self.chunk_flag(chunk, |loaded| loaded.pending_initial_persistence)
    }

    pub fn pending_whole_rewrites(&self) -> impl Iterator<Item = WholeRewriteIntent> + '_ {
        self.chunks.iter().filter_map(|(&chunk, loaded)| {
            loaded
                .whole_rewrite_epoch
                .map(|epoch| WholeRewriteIntent { chunk, epoch })
        })
    }

    pub(crate) fn whole_rewrite_epoch(&self, chunk: (i32, i32)) -> Option<u64> {
        self.chunk(chunk)?.whole_rewrite_epoch
    }

    pub(crate) fn save_dirty_chunks(&self) -> impl Iterator<Item = ((i32, i32), u64)> + '_ {
        self.chunks
            .iter()
            .filter_map(|(&chunk, loaded)| loaded.save_dirty_epoch.map(|epoch| (chunk, epoch)))
    }

    pub(crate) fn save_dirty_epoch(&self, chunk: (i32, i32)) -> Option<u64> {
        self.chunk(chunk)?.save_dirty_epoch
    }

    pub(crate) fn saved_blending_data(&self, cx: i32, cz: i32) -> Option<&Java26BlendingData> {
        self.chunk((cx, cz))?.blending.as_ref()?.as_ref()
    }

    pub(crate) fn packed_block_entities(
        &self,
        chunk: (i32, i32),
    ) -> Option<&BTreeMap<BlockEntityKey, Vec<NbtSummaryEntry>>> {
        Some(&self.chunk(chunk)?.packed_block_entities)
    }

    pub fn is_storage_backed_noise_full(&self, cx: i32, cz: i32) -> bool {
        self.chunk_flag((cx, cz), |chunk| chunk.storage_backed_full)
    }

    fn mark_whole_rewrite_pending(&mut self, chunk: (i32, i32)) -> SaveResult<WholeRewriteIntent> {
        let ordinary = self.is_ordinary_source();
        let rewritable = self.chunk(chunk).is_some_and(|loaded| {
            loaded.is_durable()
                || (ordinary
                    && matches!(loaded.generation, Some(ChunkGenerationState::Full { .. })))
        });
        if !rewritable {
            return Err(SaveError::InvalidNbt {
                path: None,
                detail: format!(
                    "noise generation chunk [{},{}] has no durable provenance to rewrite",
                    chunk.0, chunk.1
                ),
            });
        }
        self.next_whole_rewrite_epoch =
            self.next_whole_rewrite_epoch
                .checked_add(1)
                .ok_or_else(|| SaveError::InvalidNbt {
                    path: None,
                    detail: "noise generation whole-rewrite epoch overflows".to_string(),
                })?;
        let epoch = self.next_whole_rewrite_epoch;
        let loaded = self.loaded_mut(chunk);
        loaded.storage_backed_generation = false;
        loaded.storage_backed_full = false;
        loaded.whole_rewrite_epoch = Some(epoch);
        loaded.mark_save_dirty();
        Ok(WholeRewriteIntent {
            chunk,
            epoch: self.next_whole_rewrite_epoch,
        })
    }

    /// `ChunkAccess.markUnsaved` for a change no block edit records (biomes):
    /// a stored column is rewritten whole, an unstored one is written entire.
    pub fn mark_unsaved(&mut self, chunk: (i32, i32), stored: bool) -> SaveResult<()> {
        let Some(loaded) = self.chunk(chunk) else {
            return Ok(());
        };
        if loaded.pending_initial_persistence {
            return Ok(());
        }
        if stored || loaded.is_durable() {
            self.mark_whole_rewrite_pending(chunk).map(|_| ())
        } else {
            let loaded = self.loaded_mut(chunk);
            loaded.pending_initial_persistence = true;
            loaded.mark_save_dirty();
            Ok(())
        }
    }

    pub fn acknowledge_whole_rewrite(&mut self, intent: WholeRewriteIntent) -> bool {
        if self
            .chunk(intent.chunk)
            .and_then(|chunk| chunk.whole_rewrite_epoch)
            != Some(intent.epoch)
            || self
                .validate_pending_whole_rewrite_center(intent.chunk)
                .is_err()
        {
            return false;
        }
        let loaded = self.loaded_mut(intent.chunk);
        loaded.whole_rewrite_epoch = None;
        loaded.storage_backed_generation = true;
        if matches!(loaded.generation, Some(ChunkGenerationState::NoiseFull(_))) {
            loaded.storage_backed_full = true;
        }
        true
    }

    pub fn has_noise_source(&self) -> bool {
        matches!(self.source, ChunkSource::NoiseCarvers(_))
    }

    /// World coordinates; a debug view bypasses section bounds only after the
    /// target column is loaded (`LevelChunk.java:213-226`).
    pub fn get_block_state(&self, x: i32, y: i32, z: i32) -> Option<i32> {
        let (cx, lx) = split_world_coord(x);
        let (cz, lz) = split_world_coord(z);
        if let Some(column) = self.cold_column(cx, cz) {
            return column.get_block_state(lx, y, lz);
        }
        if let Some(column) = self.column(cx, cz) {
            if let Some(source) = &self.debug_view {
                return Some(source.block_state_at(x, y, z));
            }
            return column.get_block_state(lx, y, lz);
        }
        self.read_stored_block_state(x, y, z)
    }

    pub fn final_heightmap_first_available(
        &self,
        cx: i32,
        cz: i32,
        heightmap_kind_id: i32,
        local_x: i32,
        local_z: i32,
    ) -> Option<i32> {
        let chunk = self.chunk((cx, cz))?;
        match chunk.generation.as_ref()? {
            ChunkGenerationState::NoiseFull(full) => full
                .light()
                .initialized()
                .features()
                .final_heightmaps()
                .first_available(heightmap_kind_id, local_x, local_z),
            ChunkGenerationState::Full { .. } => {
                chunk
                    .heightmaps()
                    .first_available(heightmap_kind_id, local_x, local_z)
            }
            _ => None,
        }
    }

    /// The chunk-layer cell write every block change goes through: keeps the
    /// FULL heightmaps current and queues the light check
    /// (`LevelChunk.setBlockState`). World
    /// coordinates; `None` if the column is absent or y is out of range.
    pub fn set_block_state(&mut self, x: i32, y: i32, z: i32, state: i32) -> Option<i32> {
        let (cx, lx) = split_world_coord(x);
        let (cz, lz) = split_world_coord(z);
        let chunk = self.chunks.get_mut(&(cx, cz))?;
        let previous = chunk.set_block_state(lx, y, lz, state)?;
        if previous == state {
            return Some(previous);
        }
        chunk.storage_backed_full = false;
        chunk.packed_block_entities.remove(&(x, y, z));
        if let Some(ChunkGenerationState::NoiseFull(full)) = &mut chunk.generation {
            let light = full.light_mut();
            light
                .initialized
                .features
                .final_heightmaps
                .update_after_set(&chunk.column, lx, y, lz, state);
            light.is_light_correct = false;
        }
        live_updates::record_light_change_with_registry(
            &mut self.pending_light_changes,
            (x, y, z),
            previous,
            state,
            self.content_registry.as_deref(),
        );
        Some(previous)
    }

    /// Installs saved sections onto a loaded column.
    pub fn restore_section_blocks(
        &mut self,
        cx: i32,
        cz: i32,
        sections: Vec<(usize, SectionBlocks)>,
    ) -> Option<()> {
        let chunk = self.chunks.get_mut(&(cx, cz))?;
        chunk.storage_backed_full = false;
        let column = chunk.column_mut();
        for (index, blocks) in sections {
            column.replace_section_blocks(index, blocks);
        }
        Some(())
    }

    /// Live cells that differ from a deterministic flat/debug generator.
    pub fn changed_blocks(&self) -> impl Iterator<Item = ((i32, i32, i32), i32)> + '_ {
        let enabled = match &self.source {
            ChunkSource::NoiseCarvers(_) => false,
            ChunkSource::Flat(_) => {
                self.ordinary_structures.is_none() || !self.structure_status.generate_structures()
            }
            ChunkSource::Debug(_) => true,
        };
        self.chunks
            .iter()
            .filter(move |(_, chunk)| {
                enabled && matches!(chunk.generation, Some(ChunkGenerationState::Full { .. }))
            })
            .flat_map(move |(&(cx, cz), chunk)| {
                let is_flat = matches!(self.source, ChunkSource::Flat(_));
                (0..chunk.column.section_count())
                    .filter(move |&index| {
                        !is_flat || chunk.column.section(index) != self.generated.section(index)
                    })
                    .flat_map(move |index| {
                        let min_y = self.min_y() + index as i32 * SECTION_SIDE as i32;
                        let live = chunk.column.section(index);
                        (0..SECTION_SIDE).flat_map(move |ly| {
                            (0..SECTION_SIDE).flat_map(move |lz| {
                                (0..SECTION_SIDE).filter_map(move |lx| {
                                    let state = live.get(lx, ly, lz);
                                    let x = cx * SECTION_SIDE as i32 + lx as i32;
                                    let y = min_y + ly as i32;
                                    let z = cz * SECTION_SIDE as i32 + lz as i32;
                                    let generated = if is_flat {
                                        Some(self.generated.section(index).get(lx, ly, lz))
                                    } else {
                                        self.generated_block_state(x, y, z)
                                    };
                                    (generated != Some(state)).then_some(((x, y, z), state))
                                })
                            })
                        })
                    })
            })
    }

    pub(crate) fn acknowledge_save_dirty(&mut self, key: (i32, i32), epoch: u64) {
        if let Some(chunk) = self.chunks.get_mut(&key) {
            if chunk.save_dirty_epoch == Some(epoch) {
                chunk.save_dirty_epoch = None;
                chunk.derived_dirty = false;
            }
        }
    }

    pub(crate) fn acknowledge_full_chunk_save(&mut self, key: (i32, i32)) {
        let rewrite_epoch = self.chunk(key).and_then(|chunk| chunk.whole_rewrite_epoch);
        if let Some(epoch) = rewrite_epoch {
            let _ = self.acknowledge_whole_rewrite(WholeRewriteIntent { chunk: key, epoch });
        }
        if let Some(chunk) = self.chunks.get_mut(&key) {
            chunk.save_dirty_epoch = None;
            chunk.derived_dirty = false;
        }
    }

    pub fn proto_entity_records(&self, chunk_x: i32, chunk_z: i32) -> &[Vec<NbtSummaryEntry>] {
        self.chunk((chunk_x, chunk_z))
            .map(|chunk| chunk.proto_entities.as_slice())
            .unwrap_or_default()
    }

    pub fn published_worldgen_entity_records(
        &self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> &[Vec<NbtSummaryEntry>] {
        self.chunk((chunk_x, chunk_z))
            .map(|chunk| chunk.published_worldgen_entities.as_slice())
            .unwrap_or_default()
    }

    /// The ProtoChunk and FULL-published worldgen entity records still outside
    /// the entity store.
    pub fn worldgen_entity_records_mut(
        &mut self,
        chunk: (i32, i32),
    ) -> Option<[&mut Vec<Vec<NbtSummaryEntry>>; 2]> {
        let loaded = self.chunk_mut(chunk)?;
        Some([
            &mut loaded.proto_entities,
            &mut loaded.published_worldgen_entities,
        ])
    }

    pub fn take_published_worldgen_entity_records(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Vec<Vec<NbtSummaryEntry>> {
        self.chunk_mut((chunk_x, chunk_z))
            .map(|chunk| std::mem::take(&mut chunk.published_worldgen_entities))
            .unwrap_or_default()
    }

    pub fn column_count(&self) -> usize {
        self.chunks.len()
    }

    /// Every loaded column's chunk position.
    ///
    /// `ChunkMap.visibleChunkMap`'s key set, which is what the tick loop walks
    /// to find the columns it may simulate.
    pub fn loaded_chunks(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.chunks.keys().copied()
    }
}

/// World coordinate -> (chunk coordinate, chunk-local coordinate); e.g.
/// -1 -> (-1, 15).
fn split_world_coord(coord: i32) -> (i32, i32) {
    (
        coord.div_euclid(SECTION_SIDE as i32),
        coord.rem_euclid(SECTION_SIDE as i32),
    )
}

fn carvers_region_coordinates(target_chunk: [i32; 2]) -> SaveResult<[[i32; 2]; 9]> {
    let mut coordinates = [[0; 2]; 9];
    for (index, coordinate) in coordinates.iter_mut().enumerate() {
        let offset_x = i32::try_from(index % 3).expect("region x index fits i32") - 1;
        let offset_z = i32::try_from(index / 3).expect("region z index fits i32") - 1;
        coordinate[0] = target_chunk[0].checked_add(offset_x).ok_or_else(|| {
            invalid_carvers_region(target_chunk, "CARVERS region chunk x overflows Java int")
        })?;
        coordinate[1] = target_chunk[1].checked_add(offset_z).ok_or_else(|| {
            invalid_carvers_region(target_chunk, "CARVERS region chunk z overflows Java int")
        })?;
    }
    Ok(coordinates)
}

fn invalid_carvers_region(target_chunk: [i32; 2], detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!(
            "CARVERS region [{},{}]: {}",
            target_chunk[0],
            target_chunk[1],
            detail.into()
        ),
    }
}

fn invalid_noise_source(
    program: &NoiseFinalDensityProgram,
    detail: impl Into<String>,
) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!(
            "noise source {} / {}: {}",
            program.dimension_type_id(),
            program.settings_id(),
            detail.into()
        ),
    }
}

fn invalid_light_persistence(chunk: (i32, i32), detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: format!(
            "LIGHT persistence [{},{}]: {}",
            chunk.0,
            chunk.1,
            detail.into()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::block_state_types::vanilla_block_default_state_id;
    use bbb_protocol::block_types::{VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_STONE_ID};
    use bbb_protocol::codec::Encoder;
    use bbb_server_types::chunk_data::{
        ChunkFeaturesState, ChunkInitializeLightState, ChunkSection, SECTION_VOLUME,
    };
    use bbb_server_types::chunk_ticks::BlockTickType;

    const MIN_Y: i32 = -64;
    const SECTIONS: usize = 24;
    const AIR: i32 = 7;
    const STONE: i32 = 11;
    const BIOME: i32 = 3;

    fn fresh_column() -> ChunkColumn {
        ChunkColumn::new(MIN_Y, SECTIONS, AIR, BIOME)
    }

    #[test]
    fn new_column_is_uniform_air_with_bounds_checked() {
        let mut column = fresh_column();
        assert_eq!(column.get_block_state(0, MIN_Y, 0), Some(AIR));
        assert_eq!(column.get_block_state(15, MIN_Y + 384 - 1, 15), Some(AIR));
        // y bounds: -64 and 319 edges valid, outside returns None.
        assert_eq!(column.get_block_state(0, MIN_Y - 1, 0), None);
        assert_eq!(column.get_block_state(0, MIN_Y + 384, 0), None);
        // Local x/z outside 0..16 return None.
        assert_eq!(column.get_block_state(16, 0, 0), None);
        assert_eq!(column.get_block_state(-1, 0, 0), None);
        assert_eq!(column.get_block_state(0, 0, 16), None);
        assert_eq!(column.set_block_state(0, MIN_Y - 1, 0, STONE), None);
        assert_eq!(column.set_block_state(0, MIN_Y + 384, 0, STONE), None);
        assert_eq!(column.set_block_state(16, 0, 0, STONE), None);

        // Round-trip across sections at both y edges.
        assert_eq!(column.set_block_state(3, MIN_Y, 5, STONE), Some(AIR));
        assert_eq!(column.get_block_state(3, MIN_Y, 5), Some(STONE));
        assert_eq!(column.set_block_state(15, MIN_Y + 319, 15, 99), Some(AIR));
        assert_eq!(column.get_block_state(15, MIN_Y + 319, 15), Some(99));
        assert_eq!(column.set_block_state(3, MIN_Y, 5, 42), Some(STONE));
        assert_eq!(column.get_block_state(3, MIN_Y, 5), Some(42));
        // Every untouched cell stays air.
        assert_eq!(column.get_block_state(0, MIN_Y, 0), Some(AIR));
        assert_eq!(column.get_block_state(14, MIN_Y + 319, 15), Some(AIR));
    }

    #[test]
    fn first_diverging_set_converts_single_to_indirect() {
        let mut column = fresh_column();
        assert_eq!(column.set_block_state(1, 0, 1, STONE), Some(AIR));
        match column.section(4) {
            SectionBlocks::Indirect {
                bits,
                palette,
                data,
            } => {
                assert_eq!(*bits, 4);
                assert_eq!(palette, &vec![AIR, STONE]);
                assert_eq!(data.len(), 256);
            }
            other => panic!("expected indirect section, got {other:?}"),
        }
        // Neighbouring cells still read the original value.
        assert_eq!(column.get_block_state(0, 0, 0), Some(AIR));
        assert_eq!(column.get_block_state(1, 0, 1), Some(STONE));
        // Same-value set keeps the section single and returns the value.
        let mut flat = ChunkColumn::new(MIN_Y, SECTIONS, STONE, BIOME);
        assert_eq!(flat.set_block_state(2, MIN_Y, 2, STONE), Some(STONE));
        assert_eq!(flat.section(0), &SectionBlocks::Single(STONE));
    }

    #[test]
    fn palette_grows_bits_and_reads_back_exact() {
        let mut column = fresh_column();
        // 17 distinct states (AIR + 16 new) -> 5 bits.
        for i in 0..16 {
            let state = 1000 + i;
            column.set_block_state(i, MIN_Y, 0, state);
        }
        match column.section(0) {
            SectionBlocks::Indirect { bits, .. } => assert_eq!(*bits, 5),
            other => panic!("expected indirect section, got {other:?}"),
        }
        for i in 0..16 {
            assert_eq!(column.get_block_state(i, MIN_Y, 0), Some(1000 + i));
        }

        // 300 distinct states -> ceillog2(301) = 9 bits. A y layer holds only
        // 256 cells, so the states span two layers of the same section.
        let mut column = fresh_column();
        for i in 0..300 {
            let cell = i % 256;
            let (x, y, z) = ((cell % 16), MIN_Y + (i / 256), (cell / 16));
            column.set_block_state(x, y, z, 2000 + i);
        }
        match column.section(0) {
            SectionBlocks::Indirect { bits, palette, .. } => {
                assert_eq!(*bits, 9);
                assert_eq!(palette.len(), 301);
            }
            other => panic!("expected indirect section, got {other:?}"),
        }
        for i in 0..300 {
            let cell = i % 256;
            let (x, y, z) = ((cell % 16), MIN_Y + (i / 256), (cell / 16));
            assert_eq!(column.get_block_state(x, y, z), Some(2000 + i));
        }
        assert_eq!(column.get_block_state(15, MIN_Y + 2, 15), Some(AIR));
    }

    #[test]
    fn global_wire_form_round_trips_wide_sections() {
        // 300 distinct states in one section: past 256 the wire switches to
        // the global palette form (Strategy.java:44) — bits byte 15, no
        // palette list, 1024 data words of global ids.
        let mut column = fresh_column();
        for i in 0..300 {
            let cell = i % 256;
            let (x, y, z) = ((cell % 16), MIN_Y + (i / 256), (cell / 16));
            column.set_block_state(x, y, z, 1000 + i);
        }
        let bytes = column.encode_section_data();
        assert_eq!(bytes[4], 15, "first section uses the global wire form");
        let decoded = ChunkColumn::decode_section_data(MIN_Y, &bytes).unwrap();
        for i in 0..300 {
            let cell = i % 256;
            let (x, y, z) = ((cell % 16), MIN_Y + (i / 256), (cell / 16));
            assert_eq!(decoded.get_block_state(x, y, z), Some(1000 + i));
        }
        assert_eq!(decoded.get_block_state(15, MIN_Y + 2, 15), Some(AIR));
        // Re-encode keeps the global form (the palette still holds 301 ids).
        assert_eq!(decoded.encode_section_data(), bytes);

        // Exactly 256 distinct states (AIR plus 255 writes) stay on the
        // 8-bit indirect form.
        let mut column = fresh_column();
        for i in 0..255 {
            column.set_block_state(i % 16, MIN_Y, i / 16, 2000 + i);
        }
        let bytes = column.encode_section_data();
        assert_eq!(bytes[4], 8, "256 distinct states stay indirect");
        let decoded = ChunkColumn::decode_section_data(MIN_Y, &bytes).unwrap();
        for i in 0..255 {
            assert_eq!(
                decoded.get_block_state(i % 16, MIN_Y, i / 16),
                Some(2000 + i)
            );
        }
    }

    #[test]
    fn a_single_value_flat_column_round_trips_its_wire_bytes() {
        let air = vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).unwrap();
        let stone = vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).unwrap();
        // The wire form a flat column takes once a save has been through
        // `PalettedContainer.unpack` (PalettedContainer.java:182-210), which
        // collapses a uniform section back to a single-value palette: i16
        // non-air count, i16 fluid count 0, u8 0 + varint state, u8 0 + varint
        // biome, for each of the 24 overworld sections.
        let mut expected = Encoder::new();
        for section in 0..SECTIONS {
            let section_min_y = MIN_Y + section as i32 * 16;
            let solid = section_min_y <= 63;
            expected.write_i16(if solid { 4096 } else { 0 });
            expected.write_i16(0);
            expected.write_u8(0);
            expected.write_var_i32(if solid { stone } else { air });
            expected.write_u8(0);
            expected.write_var_i32(0);
        }
        let expected = expected.into_inner();

        let column = ChunkColumn::decode_section_data(MIN_Y, &expected).unwrap();
        assert_eq!(column.section_count(), SECTIONS);
        assert_eq!(column.air_state_id(), air);
        assert_eq!(column.get_block_state(0, 63, 0), Some(stone));
        assert_eq!(column.get_block_state(15, -64, 15), Some(stone));
        assert_eq!(column.get_block_state(0, 64, 0), Some(air));
        assert_eq!(column.get_block_state(0, 319, 0), Some(air));
        // Encoding the decoded flat column reproduces the same bytes.
        assert_eq!(column.encode_section_data(), expected);
    }

    struct XorShift(u64);

    impl XorShift {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
    }

    #[test]
    fn encode_decode_round_trip_randomized_column() {
        let mut column = fresh_column();
        let mut rng = XorShift(0x9e37_79b9_7f4a_7c15);
        // Wide state pool exercises multi-bit palettes; y pool covers the
        // full column height.
        for _ in 0..20_000 {
            let x = (rng.next() % 16) as i32;
            let z = (rng.next() % 16) as i32;
            let y = MIN_Y + (rng.next() % 384) as i32;
            let state = (rng.next() % 40) as i32;
            column.set_block_state(x, y, z, state);
        }
        // Force one section past 256 distinct states for wide-bit coverage.
        for i in 0..300 {
            let cell = i % 256;
            let (x, y, z) = ((cell % 16), MIN_Y + 100 + (i / 256), (cell / 16));
            column.set_block_state(x, y, z, 5000 + i);
        }

        let bytes = column.encode_section_data();
        let decoded = ChunkColumn::decode_section_data(MIN_Y, &bytes).unwrap();
        assert_eq!(decoded.section_count(), SECTIONS);
        assert_eq!(decoded.biome_id(), Some(BIOME));
        for y in MIN_Y..MIN_Y + 384 {
            for x in 0..16 {
                for z in 0..16 {
                    assert_eq!(
                        decoded.get_block_state(x, y, z),
                        column.get_block_state(x, y, z),
                        "mismatch at ({x}, {y}, {z})"
                    );
                }
            }
        }
        // Byte-identical re-encode after decode is covered by the flat-fixture
        // test; here the air id falls back (no all-air section), so only
        // block states, palettes, and biome survive the round trip.
    }

    fn flat_store() -> ChunkStore {
        ChunkStore::new(
            crate::world_creation::default_overworld_level_source().unwrap(),
            BIOME,
        )
    }

    #[test]
    fn ticking_full_status_is_source_specific() {
        let features = ChunkFeaturesState {
            postprocessing_positions: Vec::new(),
            carving_mask: Vec::new(),
            final_heightmaps: FeatureHeightmaps::prime(&fresh_column()),
        };
        let initialized = ChunkInitializeLightState {
            features: features.clone(),
            lowest_sky_source_y: [0; 16 * 16],
            non_empty_sections: Vec::new(),
        };
        let light = ChunkLightState {
            initialized: initialized.clone(),

            is_light_correct: true,
        };
        let spawn = bbb_server_types::chunk_data::spawn::ChunkSpawnState::from_light(
            light.clone(),
            false,
            true,
        );
        let statuses = [
            ChunkGenerationState::Full {
                surface_y: 63,
                postprocessing_positions: Vec::new(),
            },
            ChunkGenerationState::Carvers {
                postprocessing_positions: Vec::new(),
                carving_mask: Vec::new(),
            },
            ChunkGenerationState::Features(features),
            ChunkGenerationState::InitializeLight(initialized),
            ChunkGenerationState::Light(light),
            ChunkGenerationState::Spawn(spawn.clone()),
            ChunkGenerationState::NoiseFull(
                bbb_server_types::chunk_data::full::ChunkNoiseFullState::from_spawn(spawn),
            ),
        ];
        let mut flat = flat_store();
        let mut noise = ChunkStore::new_overworld_carvers_with_structure_generation(
            noise_generator::overworld_program(true, 0),
            false,
        )
        .unwrap();

        assert!(!flat.is_ticking_full(0, 0));
        assert!(!noise.is_ticking_full(0, 0));
        flat.insert_column(0, 0, fresh_column());
        noise.insert_column(0, 0, fresh_column());
        for status in statuses {
            let flat_expected = matches!(&status, ChunkGenerationState::Full { .. });
            let noise_expected = matches!(&status, ChunkGenerationState::NoiseFull(_));
            flat.set_generation((0, 0), status.clone());
            noise.set_generation((0, 0), status);
            assert_eq!(flat.is_ticking_full(0, 0), flat_expected);
            assert_eq!(noise.is_ticking_full(0, 0), noise_expected);
        }
    }

    #[test]
    fn chunk_store_maps_world_coordinates() {
        let (_, stone, ..) = vanilla_states();
        let mut store = flat_store();
        let floor = store.generated_block_state(0, MIN_Y, 0).unwrap();
        store.load_column(-1, -1);
        store.load_column(0, 0);
        assert_eq!(store.column_count(), 2);

        // x = -1 -> chunk -1, local 15.
        assert_eq!(store.set_block_state(-1, MIN_Y, -1, 4242), Some(floor));
        assert_eq!(store.get_block_state(-1, MIN_Y, -1), Some(4242));
        assert_eq!(
            store.column(-1, -1).unwrap().get_block_state(15, MIN_Y, 15),
            Some(4242)
        );
        // Positive boundary: x = 16 -> chunk 1, local 0 (unloaded -> None; the
        // store only generates where a caller asks it to).
        assert_eq!(store.get_block_state(16, MIN_Y, 0), None);
        assert_eq!(store.set_block_state(16, MIN_Y, 0, stone), None);
        // Outside the build height nothing is stored at all.
        assert_eq!(store.set_block_state(0, MIN_Y + 384, 0, stone), None);
        assert_eq!(store.set_block_state(0, MIN_Y, 0, 4242), Some(floor));
        assert_eq!(store.get_block_state(0, MIN_Y, 0), Some(4242));

        // insert_column replaces an existing column.
        store.insert_column(0, 0, store.generated_column());
        assert_eq!(store.get_block_state(0, MIN_Y, 0), Some(floor));
        assert_eq!(store.column_count(), 2);
    }

    #[test]
    fn biome_id_at_quart_selects_negative_chunks_and_local_quarts() {
        let mut store = flat_store();
        let quart_y = MIN_Y.div_euclid(4);
        let mut negative = ChunkColumn::new(MIN_Y, SECTIONS, AIR, 10);
        negative.set_noise_biome(3, quart_y, 3, 11).unwrap();
        store.insert_column(-1, -1, negative);
        store.insert_column(0, 0, ChunkColumn::new(MIN_Y, SECTIONS, AIR, 20));

        assert_eq!(store.biome_id_at_quart(-1, quart_y, -1), Some(11));
        assert_eq!(store.biome_id_at_quart(-4, quart_y, -4), Some(10));
        assert_eq!(store.biome_id_at_quart(0, quart_y, 0), Some(20));
        assert_eq!(store.biome_id_at_quart(-5, quart_y, -1), None);
        assert_eq!(store.biome_id_at(-1, MIN_Y, -1), Some(11));
    }

    #[test]
    fn biome_id_at_quart_clamps_to_column_height() {
        let mut store = flat_store();
        let min_quart_y = MIN_Y.div_euclid(4);
        let max_quart_y = min_quart_y + SECTIONS as i32 * 4 - 1;
        let mut column = ChunkColumn::new(MIN_Y, SECTIONS, AIR, 10);
        column.set_noise_biome(0, min_quart_y, 0, 21).unwrap();
        column.set_noise_biome(0, max_quart_y, 0, 22).unwrap();
        store.insert_column(0, 0, column);

        assert_eq!(store.biome_id_at_quart(0, i32::MIN, 0), Some(21));
        assert_eq!(store.biome_id_at_quart(0, min_quart_y - 1, 0), Some(21));
        assert_eq!(store.biome_id_at_quart(0, max_quart_y + 1, 0), Some(22));
        assert_eq!(store.biome_id_at_quart(0, i32::MAX, 0), Some(22));
        assert_eq!(store.biome_id_at(0, i32::MIN, 0), Some(21));
        assert_eq!(store.biome_id_at(0, i32::MAX, 0), Some(22));
    }

    #[test]
    fn biome_id_at_quart_in_chunk_keeps_the_selected_column() {
        let mut store = flat_store();
        let min_quart_y = MIN_Y.div_euclid(4);
        let max_quart_y = min_quart_y + SECTIONS as i32 * 4 - 1;
        let mut start = ChunkColumn::new(MIN_Y, SECTIONS, AIR, 10);
        start.set_noise_biome(1, min_quart_y, 1, 30).unwrap();
        start.set_noise_biome(1, max_quart_y, 1, 31).unwrap();
        let mut adjacent = ChunkColumn::new(MIN_Y, SECTIONS, AIR, 20);
        adjacent.set_noise_biome(1, min_quart_y, 1, 40).unwrap();
        store.insert_column(0, 0, start);
        store.insert_column(1, -1, adjacent);

        assert_eq!(store.biome_id_at_quart(5, min_quart_y, -3), Some(40));
        assert_eq!(
            store.biome_id_at_quart_in_chunk(0, 0, 5, min_quart_y, -3),
            Some(30)
        );
        assert_eq!(
            store.biome_id_at_quart_in_chunk(0, 0, 5, i32::MAX, -3),
            Some(31)
        );
    }

    #[test]
    fn an_unloaded_column_is_generated_on_demand() {
        let mut store = flat_store();
        assert_eq!(store.column_count(), 0);
        assert!(!store.is_loaded(7, -3));
        assert_eq!(store.get_block_state(7 * 16, MIN_Y, -3 * 16), None);

        // Every column is the same flat stack, wherever it is asked for.
        let floor = store.generated_block_state(0, MIN_Y, 0).unwrap();
        let air = store.air_state_id();
        for (cx, cz) in [(0, 0), (7, -3), (-1_000_000, 999_999)] {
            store.load_column(cx, cz);
            assert!(store.is_loaded(cx, cz));
            let column = store.column(cx, cz).unwrap();
            assert_eq!(column.min_y(), store.min_y());
            assert_eq!(column.section_count(), store.section_count());
            assert_eq!(column.get_block_state(0, MIN_Y, 0), Some(floor));
            assert_eq!(column.get_block_state(15, MIN_Y + 383, 15), Some(air));
        }
        assert_eq!(store.column_count(), 3);

        // Loading twice keeps the edits the first load collected.
        store.set_block_state(7 * 16, MIN_Y, -3 * 16, 4242);
        store.load_column(7, -3);
        assert_eq!(store.get_block_state(7 * 16, MIN_Y, -3 * 16), Some(4242));

        // The generated answer is the base an edit is measured against, and it
        // never moves with the world.
        assert_eq!(
            store.generated_block_state(7 * 16, MIN_Y, -3 * 16),
            Some(floor)
        );
        assert_eq!(store.generated_block_state(0, MIN_Y + 384, 0), None);
        assert!(store.is_inside_build_height(MIN_Y));
        assert!(store.is_inside_build_height(MIN_Y + 383));
        assert!(!store.is_inside_build_height(MIN_Y - 1));
        assert!(!store.is_inside_build_height(MIN_Y + 384));
    }

    /// The Vanilla state ids the ticking-counter tests use. They are derived
    /// from the block registry, never spelled out, and every one of them is a
    /// *state*: `isRandomlyTicking` is cached per state
    /// (BlockBehaviour.java:506), so two states of one block routinely
    /// disagree.
    fn vanilla_states() -> (i32, i32, i32, i32, i32, i32, i32, i32) {
        use bbb_protocol::block_types::{
            VANILLA_BLOCK_GRASS_BLOCK_ID, VANILLA_BLOCK_LAVA_ID, VANILLA_BLOCK_OAK_LEAVES_ID,
            VANILLA_BLOCK_WATER_ID, VANILLA_BLOCK_WHEAT_ID,
        };
        let state = |block| vanilla_block_default_state_id(block).unwrap();
        let wheat = state(VANILLA_BLOCK_WHEAT_ID);
        let leaves = state(VANILLA_BLOCK_OAK_LEAVES_ID);
        (
            state(VANILLA_BLOCK_AIR_ID),
            state(VANILLA_BLOCK_STONE_ID),
            state(VANILLA_BLOCK_GRASS_BLOCK_ID),
            state(VANILLA_BLOCK_LAVA_ID),
            state(VANILLA_BLOCK_WATER_ID),
            // `wheat[age=0]`, then `wheat[age=7]`: states of one block run
            // contiguously, and `age` is the only property.
            wheat,
            wheat + 7,
            // `oak_leaves` default is `distance=7,persistent=false`; two ids
            // below it is the `persistent=true` state of the same distance.
            leaves - 2,
        )
    }

    #[test]
    fn section_ticking_counters_follow_vanilla_per_state_random_ticking() {
        let (air, stone, grass, lava, water, wheat_young, wheat_ripe, leaves_persistent) =
            vanilla_states();
        let mut column = ChunkColumn::new(MIN_Y, SECTIONS, air, BIOME);
        let section = 8;
        let y = MIN_Y + section as i32 * 16;
        let counts = |column: &ChunkColumn| {
            let entry = column.chunk_section(section);
            (entry.ticking_block_count(), entry.ticking_fluid_count())
        };

        // A fresh air column ticks nowhere.
        assert_eq!(counts(&column), (0, 0));
        assert!(!column.chunk_section(section).is_randomly_ticking());

        // `grass_block` randomly ticks; `stone` does not, so replacing it
        // takes the counter back down.
        column.set_block_state(0, y, 0, grass);
        assert_eq!(counts(&column), (1, 0));
        assert!(column.chunk_section(section).is_randomly_ticking_blocks());
        assert!(!column.chunk_section(section).is_randomly_ticking_fluids());
        column.set_block_state(0, y, 0, stone);
        assert_eq!(counts(&column), (0, 0));

        // Lava is the one block whose fluid randomly ticks too
        // (LavaFluid.java:221-223 is the only override), so it lands in both
        // counters; water lands in neither.
        column.set_block_state(1, y, 0, lava);
        assert_eq!(counts(&column), (1, 1));
        assert!(column.chunk_section(section).is_randomly_ticking());
        assert!(column.chunk_section(section).is_randomly_ticking_fluids());
        column.set_block_state(1, y, 0, water);
        assert_eq!(counts(&column), (0, 0));

        // Two states of one block disagree: `CropBlock.isRandomlyTicking` is
        // `age < MAX_AGE`, and `LeavesBlock` needs `!persistent`. A
        // block-keyed table would get both of these wrong.
        column.set_block_state(2, y, 0, wheat_young);
        assert_eq!(counts(&column), (1, 0));
        column.set_block_state(2, y, 0, wheat_ripe);
        assert_eq!(counts(&column), (0, 0));
        column.set_block_state(2, y, 0, leaves_persistent);
        assert_eq!(counts(&column), (0, 0));

        // Counting is per cell, and a same-value write changes nothing.
        for x in 0..5 {
            column.set_block_state(x, y + 1, 3, grass);
        }
        assert_eq!(counts(&column), (5, 0));
        column.set_block_state(4, y + 1, 3, grass);
        assert_eq!(counts(&column), (5, 0));
        // Other sections are untouched.
        assert_eq!(column.chunk_section(section - 1).ticking_block_count(), 0);
    }

    #[test]
    fn section_ticking_counters_are_recovered_from_stored_blocks() {
        let (air, _stone, grass, lava, ..) = vanilla_states();
        // A uniform section: every one of the 4096 cells counts.
        let uniform = ChunkSection::new(SectionBlocks::Single(lava));
        assert_eq!(uniform.ticking_block_count(), SECTION_VOLUME as u16);
        assert_eq!(uniform.ticking_fluid_count(), SECTION_VOLUME as u16);
        assert_eq!(
            ChunkSection::new(SectionBlocks::Single(air)).ticking_block_count(),
            0
        );

        // Decoding wire bytes rebuilds the counters, exactly as vanilla runs
        // `recalcBlockCounts` for a section that arrives from storage
        // (LevelChunkSection.java:33-35).
        let mut column = ChunkColumn::new(MIN_Y, SECTIONS, air, BIOME);
        column.set_block_state(0, MIN_Y, 0, grass);
        column.set_block_state(1, MIN_Y, 0, lava);
        column.set_block_state(2, MIN_Y, 0, lava);
        assert_eq!(column.chunk_section(0).ticking_block_count(), 3);
        assert_eq!(column.chunk_section(0).ticking_fluid_count(), 2);

        let decoded =
            ChunkColumn::decode_section_data(MIN_Y, &column.encode_section_data()).unwrap();
        assert_eq!(decoded.chunk_section(0).ticking_block_count(), 3);
        assert_eq!(decoded.chunk_section(0).ticking_fluid_count(), 2);
        assert!(decoded.chunk_section(0).is_randomly_ticking());
        assert!(!decoded.chunk_section(1).is_randomly_ticking());
    }

    #[test]
    fn tick_containers_hang_off_the_column_and_stay_out_of_the_section_wire() {
        use bbb_protocol::block_types::VANILLA_BLOCK_REDSTONE_WIRE_ID;
        use bbb_protocol::packets::BlockPos;

        use bbb_server_types::chunk_ticks::ScheduledTick;

        let (air, stone, ..) = vanilla_states();
        let mut column = ChunkColumn::new(MIN_Y, SECTIONS, air, BIOME);
        assert!(column.block_ticks().is_empty());
        assert!(column.fluid_ticks().is_empty());

        let wire = BlockTickType::of_block_type(VANILLA_BLOCK_REDSTONE_WIRE_ID).unwrap();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        assert!(column
            .block_ticks_mut()
            .schedule(ScheduledTick::new(wire, pos, 40, 0)));
        assert!(column.block_ticks().has_scheduled_tick(pos, wire));
        assert_eq!(column.block_ticks().count(), 1);

        // Ticks are chunk state, not section state: scheduling one changes no
        // wire byte, and a decoded column starts with none.
        let bytes = column.encode_section_data();
        column.set_block_state(3, MIN_Y, 5, stone);
        let with_block = column.encode_section_data();
        assert_ne!(bytes, with_block, "a block edit does move the wire bytes");
        assert_eq!(
            bytes,
            ChunkColumn::new(MIN_Y, SECTIONS, air, BIOME).encode_section_data()
        );
        let decoded = ChunkColumn::decode_section_data(MIN_Y, &with_block).unwrap();
        assert!(decoded.block_ticks().is_empty());
    }
}

#[cfg(test)]
mod biome_source_tests;
