//! `MultiPackResourceManager`/`FallbackResourceManager` equivalents: merge the selected
//! packs' `SERVER_DATA` content into one view with vanilla override and filter
//! semantics.
//!
//! Packs are supplied in ascending priority order (the `PackRepository` selection
//! order; vanilla pack first). Merging follows
//! `FallbackResourceManager.listResources` (`FallbackResourceManager.java:153-185`):
//! walking packs from low to high priority, each pack's paths overwrite the same
//! `(namespace, path)` collected from lower packs, and before a pack contributes its
//! own content its `pack.mcmeta` `filter` section removes already-collected lower-pack
//! paths it blocks (`PackEntry.filterAll`). A filter never hides the pack's own
//! content (`FallbackResourceManager.getResource`, lines 69-85).

use std::collections::BTreeMap;

use super::pack::ServerPackResources;

/// The parsed `pack.mcmeta` `filter` section plugs into the overlay seam.
impl PackPathFilter for bbb_registry_codec::pack_filter::ResourceFilter {
    fn is_blocked(&self, namespace: &str, path: &str) -> bool {
        self.is_path_blocked(namespace, path)
    }
}

/// `ResourceFilterSection` application surface. Implemented by the pack model's parsed
/// `filter` section; `is_blocked` combines namespace-level and path-level rules
/// (`ResourceFilterSection.isNamespaceFiltered`/`isPathFiltered`).
pub trait PackPathFilter {
    fn is_blocked(&self, namespace: &str, path: &str) -> bool;
}

/// One selected pack in priority order, with its parsed `filter` section when the
/// pack model produced one.
pub struct PackOverlayInput<'a> {
    pub pack_id: &'a str,
    pub resources: &'a dyn ServerPackResources,
    pub filter: Option<&'a dyn PackPathFilter>,
}

/// `CompositePackResources` (`CompositePackResources.java:17-66`): layers a pack's
/// active `pack.mcmeta` `overlays` over its primary `ServerPackResources`.
///
/// Later active overlays override earlier overlays and all active overlays
/// override the primary pack for the same `(namespace, path)`, while `read_root`
/// (`pack.mcmeta`) stays on the primary pack. Because the pack presents a single
/// merged `ServerPackResources` surface, a pack's own `filter` section never
/// hides its own overlays, and overlay tag files replace the primary pack's tag
/// file within that pack rather than stacking twice.
pub struct LayeredPackResources<'a> {
    primary: &'a dyn ServerPackResources,
    overlays: Vec<Box<dyn ServerPackResources>>,
}

impl<'a> LayeredPackResources<'a> {
    pub fn new(
        primary: &'a dyn ServerPackResources,
        overlays: Vec<Box<dyn ServerPackResources>>,
    ) -> Self {
        Self { primary, overlays }
    }

    pub fn from_server_pack(pack: &'a super::pack_repository::ServerPack) -> Self {
        let overlays = pack
            .meta
            .active_overlays()
            .into_iter()
            .filter_map(|dir| pack.resources.open_overlay(dir))
            .collect();
        Self::new(pack.resources.as_ref(), overlays)
    }
}

impl ServerPackResources for LayeredPackResources<'_> {
    fn provided_namespaces(&self) -> Vec<String> {
        if self.overlays.is_empty() {
            return self.primary.provided_namespaces();
        }
        let mut namespaces = self.primary.provided_namespaces();
        for overlay in &self.overlays {
            namespaces.extend(overlay.provided_namespaces());
        }
        namespaces.sort();
        namespaces.dedup();
        namespaces
    }

    fn list_paths(&self, namespace: &str, dir: &str) -> Vec<String> {
        if self.overlays.is_empty() {
            return self.primary.list_paths(namespace, dir);
        }
        let mut paths = self.primary.list_paths(namespace, dir);
        for overlay in &self.overlays {
            paths.extend(overlay.list_paths(namespace, dir));
        }
        paths.sort();
        paths.dedup();
        paths
    }

    fn contains(&self, namespace: &str, path: &str) -> bool {
        self.overlays
            .iter()
            .rev()
            .any(|overlay| overlay.contains(namespace, path))
            || self.primary.contains(namespace, path)
    }

    fn read(&self, namespace: &str, path: &str) -> Option<Vec<u8>> {
        for overlay in self.overlays.iter().rev() {
            if let Some(bytes) = overlay.read(namespace, path) {
                return Some(bytes);
            }
        }
        self.primary.read(namespace, path)
    }

    fn read_shared(&self, namespace: &str, path: &str) -> Option<std::sync::Arc<[u8]>> {
        for overlay in self.overlays.iter().rev() {
            if let Some(bytes) = overlay.read_shared(namespace, path) {
                return Some(bytes);
            }
        }
        self.primary.read_shared(namespace, path)
    }

    fn read_root(&self, path: &str) -> Option<Vec<u8>> {
        self.primary.read_root(path)
    }

    fn open_overlay(&self, directory: &str) -> Option<Box<dyn ServerPackResources>> {
        self.primary.open_overlay(directory)
    }
}

/// The merged `SERVER_DATA` view over the selected packs
/// (`MultiPackResourceManager` for the registry loader).
#[derive(Debug, Clone)]
pub struct MergedServerDataView {
    /// Winning pack index per `(namespace, path)`, sorted for deterministic iteration.
    winners: BTreeMap<(String, String), usize>,
    pack_ids: Vec<String>,
}

impl MergedServerDataView {
    /// Merge the selected packs in ascending priority order
    /// (`FallbackResourceManager.listResources`).
    pub fn merge(packs: &[PackOverlayInput<'_>]) -> Self {
        let mut winners: BTreeMap<(String, String), usize> = BTreeMap::new();
        for (index, pack) in packs.iter().enumerate() {
            // `PackEntry.filterAll`: this pack's filter hides already-collected
            // lower-priority content.
            if let Some(filter) = pack.filter {
                winners.retain(|(namespace, path), _| !filter.is_blocked(namespace, path));
            }
            for namespace in pack.resources.provided_namespaces() {
                for path in pack.resources.list_paths(&namespace, "") {
                    winners.insert((namespace.clone(), path), index);
                }
            }
        }
        Self {
            winners,
            pack_ids: packs.iter().map(|pack| pack.pack_id.to_string()).collect(),
        }
    }

    /// All `(namespace, path)` pairs visible under `namespace` and directory `dir`
    /// (prefix match on `<dir>/`), sorted; the caller applies `FileToIdConverter`
    /// extension rules.
    pub fn list(&self, namespace: &str, dir: &str) -> Vec<&str> {
        let prefix = if dir.is_empty() {
            String::new()
        } else {
            format!("{dir}/")
        };
        let start = (namespace.to_string(), prefix.clone());
        self.winners
            .range(start..)
            .take_while(|((ns, path), _)| ns == namespace && path.starts_with(&prefix))
            .map(|((_, path), _)| path.as_str())
            .collect()
    }

    /// All namespaces visible in the merged view, sorted.
    pub fn namespaces(&self) -> Vec<&str> {
        let mut namespaces: Vec<&str> = self
            .winners
            .keys()
            .map(|(namespace, _)| namespace.as_str())
            .collect();
        namespaces.sort_unstable();
        namespaces.dedup();
        namespaces
    }

    /// The winning pack for `(namespace, path)`: its id and a reader for the bytes.
    pub fn winner<'a>(
        &self,
        packs: &'a [PackOverlayInput<'_>],
        namespace: &str,
        path: &str,
    ) -> Option<(&'a str, &'a dyn ServerPackResources)> {
        let index = *self
            .winners
            .get(&(namespace.to_string(), path.to_string()))?;
        let pack = &packs[index];
        Some((pack.pack_id, pack.resources))
    }

    pub fn pack_ids(&self) -> &[String] {
        &self.pack_ids
    }
}

/// `FallbackResourceManager.getResourceStack` (`FallbackResourceManager.java:87-127`):
/// every pack contributing `(namespace, path)` in ascending priority order. The walk
/// runs from the highest priority down: a pack holding the path contributes its copy,
/// and the walk stops above the highest pack whose `filter` section blocks the path —
/// a filter never hides its own pack's contribution.
pub fn stacked_resources<'a>(
    packs: &'a [PackOverlayInput<'a>],
    namespace: &str,
    path: &str,
) -> Vec<&'a PackOverlayInput<'a>> {
    let mut stack = Vec::new();
    for pack in packs.iter().rev() {
        if pack.resources.contains(namespace, path) {
            stack.push(pack);
        }
        if pack
            .filter
            .is_some_and(|filter| filter.is_blocked(namespace, path))
        {
            break;
        }
    }
    stack.reverse();
    stack
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap as Map;

    struct MockPack {
        namespaces: Vec<String>,
        files: Map<(String, String), Vec<u8>>,
    }

    impl MockPack {
        fn new(files: &[(&str, &str, &str)]) -> Self {
            let mut namespaces = Vec::new();
            let mut map = Map::new();
            for (namespace, path, content) in files {
                if !namespaces.contains(&namespace.to_string()) {
                    namespaces.push(namespace.to_string());
                }
                map.insert(
                    (namespace.to_string(), path.to_string()),
                    content.as_bytes().to_vec(),
                );
            }
            namespaces.sort();
            Self {
                namespaces,
                files: map,
            }
        }
    }

    impl ServerPackResources for MockPack {
        fn provided_namespaces(&self) -> Vec<String> {
            self.namespaces.clone()
        }

        fn list_paths(&self, namespace: &str, dir: &str) -> Vec<String> {
            let prefix = if dir.is_empty() {
                String::new()
            } else {
                format!("{dir}/")
            };
            self.files
                .keys()
                .filter(|(ns, path)| ns == namespace && path.starts_with(&prefix))
                .map(|(_, path)| path.clone())
                .collect()
        }

        fn read(&self, namespace: &str, path: &str) -> Option<Vec<u8>> {
            self.files
                .get(&(namespace.to_string(), path.to_string()))
                .cloned()
        }

        fn read_root(&self, _path: &str) -> Option<Vec<u8>> {
            None
        }
    }

    struct BlockPath(&'static str);

    impl PackPathFilter for BlockPath {
        fn is_blocked(&self, _namespace: &str, path: &str) -> bool {
            path == self.0
        }
    }

    fn input<'a>(
        pack_id: &'a str,
        resources: &'a dyn ServerPackResources,
        filter: Option<&'a dyn PackPathFilter>,
    ) -> PackOverlayInput<'a> {
        PackOverlayInput {
            pack_id,
            resources,
            filter,
        }
    }

    #[test]
    fn later_pack_overrides_same_id_and_keeps_unique_content() {
        let low = MockPack::new(&[
            ("minecraft", "damage_type/cactus.json", "low"),
            ("minecraft", "damage_type/drown.json", "low"),
        ]);
        let high = MockPack::new(&[("minecraft", "damage_type/cactus.json", "high")]);
        let packs = [input("vanilla", &low, None), input("file/x", &high, None)];
        let view = MergedServerDataView::merge(&packs);

        assert_eq!(
            view.list("minecraft", "damage_type"),
            ["damage_type/cactus.json", "damage_type/drown.json"]
        );
        let (pack_id, resources) = view
            .winner(&packs, "minecraft", "damage_type/cactus.json")
            .unwrap();
        assert_eq!(pack_id, "file/x");
        assert_eq!(
            resources.read("minecraft", "damage_type/cactus.json"),
            Some(b"high".to_vec())
        );
        let (pack_id, _) = view
            .winner(&packs, "minecraft", "damage_type/drown.json")
            .unwrap();
        assert_eq!(pack_id, "vanilla");
    }

    #[test]
    fn higher_pack_filter_hides_lower_content_but_not_its_own() {
        let low = MockPack::new(&[
            ("minecraft", "damage_type/cactus.json", "low"),
            ("minecraft", "damage_type/drown.json", "low"),
        ]);
        let high = MockPack::new(&[("minecraft", "damage_type/cactus.json", "high")]);
        let filter = BlockPath("damage_type/drown.json");
        let packs = [
            input("vanilla", &low, None),
            input("file/x", &high, Some(&filter)),
        ];
        let view = MergedServerDataView::merge(&packs);

        assert_eq!(
            view.list("minecraft", "damage_type"),
            ["damage_type/cactus.json"]
        );
        let (pack_id, _) = view
            .winner(&packs, "minecraft", "damage_type/cactus.json")
            .unwrap();
        assert_eq!(pack_id, "file/x");
    }

    #[test]
    fn lower_pack_filter_does_not_hide_higher_content() {
        let low = MockPack::new(&[("minecraft", "damage_type/cactus.json", "low")]);
        let high = MockPack::new(&[("minecraft", "damage_type/cactus.json", "high")]);
        let filter = BlockPath("damage_type/cactus.json");
        let packs = [
            input("vanilla", &low, Some(&filter)),
            input("file/x", &high, None),
        ];
        let view = MergedServerDataView::merge(&packs);
        let (pack_id, _) = view
            .winner(&packs, "minecraft", "damage_type/cactus.json")
            .unwrap();
        assert_eq!(pack_id, "file/x");
    }

    #[test]
    fn stack_collects_all_providers_in_ascending_priority() {
        let low = MockPack::new(&[("minecraft", "tags/damage_type/a.json", "low")]);
        let mid = MockPack::new(&[("minecraft", "tags/damage_type/a.json", "mid")]);
        let high = MockPack::new(&[("minecraft", "tags/damage_type/a.json", "high")]);
        let packs = [
            input("vanilla", &low, None),
            input("file/x", &mid, None),
            input("file/y", &high, None),
        ];
        let stack = stacked_resources(&packs, "minecraft", "tags/damage_type/a.json");
        let ids: Vec<&str> = stack.iter().map(|pack| pack.pack_id).collect();
        assert_eq!(ids, ["vanilla", "file/x", "file/y"]);
        assert!(stacked_resources(&packs, "minecraft", "tags/damage_type/b.json").is_empty());
    }

    #[test]
    fn stack_stops_above_the_highest_blocking_filter() {
        let low = MockPack::new(&[("minecraft", "tags/damage_type/a.json", "low")]);
        // The filtering pack does not provide the path itself: it still hides lower
        // copies (`pushFilterOnly`).
        let mid = MockPack::new(&[]);
        let high = MockPack::new(&[("minecraft", "tags/damage_type/a.json", "high")]);
        let filter = BlockPath("tags/damage_type/a.json");
        let packs = [
            input("vanilla", &low, None),
            input("file/x", &mid, Some(&filter)),
            input("file/y", &high, None),
        ];
        let stack = stacked_resources(&packs, "minecraft", "tags/damage_type/a.json");
        let ids: Vec<&str> = stack.iter().map(|pack| pack.pack_id).collect();
        assert_eq!(ids, ["file/y"]);
        // A filter never hides its own pack's contribution.
        let self_filtered = MockPack::new(&[("minecraft", "tags/damage_type/a.json", "mid")]);
        let packs = [
            input("vanilla", &low, None),
            input("file/x", &self_filtered, Some(&filter)),
        ];
        let stack = stacked_resources(&packs, "minecraft", "tags/damage_type/a.json");
        let ids: Vec<&str> = stack.iter().map(|pack| pack.pack_id).collect();
        assert_eq!(ids, ["file/x"]);
    }

    #[test]
    fn layered_pack_overlays_override_primary_and_ignore_inactive_layers() {
        struct MockOverlayPack {
            primary: MockPack,
            overlays: BTreeMap<String, MockPack>,
        }

        impl ServerPackResources for MockOverlayPack {
            fn provided_namespaces(&self) -> Vec<String> {
                self.primary.provided_namespaces()
            }

            fn list_paths(&self, namespace: &str, dir: &str) -> Vec<String> {
                self.primary.list_paths(namespace, dir)
            }

            fn read(&self, namespace: &str, path: &str) -> Option<Vec<u8>> {
                self.primary.read(namespace, path)
            }

            fn read_root(&self, path: &str) -> Option<Vec<u8>> {
                self.primary.read_root(path)
            }

            fn open_overlay(&self, directory: &str) -> Option<Box<dyn ServerPackResources>> {
                let overlay = self.overlays.get(directory)?;
                Some(Box::new(MockPack {
                    namespaces: overlay.namespaces.clone(),
                    files: overlay.files.clone(),
                }))
            }
        }

        let pack = MockOverlayPack {
            primary: MockPack::new(&[
                ("minecraft", "damage_type/a.json", "primary-a"),
                ("minecraft", "damage_type/b.json", "primary-b"),
                ("minecraft", "tags/damage_type/t.json", "primary-tag"),
            ]),
            overlays: BTreeMap::from([
                (
                    "old_100".to_string(),
                    MockPack::new(&[("minecraft", "damage_type/a.json", "old-100")]),
                ),
                (
                    "v101_a".to_string(),
                    MockPack::new(&[
                        ("minecraft", "damage_type/a.json", "overlay-1"),
                        ("overlay_ns", "damage_type/c.json", "overlay-only"),
                        ("minecraft", "tags/damage_type/t.json", "overlay-tag"),
                    ]),
                ),
                (
                    "v101_b".to_string(),
                    MockPack::new(&[("minecraft", "damage_type/a.json", "overlay-2")]),
                ),
            ]),
        };
        let meta = super::super::pack_mcmeta::ServerPackMeta::parse(
            br#"{
                "pack": {"description": "d", "min_format": 101, "max_format": 101},
                "overlays": {"entries": [
                    {"min_format": 100, "max_format": 100, "directory": "old_100"},
                    {"min_format": [101, 0], "max_format": [101, 1], "directory": "v101_a"},
                    {"min_format": [101, 1], "max_format": [102, 0], "directory": "v101_b"}
                ]}
            }"#,
        )
        .unwrap();
        let active: Vec<Box<dyn ServerPackResources>> = meta
            .active_overlays()
            .into_iter()
            .filter_map(|dir| pack.open_overlay(dir))
            .collect();
        assert_eq!(active.len(), 2);

        let layered = LayeredPackResources::new(&pack, active);
        let base = MockPack::new(&[
            ("minecraft", "damage_type/a.json", "vanilla-a"),
            ("minecraft", "tags/damage_type/t.json", "vanilla-tag"),
        ]);
        let filter = BlockPath("damage_type/a.json");
        let packs = [
            input("vanilla", &base, None),
            input("file/layered", &layered, Some(&filter)),
        ];
        let view = MergedServerDataView::merge(&packs);
        assert_eq!(view.namespaces(), ["minecraft", "overlay_ns"]);
        assert_eq!(
            view.list("minecraft", "damage_type"),
            ["damage_type/a.json", "damage_type/b.json"]
        );
        let (_, res_a) = view
            .winner(&packs, "minecraft", "damage_type/a.json")
            .unwrap();
        assert_eq!(
            res_a.read("minecraft", "damage_type/a.json"),
            Some(b"overlay-2".to_vec())
        );
        let (_, res_b) = view
            .winner(&packs, "minecraft", "damage_type/b.json")
            .unwrap();
        assert_eq!(
            res_b.read("minecraft", "damage_type/b.json"),
            Some(b"primary-b".to_vec())
        );
        let (_, res_c) = view
            .winner(&packs, "overlay_ns", "damage_type/c.json")
            .unwrap();
        assert_eq!(
            res_c.read("overlay_ns", "damage_type/c.json"),
            Some(b"overlay-only".to_vec())
        );

        let stack = stacked_resources(&packs, "minecraft", "tags/damage_type/t.json");
        assert_eq!(stack.len(), 2);
        assert_eq!(
            stack[1]
                .resources
                .read("minecraft", "tags/damage_type/t.json"),
            Some(b"overlay-tag".to_vec())
        );
    }
}
