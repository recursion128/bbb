//! World-save limits, paths and the error every persistence reader shares.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use bbb_protocol::{normalize_resource_location, JavaString, NbtSummaryEntry, NbtSummaryValue};
use thiserror::Error;

/// The version strings `PrimaryLevelData.writeVersionTag` (:157-160) writes and
/// every reader checks.
pub const JAVA_26_1_NAME: &str = "26.1";

pub const JAVA_26_1_DATA_VERSION: i32 = 4786;
pub const JAVA_26_1_STORAGE_VERSION: i32 = 19_133;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Java26BuiltinDimension {
    Overworld,
    Nether,
    End,
}

impl Java26BuiltinDimension {
    pub const ALL: [Self; 3] = [Self::Overworld, Self::Nether, Self::End];

    pub fn resource_id(self) -> &'static str {
        match self {
            Self::Overworld => "minecraft:overworld",
            Self::Nether => "minecraft:the_nether",
            Self::End => "minecraft:the_end",
        }
    }

    /// The dimension one `ResourceKey<LevelStem>` id names.
    ///
    /// `Registries.levelStemToLevel` is the identity on the id
    /// (`Registries.java`), so a stem key and a level key are the same string
    /// and this is the only place the two surfaces meet.
    pub fn from_resource_id(resource_id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|dimension| dimension.resource_id() == resource_id)
    }

    pub fn directory_name(self) -> &'static str {
        match self {
            Self::Overworld => "overworld",
            Self::Nether => "the_nether",
            Self::End => "the_end",
        }
    }

    /// Built-in `coordinate_scale` for this dimension's default type
    /// (`overworld.json` 1, `the_nether.json` 8, `the_end.json` 1).
    ///
    /// `DimensionType.getTeleportationScale` (`DimensionType.java:109-113`)
    /// divides the leaving type's scale by the arriving type's; `/execute in`
    /// uses that ratio on X/Z (`CommandSourceStack.withLevel`,
    /// `CommandSourceStack.java:307-329`).
    pub fn coordinate_scale(self) -> f64 {
        match self {
            Self::Overworld => 1.0,
            Self::Nether => 8.0,
            Self::End => 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Java26LevelKey {
    resource_id: String,
    namespace_end: usize,
}

impl Java26LevelKey {
    pub fn parse(raw: &str) -> SaveResult<Self> {
        let resource_id = normalize_resource_location(raw.to_string()).map_err(|_| {
            SaveError::InvalidLevelKey {
                key: raw.to_string(),
                detail: "must be a valid resource location",
            }
        })?;
        let namespace_end = resource_id
            .find(':')
            .expect("normalized resource locations contain a namespace separator");
        let namespace = &resource_id[..namespace_end];
        let path = &resource_id[namespace_end + 1..];
        if namespace.is_empty() || matches!(namespace, "." | "..") {
            return Err(SaveError::InvalidLevelKey {
                key: raw.to_string(),
                detail: "namespace must name one safe storage directory",
            });
        }
        if path.is_empty()
            || path
                .split('/')
                .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
        {
            return Err(SaveError::InvalidLevelKey {
                key: raw.to_string(),
                detail: "path must contain only non-empty, non-dot storage segments",
            });
        }
        Ok(Self {
            resource_id,
            namespace_end,
        })
    }

    pub fn from_builtin(dimension: Java26BuiltinDimension) -> Self {
        Self::parse(dimension.resource_id()).expect("built-in level keys are storage-safe")
    }

    pub fn builtin(&self) -> Option<Java26BuiltinDimension> {
        Java26BuiltinDimension::from_resource_id(self.as_str())
    }

    pub fn as_str(&self) -> &str {
        &self.resource_id
    }

    pub fn resource_id(&self) -> &str {
        self.as_str()
    }

    pub fn namespace(&self) -> &str {
        &self.resource_id[..self.namespace_end]
    }

    pub fn path(&self) -> &str {
        &self.resource_id[self.namespace_end + 1..]
    }

    pub fn storage_relative_path(&self) -> PathBuf {
        PathBuf::from(self.namespace()).join(self.path())
    }

    pub fn builtin_keys() -> [Self; 3] {
        Java26BuiltinDimension::ALL.map(Self::from_builtin)
    }
}

impl From<Java26BuiltinDimension> for Java26LevelKey {
    fn from(dimension: Java26BuiltinDimension) -> Self {
        Self::from_builtin(dimension)
    }
}

impl Ord for Java26LevelKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.builtin(), other.builtin()) {
            (Some(left), Some(right)) => left.cmp(&right),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            // `Identifier.compareTo` orders path before namespace
            // (`Identifier.java:142-148`).
            (None, None) => self
                .path()
                .cmp(other.path())
                .then_with(|| self.namespace().cmp(other.namespace())),
        }
    }
}

impl PartialOrd for Java26LevelKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSaveLimits {
    pub max_compressed_nbt_bytes: usize,
    pub max_decompressed_nbt_bytes: usize,
    pub max_region_bytes: u64,
    pub max_chunk_compressed_bytes: usize,
    pub max_chunk_decompressed_bytes: usize,
    pub max_external_chunk_bytes: usize,
}

impl Default for WorldSaveLimits {
    fn default() -> Self {
        Self {
            max_compressed_nbt_bytes: 32 * 1024 * 1024,
            max_decompressed_nbt_bytes: 128 * 1024 * 1024,
            max_region_bytes: 1024 * 1024 * 1024,
            max_chunk_compressed_bytes: 64 * 1024 * 1024,
            max_chunk_decompressed_bytes: 128 * 1024 * 1024,
            max_external_chunk_bytes: 64 * 1024 * 1024,
        }
    }
}

impl WorldSaveLimits {
    pub fn validate(self) -> SaveResult<Self> {
        if self.max_compressed_nbt_bytes == 0 {
            return Err(SaveError::InvalidLimits(
                "max_compressed_nbt_bytes must be non-zero",
            ));
        }
        if self.max_decompressed_nbt_bytes == 0 {
            return Err(SaveError::InvalidLimits(
                "max_decompressed_nbt_bytes must be non-zero",
            ));
        }
        if self.max_region_bytes < 8192 {
            return Err(SaveError::InvalidLimits(
                "max_region_bytes must fit both Anvil header sectors",
            ));
        }
        if self.max_chunk_compressed_bytes == 0
            || self.max_chunk_decompressed_bytes == 0
            || self.max_external_chunk_bytes == 0
        {
            return Err(SaveError::InvalidLimits(
                "chunk compressed, decompressed, and external limits must be non-zero",
            ));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedCompound {
    root_name: JavaString,
    value: NbtSummaryValue,
}

impl NamedCompound {
    pub fn new(root_name: impl Into<JavaString>, entries: Vec<NbtSummaryEntry>) -> Self {
        Self {
            root_name: root_name.into(),
            value: NbtSummaryValue::Compound(entries),
        }
    }

    pub fn try_from_summary(
        root_name: impl Into<JavaString>,
        value: NbtSummaryValue,
    ) -> SaveResult<Self> {
        if !matches!(value, NbtSummaryValue::Compound(_)) {
            return Err(SaveError::InvalidNbt {
                path: None,
                detail: "persistent NBT root must be a compound".to_string(),
            });
        }
        Ok(Self {
            root_name: root_name.into(),
            value,
        })
    }

    pub fn root_name(&self) -> &JavaString {
        &self.root_name
    }

    pub fn entries(&self) -> &[NbtSummaryEntry] {
        match &self.value {
            NbtSummaryValue::Compound(entries) => entries,
            _ => unreachable!("NamedCompound always wraps an NbtSummaryValue::Compound"),
        }
    }

    pub fn entries_mut(&mut self) -> &mut Vec<NbtSummaryEntry> {
        match &mut self.value {
            NbtSummaryValue::Compound(entries) => entries,
            _ => unreachable!("NamedCompound always wraps an NbtSummaryValue::Compound"),
        }
    }

    pub fn into_parts(self) -> (JavaString, Vec<NbtSummaryEntry>) {
        match self.value {
            NbtSummaryValue::Compound(entries) => (self.root_name, entries),
            _ => unreachable!("NamedCompound always wraps an NbtSummaryValue::Compound"),
        }
    }

    pub fn as_summary(&self) -> &NbtSummaryValue {
        &self.value
    }

    pub fn to_summary(&self) -> NbtSummaryValue {
        self.value.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldSavePaths {
    root: PathBuf,
}

impl WorldSavePaths {
    pub fn new(root: impl Into<PathBuf>) -> SaveResult<Self> {
        let root = root.into();
        if root.as_os_str().is_empty() {
            return Err(SaveError::InvalidPath {
                path: root,
                detail: "world root must not be empty",
            });
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn level_dat(&self) -> PathBuf {
        self.root.join("level.dat")
    }

    pub fn level_dat_old(&self) -> PathBuf {
        self.root.join("level.dat_old")
    }

    pub fn session_lock(&self) -> PathBuf {
        self.root.join("session.lock")
    }

    pub fn player_data_dir(&self) -> PathBuf {
        self.root.join("players").join("data")
    }

    /// Vanilla `LevelResource.PLAYER_STATS_DIR`
    /// (`LevelResource.java:5`): `<world>/players/stats`, beside the
    /// playerdata rather than at the save root.
    pub fn player_stats_dir(&self) -> PathBuf {
        self.root.join("players").join("stats")
    }

    /// One player's statistics file, which `PlayerList.locateStatsFile`
    /// (`PlayerList.java:765-786`) names after the profile's UUID. Its legacy
    /// `<name>.json` migration is not modelled: no save this server can have
    /// written carries one, and moving a file on load is a write the read path
    /// must not make.
    pub fn player_stats_file(&self, uuid: uuid::Uuid) -> PathBuf {
        self.player_stats_dir()
            .join(format!("{}.json", uuid.hyphenated()))
    }

    /// Vanilla `LevelResource.DATA` (`<world>/data`).
    pub fn saved_data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    /// `WorldGenSettings.TYPE` id `minecraft:world_gen_settings` resolved like
    /// vanilla `SavedDataStorage.getDataFile`: `<world>/data/minecraft/world_gen_settings.dat`.
    pub fn world_gen_settings_dat(&self) -> PathBuf {
        self.saved_data_dir()
            .join("minecraft")
            .join("world_gen_settings.dat")
    }

    /// `GameRuleMap.TYPE` id `minecraft:game_rules` (`GameRuleMap.java:21-23`)
    /// resolved the same way (`SavedDataStorage.java:56-63`):
    /// `<world>/data/minecraft/game_rules.dat`.
    pub fn game_rules_dat(&self) -> PathBuf {
        self.saved_data_dir()
            .join("minecraft")
            .join("game_rules.dat")
    }

    /// `RandomSequences.TYPE` id `minecraft:random_sequences`
    /// (`RandomSequences.java:26-28`) resolved the same way
    /// (`SavedDataStorage.java:56-63`):
    /// `<world>/data/minecraft/random_sequences.dat`.
    pub fn random_sequences_dat(&self) -> PathBuf {
        self.saved_data_dir()
            .join("minecraft")
            .join("random_sequences.dat")
    }

    /// `ServerClockManager.TYPE` id `minecraft:world_clocks`
    /// (`ServerClockManager.java:24-30`) resolved by `SavedDataStorage` below
    /// the level data folder.
    pub fn world_clocks_dat(&self) -> PathBuf {
        self.saved_data_dir()
            .join("minecraft")
            .join("world_clocks.dat")
    }

    /// `MapId.key` (`MapId.java:22-23`) resolves below the SavedData root as
    /// `minecraft:maps/<signed id>`.
    pub fn maps_dir(&self) -> PathBuf {
        self.saved_data_dir().join("minecraft").join("maps")
    }

    pub fn map_data_dat(&self, id: i32) -> PathBuf {
        self.maps_dir().join(format!("{id}.dat"))
    }

    pub fn map_index_dat(&self) -> PathBuf {
        self.maps_dir().join("last_id.dat")
    }

    pub fn overworld_dimension_dir(&self) -> PathBuf {
        self.builtin_dimension_dir(Java26BuiltinDimension::Overworld)
    }

    pub fn builtin_dimension_dir(&self, dimension: Java26BuiltinDimension) -> PathBuf {
        self.dimension_dir(&dimension.into())
    }

    /// `DimensionType.getStorageFolder` (`DimensionType.java:115-117`).
    pub fn dimension_dir(&self, key: &Java26LevelKey) -> PathBuf {
        self.root
            .join("dimensions")
            .join(key.storage_relative_path())
    }

    pub fn region_dir(&self) -> PathBuf {
        self.overworld_dimension_dir().join("region")
    }

    pub fn entities_dir(&self) -> PathBuf {
        self.overworld_dimension_dir().join("entities")
    }

    pub fn poi_dir(&self) -> PathBuf {
        self.overworld_dimension_dir().join("poi")
    }
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("invalid world-save limits: {0}")]
    InvalidLimits(&'static str),
    #[error("invalid world-save path {path:?}: {detail}")]
    InvalidPath { path: PathBuf, detail: &'static str },
    #[error("invalid level key {key:?}: {detail}")]
    InvalidLevelKey { key: String, detail: &'static str },
    #[error("world-save budget exceeded for {resource}: {actual} > {limit}")]
    BudgetExceeded {
        resource: &'static str,
        actual: u64,
        limit: u64,
    },
    #[error("invalid persistent NBT{path_suffix}: {detail}", path_suffix = path.as_ref().map(|path| format!(" at {}", path.display())).unwrap_or_default())]
    InvalidNbt {
        path: Option<PathBuf>,
        detail: String,
    },
    #[error("invalid Anvil region {path}: {detail}", path = path.display())]
    InvalidRegion { path: PathBuf, detail: String },
    /// The one wording every refused record carries; this server reads 26.1
    /// data only and leaves upgrading to the official server.
    #[error(
        "data version {actual} is not Minecraft 26.1 ({expected}); open the world once with the official 26.1 server using --forceUpgrade"
    )]
    UnsupportedDataVersion { expected: i32, actual: i32 },
    #[error("unsupported Java storage version {actual}; expected {expected}")]
    UnsupportedStorageVersion { expected: i32, actual: i32 },
    #[error("unsupported Anvil compression id {id} in {path}", path = path.display())]
    UnsupportedCompression { path: PathBuf, id: u8 },
    #[error("world-save I/O failed while {operation} {path}: {source}", path = path.display())]
    Io {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("atomic world-save publication failed for {path}: {detail}", path = path.display())]
    AtomicPublish { path: PathBuf, detail: String },
    #[error(
        "neither level.dat nor level.dat_old was usable; primary: {primary}; fallback: {fallback}"
    )]
    NoUsableLevelData { primary: String, fallback: String },
}

pub type SaveResult<T> = std::result::Result<T, SaveError>;

impl From<bbb_protocol::chunk_light::LightError> for SaveError {
    fn from(error: bbb_protocol::chunk_light::LightError) -> Self {
        Self::InvalidNbt {
            path: None,
            detail: error.to_string(),
        }
    }
}

pub mod world_gen;
pub use self::world_gen::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_version_and_default_budgets_are_explicit_and_valid() {
        assert_eq!(JAVA_26_1_DATA_VERSION, 4786);
        assert_eq!(JAVA_26_1_STORAGE_VERSION, 19_133);
        WorldSaveLimits::default().validate().unwrap();
        let invalid = WorldSaveLimits {
            max_region_bytes: 8191,
            ..WorldSaveLimits::default()
        };
        assert!(matches!(
            invalid.validate(),
            Err(SaveError::InvalidLimits(
                "max_region_bytes must fit both Anvil header sectors"
            ))
        ));
    }

    #[test]
    fn world_save_paths_follow_vanilla_level_resources() {
        let paths = WorldSavePaths::new("world").unwrap();
        assert_eq!(paths.level_dat(), Path::new("world/level.dat"));
        assert_eq!(paths.level_dat_old(), Path::new("world/level.dat_old"));
        assert_eq!(paths.session_lock(), Path::new("world/session.lock"));
        assert_eq!(paths.player_data_dir(), Path::new("world/players/data"));
        assert_eq!(
            paths.map_data_dat(-42),
            Path::new("world/data/minecraft/maps/-42.dat")
        );
        assert_eq!(
            paths.map_index_dat(),
            Path::new("world/data/minecraft/maps/last_id.dat")
        );
        assert_eq!(
            paths.overworld_dimension_dir(),
            Path::new("world/dimensions/minecraft/overworld")
        );
        assert_eq!(
            paths.region_dir(),
            Path::new("world/dimensions/minecraft/overworld/region")
        );
        assert_eq!(
            paths.entities_dir(),
            Path::new("world/dimensions/minecraft/overworld/entities")
        );
        assert_eq!(
            paths.poi_dir(),
            Path::new("world/dimensions/minecraft/overworld/poi")
        );
        assert_eq!(
            paths.dimension_dir(&Java26LevelKey::parse("example:sky/moon").unwrap()),
            Path::new("world/dimensions/example/sky/moon")
        );
    }

    #[test]
    fn level_keys_normalize_validate_and_round_trip_builtins() {
        let overworld = Java26LevelKey::parse("overworld").unwrap();
        assert_eq!(overworld.as_str(), "minecraft:overworld");
        assert_eq!(Java26LevelKey::parse(":overworld").unwrap(), overworld);
        assert_eq!(overworld.builtin(), Some(Java26BuiltinDimension::Overworld));
        assert_eq!(
            Java26LevelKey::builtin_keys(),
            Java26BuiltinDimension::ALL.map(Java26LevelKey::from)
        );
        let custom = Java26LevelKey::parse("example:sky/moon").unwrap();
        assert_eq!(custom.namespace(), "example");
        assert_eq!(custom.path(), "sky/moon");
        assert_eq!(custom.resource_id(), "example:sky/moon");
        assert_eq!(custom.builtin(), None);
    }

    #[test]
    fn level_keys_order_builtins_before_deterministic_custom_keys() {
        let mut keys = [
            Java26LevelKey::parse("z:alpha").unwrap(),
            Java26LevelKey::parse("example:beta").unwrap(),
            Java26LevelKey::from(Java26BuiltinDimension::End),
            Java26LevelKey::parse("a:alpha").unwrap(),
            Java26LevelKey::from(Java26BuiltinDimension::Overworld),
            Java26LevelKey::from(Java26BuiltinDimension::Nether),
        ];
        keys.sort();
        assert_eq!(
            keys.map(|key| key.resource_id),
            [
                "minecraft:overworld",
                "minecraft:the_nether",
                "minecraft:the_end",
                "a:alpha",
                "z:alpha",
                "example:beta",
            ]
        );
    }

    #[test]
    fn level_key_storage_paths_reject_traversal_shapes() {
        for invalid in [
            "",
            ".:moon",
            "example:",
            "example:/moon",
            "example:moon/",
            "example:moon//child",
            "example:./moon",
            "example:moon/./child",
            "example:../moon",
            "example:moon/../child",
            "Example:moon",
        ] {
            assert!(
                matches!(
                    Java26LevelKey::parse(invalid),
                    Err(SaveError::InvalidLevelKey { .. })
                ),
                "accepted unsafe level key {invalid:?}"
            );
        }
    }

    #[test]
    fn named_compound_rejects_non_compound_roots() {
        assert!(matches!(
            NamedCompound::try_from_summary("", NbtSummaryValue::Int(1)),
            Err(SaveError::InvalidNbt { path: None, .. })
        ));
    }

    #[test]
    fn named_compound_keeps_root_units_through_parts() {
        for units in [vec![0xd800], vec![0xd801], vec![0xfffd], vec![b'?' as u16]] {
            let name = JavaString::from(units.clone());
            let document = NamedCompound::new(name.clone(), Vec::new());
            assert_eq!(document.root_name().units(), units);
            let (restored, entries) = document.into_parts();
            assert_eq!(restored.units(), units);
            assert!(entries.is_empty());
        }
    }
}
