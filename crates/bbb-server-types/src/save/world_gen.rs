//! Saved world generator settings, dimension geometries, and world creation value types.

use std::collections::BTreeMap;

use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_membership_26_1;
use bbb_protocol::nbt_summary::entry;
use bbb_protocol::{
    normalize_resource_location, JavaString, NbtSummaryEntry, NbtSummaryValue,
    PersistentRegistryMembership,
};

use super::{Java26LevelKey, NamedCompound, SaveError, SaveResult};
use crate::dimension_type::{builtin_dimension_type_shape_facts, DimensionTypeShape};
use crate::fixture::FlatFixture;

const CHUNK_GENERATOR_REGISTRY: &str = "minecraft:chunk_generator";

/// The pack and feature inventory a built-in-only 26.1 world carries.
pub const BUILTIN_ONLY_ENABLED_PACKS: &[&str] = &["vanilla"];
pub const BUILTIN_ONLY_DISABLED_PACKS: &[&str] = &[
    "trade_rebalance",
    "redstone_experiments",
    "minecart_improvements",
];
pub const BUILTIN_ONLY_ENABLED_FEATURES: &[&str] = &["minecraft:vanilla"];

/// The optional data-pack configuration emitted by `WorldDataConfiguration.MAP_CODEC`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26DataPacks {
    pub enabled: Vec<JavaString>,
    pub disabled: Vec<JavaString>,
}

impl Java26DataPacks {
    pub fn new(enabled: Vec<String>, disabled: Vec<String>) -> Self {
        Self {
            enabled: enabled.into_iter().map(Into::into).collect(),
            disabled: disabled.into_iter().map(Into::into).collect(),
        }
    }

    pub fn enabled(&self) -> &[JavaString] {
        &self.enabled
    }

    pub fn disabled(&self) -> &[JavaString] {
        &self.disabled
    }
}

/// One decoded `LevelStem` entry, keyed separately by its level resource id.
#[derive(Debug, Clone, PartialEq)]
pub struct Java26LevelStem {
    pub dimension_type: String,
    pub generator: String,
    /// Raw `LevelStem.generator` payload retained for the writer round-trip.
    pub raw_generator: NbtSummaryValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Java26WorldGenSettings {
    pub seed: i64,
    pub generate_structures: bool,
    pub generate_bonus_chest: bool,
    pub legacy_custom_options: Option<JavaString>,
    /// Stems keyed by their `ResourceKey<LevelStem>` id; `minecraft:overworld`
    /// is always present (`WorldDimensions` compact constructor).
    pub dimensions: BTreeMap<Java26LevelKey, Java26LevelStem>,
    /// Raw file payload retained for the writer round-trip.
    pub document: NamedCompound,
    pub decompressed_bytes: usize,
}

pub fn parse_level_stem(stem_entry: &NbtSummaryEntry) -> Option<Java26LevelStem> {
    let NbtSummaryValue::Compound(fields) = &stem_entry.value else {
        return None;
    };
    let resource_id = |value: &NbtSummaryValue| match value {
        NbtSummaryValue::String(value) => value
            .try_to_string()
            .ok()
            .and_then(|raw| normalize_resource_location(raw).ok()),
        _ => None,
    };
    fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
        entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| &entry.value)
    }
    let dimension_type = resource_id(field(fields, "type")?)?;
    let generator_value = field(fields, "generator")?;
    let NbtSummaryValue::Compound(generator_fields) = generator_value else {
        return None;
    };
    let generator = resource_id(field(generator_fields, "type")?)?;
    if !matches!(
        vanilla_persistent_registry_membership_26_1(CHUNK_GENERATOR_REGISTRY, &generator),
        PersistentRegistryMembership::Present
    ) {
        return None;
    }
    Some(Java26LevelStem {
        dimension_type,
        generator,
        raw_generator: generator_value.clone(),
    })
}

/// Default `minecraft:overworld` stem (`LevelStorageSource.java:153-164`).
pub fn default_overworld_level_stem() -> SaveResult<Java26LevelStem> {
    let fixture = FlatFixture::new().map_err(|error| SaveError::InvalidNbt {
        path: None,
        detail: format!("world_creation: the flat fixture is unavailable: {error}"),
    })?;
    let shape = builtin_dimension_type_shape_facts("minecraft:overworld")
        .expect("built-in overworld dimension type shape");
    let height = fixture
        .floor_y()
        .checked_sub(shape.min_y)
        .and_then(|height| height.checked_add(1))
        .filter(|height| *height > 0)
        .ok_or_else(|| SaveError::InvalidNbt {
            path: None,
            detail: "world_creation: the fixture column has no height".to_string(),
        })?;
    let raw_generator = NbtSummaryValue::Compound(vec![
        entry("type", NbtSummaryValue::string("minecraft:flat")),
        entry(
            "settings",
            NbtSummaryValue::Compound(vec![
                entry("biome", NbtSummaryValue::string("minecraft:plains")),
                entry("features", NbtSummaryValue::Byte(0)),
                entry("lakes", NbtSummaryValue::Byte(0)),
                entry(
                    "layers",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("block", NbtSummaryValue::string(fixture.floor_block_name())),
                        entry("height", NbtSummaryValue::Int(height)),
                    ])]),
                ),
                entry("structure_overrides", NbtSummaryValue::List(Vec::new())),
            ]),
        ),
    ]);
    Ok(Java26LevelStem {
        dimension_type: "minecraft:overworld".to_string(),
        generator: "minecraft:flat".to_string(),
        raw_generator,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DimensionSectionGeometry {
    pub min_section_y: i32,
    pub section_count: usize,
}

impl DimensionSectionGeometry {
    pub fn of(shape: &DimensionTypeShape) -> SaveResult<Self> {
        // `DimensionType.java:61-80` validates section-aligned `min_y` and `height`.
        Ok(Self {
            min_section_y: shape.min_y / 16,
            section_count: usize::try_from(shape.section_count()).map_err(|_| {
                SaveError::InvalidNbt {
                    path: None,
                    detail: "world_gen_settings.data.dimensions: a dimension type declares a negative section count"
                        .to_string(),
                }
            })?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26DimensionGeometry {
    dimension_type_id: String,
    sections: DimensionSectionGeometry,
}

impl Java26DimensionGeometry {
    pub fn new(dimension_type_id: impl Into<String>, sections: DimensionSectionGeometry) -> Self {
        Self {
            dimension_type_id: dimension_type_id.into(),
            sections,
        }
    }

    pub fn dimension_type_id(&self) -> &str {
        &self.dimension_type_id
    }

    pub const fn sections(&self) -> DimensionSectionGeometry {
        self.sections
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26DimensionGeometries {
    entries: BTreeMap<Java26LevelKey, Java26DimensionGeometry>,
}

impl Java26DimensionGeometries {
    pub fn from_entries(entries: BTreeMap<Java26LevelKey, Java26DimensionGeometry>) -> Self {
        Self { entries }
    }

    pub fn get(&self, dimension: &Java26LevelKey) -> Option<&Java26DimensionGeometry> {
        self.entries.get(dimension)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Java26LevelKey, &Java26DimensionGeometry)> {
        self.entries.iter()
    }

    pub fn keys(&self) -> impl Iterator<Item = &Java26LevelKey> {
        self.entries.keys()
    }

    pub fn from_builtin_settings(settings: &Java26WorldGenSettings) -> Self {
        Self::from_stems(&settings.dimensions, |stem| {
            builtin_dimension_type_shape_facts(&stem.dimension_type)
        })
    }

    pub fn from_stems(
        stems: &BTreeMap<Java26LevelKey, Java26LevelStem>,
        mut shape: impl FnMut(&Java26LevelStem) -> Option<DimensionTypeShape>,
    ) -> Self {
        let mut entries = BTreeMap::new();
        for (name, stem) in stems {
            let Some(sections) =
                shape(stem).and_then(|shape| DimensionSectionGeometry::of(&shape).ok())
            else {
                continue;
            };
            entries.insert(
                name.clone(),
                Java26DimensionGeometry {
                    dimension_type_id: stem.dimension_type.clone(),
                    sections,
                },
            );
        }
        Self { entries }
    }
}

/// The world seed of the fixture world (`0`).
pub const CREATED_WORLD_SEED: i64 = 0;

/// The `level-type` world presets a new save is created from (`WorldPresets.java:34-37`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Java26WorldPreset {
    Normal,
    Flat,
    LargeBiomes,
    Amplified,
}

impl Java26WorldPreset {
    /// A preset id, namespaced or not, or one of the legacy names
    /// `DedicatedServerProperties.java:251-253` maps.
    pub fn by_name(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        match name.strip_prefix("minecraft:").unwrap_or(&name) {
            "normal" | "default" => Some(Self::Normal),
            "flat" => Some(Self::Flat),
            "large_biomes" | "largebiomes" => Some(Self::LargeBiomes),
            "amplified" => Some(Self::Amplified),
            _ => None,
        }
    }

    pub const fn preset_path(self) -> &'static str {
        match self {
            Self::Normal => "worldgen/world_preset/normal.json",
            Self::Flat => "worldgen/world_preset/flat.json",
            Self::LargeBiomes => "worldgen/world_preset/large_biomes.json",
            Self::Amplified => "worldgen/world_preset/amplified.json",
        }
    }
}

/// `WorldOptions` plus the chosen preset (`Main.java:249-250`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Java26WorldOptions {
    pub seed: i64,
    pub generate_structures: bool,
    pub preset: Java26WorldPreset,
    /// `LevelSettings.DifficultySettings.hardcore` (`Main.java:245`).
    pub hardcore: bool,
}

/// Created level metadata (`Main.java:243-245`, `PrimaryLevelData.java:148`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26WorldCreation {
    pub level_name: String,
    pub game_type: i32,
    pub difficulty: String,
    pub spawn_position: [i32; 3],
    pub last_played: i64,
}

/// One chunk a created save is published with.
#[derive(Debug, Clone, PartialEq)]
pub struct Java26CreatedChunk {
    pub dimension: Java26LevelKey,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub document: NamedCompound,
}

/// Everything a directory needs to become a save.
#[derive(Debug, Clone, PartialEq)]
pub struct Java26CreatedSave {
    pub level: NamedCompound,
    pub world_gen_settings: NamedCompound,
    pub chunks: Vec<Java26CreatedChunk>,
}
