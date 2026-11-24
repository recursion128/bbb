//! Exact, immutable projection of Java 26.1 terrain chunk NBT.

use std::collections::BTreeMap;

use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::schema_support::{bool_or, field, long_or, require_java_26_1_root};
use bbb_server_types::save::{NamedCompound, SaveResult};

pub mod block_entities;
pub mod heightmaps;
pub mod optional_fields;
pub mod sections;
pub mod ticks;

#[cfg(test)]
pub mod tests;

pub use self::block_entities::replace_block_entities;
pub use self::sections::parse_block_state;
pub use self::ticks::write_chunk_ticks;

use self::block_entities::parse_block_entity_list;
use self::heightmaps::{parse_heightmaps, parse_post_processing};
use self::optional_fields::{carving_mask, parse_optional_generation_fields, structures};
use self::ticks::TickKind;

const BLOCK_ENTRY_COUNT: usize = 16 * 16 * 16;
const BIOME_ENTRY_COUNT: usize = 4 * 4 * 4;
const LIGHT_ARRAY_BYTES: usize = 2048;
pub const JAVA26_BLENDING_HEIGHT_COUNT: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Java26ChunkContext {
    pub expected_x: i32,
    pub expected_z: i32,
    pub min_section_y: i32,
    pub section_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Java26ChunkStatus {
    Empty,
    StructureStarts,
    StructureReferences,
    Biomes,
    Noise,
    Surface,
    Carvers,
    Features,
    InitializeLight,
    Light,
    Spawn,
    Full,
}

impl Java26ChunkStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "minecraft:empty",
            Self::StructureStarts => "minecraft:structure_starts",
            Self::StructureReferences => "minecraft:structure_references",
            Self::Biomes => "minecraft:biomes",
            Self::Noise => "minecraft:noise",
            Self::Surface => "minecraft:surface",
            Self::Carvers => "minecraft:carvers",
            Self::Features => "minecraft:features",
            Self::InitializeLight => "minecraft:initialize_light",
            Self::Light => "minecraft:light",
            Self::Spawn => "minecraft:spawn",
            Self::Full => "minecraft:full",
        }
    }

    fn parse(value: &NbtSummaryValue) -> Option<Self> {
        let NbtSummaryValue::String(id) = value else {
            return None;
        };
        let id = id
            .try_to_string()
            .ok()
            .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())?;
        Some(match id.as_str() {
            "minecraft:empty" => Self::Empty,
            "minecraft:structure_starts" => Self::StructureStarts,
            "minecraft:structure_references" => Self::StructureReferences,
            "minecraft:biomes" => Self::Biomes,
            "minecraft:noise" => Self::Noise,
            "minecraft:surface" => Self::Surface,
            "minecraft:carvers" => Self::Carvers,
            "minecraft:features" => Self::Features,
            "minecraft:initialize_light" => Self::InitializeLight,
            "minecraft:light" => Self::Light,
            "minecraft:spawn" => Self::Spawn,
            "minecraft:full" => Self::Full,
            _ => return None,
        })
    }

    fn allows_heightmap_key(self, key: &str) -> bool {
        match self {
            Self::Empty
            | Self::StructureStarts
            | Self::StructureReferences
            | Self::Biomes
            | Self::Noise
            | Self::Surface => matches!(key, "OCEAN_FLOOR_WG" | "WORLD_SURFACE_WG"),
            Self::Carvers
            | Self::Features
            | Self::InitializeLight
            | Self::Light
            | Self::Spawn
            | Self::Full => matches!(
                key,
                "OCEAN_FLOOR" | "WORLD_SURFACE" | "MOTION_BLOCKING" | "MOTION_BLOCKING_NO_LEAVES"
            ),
        }
    }

    fn is_full(self) -> bool {
        self == Self::Full
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26BlockState {
    pub name: String,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26Palette<T> {
    pub palette: Vec<T>,
    pub data: Option<Vec<i64>>,
    pub bits_per_entry: u8,
    pub entry_count: usize,
}

impl<T> Java26Palette<T> {
    pub fn palette_index(&self, entry_index: usize) -> Option<usize> {
        if entry_index >= self.entry_count {
            return None;
        }
        if self.bits_per_entry == 0 {
            return Some(0);
        }
        let values_per_long = 64 / usize::from(self.bits_per_entry);
        let word = self
            .data
            .as_ref()
            .and_then(|data| data.get(entry_index / values_per_long))
            .map_or(0, |word| *word as u64);
        let shift = (entry_index % values_per_long) * usize::from(self.bits_per_entry);
        let mask = (1_u64 << self.bits_per_entry) - 1;
        let index = ((word >> shift) & mask) as usize;
        Some(if index < self.palette.len() { index } else { 0 })
    }

    pub fn palette_index_at(&self, x: usize, y: usize, z: usize) -> Option<usize> {
        let axis = match self.entry_count {
            BLOCK_ENTRY_COUNT => 16,
            BIOME_ENTRY_COUNT => 4,
            _ => return None,
        };
        if x >= axis || y >= axis || z >= axis {
            return None;
        }
        self.palette_index((y * axis + z) * axis + x)
    }

    pub fn value_at(&self, x: usize, y: usize, z: usize) -> Option<&T> {
        self.palette.get(self.palette_index_at(x, y, z)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26ChunkSection {
    pub y: i8,
    pub block_states: Option<Java26Palette<Java26BlockState>>,
    pub biomes: Option<Java26Palette<String>>,
    pub block_light: Option<Vec<i8>>,
    pub sky_light: Option<Vec<i8>>,
}

pub use bbb_server_types::chunk_ticks::Java26SavedTick;

/// The packed old-generation area retained by `BlendingData.Packed.CODEC`
/// (`BlendingData.java:401-416`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26BlendingData {
    min_section: i32,
    max_section: i32,
    height_bits: Option<[u64; JAVA26_BLENDING_HEIGHT_COUNT]>,
}

/// Pending block-neighbor migration serialized by `UpgradeData.write`
/// (`UpgradeData.java:41-107,191-239`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26UpgradeData {
    sides: u8,
    indices: Option<BTreeMap<usize, Vec<i32>>>,
    neighbor_block_ticks: Option<Vec<Java26SavedTick>>,
    neighbor_fluid_ticks: Option<Vec<Java26SavedTick>>,
}

impl Java26UpgradeData {
    pub const fn sides(&self) -> u8 {
        self.sides
    }

    pub const fn indices(&self) -> Option<&BTreeMap<usize, Vec<i32>>> {
        self.indices.as_ref()
    }

    pub fn neighbor_block_ticks(&self) -> Option<&[Java26SavedTick]> {
        self.neighbor_block_ticks.as_deref()
    }

    pub fn neighbor_fluid_ticks(&self) -> Option<&[Java26SavedTick]> {
        self.neighbor_fluid_ticks.as_deref()
    }

    pub fn has_pending_work(&self) -> bool {
        self.sides != 0
            || self
                .indices
                .as_ref()
                .is_some_and(|sections| sections.values().any(|indices| !indices.is_empty()))
            || self
                .neighbor_block_ticks
                .as_ref()
                .is_some_and(|ticks| !ticks.is_empty())
            || self
                .neighbor_fluid_ticks
                .as_ref()
                .is_some_and(|ticks| !ticks.is_empty())
    }
}

/// Pending height-extension status work serialized by
/// `BelowZeroRetrogen.CODEC` (`BelowZeroRetrogen.java:25-42`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26BelowZeroRetrogen {
    target_status: Java26ChunkStatus,
    missing_bedrock: Option<Vec<u64>>,
}

impl Java26BelowZeroRetrogen {
    pub const fn target_status(&self) -> Java26ChunkStatus {
        self.target_status
    }

    pub fn missing_bedrock_words(&self) -> Option<&[u64]> {
        self.missing_bedrock.as_deref()
    }

    pub fn has_bedrock_hole(&self, x: i32, z: i32) -> bool {
        let bit = usize::try_from((z & 15) * 16 + (x & 15)).expect("masked index is nonnegative");
        self.missing_bedrock
            .as_ref()
            .and_then(|words| words.get(bit / 64))
            .is_some_and(|word| word & (1_u64 << (bit % 64)) != 0)
    }
}

impl Java26BlendingData {
    pub const fn min_section(&self) -> i32 {
        self.min_section
    }

    pub const fn max_section(&self) -> i32 {
        self.max_section
    }

    pub const fn height_bits(&self) -> Option<&[u64; JAVA26_BLENDING_HEIGHT_COUNT]> {
        self.height_bits.as_ref()
    }

    pub fn heights(&self) -> Option<[f64; JAVA26_BLENDING_HEIGHT_COUNT]> {
        self.height_bits.map(|bits| bits.map(f64::from_bits))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26ChunkData {
    x: i32,
    min_section_y: i32,
    z: i32,
    last_update: i64,
    inhabited_time: i64,
    status: Java26ChunkStatus,
    blending_data: Option<Java26BlendingData>,
    upgrade_data: Option<Java26UpgradeData>,
    below_zero_retrogen: Option<Java26BelowZeroRetrogen>,
    legacy_generation_metadata: Java26LegacyGenerationMetadata,
    is_light_on: bool,
    sections: Vec<Java26ChunkSection>,
    block_entities: Vec<Vec<NbtSummaryEntry>>,
    entities: Option<Vec<Vec<NbtSummaryEntry>>>,
    block_ticks: Vec<Java26SavedTick>,
    fluid_ticks: Vec<Java26SavedTick>,
    post_processing: Vec<Vec<i16>>,
    carving_mask: Option<Vec<i64>>,
    heightmaps: BTreeMap<String, Vec<i64>>,
    structures: NbtSummaryValue,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Java26LegacyGenerationMetadata {
    upgrade_data: bool,
    blending_data: bool,
    below_zero_retrogen: bool,
}

impl Java26LegacyGenerationMetadata {
    pub const fn is_empty(self) -> bool {
        !self.upgrade_data && !self.blending_data && !self.below_zero_retrogen
    }

    pub const fn has_upgrade_data(self) -> bool {
        self.upgrade_data
    }

    pub const fn has_blending_data(self) -> bool {
        self.blending_data
    }

    pub const fn has_below_zero_retrogen(self) -> bool {
        self.below_zero_retrogen
    }

    pub fn field_names(self) -> impl Iterator<Item = &'static str> {
        [
            self.upgrade_data.then_some("UpgradeData"),
            self.blending_data.then_some("blending_data"),
            self.below_zero_retrogen.then_some("below_zero_retrogen"),
        ]
        .into_iter()
        .flatten()
    }
}

impl Java26ChunkData {
    // SerializableChunkData.java:110-122,213-218: unreadable fields default and misplaced chunks relocate.
    pub fn parse(
        raw: impl std::borrow::Borrow<NamedCompound>,
        context: Java26ChunkContext,
    ) -> SaveResult<Self> {
        Self::parse_with_content_registry(
            raw,
            context,
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        )
    }

    /// Registry-aware form of [`Self::parse`] preserving registered modded
    /// block palette entries and block ticks.
    pub fn parse_with_content_registry(
        raw: impl std::borrow::Borrow<NamedCompound>,
        context: Java26ChunkContext,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> SaveResult<Self> {
        let raw = raw.borrow();
        let entries = require_java_26_1_root(raw, "chunk")?;
        let x = context.expected_x;
        let z = context.expected_z;
        let status = field(entries, "Status")
            .and_then(Java26ChunkStatus::parse)
            .unwrap_or(Java26ChunkStatus::Empty);
        let optional_generation =
            parse_optional_generation_fields(entries, context, status, content_registry);
        let legacy_generation_metadata = Java26LegacyGenerationMetadata {
            upgrade_data: optional_generation.upgrade_data.is_some(),
            blending_data: optional_generation.blending_data.is_some(),
            below_zero_retrogen: optional_generation.below_zero_retrogen.is_some(),
        };
        // ProtoChunk keeps raw entity tags until promotion
        // (`SerializableChunkData.java:369-371`); a full chunk ignores them.
        let entities = (!status.is_full()).then(|| match field(entries, "entities") {
            Some(NbtSummaryValue::List(values)) => values
                .iter()
                .filter_map(|value| match value {
                    NbtSummaryValue::Compound(entries) => Some(entries.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        });
        let ticks = |name, kind| {
            field(entries, name)
                .map(|value| {
                    ticks::parse_ticks_with_content_registry(
                        value,
                        context,
                        true,
                        kind,
                        content_registry,
                    )
                })
                .unwrap_or_default()
        };
        Ok(Self {
            x,
            min_section_y: context.min_section_y,
            z,
            last_update: long_or(entries, "LastUpdate", 0),
            inhabited_time: long_or(entries, "InhabitedTime", 0),
            status,
            blending_data: optional_generation.blending_data,
            upgrade_data: optional_generation.upgrade_data,
            below_zero_retrogen: optional_generation.below_zero_retrogen,
            legacy_generation_metadata,
            is_light_on: bool_or(entries, "isLightOn", false),
            sections: sections::parse_sections_with_content_registry(
                entries,
                context,
                content_registry,
            ),
            block_entities: parse_block_entity_list(entries),
            entities,
            block_ticks: ticks("block_ticks", TickKind::Block),
            fluid_ticks: ticks("fluid_ticks", TickKind::Fluid),
            post_processing: parse_post_processing(entries, context.section_count),
            carving_mask: carving_mask(entries),
            heightmaps: parse_heightmaps(entries, status, context),
            structures: structures(entries),
        })
    }

    pub fn x(&self) -> i32 {
        self.x
    }

    pub fn min_section_y(&self) -> i32 {
        self.min_section_y
    }

    pub fn z(&self) -> i32 {
        self.z
    }

    pub fn last_update(&self) -> i64 {
        self.last_update
    }

    pub fn inhabited_time(&self) -> i64 {
        self.inhabited_time
    }

    pub fn status(&self) -> Java26ChunkStatus {
        self.status
    }

    pub fn legacy_generation_metadata(&self) -> Java26LegacyGenerationMetadata {
        self.legacy_generation_metadata
    }

    pub fn blending_data(&self) -> Option<&Java26BlendingData> {
        self.blending_data.as_ref()
    }

    pub fn upgrade_data(&self) -> Option<&Java26UpgradeData> {
        self.upgrade_data.as_ref()
    }

    pub fn below_zero_retrogen(&self) -> Option<&Java26BelowZeroRetrogen> {
        self.below_zero_retrogen.as_ref()
    }

    pub fn is_light_on(&self) -> bool {
        self.is_light_on
    }

    pub fn sections(&self) -> &[Java26ChunkSection] {
        &self.sections
    }

    pub fn block_entities(&self) -> &[Vec<NbtSummaryEntry>] {
        &self.block_entities
    }

    pub fn entities(&self) -> Option<&[Vec<NbtSummaryEntry>]> {
        self.entities.as_deref()
    }

    pub fn block_ticks(&self) -> &[Java26SavedTick] {
        &self.block_ticks
    }

    pub fn fluid_ticks(&self) -> &[Java26SavedTick] {
        &self.fluid_ticks
    }

    pub fn post_processing(&self) -> &[Vec<i16>] {
        &self.post_processing
    }

    pub fn carving_mask(&self) -> Option<&[i64]> {
        self.carving_mask.as_deref()
    }

    pub fn structures(&self) -> &NbtSummaryValue {
        &self.structures
    }

    pub fn heightmaps(&self) -> &BTreeMap<String, Vec<i64>> {
        &self.heightmaps
    }
}
