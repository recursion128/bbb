//! `TagLoader` equivalent: load the `tags/` tree for the loaded dynamic registries
//! from the stacked datapack view (`TagLoader.java:48-116`).
//!
//! Semantics mirrored from vanilla:
//!
//! - Tag files live at `data/<namespace>/tags/<registry key path>/<tag path>.json`
//!   (`Registries.tagsDirPath`, `Registries.java:331-333`); every namespace is
//!   searched.
//! - Files accumulate across packs in ascending priority order
//!   (`FallbackResourceManager.getResourceStack`); `replace: true` clears the entries
//!   collected from lower packs (`TagLoader.load`, lines 61-63).
//! - A malformed file is skipped with a recorded warning, never fatal
//!   (`TagLoader.load`, lines 67-69: warn-only).
//! - `TagFile` is `{values: [...], replace: false}`; entries are `"id"` / `"#tag"`
//!   strings or `{id, required}` objects with `required` defaulting to true
//!   (`TagFile.java`, `TagEntry.java:14-23`). A missing optional entry is dropped
//!   silently; a tag with any missing required reference fails to build and is dropped
//!   with a warning (`TagLoader.tryBuildTag`/`build`, lines 76-116).
//! - Nested `#` references resolve through `DependencySorter`
//!   (`DependencySorter.java:13-53`): dependencies order the build and a back edge
//!   that would close a cycle is dropped silently (`addDependencyIfNotCyclic`).
//! - A registry whose tag map ends up empty contributes no reload
//!   (`TagLoader.loadPendingTags`, lines 143-150).
//!
//! Element lookups resolve against the loaded dynamic registry ids or the generated
//! protocol catalogs for Vanilla's fixed registries (`TagLoader.ElementLookup`
//! semantics: the `required` flag changes failure handling, not lookup). The startup
//! report includes supported STATIC catalogs, including registries without built-in
//! tags. Vanilla loads every existing registry (`TagLoader.java:122-149`); remaining
//! static catalogs are still a coverage gap. Raw holder ids belong to publication.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use bbb_protocol::content_registry::ContentRegistry;
use bbb_registry_codec::{decode_loaded_registry_json, RegistryJsonValue as Value};

use super::dynamic_registry::DatapackGeneration;
use super::registry_data_loader::DatapackLoadLimits;
use super::resource_overlay::{stacked_resources, PackOverlayInput};

/// One parsed `TagEntry` (`TagEntry.java`): an element id or a `#` tag reference,
/// each with the `required` flag (default true).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagEntry {
    Element { id: String, required: bool },
    Tag { id: String, required: bool },
}

impl TagEntry {
    fn required(&self) -> bool {
        match self {
            Self::Element { required, .. } | Self::Tag { required, .. } => *required,
        }
    }

    /// `TagEntry.visitRequiredDependencies` / `visitOptionalDependencies`.
    fn visit_dependency(&self) -> Option<&str> {
        match self {
            Self::Tag { id, .. } => Some(id),
            Self::Element { .. } => None,
        }
    }
}

impl fmt::Display for TagEntry {
    /// `TagEntry.toString` (`TagEntry.java:96-108`): `#` prefix for tag references,
    /// `?` suffix for optional entries.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (hash, id, required) = match self {
            Self::Element { id, required } => ("", id, required),
            Self::Tag { id, required } => ("#", id, required),
        };
        write!(f, "{hash}{id}")?;
        if !required {
            write!(f, "?")?;
        }
        Ok(())
    }
}

/// One parsed tag file (`TagFile.java`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagFile {
    pub entries: Vec<TagEntry>,
    pub replace: bool,
}

/// Normalize a `TAG_OR_ELEMENT_ID` string into `<namespace>:<path>`; the default
/// namespace is `minecraft` (`Identifier.withDefaultNamespace`). Returns `None` for
/// strings that cannot be an identifier (empty segments, more than one `:`).
fn normalize_identifier(raw: &str) -> Option<String> {
    let (namespace, path) = match raw.split_once(':') {
        Some((namespace, path)) => {
            if path.contains(':') {
                return None;
            }
            (namespace, path)
        }
        None => ("minecraft", raw),
    };
    if namespace.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("{namespace}:{path}"))
}

/// Parse one tag entry (`TagEntry.CODEC`): a string (`"id"` / `"#id"`, required) or
/// an object `{id, required}` with `required` defaulting to true.
fn parse_tag_entry(value: &Value) -> Option<TagEntry> {
    let (raw, required) = match value {
        Value::String(raw) => (raw, true),
        Value::Object(_) => {
            let Value::String(raw) = value.get("id")? else {
                return None;
            };
            let required = match value.get("required") {
                Some(flag) => flag.as_bool()?,
                None => true,
            };
            (raw, required)
        }
        _ => return None,
    };
    raw.with_utf8(|raw| {
        let (tag, raw) = match raw.strip_prefix('#') {
            Some(rest) => (true, rest),
            None => (false, raw),
        };
        let id = normalize_identifier(raw)?;
        Some(if tag {
            TagEntry::Tag { id, required }
        } else {
            TagEntry::Element { id, required }
        })
    })?
}

/// Parse one tag file (`TagFile.CODEC`): `values` is required, `replace` defaults to
/// false. Any shape violation fails the whole file (the caller skips it warn-only).
pub fn parse_tag_file(bytes: &[u8]) -> Option<TagFile> {
    parse_tag_file_with_limit(bytes, bytes.len())
}

fn parse_tag_file_with_limit(bytes: &[u8], maximum_bytes: usize) -> Option<TagFile> {
    let source = decode_loaded_registry_json(bytes, maximum_bytes).ok()?;
    let value = source.json()?;
    let values = value.get("values")?.as_array()?;
    let entries = values
        .iter()
        .map(parse_tag_entry)
        .collect::<Option<Vec<_>>>()?;
    let replace = match value.get("replace") {
        Some(flag) => flag.as_bool()?,
        None => false,
    };
    Some(TagFile { entries, replace })
}

/// `TagLoader.EntryWithSource`: one entry attributed to the pack that supplied it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryWithSource {
    pub entry: TagEntry,
    pub source: String,
}

impl fmt::Display for EntryWithSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (from {})", self.entry, self.source)
    }
}

/// One warn-only tag loading event (`TagLoader` logs and continues).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagLoadWarning {
    pub registry_id: String,
    /// The tag id for build failures; `None` for file-level parse failures.
    pub tag_id: Option<String>,
    /// The supplying pack for parse failures; `None` for build failures.
    pub pack: Option<String>,
    pub message: String,
}

impl fmt::Display for TagLoadWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "registry {}", self.registry_id)?;
        if let Some(tag) = &self.tag_id {
            write!(f, " tag {tag}")?;
        }
        if let Some(pack) = &self.pack {
            write!(f, " (pack {pack})")?;
        }
        write!(f, ": {}", self.message)
    }
}

/// What `TagLoader.load` (`TagLoader.java:48-74`) answers: the tag files' entries
/// for one registry, still unresolved. A caller whose element set is not final at
/// load time holds this and builds later, the way `ServerFunctionLibrary` holds the
/// entries until the compiled function map has replaced the old one
/// (`ServerFunctionLibrary.java:76-78`, `:113-114`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedTagEntries {
    pub registry_id: String,
    pub tags: BTreeMap<String, Vec<EntryWithSource>>,
}

/// The built tags of one registry: tag id → element ids in `LinkedHashSet`
/// insertion order (`TagLoader.tryBuildTag`, lines 76-87).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedRegistryTags {
    pub registry_id: String,
    pub tags: BTreeMap<String, Vec<String>>,
}

impl LoadedRegistryTags {
    pub fn tag(&self, tag_id: &str) -> Option<&[String]> {
        self.tags.get(tag_id).map(Vec::as_slice)
    }
}

/// The warn-only tag load outcome for one datapack generation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagLoadReport {
    /// One entry per registry with at least one built tag: dynamic generation order,
    /// followed by the fixed network-safe registry order.
    pub registries: Vec<LoadedRegistryTags>,
    pub warnings: Vec<TagLoadWarning>,
}

impl TagLoadReport {
    pub fn registry(&self, registry_id: &str) -> Option<&LoadedRegistryTags> {
        self.registries
            .iter()
            .find(|registry| registry.registry_id == registry_id)
    }
}

/// Load and build the tags of every registry in the generation
/// (`TagLoader.loadTagsForExistingRegistries`, lines 122-124, restricted to the D1
/// dynamic layer). Never fails: malformed files and unbuildable tags are recorded in
/// the report's warnings.
pub fn load_tags_for_generation(
    packs: &[PackOverlayInput<'_>],
    generation: &DatapackGeneration,
    limits: DatapackLoadLimits,
) -> TagLoadReport {
    let mut report = TagLoadReport::default();
    for registry in &generation.registries {
        let element_ids: Vec<&str> = registry
            .elements
            .iter()
            .map(|element| element.id.as_str())
            .collect();
        let loaded = load_tags_for_registry(
            packs,
            &registry.registry_id,
            &element_ids,
            limits,
            &mut report.warnings,
        );
        if let Some(loaded) = loaded {
            report.registries.push(loaded);
        }
    }
    report
}

/// Build the complete startup tag report from the same selected-pack stack as the
/// dynamic registries. Vanilla loads STATIC tags before WORLDGEN so registry codecs
/// can resolve them (`WorldLoader.java:35-47`) and later serializes tags for both
/// synchronized and STATIC registries (`TagNetworkSerialization.java:20-45`). BBB's
/// shallow dynamic load does not consume tag holders yet, so this extends the already
/// built dynamic report with the fixed network-safe registries in stable registry and
/// holder-catalog order.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn load_tags_for_startup(
    packs: &[PackOverlayInput<'_>],
    generation: &DatapackGeneration,
    limits: DatapackLoadLimits,
) -> TagLoadReport {
    load_tags_for_startup_with_content_registry(
        packs,
        generation,
        limits,
        ContentRegistry::vanilla(),
    )
}

pub(super) fn load_tags_for_startup_with_content_registry(
    packs: &[PackOverlayInput<'_>],
    generation: &DatapackGeneration,
    limits: DatapackLoadLimits,
    content_registry: &ContentRegistry,
) -> TagLoadReport {
    let mut report = load_tags_for_generation(packs, generation, limits);
    for (registry_id, element_ids) in
        network_safe_static_registry_elements_with_content_registry(content_registry)
    {
        let loaded = load_tags_for_registry(
            packs,
            registry_id,
            &element_ids,
            limits,
            &mut report.warnings,
        );
        if let Some(loaded) = loaded {
            report.registries.push(loaded);
        }
    }
    report
}

pub fn network_safe_static_registry_elements() -> Vec<(&'static str, Vec<&'static str>)> {
    use bbb_facts::static_registry_elements::{
        vanilla_static_registry_elements, VANILLA_STATIC_REGISTRY_ELEMENTS,
    };

    // Preserve the existing report order while completing the STATIC layer.
    let existing = [
        "minecraft:block",
        "minecraft:item",
        "minecraft:entity_type",
        "minecraft:fluid",
        "minecraft:game_event",
        "minecraft:point_of_interest_type",
        "minecraft:potion",
        "minecraft:mob_effect",
        "minecraft:sound_event",
        "minecraft:attribute",
    ];
    let mut registries: Vec<_> = existing
        .into_iter()
        .map(|registry| {
            (
                registry,
                vanilla_static_registry_elements(registry).unwrap().to_vec(),
            )
        })
        .collect();
    registries.extend(
        VANILLA_STATIC_REGISTRY_ELEMENTS
            .iter()
            .filter(|(registry, _)| !existing.contains(registry))
            .map(|(registry, elements)| (*registry, elements.to_vec())),
    );
    registries
}

/// Content-registry-aware form of [`network_safe_static_registry_elements`] that
/// appends registered custom blocks, items, entity types, sound events, and
/// particle types to their respective static registries.
pub fn network_safe_static_registry_elements_with_content_registry<'a>(
    content_registry: &'a ContentRegistry,
) -> Vec<(&'static str, Vec<&'a str>)> {
    let mut registries: Vec<(&'static str, Vec<&'a str>)> = network_safe_static_registry_elements();
    if content_registry.is_vanilla_only() {
        return registries;
    }
    for (registry_id, elements) in &mut registries {
        match *registry_id {
            "minecraft:block" => {
                elements.extend(
                    content_registry
                        .blocks()
                        .iter()
                        .map(|spec| spec.resource_id.as_str()),
                );
            }
            "minecraft:item" => {
                elements.extend(
                    content_registry
                        .items()
                        .iter()
                        .map(|spec| spec.resource_id.as_str()),
                );
            }
            "minecraft:entity_type" => {
                elements.extend(
                    content_registry
                        .entities()
                        .iter()
                        .map(|spec| spec.resource_id.as_str()),
                );
            }
            "minecraft:sound_event" => {
                elements.extend(
                    content_registry
                        .sound_events()
                        .iter()
                        .map(|spec| spec.resource_id.as_str()),
                );
            }
            "minecraft:particle_type" => {
                elements.extend(
                    content_registry
                        .particles()
                        .iter()
                        .map(|spec| spec.resource_id.as_str()),
                );
            }
            _ => {}
        }
    }
    registries
}

/// `TagLoader.loadTagsForRegistry` (`TagLoader.java:130-135`): `load` then `build`.
/// Element lookups resolve against `element_ids`
/// (`ElementLookup.fromFrozenRegistry`); never fails — malformed files and
/// unbuildable tags land in `warnings`.
pub fn load_tags_for_registry(
    packs: &[PackOverlayInput<'_>],
    registry_id: &str,
    element_ids: &[&str],
    limits: DatapackLoadLimits,
    warnings: &mut Vec<TagLoadWarning>,
) -> Option<LoadedRegistryTags> {
    let entries = load_tag_entries_for_registry(packs, registry_id, limits, warnings)?;
    build_registry_tags(&entries, element_ids, warnings)
}

/// `TagLoader.load` (`TagLoader.java:48-74`): collect every tag file's entries for
/// one registry across the stacked pack view, resolving nothing. `None` when the
/// registry has no tag file at all.
pub fn load_tag_entries_for_registry(
    packs: &[PackOverlayInput<'_>],
    registry_id: &str,
    limits: DatapackLoadLimits,
    warnings: &mut Vec<TagLoadWarning>,
) -> Option<LoadedTagEntries> {
    let registry_path = registry_id
        .split_once(':')
        .map(|(_, path)| path)
        .unwrap_or(registry_id);
    let directory = format!("tags/{registry_path}");
    let dir_prefix = format!("{directory}/");

    // Candidate tag files: the union of every pack's listing under the tags
    // directory; `stacked_resources` then applies the per-path filter masking.
    let mut files: BTreeSet<(String, String)> = BTreeSet::new();
    for pack in packs {
        for namespace in pack.resources.provided_namespaces() {
            for path in pack.resources.list_paths(&namespace, &directory) {
                if path.starts_with(&dir_prefix) && path.ends_with(".json") {
                    files.insert((namespace.clone(), path));
                }
            }
        }
    }

    let mut builders: BTreeMap<String, Vec<EntryWithSource>> = BTreeMap::new();
    for (namespace, path) in files {
        let Some(tag_path) = path[dir_prefix.len()..].strip_suffix(".json") else {
            continue;
        };
        let tag_id = format!("{namespace}:{tag_path}");
        for resource in stacked_resources(packs, &namespace, &path) {
            let Some(bytes) = resource.resources.read(&namespace, &path) else {
                continue;
            };
            if bytes.len() > limits.max_element_bytes {
                warnings.push(TagLoadWarning {
                    registry_id: registry_id.to_string(),
                    tag_id: None,
                    pack: Some(resource.pack_id.to_string()),
                    message: format!(
                        "tag file {path} exceeds the element budget ({} bytes)",
                        bytes.len()
                    ),
                });
                continue;
            }
            match parse_tag_file_with_limit(&bytes, limits.max_element_bytes) {
                Some(parsed) => {
                    let contents = builders.entry(tag_id.clone()).or_default();
                    if parsed.replace {
                        contents.clear();
                    }
                    contents.extend(parsed.entries.into_iter().map(|entry| EntryWithSource {
                        entry,
                        source: resource.pack_id.to_string(),
                    }));
                }
                None => warnings.push(TagLoadWarning {
                    registry_id: registry_id.to_string(),
                    tag_id: None,
                    pack: Some(resource.pack_id.to_string()),
                    message: format!("couldn't read tag list {tag_id} from {path}"),
                }),
            }
        }
    }

    if builders.is_empty() {
        None
    } else {
        Some(LoadedTagEntries {
            registry_id: registry_id.to_string(),
            tags: builders,
        })
    }
}

/// `TagLoader.build` (`TagLoader.java:89-116`) over entries a previous `load`
/// answered, against the element set the caller has now. `None` when no tag builds.
pub fn build_registry_tags(
    entries: &LoadedTagEntries,
    element_ids: &[&str],
    warnings: &mut Vec<TagLoadWarning>,
) -> Option<LoadedRegistryTags> {
    let tags = build_tags(&entries.registry_id, element_ids, &entries.tags, warnings);
    if tags.is_empty() {
        None
    } else {
        Some(LoadedRegistryTags {
            registry_id: entries.registry_id.clone(),
            tags,
        })
    }
}

/// `TagLoader.build` (`TagLoader.java:89-116`): order tags by their `#` dependencies
/// with a cycle-safe `DependencySorter` walk, then build each tag in that order.
fn build_tags(
    registry_id: &str,
    element_ids: &[&str],
    builders: &BTreeMap<String, Vec<EntryWithSource>>,
    warnings: &mut Vec<TagLoadWarning>,
) -> BTreeMap<String, Vec<String>> {
    let element_ids: BTreeSet<&str> = element_ids.iter().copied().collect();

    // `DependencySorter.orderByDependencies` adds every required edge before any
    // optional edge, then drops an edge that would close a cycle
    // (`DependencySorter.java:42-45`). Mixing the two passes would let an optional
    // edge displace a later required edge and change which tags can build.
    let mut dependencies: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for required in [true, false] {
        for (id, entries) in builders {
            for entry in entries {
                let entry = &entry.entry;
                if entry.required() == required {
                    if let Some(dependency) = entry.visit_dependency() {
                        if !is_cyclic(&dependencies, id, dependency) {
                            let targets = dependencies.entry(id.as_str()).or_default();
                            if !targets.contains(&dependency) {
                                targets.push(dependency);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut visited: BTreeSet<&str> = BTreeSet::new();
    let mut order: Vec<&str> = Vec::new();
    for id in builders.keys() {
        visit_dependencies(id, &dependencies, builders, &mut visited, &mut order);
    }

    let mut built: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for id in order {
        let entries = &builders[id];
        match try_build_tag(&element_ids, &built, entries) {
            Ok(values) => {
                built.insert(id.to_string(), values);
            }
            Err(missing) => warnings.push(TagLoadWarning {
                registry_id: registry_id.to_string(),
                tag_id: Some(id.to_string()),
                pack: None,
                message: format!(
                    "couldn't load tag {id} as it is missing following references: {}",
                    missing
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }),
        }
    }
    built
}

/// `DependencySorter.isCyclic`: true when `from == to` or `to` already reaches `from`.
fn is_cyclic<'a>(dependencies: &BTreeMap<&'a str, Vec<&'a str>>, from: &str, to: &'a str) -> bool {
    if from == to {
        return true;
    }
    let mut visited: BTreeSet<&'a str> = BTreeSet::new();
    let mut stack = vec![to];
    while let Some(node) = stack.pop() {
        if !visited.insert(node) {
            continue;
        }
        let Some(targets) = dependencies.get(node) else {
            continue;
        };
        for &dependency in targets {
            if dependency == from {
                return true;
            }
            stack.push(dependency);
        }
    }
    false
}

/// `DependencySorter.visitDependenciesAndElement`: depth-first over dependencies,
/// emitting ids present in the builder map exactly once.
fn visit_dependencies<'a>(
    id: &'a str,
    dependencies: &BTreeMap<&str, Vec<&'a str>>,
    builders: &BTreeMap<String, Vec<EntryWithSource>>,
    visited: &mut BTreeSet<&'a str>,
    order: &mut Vec<&'a str>,
) {
    if !visited.insert(id) {
        return;
    }
    if let Some(targets) = dependencies.get(id) {
        for dependency in targets {
            visit_dependencies(dependency, dependencies, builders, visited, order);
        }
    }
    if builders.contains_key(id) {
        order.push(id);
    }
}

/// `TagLoader.tryBuildTag` (`TagLoader.java:76-87`): insertion-ordered de-duplicated
/// element ids, or the list of missing required entries.
fn try_build_tag<'a>(
    element_ids: &BTreeSet<&str>,
    built: &BTreeMap<String, Vec<String>>,
    entries: &'a [EntryWithSource],
) -> Result<Vec<String>, Vec<&'a EntryWithSource>> {
    let mut values: Vec<String> = Vec::new();
    let mut missing = Vec::new();
    for entry in entries {
        let resolved: Option<Vec<String>> = match &entry.entry {
            TagEntry::Element { id, .. } => {
                element_ids.contains(id.as_str()).then(|| vec![id.clone()])
            }
            TagEntry::Tag { id, .. } => built.get(id).cloned(),
        };
        match resolved {
            Some(ids) => {
                for id in ids {
                    if !values.contains(&id) {
                        values.push(id);
                    }
                }
            }
            None if !entry.entry.required() => {}
            None => missing.push(entry),
        }
    }
    if missing.is_empty() {
        Ok(values)
    } else {
        Err(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic_registry::{LoadedDynamicRegistry, RawRegistryElement};

    fn element(id: &str) -> RawRegistryElement {
        RawRegistryElement {
            id: id.to_string(),
            source_pack: "test".to_string(),
            source_json: b"null".as_slice().into(),
            source: None,
            codec_value: None,
        }
    }

    fn registry(registry_id: &str, element_ids: &[&str]) -> LoadedDynamicRegistry {
        LoadedDynamicRegistry {
            registry_id: registry_id.to_string(),
            elements: element_ids.iter().map(|id| element(id)).collect(),
        }
    }

    fn build(
        registry: &LoadedDynamicRegistry,
        builders: BTreeMap<String, Vec<EntryWithSource>>,
        warnings: &mut Vec<TagLoadWarning>,
    ) -> BTreeMap<String, Vec<String>> {
        let ids: Vec<&str> = registry
            .elements
            .iter()
            .map(|element| element.id.as_str())
            .collect();
        build_tags(&registry.registry_id, &ids, &builders, warnings)
    }

    #[test]
    fn parses_string_and_object_entries_with_required_default() {
        let parsed = parse_tag_file(
            br##"{"values": ["a:b", "#c:d", {"id": "e:f", "required": false}, {"id": "#g:h"}]}"##,
        )
        .unwrap();
        assert_eq!(
            parsed.entries,
            [
                TagEntry::Element {
                    id: "a:b".to_string(),
                    required: true
                },
                TagEntry::Tag {
                    id: "c:d".to_string(),
                    required: true
                },
                TagEntry::Element {
                    id: "e:f".to_string(),
                    required: false
                },
                TagEntry::Tag {
                    id: "g:h".to_string(),
                    required: true
                },
            ]
        );
        assert!(!parsed.replace);
    }

    #[test]
    fn defaults_the_minecraft_namespace_and_replaces_lower_entries() {
        let parsed = parse_tag_file(br##"{"replace": true, "values": ["plains"]}"##).unwrap();
        assert!(parsed.replace);
        assert_eq!(
            parsed.entries,
            [TagEntry::Element {
                id: "minecraft:plains".to_string(),
                required: true
            }]
        );
    }

    #[test]
    fn builds_nested_tags_in_dependency_order() {
        let registry = registry("minecraft:worldgen/biome", &["minecraft:a", "minecraft:b"]);
        let mut builders: BTreeMap<String, Vec<EntryWithSource>> = BTreeMap::new();
        let entry = |entry: TagEntry| EntryWithSource {
            entry,
            source: "test".to_string(),
        };
        builders.insert(
            "minecraft:inner".to_string(),
            vec![entry(TagEntry::Element {
                id: "minecraft:a".to_string(),
                required: true,
            })],
        );
        builders.insert(
            "minecraft:outer".to_string(),
            vec![
                entry(TagEntry::Tag {
                    id: "minecraft:inner".to_string(),
                    required: true,
                }),
                entry(TagEntry::Element {
                    id: "minecraft:b".to_string(),
                    required: true,
                }),
                // Duplicate of the element contributed through `#inner`.
                entry(TagEntry::Element {
                    id: "minecraft:a".to_string(),
                    required: true,
                }),
            ],
        );
        let mut warnings = Vec::new();
        let built = build(&registry, builders, &mut warnings);
        assert!(warnings.is_empty());
        assert_eq!(built["minecraft:inner"], ["minecraft:a"]);
        assert_eq!(built["minecraft:outer"], ["minecraft:a", "minecraft:b"]);
    }

    /// `ServerFunctionLibrary` builds one `load` against a map replaced later
    /// (`ServerFunctionLibrary.java:76-78`, `:113-114`), so the same entries have
    /// to answer whatever element set the caller holds at build time.
    #[test]
    fn one_load_builds_against_each_element_set_it_is_given() {
        let entry = |entry: TagEntry| EntryWithSource {
            entry,
            source: "test".to_string(),
        };
        let entries = LoadedTagEntries {
            registry_id: "minecraft:function".to_string(),
            tags: BTreeMap::from([
                (
                    "minecraft:tick".to_string(),
                    vec![
                        entry(TagEntry::Element {
                            id: "test:kept".to_string(),
                            required: true,
                        }),
                        entry(TagEntry::Element {
                            id: "test:optional".to_string(),
                            required: false,
                        }),
                    ],
                ),
                (
                    "minecraft:load".to_string(),
                    vec![entry(TagEntry::Element {
                        id: "test:optional".to_string(),
                        required: true,
                    })],
                ),
            ]),
        };

        let mut warnings = Vec::new();
        let both = build_registry_tags(&entries, &["test:kept", "test:optional"], &mut warnings)
            .expect("both tags build");
        assert!(warnings.is_empty());
        assert_eq!(
            both.tag("minecraft:tick").unwrap(),
            ["test:kept", "test:optional"]
        );
        assert_eq!(both.tag("minecraft:load").unwrap(), ["test:optional"]);

        let mut warnings = Vec::new();
        let reduced =
            build_registry_tags(&entries, &["test:kept"], &mut warnings).expect("one tag builds");
        assert_eq!(reduced.tag("minecraft:tick").unwrap(), ["test:kept"]);
        assert_eq!(reduced.tag("minecraft:load"), None);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].tag_id.as_deref(), Some("minecraft:load"));
        assert!(warnings[0].message.contains("test:optional"));
    }

    #[test]
    fn self_referencing_tag_and_diamond_graph_do_not_overflow_or_hang() {
        let mut builders = BTreeMap::new();
        builders.insert(
            "test:self_loop".to_string(),
            vec![EntryWithSource {
                entry: TagEntry::Tag {
                    id: "test:self_loop".to_string(),
                    required: true,
                },
                source: "pack".to_string(),
            }],
        );
        builders.insert(
            "test:caller".to_string(),
            vec![EntryWithSource {
                entry: TagEntry::Tag {
                    id: "test:self_loop".to_string(),
                    required: true,
                },
                source: "pack".to_string(),
            }],
        );
        let entries = LoadedTagEntries {
            registry_id: "minecraft:item".to_string(),
            tags: builders,
        };
        let mut warnings = Vec::new();
        let built = build_registry_tags(&entries, &["minecraft:stone"], &mut warnings);
        assert!(built.is_none());
        assert_eq!(warnings.len(), 2);

        // Build a 40-level diamond DAG (2^40 distinct paths) with duplicate edges to confirm
        // cycle detection visits each node once instead of hanging exponentially.
        let mut diamond = BTreeMap::new();
        diamond.insert(
            "test:d40".to_string(),
            vec![EntryWithSource {
                entry: TagEntry::Element {
                    id: "minecraft:stone".to_string(),
                    required: true,
                },
                source: "pack".to_string(),
            }],
        );
        for level in 0..40 {
            let next_a = format!("test:d{}", level + 1);
            let next_b = format!("test:b{}", level + 1);
            if level + 1 < 40 {
                diamond.insert(
                    next_b.clone(),
                    vec![
                        EntryWithSource {
                            entry: TagEntry::Tag {
                                id: format!("test:d{}", level + 2),
                                required: true,
                            },
                            source: "pack".to_string(),
                        },
                        EntryWithSource {
                            entry: TagEntry::Tag {
                                id: format!("test:d{}", level + 2),
                                required: true,
                            },
                            source: "pack".to_string(),
                        },
                    ],
                );
                diamond.insert(
                    format!("test:d{level}"),
                    vec![
                        EntryWithSource {
                            entry: TagEntry::Tag {
                                id: next_a,
                                required: true,
                            },
                            source: "pack".to_string(),
                        },
                        EntryWithSource {
                            entry: TagEntry::Tag {
                                id: next_b,
                                required: true,
                            },
                            source: "pack".to_string(),
                        },
                    ],
                );
            } else {
                diamond.insert(
                    format!("test:d{level}"),
                    vec![EntryWithSource {
                        entry: TagEntry::Tag {
                            id: next_a,
                            required: true,
                        },
                        source: "pack".to_string(),
                    }],
                );
            }
        }
        let diamond_entries = LoadedTagEntries {
            registry_id: "minecraft:item".to_string(),
            tags: diamond,
        };
        let mut diamond_warnings = Vec::new();
        let built_diamond = build_registry_tags(
            &diamond_entries,
            &["minecraft:stone"],
            &mut diamond_warnings,
        )
        .expect("diamond DAG builds");
        assert!(diamond_warnings.is_empty(), "{diamond_warnings:?}");
        assert_eq!(built_diamond.tag("test:d0").unwrap(), ["minecraft:stone"]);
    }

    #[test]
    fn static_network_safe_registry_catalogs_are_complete_for_builtin_tags() {
        use super::super::pack::ServerPackResources;
        use super::super::vanilla_snapshot::VanillaDatapackSnapshot;

        let snapshot = VanillaDatapackSnapshot::embedded_shared().unwrap();
        let vanilla = snapshot.vanilla_pack();
        let packs = [PackOverlayInput {
            pack_id: "vanilla",
            resources: &vanilla as &dyn ServerPackResources,
            filter: None,
        }];
        let mut report = TagLoadReport::default();
        for (registry_id, element_ids) in network_safe_static_registry_elements() {
            let loaded = load_tags_for_registry(
                &packs,
                registry_id,
                &element_ids,
                DatapackLoadLimits::default(),
                &mut report.warnings,
            );
            if let Some(loaded) = loaded {
                report.registries.push(loaded);
            }
        }

        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
        let registry_counts: Vec<(&str, usize)> = report
            .registries
            .iter()
            .map(|registry| (registry.registry_id.as_str(), registry.tags.len()))
            .collect();
        assert_eq!(
            registry_counts,
            [
                ("minecraft:block", 248),
                ("minecraft:item", 207),
                ("minecraft:entity_type", 47),
                ("minecraft:fluid", 6),
                ("minecraft:game_event", 5),
                ("minecraft:point_of_interest_type", 3),
                ("minecraft:potion", 1),
            ]
        );
    }
}
