//! Flat stem settings and dimension-bound base terrain.
//! Non-motion-blocking layers are deferred to the shared FEATURES region.

use bbb_protocol::{
    block_state_types::vanilla_block_default_state_id,
    block_types::{vanilla_block_type, vanilla_block_type_for_resource_id, VANILLA_BLOCK_AIR_ID},
    heightmap_types::{
        VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID,
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID,
    },
    NbtSummaryValue,
};

use std::collections::BTreeMap;

use crate::dimension_type::{builtin_dimension_type_shape, dimension_type_shape};
use crate::dynamic_registry::DatapackGeneration;
use bbb_registry_codec::registry_codec::DimensionTypeScalars;
use bbb_server_types::chunk_data::ChunkColumn;
use bbb_server_types::fluid::{
    block_state_blocks_motion, block_state_fluid_state, block_state_is_air,
};
use bbb_server_types::save::{Java26LevelKey, Java26LevelStem, SaveResult};
use bbb_server_types::schema_support::{bool_or, field, int_or, invalid_schema, list_or_empty};

const SCHEMA: &str = "flat_generator";

/// The `ChunkGenerators.java:10-14` dispatch key this module owns.
pub const FLAT_GENERATOR_ID: &str = "minecraft:flat";

/// `Biomes.PLAINS`, which `FlatLevelGeneratorSettings.getBiome`
/// (`FlatLevelGeneratorSettings.java:87-94`) falls back to when the settings
/// name no biome at all.
pub const DEFAULT_FLAT_BIOME: &str = "minecraft:plains";

/// `FlatLevelSource.getSeaLevel` (`FlatLevelSource.java:150-152`) is a literal.
const FLAT_SEA_LEVEL: i32 = -63;

/// The section size a chunk column is cut into.
const SECTION_HEIGHT: i32 = 16;

/// One entry of the settings' `layers` list (`FlatLayerInfo.java:11-29`).
///
/// The block is stored already resolved: vanilla's codec reads a `Block` and
/// `getBlockState` hands out that block's default state, so a layer never
/// carries properties of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatLayerInfo {
    height: i32,
    block: &'static str,
    block_state_id: i32,
}

impl FlatLayerInfo {
    /// The declared repeat count, `0..=DimensionTypeScalars::Y_SIZE`
    /// (`FlatLayerInfo.java:13`).
    pub const fn height(&self) -> i32 {
        self.height
    }

    /// The resource id of the block this layer resolved to, which is
    /// `minecraft:air` whenever the declared id is not a built-in block.
    pub const fn block(&self) -> &'static str {
        self.block
    }

    /// `FlatLayerInfo.getBlockState` (`FlatLayerInfo.java:29-31`) as a global
    /// block-state id.
    pub const fn block_state_id(&self) -> i32 {
        self.block_state_id
    }

    fn parse(value: &NbtSummaryValue) -> Option<Self> {
        let NbtSummaryValue::Compound(fields) = value else {
            return None;
        };
        let height = int_or(fields, "height", 0).clamp(0, DimensionTypeScalars::Y_SIZE);
        let block_type_id = match field(fields, "block") {
            Some(NbtSummaryValue::String(id)) => id
                .try_to_string()
                .ok()
                .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())
                .and_then(|id| vanilla_block_type_for_resource_id(&id)),
            _ => None,
        }
        .map_or(VANILLA_BLOCK_AIR_ID, |entry| entry.id);
        Some(Self {
            height,
            block: vanilla_block_type(block_type_id)?.resource_id,
            block_state_id: vanilla_block_default_state_id(block_type_id)?,
        })
    }
}

/// Which structure sets a flat world is allowed to place
/// (`FlatLevelGeneratorSettings.java:38`).
///
/// `RegistryCodecs.homogeneousList` decodes through `HolderSetCodec`
/// (`HolderSetCodec.java:39-40`), whose persistent form is either a `#tag`
/// string or the element list, and `ExtraCodecs.compactListCodec` lets that
/// list collapse to a single bare id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlatStructureOverrides {
    Tag(String),
    Sets(Vec<String>),
}

/// `FlatLevelGeneratorSettings` (`FlatLevelGeneratorSettings.java:34-60`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatLevelGeneratorSettings {
    structure_overrides: Option<FlatStructureOverrides>,
    layers_info: Vec<FlatLayerInfo>,
    add_lakes: bool,
    decoration: bool,
    biome: String,
    layers: Vec<i32>,
    void_gen: bool,
}

impl FlatLevelGeneratorSettings {
    pub fn from_level_stem(stem: &Java26LevelStem, path: &str) -> SaveResult<Self> {
        let fields = match &stem.raw_generator {
            NbtSummaryValue::Compound(fields) => fields.as_slice(),
            _ => &[],
        };
        Self::parse(
            field(fields, "settings").unwrap_or(&NbtSummaryValue::Compound(Vec::new())),
            path,
        )
    }

    pub fn parse(settings: &NbtSummaryValue, _path: &str) -> SaveResult<Self> {
        let fields = match settings {
            NbtSummaryValue::Compound(fields) => fields.as_slice(),
            _ => &[],
        };
        let structure_overrides =
            field(fields, "structure_overrides").and_then(parse_structure_overrides);
        let mut layers_info: Vec<_> = list_or_empty(fields, "layers")
            .iter()
            .filter_map(FlatLayerInfo::parse)
            .collect();
        let mut remaining = DimensionTypeScalars::Y_SIZE;
        for layer in &mut layers_info {
            layer.height = layer.height.min(remaining);
            remaining -= layer.height;
        }
        let biome = match field(fields, "biome") {
            Some(NbtSummaryValue::String(id)) => id
                .try_to_string()
                .ok()
                .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok()),
            _ => None,
        }
        .unwrap_or_else(|| DEFAULT_FLAT_BIOME.to_string());
        let mut settings = Self {
            structure_overrides,
            layers_info,
            add_lakes: bool_or(fields, "lakes", false),
            decoration: bool_or(fields, "features", false),
            biome,
            layers: Vec::new(),
            void_gen: false,
        };
        settings.update_layers()?;
        Ok(settings)
    }

    /// `FlatLevelGeneratorSettings.updateLayers`
    /// (`FlatLevelGeneratorSettings.java:196-206`): every layer contributes its
    /// state once per block of its height, bottom layer first.
    fn update_layers(&mut self) -> SaveResult<()> {
        self.layers.clear();
        for layer in &self.layers_info {
            for _ in 0..layer.height {
                self.layers.push(layer.block_state_id);
            }
        }
        // `s.is(Blocks.AIR)` is block identity, so `cave_air`/`void_air`
        // layers are not void generation, and an empty layer list is.
        let air_state_id = block_default_state_id(VANILLA_BLOCK_AIR_ID)?;
        self.void_gen = self.layers.iter().all(|state| *state == air_state_id);
        Ok(())
    }

    /// The declared layers, bottom first (`getLayersInfo`).
    pub fn layers_info(&self) -> &[FlatLayerInfo] {
        &self.layers_info
    }

    /// One global block-state id per block of dimension height, bottom first
    /// (`getLayers`).
    pub fn layers(&self) -> &[i32] {
        &self.layers
    }

    pub fn biome(&self) -> &str {
        &self.biome
    }

    pub const fn add_lakes(&self) -> bool {
        self.add_lakes
    }

    pub const fn decoration(&self) -> bool {
        self.decoration
    }

    /// `None` means *every* structure set: an absent `structure_overrides`
    /// makes `FlatLevelSource.createState` (`FlatLevelSource.java:40-46`) fall
    /// back to the whole registry rather than to none.
    pub fn structure_overrides(&self) -> Option<&FlatStructureOverrides> {
        self.structure_overrides.as_ref()
    }

    /// Whether every expanded layer is `minecraft:air`.
    pub const fn is_void_gen(&self) -> bool {
        self.void_gen
    }
}

/// `FlatLevelSource` (`FlatLevelSource.java:28-153`) bound to one dimension's
/// height, which is the only thing besides the settings that its chunk output
/// depends on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatLevelSource {
    settings: FlatLevelGeneratorSettings,
    min_y: i32,
    height: i32,
    air_block_state_id: i32,
    decoration_layers_prepared: bool,
}

impl FlatLevelSource {
    /// Binds settings to a dimension whose `min_y`/`height` satisfy the
    /// `DimensionType` compact constructor (`DimensionType.java:61-81`), which
    /// is what makes the section count exact.
    pub fn new(settings: FlatLevelGeneratorSettings, min_y: i32, height: i32) -> SaveResult<Self> {
        DimensionTypeScalars::check_shape(min_y, height, None)
            .map_err(|message| invalid_schema(SCHEMA, message))?;
        Ok(Self {
            settings,
            min_y,
            height,
            air_block_state_id: block_default_state_id(VANILLA_BLOCK_AIR_ID)?,
            decoration_layers_prepared: false,
        })
    }

    pub const fn settings(&self) -> &FlatLevelGeneratorSettings {
        &self.settings
    }

    pub fn prepare_decoration_layers(&mut self) {
        self.decoration_layers_prepared = true;
    }

    pub fn deferred_decoration_layers(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.settings
            .layers
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, state)| {
                (self.decoration_layers_prepared && is_deferred_layer(state))
                    .then_some((self.min_y + index as i32, state))
            })
    }

    pub fn base_column_state(&self, y: i32) -> Option<i32> {
        if !(self.min_y..self.min_y + self.height).contains(&y) {
            return None;
        }
        let state = self
            .settings
            .layers
            .get((y - self.min_y) as usize)
            .copied()
            .unwrap_or(self.air_block_state_id);
        Some(
            if self.decoration_layers_prepared && is_deferred_layer(state) {
                self.air_block_state_id
            } else {
                state
            },
        )
    }

    pub const fn min_y(&self) -> i32 {
        self.min_y
    }

    pub const fn height(&self) -> i32 {
        self.height
    }

    /// The generator one declared stem runs, sized by the `dimension_type`
    /// the same stem names.
    ///
    /// The two halves of a `LevelStem` are exactly the two arguments
    /// [`Self::new`] takes: `generator` says what the column is made of and
    /// `type` says how tall it is. Neither is a property of *which* dimension
    /// the stem is filed under, which is why nothing here looks at the key --
    /// a save is free to file `minecraft:overworld_caves` under the overworld
    /// and vanilla sizes that level from the type, not from the name.
    ///
    /// Fails closed on a generator this server cannot run: the dispatch key on
    /// the stem is what decides which codec the payload is, so a save naming
    /// `minecraft:noise` is refused here rather than served flat terrain it
    /// never asked for.
    fn from_level_stem(
        stem: &Java26LevelStem,
        generation: Option<&DatapackGeneration>,
        path: &str,
    ) -> SaveResult<Self> {
        let settings = FlatLevelGeneratorSettings::from_level_stem(stem, path)?;
        let shape = match generation {
            Some(generation) => dimension_type_shape(generation, &stem.dimension_type),
            None => builtin_dimension_type_shape(&stem.dimension_type),
        }
        .map_err(|error| invalid_schema(&format!("{path}.type"), error.to_string()))?;
        Self::new(settings, shape.min_y, shape.height)
    }

    /// `FlatLevelSource.getSeaLevel` (`FlatLevelSource.java:150-152`).
    ///
    /// A flat world answers with this constant whatever its layers are, so it
    /// is a property of the generator rather than of the settings — which is
    /// why it is not `-64 + something`.
    pub const fn sea_level(&self) -> i32 {
        FLAT_SEA_LEVEL
    }

    /// `FlatLevelSource.getSpawnHeight` (`FlatLevelSource.java:62-64`).
    pub fn spawn_height(&self) -> i32 {
        let layers = i32::try_from(self.settings.layers.len()).unwrap_or(i32::MAX);
        self.min_y + self.height.min(layers)
    }

    /// How many layers `fillFromNoise` writes: `Math.min(chunk.getHeight(),
    /// layers.size())` (`FlatLevelSource.java:72`). Layers above the dimension
    /// are declared but never placed.
    fn filled_layer_count(&self) -> usize {
        let height = usize::try_from(self.height).unwrap_or(0);
        height.min(self.settings.layers.len())
    }

    /// `FlatLevelSource.fillFromNoise` (`FlatLevelSource.java:67-91`).
    ///
    /// The chunk position is carried into the result and never read while
    /// generating: a flat column is the same 16x16 stack of layer states at
    /// every `(chunk_x, chunk_z)`. The biome comes from the caller because
    /// resolving `settings.biome()` to a wire id needs the dimension's biome
    /// registry, which this module does not own.
    pub fn generate_chunk(&self, chunk_x: i32, chunk_z: i32, biome_id: i32) -> FlatGeneratedChunk {
        self.generate_chunk_with_biome_registry(
            chunk_x,
            chunk_z,
            biome_id,
            bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN,
        )
    }

    pub fn generate_chunk_with_biome_registry(
        &self,
        chunk_x: i32,
        chunk_z: i32,
        biome_id: i32,
        biome_registry_len: usize,
    ) -> FlatGeneratedChunk {
        let section_count = usize::try_from(self.height / SECTION_HEIGHT).unwrap_or(0);
        let mut column = ChunkColumn::new_with_biome_registry(
            self.min_y,
            section_count,
            self.air_block_state_id,
            biome_id,
            biome_registry_len,
        );
        // `Heightmap`'s backing storage starts zeroed, and `getFirstAvailable`
        // adds `minY` to it (`Heightmap.java:114-122`).
        let mut ocean_floor_wg = self.min_y;
        let mut world_surface_wg = self.min_y;

        for layer_index in 0..self.filled_layer_count() {
            let block_state_id = self.settings.layers[layer_index];
            if self.decoration_layers_prepared && is_deferred_layer(block_state_id) {
                continue;
            }
            let y = self.min_y + layer_index as i32;
            for x in 0..16 {
                for z in 0..16 {
                    column.set_block_state(x, y, z, block_state_id);
                }
            }
            // `Heightmap.update` (`Heightmap.java:81-107`) also has a branch
            // that walks back down when a non-opaque state replaces the
            // current top. It cannot fire here: that branch needs
            // `firstAvailable - 1 == y`, and a bottom-up fill never revisits a
            // y, so `firstAvailable` is always `<= y`.
            if HeightmapPredicate::MaterialMotionBlocking.test(block_state_id) {
                ocean_floor_wg = y + 1;
            }
            if HeightmapPredicate::NotAir.test(block_state_id) {
                world_surface_wg = y + 1;
            }
        }

        FlatGeneratedChunk {
            chunk_x,
            chunk_z,
            column,
            ocean_floor_wg,
            world_surface_wg,
        }
    }

    /// `FlatLevelSource.getBaseHeight` (`FlatLevelSource.java:94-105`), which
    /// answers without generating anything.
    ///
    /// `None` for the heightmap kinds whose predicate also reads the fluid
    /// state (see [`HeightmapPredicate::for_kind`]). The `x`/`z` arguments
    /// vanilla takes are omitted because it never reads them.
    ///
    /// Note vanilla's own scan bound, `Math.min(layers.size() - 1,
    /// heightAccessor.getMaxY())`, compares a layer *index* against an
    /// absolute y. It is reproduced rather than corrected, because it is what
    /// decides the answer for a settings object with more layers than the
    /// dimension is tall.
    pub fn base_height(&self, heightmap_kind_id: i32) -> Option<i32> {
        let predicate = HeightmapPredicate::for_kind(heightmap_kind_id)?;
        let max_y = self.min_y + self.height - 1;
        let highest = i32::try_from(self.settings.layers.len())
            .unwrap_or(i32::MAX)
            .checked_sub(1)?
            .min(max_y);
        for layer_index in (0..=highest).rev() {
            let block_state_id = self.settings.layers[layer_index as usize];
            if self.decoration_layers_prepared && is_deferred_layer(block_state_id) {
                continue;
            }
            if predicate.test(block_state_id) {
                return Some(self.min_y + layer_index + 1);
            }
        }
        Some(self.min_y)
    }
}

// FlatLevelGeneratorSettings.java:165-175; Heightmap.java:149: fluids remain in base terrain.
fn is_deferred_layer(state: i32) -> bool {
    !block_state_blocks_motion(state) && block_state_fluid_state(state).is_empty()
}

/// One chunk column a [`FlatLevelSource`] produced, with the two world-gen
/// heightmaps `fillFromNoise` primes alongside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatGeneratedChunk {
    chunk_x: i32,
    chunk_z: i32,
    column: ChunkColumn,
    ocean_floor_wg: i32,
    world_surface_wg: i32,
}

impl FlatGeneratedChunk {
    pub const fn chunk_x(&self) -> i32 {
        self.chunk_x
    }

    pub const fn chunk_z(&self) -> i32 {
        self.chunk_z
    }

    pub const fn column(&self) -> &ChunkColumn {
        &self.column
    }

    pub fn into_column(self) -> ChunkColumn {
        self.column
    }

    /// `OCEAN_FLOOR_WG` (`Heightmap.java:145`).
    pub const fn ocean_floor_wg(&self) -> i32 {
        self.ocean_floor_wg
    }

    /// `WORLD_SURFACE_WG` (`Heightmap.java:145`).
    pub const fn world_surface_wg(&self) -> i32 {
        self.world_surface_wg
    }

    /// The `Heightmap.getFirstAvailable` value for one of the two kinds
    /// `fillFromNoise` primes; `None` for every other kind, which a flat
    /// generator leaves for later stages to build.
    ///
    /// There is one value per kind rather than 256, because a flat column is
    /// position-independent: every entry of the packed map holds this.
    pub const fn heightmap_first_available(&self, heightmap_kind_id: i32) -> Option<i32> {
        match heightmap_kind_id {
            VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID => Some(self.ocean_floor_wg),
            VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID => Some(self.world_surface_wg),
            _ => None,
        }
    }
}

/// The `Heightmap.Types` predicates that read the block state alone
/// (`Heightmap.java:30-31`, `Heightmap.java:142-153`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeightmapPredicate {
    NotAir,
    MaterialMotionBlocking,
}

impl HeightmapPredicate {
    /// `MOTION_BLOCKING` and `MOTION_BLOCKING_NO_LEAVES` are absent on purpose:
    /// their predicates also test `getFluidState`, and neither this generator
    /// nor its caller models fluid state, so answering them would be guessing.
    const fn for_kind(heightmap_kind_id: i32) -> Option<Self> {
        match heightmap_kind_id {
            VANILLA_HEIGHTMAP_WORLD_SURFACE_WG_ID | VANILLA_HEIGHTMAP_WORLD_SURFACE_ID => {
                Some(Self::NotAir)
            }
            VANILLA_HEIGHTMAP_OCEAN_FLOOR_WG_ID | VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID => {
                Some(Self::MaterialMotionBlocking)
            }
            _ => None,
        }
    }

    /// A state id outside the generated catalog cannot reach here: every id a
    /// layer holds was resolved through it.
    fn test(self, block_state_id: i32) -> bool {
        match self {
            Self::NotAir => !block_state_is_air(block_state_id),
            Self::MaterialMotionBlocking => block_state_blocks_motion(block_state_id),
        }
    }
}

fn parse_structure_overrides(value: &NbtSummaryValue) -> Option<FlatStructureOverrides> {
    let resource_id = |value: &NbtSummaryValue| match value {
        NbtSummaryValue::String(id) => id
            .try_to_string()
            .ok()
            .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok()),
        _ => None,
    };
    match value {
        NbtSummaryValue::String(raw) if raw.units().first() == Some(&u16::from(b'#')) => {
            bbb_protocol::normalize_resource_location(String::from_utf16(&raw.units()[1..]).ok()?)
                .ok()
                .map(|id| FlatStructureOverrides::Tag(format!("#{id}")))
        }
        NbtSummaryValue::String(_) => {
            resource_id(value).map(|id| FlatStructureOverrides::Sets(vec![id]))
        }
        NbtSummaryValue::List(values) => Some(FlatStructureOverrides::Sets(
            values.iter().filter_map(resource_id).collect(),
        )),
        _ => None,
    }
}

/// `Block.defaultBlockState()` for a built-in block.
fn block_default_state_id(block_type_id: i32) -> SaveResult<i32> {
    vanilla_block_default_state_id(block_type_id)
        .ok_or_else(|| missing_catalog_fact("block default state"))
}

fn missing_catalog_fact(fact: &str) -> bbb_server_types::save::SaveError {
    invalid_schema(
        SCHEMA,
        format!("the generated Vanilla 26.1 catalog is missing a {fact}"),
    )
}

#[cfg(test)]
pub mod tests;

/// Whether this server has a generator for what one stem declares.
///
/// The dispatch key alone decides, which is what makes this answerable without
/// reading the payload: `LevelStem.CODEC` picks the generator codec off
/// `generator.type`, so a stem naming anything but the one generator this
/// server implements carries a document it cannot read at all.
#[must_use]
pub fn stem_generator_is_served(stem: &Java26LevelStem) -> bool {
    stem.generator == FLAT_GENERATOR_ID
}

/// The stems a save declares, split into the ones this server can run and the
/// ones it cannot.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclaredLevelSources {
    /// One source per stem this server has a generator for.
    pub served: BTreeMap<Java26LevelKey, FlatLevelSource>,
    /// The generator id of every stem it does not, keyed the same way.
    pub unserved: BTreeMap<Java26LevelKey, String>,
}

/// Reads a save's stem map, keeping the stems this server can run and naming
/// the ones it cannot.
///
/// A save written by anything but this server declares generators it does not
/// implement: vanilla's own `flat` world preset leaves `minecraft:noise` on the
/// nether and the end stems, so refusing the whole map would refuse every
/// official world. What vanilla does with a stem is build a level for it, and
/// what this server does with a stem it has no generator for is decline to
/// build that one level -- the save still opens, the level is not among the
/// ones it serves, and nothing can be sent there.
///
/// Only the generator id is allowed to put a stem in `unserved`. A stem that
/// says `minecraft:flat` and then fails to parse is a malformed save and still
/// fails the whole read, because that is a document this server *can* read and
/// did not.
pub fn served_level_sources_from_stems(
    dimensions: &BTreeMap<Java26LevelKey, Java26LevelStem>,
    generation: Option<&super::dynamic_registry::SharedDatapackGeneration>,
    path: &str,
) -> SaveResult<DeclaredLevelSources> {
    let generation = generation.map(std::ops::Deref::deref);
    let mut served = BTreeMap::new();
    let mut unserved = BTreeMap::new();
    for (name, stem) in dimensions {
        if !stem_generator_is_served(stem) {
            unserved.insert(name.clone(), stem.generator.clone());
            continue;
        }
        served.insert(
            name.clone(),
            FlatLevelSource::from_level_stem(
                stem,
                generation,
                &format!("{path}.{}", name.as_str()),
            )?,
        );
    }
    Ok(DeclaredLevelSources { served, unserved })
}
