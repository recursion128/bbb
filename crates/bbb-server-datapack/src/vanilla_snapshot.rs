//! Reader for the embedded vanilla 26.1 built-in datapack snapshot.
//!
//! Vanilla's built-in server datapack is the `data/minecraft` tree inside the server
//! jar (`ServerPacksSource.createVanillaPackSource`, `ServerPacksSource.java:48-55`).
//! BBB has no jar; the registry-surface subtree (plus `dimension/`, the raw
//! `structure/` template tree, and the built-in feature packs under `datapacks/`) is
//! snapshotted by
//! `scripts/generate-vanilla-datapack-snapshot.py` into a deterministic gzip blob that
//! this module decodes once at startup. There is no runtime dependency on an external
//! vanilla data path.
//!
//! Blob format (frozen contract with the generator): a deterministic gzip stream
//! (mtime=0) over records sorted lexicographically by path bytes, each record
//! `[u32 LE path byte length][path UTF-8][u64 LE data byte length][data]`. Paths are
//! relative to `data/minecraft/` with forward slashes, e.g.
//! `worldgen/biome/plains.json` or `datapacks/trade_rebalance/pack.mcmeta`.

use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;
use std::sync::{Arc, OnceLock};

use flate2::read::GzDecoder;

use super::pack::ServerPackResources;

/// Decompressed-size budget for the snapshot; the 26.1 registry surface is about
/// 2.1 MiB raw, so 64 MiB leaves ample headroom while rejecting corrupt streams.
pub const MAX_DECOMPRESSED_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

/// A snapshot format violation; corrupt embedded content is a build/generation bug,
/// not user input, but it is still rejected with attribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotFormatError(pub String);

impl fmt::Display for SnapshotFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid vanilla datapack snapshot: {}", self.0)
    }
}

impl std::error::Error for SnapshotFormatError {}

/// The decoded built-in datapack snapshot: decompressed records indexed by path.
#[derive(Debug)]
pub struct VanillaDatapackSnapshot {
    records: BTreeMap<String, Arc<[u8]>>,
}

/// The checked-in snapshot produced by `scripts/generate-vanilla-datapack-snapshot.py`
/// (drift is caught by its `--check` mode and the generator's pytest).
const EMBEDDED_SNAPSHOT: &[u8] = include_bytes!("../data/vanilla_datapack_snapshot_26_1.bin.gz");

static SHARED_EMBEDDED_SNAPSHOT: OnceLock<
    Result<Arc<VanillaDatapackSnapshot>, SnapshotFormatError>,
> = OnceLock::new();

impl VanillaDatapackSnapshot {
    /// Decode the checked-in embedded snapshot. This never fails in a correct build;
    /// a format error here means the committed artifact is corrupt.
    pub fn embedded() -> Result<Self, SnapshotFormatError> {
        Self::from_bytes(EMBEDDED_SNAPSHOT)
    }

    /// The process-wide decoded and validated embedded snapshot.
    pub fn embedded_shared() -> Result<Arc<Self>, SnapshotFormatError> {
        SHARED_EMBEDDED_SNAPSHOT
            .get_or_init(|| Self::embedded().map(Arc::new))
            .clone()
    }

    /// Decode and validate a snapshot blob.
    pub fn from_bytes(gzip_bytes: &[u8]) -> Result<Self, SnapshotFormatError> {
        let decoder = GzDecoder::new(gzip_bytes);
        let mut data = Vec::new();
        decoder
            .take(MAX_DECOMPRESSED_SNAPSHOT_BYTES as u64 + 1)
            .read_to_end(&mut data)
            .map_err(|error| SnapshotFormatError(format!("gzip decode failed: {error}")))?;
        if data.len() > MAX_DECOMPRESSED_SNAPSHOT_BYTES {
            return Err(SnapshotFormatError(format!(
                "decompressed snapshot exceeds the {MAX_DECOMPRESSED_SNAPSHOT_BYTES} byte budget"
            )));
        }

        let mut records = BTreeMap::new();
        let mut cursor = 0usize;
        let mut previous_path: Option<String> = None;
        while cursor < data.len() {
            let path_len = read_u32(&data, &mut cursor)? as usize;
            let path = read_str(&data, &mut cursor, path_len)?;
            if path.is_empty()
                || path.starts_with('/')
                || path.split('/').any(|part| part.is_empty() || part == "..")
            {
                return Err(SnapshotFormatError(format!(
                    "record path {path:?} is not a safe relative path"
                )));
            }
            if let Some(previous) = &previous_path {
                if *previous >= path {
                    return Err(SnapshotFormatError(format!(
                        "record paths are not strictly sorted: {previous:?} then {path:?}"
                    )));
                }
            }
            let data_len = read_u64(&data, &mut cursor)? as usize;
            let end = cursor
                .checked_add(data_len)
                .filter(|end| *end <= data.len())
                .ok_or_else(|| {
                    SnapshotFormatError(format!("record {path:?} overruns the snapshot"))
                })?;
            records.insert(path.clone(), Arc::from(&data[cursor..end]));
            previous_path = Some(path);
            cursor = end;
        }
        Ok(Self { records })
    }

    /// Sorted snapshot paths (relative to `data/minecraft/`).
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.records.keys().map(String::as_str)
    }

    /// Bytes of one snapshot path; `None` when absent.
    pub fn read_path(&self, path: &str) -> Option<&[u8]> {
        self.records.get(path).map(AsRef::as_ref)
    }

    /// Shared bytes of one snapshot path; `None` when absent.
    pub fn read_shared_path(&self, path: &str) -> Option<Arc<[u8]>> {
        self.records.get(path).cloned()
    }

    /// The tag ids one registry's `tags/` tree declares
    /// (`Registries.tagsDirPath`, Registries.java:331-333), namespaced and
    /// sorted: what `HolderLookup.listTagIds` (HolderLookup.java:25-27) answers
    /// for a registry loaded from the built-in pack alone.
    pub fn tag_ids(&self, registry_path: &str) -> Vec<String> {
        let prefix = format!("tags/{registry_path}/");
        self.records
            .keys()
            .filter_map(|path| path.strip_prefix(&prefix))
            .filter_map(|tag| tag.strip_suffix(".json"))
            .map(|tag| format!("minecraft:{tag}"))
            .collect()
    }

    /// The built-in vanilla pack: content directly under `data/minecraft/`
    /// (`ServerPacksSource.createVanillaPackSource`, namespace `minecraft`).
    pub fn vanilla_pack(self: &Arc<Self>) -> SnapshotPackResources {
        SnapshotPackResources {
            snapshot: Arc::clone(self),
            root: String::new(),
        }
    }

    /// Ids of the built-in feature packs (`ServerPacksSource.PACKS_DIR`,
    /// `ServerPacksSource.java:37`): top-level directories under `datapacks/`, sorted.
    pub fn feature_pack_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .records
            .keys()
            .filter_map(|path| path.strip_prefix("datapacks/"))
            .filter_map(|rest| rest.split('/').next())
            .map(str::to_string)
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }

    /// One built-in feature pack rooted at `datapacks/<id>/`.
    pub fn feature_pack(self: &Arc<Self>, id: &str) -> Option<SnapshotPackResources> {
        let root = format!("datapacks/{id}/");
        if id.is_empty()
            || id.contains('/')
            || !self.records.keys().any(|path| path.starts_with(&root))
        {
            return None;
        }
        Some(SnapshotPackResources {
            snapshot: Arc::clone(self),
            root,
        })
    }
}

/// `ServerPackResources` over one rooted subtree of the snapshot. The vanilla pack
/// has an empty root: the whole snapshot tree is its `minecraft` namespace content
/// (the blob stores paths relative to `data/minecraft/`). A feature pack is rooted at
/// `datapacks/<id>/` and exposes namespaces under its own `data/` directory.
pub struct SnapshotPackResources {
    snapshot: Arc<VanillaDatapackSnapshot>,
    root: String,
}

impl SnapshotPackResources {
    /// Snapshot path for pack content `(namespace, path)`, or `None` when the pack
    /// does not provide the namespace.
    fn content_key(&self, namespace: &str, path: &str) -> Option<String> {
        if self.root.is_empty() {
            (namespace == "minecraft").then(|| path.to_string())
        } else {
            Some(format!("{}data/{namespace}/{path}", self.root))
        }
    }

    fn content_prefix(&self, namespace: &str) -> Option<String> {
        self.content_key(namespace, "")
    }
}

impl ServerPackResources for SnapshotPackResources {
    fn provided_namespaces(&self) -> Vec<String> {
        if self.root.is_empty() {
            return vec!["minecraft".to_string()];
        }
        let prefix = format!("{}data/", self.root);
        let mut namespaces: Vec<String> = self
            .snapshot
            .records
            .keys()
            .filter_map(|path| path.strip_prefix(&prefix))
            .filter_map(|rest| rest.split('/').next())
            .map(str::to_string)
            .collect();
        namespaces.sort();
        namespaces.dedup();
        namespaces
    }

    fn list_paths(&self, namespace: &str, dir: &str) -> Vec<String> {
        let Some(content_prefix) = self.content_prefix(namespace) else {
            return Vec::new();
        };
        let list_prefix = if dir.is_empty() {
            content_prefix.clone()
        } else {
            format!("{content_prefix}{dir}/")
        };
        self.snapshot
            .records
            .range(list_prefix.clone()..)
            .take_while(|(path, _)| path.starts_with(&list_prefix))
            // Returned paths stay namespace-relative: only the namespace-content
            // prefix is stripped, the directory prefix is kept.
            .filter_map(|(path, _)| path.strip_prefix(&content_prefix))
            .map(str::to_string)
            .collect()
    }

    fn contains(&self, namespace: &str, path: &str) -> bool {
        self.content_key(namespace, path)
            .is_some_and(|key| self.snapshot.records.contains_key(&key))
    }

    fn read(&self, namespace: &str, path: &str) -> Option<Vec<u8>> {
        self.snapshot
            .read_path(&self.content_key(namespace, path)?)
            .map(<[u8]>::to_vec)
    }

    fn read_shared(&self, namespace: &str, path: &str) -> Option<Arc<[u8]>> {
        self.snapshot
            .read_shared_path(&self.content_key(namespace, path)?)
    }

    fn read_root(&self, path: &str) -> Option<Vec<u8>> {
        self.snapshot
            .read_path(&format!("{}{path}", self.root))
            .map(<[u8]>::to_vec)
    }
}

fn read_u32(data: &[u8], cursor: &mut usize) -> Result<u32, SnapshotFormatError> {
    let end = cursor
        .checked_add(4)
        .filter(|end| *end <= data.len())
        .ok_or_else(|| SnapshotFormatError("truncated record path length".to_string()))?;
    let value = u32::from_le_bytes(data[*cursor..end].try_into().unwrap());
    *cursor = end;
    Ok(value)
}

fn read_u64(data: &[u8], cursor: &mut usize) -> Result<u64, SnapshotFormatError> {
    let end = cursor
        .checked_add(8)
        .filter(|end| *end <= data.len())
        .ok_or_else(|| SnapshotFormatError("truncated record data length".to_string()))?;
    let value = u64::from_le_bytes(data[*cursor..end].try_into().unwrap());
    *cursor = end;
    Ok(value)
}

fn read_str(data: &[u8], cursor: &mut usize, len: usize) -> Result<String, SnapshotFormatError> {
    let end = cursor
        .checked_add(len)
        .filter(|end| *end <= data.len())
        .ok_or_else(|| SnapshotFormatError("truncated record path".to_string()))?;
    let value = std::str::from_utf8(&data[*cursor..end])
        .map_err(|error| SnapshotFormatError(format!("record path is not UTF-8: {error}")))?
        .to_string();
    *cursor = end;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn build_blob(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut raw = Vec::new();
        for (path, content) in files {
            raw.extend_from_slice(&(path.len() as u32).to_le_bytes());
            raw.extend_from_slice(path.as_bytes());
            raw.extend_from_slice(&(content.len() as u64).to_le_bytes());
            raw.extend_from_slice(content);
        }
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&raw).unwrap();
        encoder.finish().unwrap()
    }

    fn snapshot(files: &[(&str, &[u8])]) -> VanillaDatapackSnapshot {
        VanillaDatapackSnapshot::from_bytes(&build_blob(files)).unwrap()
    }

    #[test]
    fn embedded_snapshot_decodes_with_expected_surface() {
        let snapshot = VanillaDatapackSnapshot::embedded_shared().unwrap();
        // The 26.1 registry surface, the `tags/` tree (D2), the `loot_table/` tree
        // (D3), the `recipe/` and `advancement/` trees (D4), the raw `structure/`
        // template tree (D5) and the three built-in feature packs
        // (`data/minecraft/datapacks/`, `ServerPacksSource.PACKS_DIR`).
        assert_eq!(snapshot.paths().count(), 8282);
        assert_eq!(
            snapshot.feature_pack_ids(),
            [
                "minecart_improvements",
                "redstone_experiments",
                "trade_rebalance"
            ]
        );
        let vanilla = snapshot.vanilla_pack();
        assert_eq!(vanilla.provided_namespaces(), ["minecraft"]);
        assert!(vanilla
            .list_paths("minecraft", "worldgen/biome")
            .contains(&"worldgen/biome/plains.json".to_string()));
        // 26.1 ships no built-in `dimension/` tree; the level stems are
        // bootstrapped in code (`WorldPresets`/`WorldDimensions`).
        assert!(vanilla.list_paths("minecraft", "dimension").is_empty());
        // D2: the built-in tag tree ships in the snapshot (`Registries.tagsDirPath`).
        assert!(vanilla
            .list_paths("minecraft", "tags/worldgen/biome")
            .contains(&"tags/worldgen/biome/is_end.json".to_string()));
        // D3: the built-in loot table tree ships in the snapshot (`LootDataType.TABLE`).
        assert!(vanilla
            .list_paths("minecraft", "loot_table")
            .contains(&"loot_table/entities/zombie.json".to_string()));
        // D4: the built-in recipe and advancement trees ship in the snapshot.
        assert!(vanilla
            .list_paths("minecraft", "recipe")
            .contains(&"recipe/crafting_table.json".to_string()));
        assert!(vanilla
            .list_paths("minecraft", "advancement")
            .contains(&"advancement/adventure/root.json".to_string()));
        // D5: the built-in structure template tree ships as raw gzip NBT bytes
        // (pass-through; verified byte-exact against the 26.1 source tree file
        // `data/minecraft/structure/empty.nbt`, sha256 f5c974ee...).
        const EMPTY_NBT_BYTES: &[u8] = &[
            0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x3d, 0xcb, 0xb1, 0x0e,
            0x82, 0x40, 0x10, 0x84, 0xe1, 0x31, 0x07, 0xca, 0x41, 0x6c, 0x7c, 0x1c, 0x6b, 0x6b,
            0x4b, 0xfb, 0x85, 0x8c, 0xc9, 0x46, 0xb8, 0x23, 0xb7, 0x5b, 0xf9, 0x58, 0x3e, 0xa1,
            0xa2, 0xe0, 0x24, 0x7f, 0x35, 0xf9, 0x5a, 0x20, 0xa2, 0x32, 0x7d, 0x32, 0x00, 0x58,
            0xda, 0x6d, 0x45, 0x34, 0x4c, 0xae, 0xae, 0x34, 0x2c, 0x8b, 0xd8, 0xf7, 0x63, 0x1e,
            0x1e, 0xd6, 0xfe, 0xde, 0x30, 0x67, 0xdb, 0xd0, 0x7f, 0x01, 0xb5, 0xb9, 0x38, 0x57,
            0x71, 0x98, 0x65, 0xa4, 0x3b, 0xbf, 0xa4, 0x41, 0x75, 0x95, 0x89, 0x38, 0x4e, 0x9a,
            0x38, 0x14, 0xb9, 0xfb, 0x59, 0xb4, 0x7c, 0x48, 0x77, 0x11, 0x97, 0x1b, 0x8b, 0x69,
            0x4e, 0xc0, 0xe9, 0x85, 0x37, 0x61, 0x9b, 0x70, 0x87, 0x96, 0x00, 0x00, 0x00,
        ];
        assert_eq!(
            vanilla.read("minecraft", "structure/empty.nbt"),
            Some(EMPTY_NBT_BYTES.to_vec())
        );
        for feature_id in snapshot.feature_pack_ids() {
            let pack = snapshot.feature_pack(&feature_id).unwrap();
            assert!(
                pack.read_root("pack.mcmeta").is_some(),
                "feature pack {feature_id} must carry its pack.mcmeta"
            );
        }
    }

    #[test]
    fn tag_ids_enumerate_the_tag_tree_of_one_registry() {
        let snapshot = VanillaDatapackSnapshot::embedded_shared().unwrap();
        let blocks = snapshot.tag_ids("block");
        assert_eq!(blocks.len(), 248);
        assert_eq!(blocks.first().unwrap(), "minecraft:acacia_logs");
        assert_eq!(blocks.last().unwrap(), "minecraft:wool_carpets");
        // `FileToIdConverter.fileToId` (FileToIdConverter.java:23-26) drops the
        // directory prefix and the extension and nothing else, so a tag path
        // keeps its own directories.
        assert!(blocks.contains(&"minecraft:mineable/pickaxe".to_string()));
        // A registry whose directory path is itself nested is named whole.
        assert_eq!(snapshot.tag_ids("worldgen/biome").len(), 68);
        assert!(snapshot.tag_ids("no_such_registry").is_empty());
    }

    #[test]
    fn round_trip_lists_and_reads_vanilla_and_feature_content() {
        let snapshot = std::sync::Arc::new(snapshot(&[
            (
                "datapacks/extra/data/minecraft/damage_type/ouch.json",
                b"{}",
            ),
            ("datapacks/extra/pack.mcmeta", b"{\"pack\":{}}"),
            (
                "dimension/overworld.json",
                b"{\"type\":\"minecraft:overworld\"}",
            ),
            ("dimension_type/overworld.json", b"{}"),
        ]));

        assert_eq!(snapshot.feature_pack_ids(), ["extra"]);
        let vanilla = snapshot.vanilla_pack();
        assert_eq!(vanilla.provided_namespaces(), ["minecraft"]);
        assert_eq!(
            vanilla.list_paths("minecraft", "dimension_type"),
            ["dimension_type/overworld.json"]
        );
        assert_eq!(
            vanilla.read("minecraft", "dimension/overworld.json"),
            Some(b"{\"type\":\"minecraft:overworld\"}".to_vec())
        );
        assert_eq!(vanilla.read_root("pack.mcmeta"), None);

        let extra = snapshot.feature_pack("extra").unwrap();
        assert_eq!(extra.provided_namespaces(), ["minecraft"]);
        assert_eq!(
            extra.read_root("pack.mcmeta"),
            Some(b"{\"pack\":{}}".to_vec())
        );
        assert_eq!(
            extra.read("minecraft", "damage_type/ouch.json"),
            Some(b"{}".to_vec())
        );
        assert!(snapshot.feature_pack("missing").is_none());
        assert!(snapshot.feature_pack("../escape").is_none());
    }
}
