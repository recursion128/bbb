//! Pack identity, format-compatibility and resource-read contracts for the Java 26.1
//! datapack surface.
//!
//! Vanilla anchors:
//! - `PackFormat.java` — `PackFormat(major, minor)`; the 26.1 game data-pack format is
//!   `(101, 1)` (`SharedConstants.java:31-33`).
//! - `PackCompatibility.java:34-43` — a pack is compatible when the game format lies
//!   inside the pack's declared `[min_format, max_format]` range; a pack without any
//!   format declaration is `UNKNOWN`.
//! - `PackSource.java:10-17` — `BUILT_IN` and `WORLD` packs are added to the selection
//!   automatically, `FEATURE` packs are not.
//! - `PackResources.java` — a pack exposes its `SERVER_DATA` content per namespace.

use std::fmt;

/// `PackFormat(major, minor)` (`PackFormat.java`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackFormat {
    pub major: u32,
    pub minor: u32,
}

impl PackFormat {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }
}

impl fmt::Display for PackFormat {
    /// `PackFormat.toString`: a `u32::MAX` minor renders as `<major>.*`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.minor == u32::MAX {
            write!(f, "{}.*", self.major)
        } else {
            write!(f, "{}.{}", self.major, self.minor)
        }
    }
}

/// `SharedConstants.getCurrentVersion().packVersion(SERVER_DATA)` for Java 26.1
/// (`SharedConstants.java:31-33`: major 101, minor 1).
pub const JAVA_26_1_SERVER_DATA_FORMAT: PackFormat = PackFormat::new(101, 1);

/// Vanilla sentinel for a pack without any format declaration
/// (`PackMetadataSection.FALLBACK_CODEC` maps it to `PackFormat.of(Integer.MAX_VALUE)`,
/// and `PackCompatibility.forVersion` then reports `UNKNOWN`).
pub const UNDECLARED_FORMAT: PackFormat = PackFormat::new(u32::MAX, 0);

/// `PackCompatibility` (`PackCompatibility.java:8-12`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackCompatibility {
    TooOld,
    TooNew,
    Unknown,
    Compatible,
}

impl PackCompatibility {
    /// `PackCompatibility.isCompatible`.
    pub fn is_compatible(self) -> bool {
        matches!(self, Self::Compatible)
    }
}

/// Inclusive `[min_format, max_format]` range declared through `pack.mcmeta`
/// (`PackFormat.IntermediaryFormat`, validated by `PackMetadataSection`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclaredFormatRange {
    pub min: PackFormat,
    pub max: PackFormat,
}

impl DeclaredFormatRange {
    /// `PackCompatibility.forVersion` (`PackCompatibility.java:34-43`): an undeclared
    /// range maps to `Unknown`; otherwise the game format must lie inside the range.
    pub fn compatibility(&self, game: PackFormat) -> PackCompatibility {
        if self.min.major == u32::MAX {
            PackCompatibility::Unknown
        } else if self.max < game {
            PackCompatibility::TooOld
        } else if game < self.min {
            PackCompatibility::TooNew
        } else {
            PackCompatibility::Compatible
        }
    }
}

/// `PackSource` for server datapacks (`PackSource.java:10-17`): the built-in vanilla
/// pack, a built-in feature pack, or a pack discovered in `<world>/datapacks`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerPackSource {
    BuiltIn,
    Feature,
    World,
}

impl ServerPackSource {
    /// `PackSource.shouldAddAutomatically`: only `FEATURE` packs do not auto-add.
    pub fn should_add_automatically(self) -> bool {
        !matches!(self, Self::Feature)
    }
}

/// Read surface of one opened pack for `PackType.SERVER_DATA` (`PackResources.java`).
///
/// All paths are pack-relative POSIX strings below the namespace root
/// (`data/<namespace>/<path>`), always with forward slashes. Implementations must
/// reject path escapes rather than resolve them outside the pack.
pub trait ServerPackResources {
    /// `PackResources.getNamespaces(SERVER_DATA)`: sorted, de-duplicated namespaces.
    fn provided_namespaces(&self) -> Vec<String>;

    /// Namespace-relative paths under `<dir>/` (e.g. `worldgen/biome`), sorted.
    /// Extension handling (`.json`, `.mcfunction`) is the caller's concern
    /// (`FileToIdConverter`).
    fn list_paths(&self, namespace: &str, dir: &str) -> Vec<String>;

    /// Exact bytes of `data/<namespace>/<path>`; `None` when absent.
    fn read(&self, namespace: &str, path: &str) -> Option<Vec<u8>>;

    /// Whether `data/<namespace>/<path>` exists in this pack without decoding
    /// or copying its contents.
    fn contains(&self, namespace: &str, path: &str) -> bool {
        self.read(namespace, path).is_some()
    }

    /// Shared immutable bytes of `data/<namespace>/<path>`; `None` when absent.
    fn read_shared(&self, namespace: &str, path: &str) -> Option<std::sync::Arc<[u8]>> {
        self.read(namespace, path).map(Into::into)
    }

    /// `PackResources.getRootResource`: bytes of a pack-root file such as
    /// `pack.mcmeta`; `None` when absent.
    fn read_root(&self, path: &str) -> Option<Vec<u8>>;

    /// Opens an overlay directory inside this pack (`<directory>/data/...`) when
    /// the backing pack format supports overlays (`PathResourcesSupplier.openFull`,
    /// `FileResourcesSupplier.openFull`).
    fn open_overlay(&self, _directory: &str) -> Option<Box<dyn ServerPackResources>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(min: PackFormat, max: PackFormat) -> DeclaredFormatRange {
        DeclaredFormatRange { min, max }
    }

    #[test]
    fn compatibility_mirrors_for_version_branches() {
        let game = JAVA_26_1_SERVER_DATA_FORMAT;
        assert_eq!(
            range(UNDECLARED_FORMAT, UNDECLARED_FORMAT).compatibility(game),
            PackCompatibility::Unknown
        );
        assert_eq!(
            range(PackFormat::new(81, 0), PackFormat::new(100, 0)).compatibility(game),
            PackCompatibility::TooOld
        );
        assert_eq!(
            range(PackFormat::new(102, 0), PackFormat::new(102, 0)).compatibility(game),
            PackCompatibility::TooNew
        );
        assert_eq!(
            range(PackFormat::new(101, 0), PackFormat::new(101, 1)).compatibility(game),
            PackCompatibility::Compatible
        );
        assert_eq!(
            range(PackFormat::new(101, 1), PackFormat::new(101, 1)).compatibility(game),
            PackCompatibility::Compatible
        );
        assert_eq!(
            range(PackFormat::new(100, 0), PackFormat::new(200, 0)).compatibility(game),
            PackCompatibility::Compatible
        );
        assert!(
            range(PackFormat::new(101, 0), PackFormat::new(101, u32::MAX))
                .compatibility(game)
                .is_compatible()
        );
    }

    #[test]
    fn pack_format_orders_major_then_minor() {
        assert!(PackFormat::new(101, 0) < PackFormat::new(101, 1));
        assert!(PackFormat::new(101, 1) < PackFormat::new(102, 0));
        assert_eq!(PackFormat::new(101, u32::MAX).to_string(), "101.*");
        assert_eq!(PackFormat::new(101, 1).to_string(), "101.1");
    }

    #[test]
    fn only_feature_packs_do_not_auto_add() {
        assert!(ServerPackSource::BuiltIn.should_add_automatically());
        assert!(ServerPackSource::World.should_add_automatically());
        assert!(!ServerPackSource::Feature.should_add_automatically());
    }
}
