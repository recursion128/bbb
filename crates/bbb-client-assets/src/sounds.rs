use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    resource_budget::read_standalone_json,
    resources::{PackResourceStack, ResourceIntegrity, ResourceLocation},
    zip_pack::{
        ZipEntrySource, MAX_ZIP_ARCHIVE_MATERIALIZED_BYTES, MAX_ZIP_ENTRY_UNCOMPRESSED_BYTES,
    },
};

const MAX_SOUND_EVENT_DEFINITIONS: u64 = 65_536;
const MAX_SOUND_ENTRIES: u64 = 262_144;
const SOUND_EVENT_LOGICAL_BYTES: u64 = 512;
const SOUND_ENTRY_LOGICAL_BYTES: u64 = 256;
const MAX_SOUND_CATALOG_MATERIALIZED_BYTES: u64 = MAX_ZIP_ARCHIVE_MATERIALIZED_BYTES;

#[derive(Debug)]
struct SoundMaterializeBudget {
    seen_paths: HashSet<PathBuf>,
    materialized_bytes: u64,
    max_entry_bytes: u64,
    max_total_bytes: u64,
}

impl Default for SoundMaterializeBudget {
    fn default() -> Self {
        Self {
            seen_paths: HashSet::new(),
            materialized_bytes: 0,
            max_entry_bytes: MAX_ZIP_ENTRY_UNCOMPRESSED_BYTES,
            max_total_bytes: MAX_SOUND_CATALOG_MATERIALIZED_BYTES,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SoundCatalog {
    pub events: BTreeMap<String, SoundEventDefinition>,
    #[serde(skip)]
    resource_generation: u64,
}

impl SoundCatalog {
    pub fn load_minecraft_assets_dir(assets_dir: impl AsRef<Path>) -> Result<Self> {
        Self::load_namespace_assets_dir("minecraft", assets_dir)
    }

    pub fn load_namespace_assets_dir(
        namespace: &str,
        namespace_assets_dir: impl AsRef<Path>,
    ) -> Result<Self> {
        ResourceLocation::parse(&format!("{namespace}:_"))?;
        let namespace_assets_dir = namespace_assets_dir.as_ref();
        let path = namespace_assets_dir.join("sounds.json");
        let raw = read_standalone_json(&path, "sound catalog")?;
        let mut catalog = Self::default();
        catalog
            .merge_raw(namespace, namespace_assets_dir, raw, None, None, None)
            .with_context(|| format!("parse sounds {}", path.display()))
            .map(|()| catalog)
    }

    pub fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let mut catalog = Self::default();
        catalog.merge_selected_resource_stack(stack, &mut SoundMaterializeBudget::default())?;
        catalog.admit_retained_collections(stack)?;
        Ok(catalog)
    }

    pub fn merge_resource_stack(&mut self, stack: &PackResourceStack) -> Result<()> {
        self.merge_selected_resource_stack(stack, &mut SoundMaterializeBudget::default())?;
        self.admit_retained_collections(stack)
    }

    fn merge_selected_resource_stack(
        &mut self,
        stack: &PackResourceStack,
        materialize_budget: &mut SoundMaterializeBudget,
    ) -> Result<()> {
        let mut lazy_virtual_entries = Vec::new();
        for namespace in stack.namespaces()? {
            let sounds_location = ResourceLocation::new(namespace.clone(), "sounds.json")?;
            for resource in stack.get_resource_stack(&sounds_location) {
                let raw: BTreeMap<String, RawSoundEventDefinition> =
                    match stack.read_json_resource(&resource, "sound catalog") {
                        Ok(raw) => raw,
                        Err(err) => {
                            stack.ensure_resource_data_admitted().map_err(|_| err)?;
                            continue;
                        }
                    };
                let namespace_assets_dir = resource
                    .path
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("sounds path has no parent"))?;
                let mut file_lazy_entries = Vec::new();
                if let Err(err) = self.merge_raw(
                    &namespace,
                    namespace_assets_dir,
                    raw,
                    Some(stack),
                    Some(&mut file_lazy_entries),
                    Some(materialize_budget),
                ) {
                    stack.ensure_resource_data_admitted().map_err(|_| err)?;
                    continue;
                }
                lazy_virtual_entries.extend(file_lazy_entries);
            }
        }
        crate::zip_pack::register_lazy_virtual_entries(lazy_virtual_entries);
        Ok(())
    }

    pub fn load_required_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let catalog = Self::load_resource_stack(stack)?;
        if catalog.is_empty() {
            bail!(
                "required sound catalog is empty; checked resource roots [{}]",
                stack
                    .roots()
                    .iter()
                    .map(|root| root.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        Ok(catalog)
    }

    pub fn from_json_bytes(
        namespace: &str,
        namespace_assets_dir: impl AsRef<Path>,
        bytes: &[u8],
    ) -> Result<Self> {
        let mut catalog = Self::default();
        let namespace_assets_dir = namespace_assets_dir.as_ref();
        catalog.merge_json_bytes(namespace, namespace_assets_dir, bytes, None)?;
        Ok(catalog)
    }

    fn merge_json_bytes(
        &mut self,
        namespace: &str,
        namespace_assets_dir: &Path,
        bytes: &[u8],
        stack: Option<&PackResourceStack>,
    ) -> Result<()> {
        ResourceLocation::parse(&format!("{namespace}:_"))?;
        let raw: BTreeMap<String, RawSoundEventDefinition> = serde_json::from_slice(bytes)?;
        self.merge_raw(namespace, namespace_assets_dir, raw, stack, None, None)
    }

    fn merge_raw(
        &mut self,
        namespace: &str,
        namespace_assets_dir: &Path,
        raw: BTreeMap<String, RawSoundEventDefinition>,
        stack: Option<&PackResourceStack>,
        mut lazy_virtual_entries: Option<&mut Vec<(PathBuf, ZipEntrySource)>>,
        mut materialize_budget: Option<&mut SoundMaterializeBudget>,
    ) -> Result<()> {
        ResourceLocation::parse(&format!("{namespace}:_"))?;
        let mut retained_event_count = self.events.len();
        let mut retained_entry_count = self.sound_entry_count()?;
        for (event_path, event) in raw {
            let location = ResourceLocation::new(namespace.to_string(), event_path.clone())?;
            let id = location.id();
            let definition = event
                .into_definition(
                    namespace,
                    id,
                    namespace_assets_dir,
                    stack,
                    lazy_virtual_entries.as_deref_mut(),
                    materialize_budget.as_deref_mut(),
                )
                .with_context(|| format!("parse sound event {event_path:?}"))?;
            self.merge_definition(
                definition,
                stack,
                &mut retained_event_count,
                &mut retained_entry_count,
            )?;
        }
        Ok(())
    }

    fn merge_definition(
        &mut self,
        definition: SoundEventDefinition,
        stack: Option<&PackResourceStack>,
        retained_event_count: &mut usize,
        retained_entry_count: &mut usize,
    ) -> Result<()> {
        let existing = self.events.get(&definition.id);
        let next_event_count = if existing.is_some() {
            *retained_event_count
        } else {
            retained_event_count
                .checked_add(1)
                .context("sound event count overflow")?
        };
        let replaced_entries = existing.map_or(0, |existing| existing.sounds.len());
        let next_entry_count = if existing.is_some_and(|_| !definition.replace) {
            retained_entry_count
                .checked_add(definition.sounds.len())
                .context("sound entry count overflow")?
        } else {
            retained_entry_count
                .checked_sub(replaced_entries)
                .and_then(|count| count.checked_add(definition.sounds.len()))
                .context("sound entry count overflow")?
        };
        admit_sound_collection_count(
            stack,
            u64::try_from(next_event_count).context("sound event count overflow")?,
            MAX_SOUND_EVENT_DEFINITIONS,
            "sound event definitions",
        )?;
        admit_sound_collection_count(
            stack,
            u64::try_from(next_entry_count).context("sound entry count overflow")?,
            MAX_SOUND_ENTRIES,
            "sound entries",
        )?;
        match self.events.get_mut(&definition.id) {
            Some(existing) if !definition.replace => existing.sounds.extend(definition.sounds),
            _ => {
                self.events.insert(definition.id.clone(), definition);
            }
        }
        *retained_event_count = next_event_count;
        *retained_entry_count = next_entry_count;
        Ok(())
    }

    pub fn event(&self, id: &str) -> Option<&SoundEventDefinition> {
        let id = ResourceLocation::parse(id).ok()?.id();
        self.events.get(&id)
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn resource_generation(&self) -> u64 {
        self.resource_generation
    }

    /// Return the unique complete buffers that Vanilla queues through
    /// `Sound.preloadIfRequired` while applying `sounds.json`.
    ///
    /// `SoundManager.Preparations.apply` visits every registered event, so an
    /// event reference does not expand the final set beyond the direct file
    /// entries already present in the catalog. `SoundBufferLibrary` keys its
    /// futures by the file resource id; the map below preserves that exact
    /// deduplication while keeping the plan deterministic.
    pub fn preload_files(&self) -> Vec<SoundPreload> {
        let mut files = BTreeMap::new();
        for event in self.events.values() {
            for sound in &event.sounds {
                if sound.kind != SoundEntryKind::File || !sound.preload {
                    continue;
                }
                let Some(ogg_path) = sound.ogg_path.as_ref() else {
                    continue;
                };
                files
                    .entry(sound.name.clone())
                    .or_insert_with(|| SoundPreload {
                        sound_name: sound.name.clone(),
                        ogg_path: ogg_path.clone(),
                    });
            }
        }
        files.into_values().collect()
    }

    pub(crate) fn set_resource_generation(&mut self, generation: u64) {
        self.resource_generation = generation;
    }

    pub(crate) fn admit_retained_collections(&self, stack: &PackResourceStack) -> Result<()> {
        let events = u64::try_from(self.events.len()).context("sound event count overflow")?;
        let entries =
            u64::try_from(self.sound_entry_count()?).context("sound entry count overflow")?;
        stack.admit_resource_collection_count(
            events,
            MAX_SOUND_EVENT_DEFINITIONS,
            "sound event definitions",
        )?;
        stack.admit_resource_collection_count(entries, MAX_SOUND_ENTRIES, "sound entries")?;
        let logical_bytes = events
            .checked_mul(SOUND_EVENT_LOGICAL_BYTES)
            .and_then(|bytes| {
                entries
                    .checked_mul(SOUND_ENTRY_LOGICAL_BYTES)
                    .and_then(|entries| bytes.checked_add(entries))
            })
            .context("sound catalog derived logical byte count overflow")?;
        stack.reserve_derived_resource_bytes(logical_bytes, "retained sound catalog")
    }

    fn sound_entry_count(&self) -> Result<usize> {
        self.events.values().try_fold(0_usize, |count, event| {
            count
                .checked_add(event.sounds.len())
                .context("sound entry count overflow")
        })
    }
}

fn admit_sound_collection_count(
    stack: Option<&PackResourceStack>,
    count: u64,
    maximum: u64,
    label: &str,
) -> Result<()> {
    if let Some(stack) = stack {
        return stack.admit_resource_collection_count(count, maximum, label);
    }
    if count > maximum {
        bail!(
            "resource collection {label} has {count} entries, exceeding the {maximum} entry limit"
        );
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundEventDefinition {
    pub id: String,
    pub replace: bool,
    pub subtitle: Option<String>,
    pub sounds: Vec<SoundEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundPreload {
    pub sound_name: String,
    pub ogg_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundEntry {
    pub name: String,
    pub kind: SoundEntryKind,
    pub volume: f32,
    pub pitch: f32,
    pub weight: i32,
    pub stream: bool,
    pub preload: bool,
    pub attenuation_distance: i32,
    pub ogg_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ogg_integrity: Option<ResourceIntegrity>,
}

impl SoundEntry {
    /// Reads and inflates a virtual/Web ZIP `.ogg` entry on demand without
    /// eagerly materializing uncompressed copies into `virtual_store`.
    pub fn read_lazy_virtual_ogg(path: &Path, max_bytes: u64) -> Option<Result<Vec<u8>>> {
        crate::zip_pack::read_lazy_virtual_entry_bounded(path, max_bytes, "audio resource")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundEntryKind {
    File,
    Event,
}

#[derive(Debug, Deserialize)]
struct RawSoundEventDefinition {
    #[serde(default)]
    replace: bool,
    subtitle: Option<String>,
    #[serde(default)]
    sounds: Vec<RawSoundEntry>,
}

impl RawSoundEventDefinition {
    fn into_definition(
        self,
        namespace: &str,
        id: String,
        namespace_assets_dir: &Path,
        stack: Option<&PackResourceStack>,
        mut lazy_virtual_entries: Option<&mut Vec<(PathBuf, ZipEntrySource)>>,
        mut materialize_budget: Option<&mut SoundMaterializeBudget>,
    ) -> Result<SoundEventDefinition> {
        let mut sounds = Vec::with_capacity(self.sounds.len());
        for sound in self.sounds {
            if let Some(entry) = sound.into_entry(
                namespace,
                namespace_assets_dir,
                stack,
                lazy_virtual_entries.as_deref_mut(),
                materialize_budget.as_deref_mut(),
            )? {
                sounds.push(entry);
            }
        }
        Ok(SoundEventDefinition {
            id,
            replace: self.replace,
            subtitle: self.subtitle,
            sounds,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawSoundEntry {
    Name(String),
    Object(RawSoundObject),
}

impl RawSoundEntry {
    fn into_entry(
        self,
        namespace: &str,
        namespace_assets_dir: &Path,
        stack: Option<&PackResourceStack>,
        lazy_virtual_entries: Option<&mut Vec<(PathBuf, ZipEntrySource)>>,
        materialize_budget: Option<&mut SoundMaterializeBudget>,
    ) -> Result<Option<SoundEntry>> {
        match self {
            Self::Name(name) => RawSoundObject {
                name,
                kind: RawSoundEntryKind::File,
                volume: 1.0,
                pitch: 1.0,
                weight: 1,
                stream: false,
                preload: false,
                attenuation_distance: 16,
            }
            .into_entry(
                namespace,
                namespace_assets_dir,
                stack,
                lazy_virtual_entries,
                materialize_budget,
            ),
            Self::Object(raw) => raw.into_entry(
                namespace,
                namespace_assets_dir,
                stack,
                lazy_virtual_entries,
                materialize_budget,
            ),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawSoundObject {
    name: String,
    #[serde(rename = "type", default)]
    kind: RawSoundEntryKind,
    #[serde(default = "default_one")]
    volume: f32,
    #[serde(default = "default_one")]
    pitch: f32,
    #[serde(default = "default_weight")]
    weight: i32,
    #[serde(default)]
    stream: bool,
    #[serde(default)]
    preload: bool,
    #[serde(default = "default_attenuation_distance")]
    attenuation_distance: i32,
}

impl RawSoundObject {
    fn into_entry(
        self,
        namespace: &str,
        namespace_assets_dir: &Path,
        stack: Option<&PackResourceStack>,
        lazy_virtual_entries: Option<&mut Vec<(PathBuf, ZipEntrySource)>>,
        materialize_budget: Option<&mut SoundMaterializeBudget>,
    ) -> Result<Option<SoundEntry>> {
        if !self.volume.is_finite() || self.volume <= 0.0 {
            bail!("invalid sound volume {}", self.volume);
        }
        if !self.pitch.is_finite() || self.pitch <= 0.0 {
            bail!("invalid sound pitch {}", self.pitch);
        }
        if self.weight <= 0 {
            bail!("invalid sound weight {}", self.weight);
        }
        let location = ResourceLocation::parse(&self.name)?;
        let name = location.id();
        let kind: SoundEntryKind = self.kind.into();
        let (ogg_path, ogg_integrity) = match kind {
            SoundEntryKind::File => match sound_ogg_resource(
                namespace_assets_dir,
                namespace,
                &location,
                stack,
                lazy_virtual_entries,
                materialize_budget,
            )? {
                Some((path, integrity)) => (Some(path), integrity),
                None => return Ok(None),
            },
            SoundEntryKind::Event => (None, None),
        };
        Ok(Some(SoundEntry {
            name,
            kind,
            volume: self.volume,
            pitch: self.pitch,
            weight: self.weight,
            stream: self.stream,
            preload: self.preload,
            attenuation_distance: self.attenuation_distance,
            ogg_path,
            ogg_integrity,
        }))
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RawSoundEntryKind {
    #[default]
    File,
    Event,
}

impl From<RawSoundEntryKind> for SoundEntryKind {
    fn from(kind: RawSoundEntryKind) -> Self {
        match kind {
            RawSoundEntryKind::File => SoundEntryKind::File,
            RawSoundEntryKind::Event => SoundEntryKind::Event,
        }
    }
}

fn sound_ogg_resource(
    namespace_assets_dir: &Path,
    namespace: &str,
    location: &ResourceLocation,
    stack: Option<&PackResourceStack>,
    lazy_virtual_entries: Option<&mut Vec<(PathBuf, ZipEntrySource)>>,
    materialize_budget: Option<&mut SoundMaterializeBudget>,
) -> Result<Option<(PathBuf, Option<ResourceIntegrity>)>> {
    let sound_location = sound_resource_location(location)?;
    if let Some(resource) = stack.and_then(|stack| stack.get_resource(&sound_location)) {
        if let Some(source) = resource.archive_source() {
            if crate::virtual_store::is_virtual_path(&resource.path) {
                if let Some(entries) = lazy_virtual_entries {
                    entries.push((resource.path.clone(), source.clone()));
                } else {
                    crate::zip_pack::register_lazy_virtual_entries(vec![(
                        resource.path.clone(),
                        source.clone(),
                    )]);
                }
                return Ok(Some((resource.path, resource.integrity)));
            }
            if let Some(budget) = materialize_budget {
                if budget.seen_paths.contains(&resource.path) {
                    return Ok(Some((resource.path, resource.integrity)));
                }
                let remaining = budget
                    .max_total_bytes
                    .saturating_sub(budget.materialized_bytes);
                let entry_limit = budget.max_entry_bytes.min(remaining);
                let archive_limit = MAX_ZIP_ARCHIVE_MATERIALIZED_BYTES.min(budget.max_total_bytes);
                let path = match source.materialize_with_limits(entry_limit, archive_limit) {
                    Ok(path) => path.to_path_buf(),
                    Err(err) => {
                        if let Some(stack) = stack {
                            return Err(stack.reject_resource_data(format!("{err:#}")));
                        }
                        return Err(err);
                    }
                };
                let actual_bytes = std::fs::metadata(&path)
                    .map(|meta| meta.len())
                    .unwrap_or_else(|_| source.uncompressed_size());
                let next_total = budget.materialized_bytes.saturating_add(actual_bytes);
                if next_total > budget.max_total_bytes {
                    let message = format!(
                        "sound catalog materialization exceeded the {} byte limit",
                        budget.max_total_bytes
                    );
                    if let Some(stack) = stack {
                        return Err(stack.reject_resource_data(message));
                    }
                    bail!(message);
                }
                budget.materialized_bytes = next_total;
                budget.seen_paths.insert(path.clone());
                return Ok(Some((path, resource.integrity)));
            }
        }
        let path = match resource.materialized_path() {
            Ok(path) => path.to_path_buf(),
            Err(err) => {
                if let Some(stack) = stack {
                    return Err(stack.reject_resource_data(format!("{err:#}")));
                }
                return Err(err);
            }
        };
        return Ok(Some((path, resource.integrity)));
    }

    if stack.is_some() {
        return Ok(None);
    }

    let assets_dir = if location.namespace() == namespace {
        namespace_assets_dir.to_path_buf()
    } else {
        namespace_assets_dir
            .parent()
            .unwrap_or(namespace_assets_dir)
            .join(location.namespace())
    };
    Ok(Some((
        assets_dir
            .join("sounds")
            .join(format!("{}.ogg", location.path())),
        None,
    )))
}

fn sound_resource_location(location: &ResourceLocation) -> Result<ResourceLocation> {
    ResourceLocation::new(
        location.namespace().to_string(),
        format!("sounds/{}.ogg", location.path()),
    )
}

fn default_one() -> f32 {
    1.0
}

fn default_weight() -> i32 {
    1
}

fn default_attenuation_distance() -> i32 {
    16
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::roots::{PackRoots, MC_VERSION};

    #[test]
    fn sound_semantic_counts_accept_exact_limits_and_reject_overflow() {
        let exact = PackResourceStack::default();
        admit_sound_collection_count(
            Some(&exact),
            MAX_SOUND_EVENT_DEFINITIONS,
            MAX_SOUND_EVENT_DEFINITIONS,
            "sound event definitions",
        )
        .unwrap();
        admit_sound_collection_count(
            Some(&exact),
            MAX_SOUND_ENTRIES,
            MAX_SOUND_ENTRIES,
            "sound entries",
        )
        .unwrap();

        for (maximum, label) in [
            (MAX_SOUND_EVENT_DEFINITIONS, "sound event definitions"),
            (MAX_SOUND_ENTRIES, "sound entries"),
        ] {
            let stack = PackResourceStack::default();
            let error = admit_sound_collection_count(Some(&stack), maximum + 1, maximum, label)
                .unwrap_err()
                .to_string();
            assert!(error.contains(label), "{error}");
            assert!(stack.ensure_resource_data_admitted().is_err());
        }
    }

    #[test]
    fn retained_sound_catalog_charges_events_and_entries_once() {
        let assets_dir = PathBuf::from("assets/minecraft");
        let catalog = SoundCatalog::from_json_bytes(
            "minecraft",
            &assets_dir,
            br#"{"test":{"sounds":["test/one","test/two"]}}"#,
        )
        .unwrap();
        let stack = PackResourceStack::default();

        catalog.admit_retained_collections(&stack).unwrap();

        let observation = stack.resource_data_memory_observation();
        assert_eq!(
            observation.derived_logical_bytes,
            SOUND_EVENT_LOGICAL_BYTES + 2 * SOUND_ENTRY_LOGICAL_BYTES
        );
    }

    #[test]
    fn pack_roots_stamp_sound_catalog_with_exact_resource_generation() {
        let root = unique_temp_dir("sound-generation");
        let assets_dir = root.join("sources/26.1/assets/minecraft");
        std::fs::create_dir_all(assets_dir.join("sounds/test")).unwrap();
        std::fs::write(
            assets_dir.join("sounds.json"),
            r#"{"test":{"sounds":["test/one"]}}"#,
        )
        .unwrap();
        std::fs::write(assets_dir.join("sounds/test/one.ogg"), []).unwrap();
        let roots = PackRoots::from_root(&root).unwrap();
        let selected = crate::SelectedClientPackStack::from_startup_dirs(&roots, []).unwrap();
        let roots = roots.with_client_resource_generation(47, selected).unwrap();

        let catalog = roots.load_sound_catalog().unwrap();

        assert_eq!(catalog.resource_generation(), 47);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parses_sound_definitions_with_vanilla_defaults_and_paths() {
        let root = unique_temp_dir("sound-catalog");
        let assets_dir = root
            .join("sources")
            .join("26.1")
            .join("assets")
            .join("minecraft");
        std::fs::create_dir_all(assets_dir.join("sounds").join("mob").join("cat")).unwrap();
        std::fs::write(
            assets_dir.join("sounds.json"),
            r#"{
              "entity.cat.ambient": {
                "subtitle": "subtitles.entity.cat.ambient",
                "sounds": [
                  "mob/cat/meow1",
                  {
                    "name": "mob/cat/meow2",
                    "volume": 0.6,
                    "pitch": 1.2,
                    "weight": 3,
                    "stream": true,
                    "preload": true,
                    "attenuation_distance": 32
                  },
                  "custom:mob/cat/meow3",
                  {
                    "name": "entity.cat.purr",
                    "type": "event"
                  }
                ]
              },
              "entity.cat.purr": {
                "replace": true,
                "sounds": []
              }
            }"#,
        )
        .unwrap();

        let catalog = SoundCatalog::load_minecraft_assets_dir(&assets_dir).unwrap();
        let event = catalog.event("entity.cat.ambient").unwrap();

        assert_eq!(catalog.len(), 2);
        assert_eq!(event.id, "minecraft:entity.cat.ambient");
        assert_eq!(
            event.subtitle.as_deref(),
            Some("subtitles.entity.cat.ambient")
        );
        assert!(!event.replace);
        assert_eq!(event.sounds.len(), 4);
        assert_eq!(
            event.sounds[0],
            SoundEntry {
                name: "minecraft:mob/cat/meow1".to_string(),
                kind: SoundEntryKind::File,
                volume: 1.0,
                pitch: 1.0,
                weight: 1,
                stream: false,
                preload: false,
                attenuation_distance: 16,
                ogg_path: Some(assets_dir.join("sounds").join("mob/cat/meow1.ogg")),
                ogg_integrity: None,
            }
        );
        assert_eq!(event.sounds[1].volume, 0.6);
        assert_eq!(event.sounds[1].pitch, 1.2);
        assert_eq!(event.sounds[1].weight, 3);
        assert!(event.sounds[1].stream);
        assert!(event.sounds[1].preload);
        assert_eq!(event.sounds[1].attenuation_distance, 32);
        assert_eq!(event.sounds[2].name, "custom:mob/cat/meow3");
        assert_eq!(
            event.sounds[2].ogg_path,
            Some(
                assets_dir
                    .parent()
                    .unwrap()
                    .join("custom")
                    .join("sounds")
                    .join("mob/cat/meow3.ogg")
            )
        );
        assert_eq!(event.sounds[3].kind, SoundEntryKind::Event);
        assert!(event.sounds[3].ogg_path.is_none());
        assert!(catalog.event("minecraft:entity.cat.purr").unwrap().replace);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn preload_plan_matches_vanilla_complete_buffer_queue_and_deduplicates_resource_ids() {
        let assets_dir = PathBuf::from("assets/minecraft");
        let catalog = SoundCatalog::from_json_bytes(
            "minecraft",
            &assets_dir,
            br#"{
              "direct": {
                "sounds": [
                  {"name":"shared/preloaded","preload":true},
                  {"name":"music/streamed","stream":true,"preload":true},
                  {"name":"lazy/not_preloaded","preload":false}
                ]
              },
              "duplicate": {
                "sounds": [
                  {"name":"shared/preloaded","preload":true},
                  {"name":"direct","type":"event","preload":true}
                ]
              }
            }"#,
        )
        .unwrap();

        let preloads = catalog.preload_files();

        assert_eq!(
            preloads,
            vec![
                SoundPreload {
                    sound_name: "minecraft:music/streamed".to_string(),
                    ogg_path: assets_dir.join("sounds/music/streamed.ogg"),
                },
                SoundPreload {
                    sound_name: "minecraft:shared/preloaded".to_string(),
                    ogg_path: assets_dir.join("sounds/shared/preloaded.ogg"),
                },
            ]
        );
    }

    #[test]
    fn pack_roots_loads_sound_catalog_from_minecraft_assets() {
        let root = unique_temp_dir("sound-roots");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        std::fs::create_dir_all(&assets_dir).unwrap();
        write_file(&assets_dir.join("sounds").join("random/click_stereo.ogg"));
        std::fs::write(
            assets_dir.join("sounds.json"),
            r#"{
              "ui.button.click": {
                "sounds": ["random/click_stereo"]
              }
            }"#,
        )
        .unwrap();

        let roots = PackRoots::from_root(&root).unwrap();
        let catalog = roots.load_sound_catalog().unwrap();

        assert_eq!(
            catalog.event("ui.button.click").unwrap().sounds[0].ogg_path,
            Some(assets_dir.join("sounds").join("random/click_stereo.ogg"))
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn required_sound_catalog_errors_when_resource_stack_has_no_sounds() {
        let root = unique_temp_dir("sound-required-empty");
        std::fs::create_dir_all(root.join("sources").join(MC_VERSION))
            .expect("test source root should exist");
        let roots = PackRoots::from_root(&root).unwrap();

        let err = roots.load_required_sound_catalog().unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("load required sound catalog"));
        assert!(message.contains("sounds.json"));
        assert!(message.contains("required sound catalog is empty"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn required_sound_catalog_loads_minimal_valid_catalog() {
        let root = unique_temp_dir("sound-required-valid");
        let assets_dir = root
            .join("sources")
            .join(MC_VERSION)
            .join("assets")
            .join("minecraft");
        write_file(&assets_dir.join("sounds").join("random/click.ogg"));
        write_json(
            &assets_dir.join("sounds.json"),
            r#"{
              "ui.button.click": {
                "sounds": ["random/click"]
              }
            }"#,
        );
        let roots = PackRoots::from_root(&root).unwrap();

        let catalog = roots.load_required_sound_catalog().unwrap();

        assert_eq!(catalog.len(), 1);
        assert!(catalog.event("minecraft:ui.button.click").is_some());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sound_catalog_merges_resource_stack_definitions_in_pack_order() {
        let root = unique_temp_dir("sound-stack-merge");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        let base_assets = base.join("assets").join("minecraft");
        let overlay_assets = overlay.join("assets").join("minecraft");
        write_file(&base_assets.join("sounds").join("random").join("click.ogg"));
        let overlay_ogg = overlay_assets
            .join("sounds")
            .join("random")
            .join("click.ogg");
        write_file(&overlay_ogg);
        write_json(
            &base_assets.join("sounds.json"),
            r#"{
              "ui.button.click": {
                "subtitle": "subtitles.ui.button.click",
                "sounds": ["random/click"]
              }
            }"#,
        );
        write_json(
            &overlay_assets.join("sounds.json"),
            r#"{
              "ui.button.click": {
                "subtitle": "subtitles.ui.button.overlay",
                "sounds": [
                  {
                    "name": "random/click",
                    "volume": 0.5
                  }
                ]
              }
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let catalog = roots.load_sound_catalog().unwrap();
        let event = catalog.event("minecraft:ui.button.click").unwrap();

        assert_eq!(event.subtitle.as_deref(), Some("subtitles.ui.button.click"));
        assert_eq!(event.sounds.len(), 2);
        assert_eq!(event.sounds[0].ogg_path, Some(overlay_ogg.clone()));
        assert_eq!(event.sounds[1].ogg_path, Some(overlay_ogg));
        assert_eq!(event.sounds[1].volume, 0.5);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sound_catalog_replace_clears_lower_priority_sounds() {
        let root = unique_temp_dir("sound-stack-replace");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        let base_assets = base.join("assets").join("minecraft");
        let overlay_assets = overlay.join("assets").join("minecraft");
        write_file(
            &base_assets
                .join("sounds")
                .join("music")
                .join("menu_base.ogg"),
        );
        let overlay_ogg = overlay_assets
            .join("sounds")
            .join("music")
            .join("menu_overlay.ogg");
        write_file(&overlay_ogg);
        write_json(
            &base_assets.join("sounds.json"),
            r#"{
              "music.menu": {
                "subtitle": "subtitles.music.base",
                "sounds": ["music/menu_base"]
              }
            }"#,
        );
        write_json(
            &overlay_assets.join("sounds.json"),
            r#"{
              "music.menu": {
                "replace": true,
                "subtitle": "subtitles.music.overlay",
                "sounds": ["music/menu_overlay"]
              }
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let catalog = roots.load_sound_catalog().unwrap();
        let event = catalog.event("music.menu").unwrap();

        assert!(event.replace);
        assert_eq!(event.subtitle.as_deref(), Some("subtitles.music.overlay"));
        assert_eq!(event.sounds.len(), 1);
        assert_eq!(event.sounds[0].name, "minecraft:music/menu_overlay");
        assert_eq!(event.sounds[0].ogg_path, Some(overlay_ogg));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sound_catalog_skips_invalid_resource_stack_definitions() {
        let root = unique_temp_dir("sound-stack-invalid");
        let base = root.join("sources").join(MC_VERSION);
        let overlay = root.join("overlay");
        let base_assets = base.join("assets").join("minecraft");
        let overlay_assets = overlay.join("assets").join("minecraft");
        write_file(&base_assets.join("sounds").join("random").join("click.ogg"));
        write_json(
            &base_assets.join("sounds.json"),
            r#"{
              "ui.button.click": {
                "sounds": ["random/click"]
              }
            }"#,
        );
        write_json(
            &overlay_assets.join("sounds.json"),
            r#"{
              "ui.button.click": {
                "sounds": [
                  {
                    "name": "random/click",
                    "volume": 0.0
                  }
                ]
              }
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([overlay]);
        let catalog = roots.load_sound_catalog().unwrap();
        let event = catalog.event("minecraft:ui.button.click").unwrap();

        assert_eq!(event.sounds.len(), 1);
        assert_eq!(event.sounds[0].name, "minecraft:random/click");
        assert_eq!(
            event.sounds[0].ogg_path,
            Some(base_assets.join("sounds").join("random").join("click.ogg"))
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sound_catalog_skips_missing_file_sounds_from_resource_stack() {
        let root = unique_temp_dir("sound-stack-missing-files");
        let base = root.join("sources").join(MC_VERSION);
        let assets_dir = base.join("assets").join("minecraft");
        let present_ogg = assets_dir.join("sounds").join("random").join("present.ogg");
        write_file(&present_ogg);
        write_json(
            &assets_dir.join("sounds.json"),
            r#"{
              "ui.button.click": {
                "sounds": [
                  "random/present",
                  "random/missing",
                  {
                    "name": "ui.forward",
                    "type": "event"
                  }
                ]
              },
              "ui.forward": {
                "sounds": ["random/missing_reference_target"]
              }
            }"#,
        );

        let roots = PackRoots::from_root(&root).unwrap();
        let catalog = roots.load_sound_catalog().unwrap();
        let event = catalog.event("minecraft:ui.button.click").unwrap();
        let referenced = catalog.event("minecraft:ui.forward").unwrap();

        assert_eq!(event.sounds.len(), 2);
        assert_eq!(event.sounds[0].name, "minecraft:random/present");
        assert_eq!(event.sounds[0].ogg_path, Some(present_ogg));
        assert_eq!(event.sounds[1].kind, SoundEntryKind::Event);
        assert_eq!(event.sounds[1].name, "minecraft:ui.forward");
        assert!(event.sounds[1].ogg_path.is_none());
        assert!(referenced.sounds.is_empty());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sound_catalog_loads_all_namespaces_from_resource_stack() {
        let root = unique_temp_dir("sound-stack-namespaces");
        let base = root.join("sources").join(MC_VERSION);
        let custom_pack = root.join("packs/custom");
        let custom_assets = custom_pack.join("assets").join("custom");
        std::fs::create_dir_all(base.join("assets/minecraft")).unwrap();
        write_file(
            &custom_assets
                .join("sounds")
                .join("ambient")
                .join("wind.ogg"),
        );
        write_json(
            &custom_assets.join("sounds.json"),
            r#"{
              "ambient.wind": {
                "sounds": ["custom:ambient/wind"]
              }
            }"#,
        );

        let roots = PackRoots::from_root(&root)
            .unwrap()
            .with_resource_pack_dirs([custom_pack]);
        let catalog = roots.load_sound_catalog().unwrap();
        let event = catalog.event("custom:ambient.wind").unwrap();

        assert_eq!(catalog.len(), 1);
        assert_eq!(event.sounds[0].name, "custom:ambient/wind");
        assert_eq!(
            event.sounds[0].ogg_path,
            Some(
                custom_assets
                    .join("sounds")
                    .join("ambient")
                    .join("wind.ogg")
            )
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn virtual_zip_pack_sounds_resolve_lazily_without_eager_decompression() {
        use std::{io::Write, sync::Arc};
        use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut cursor);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            zip.start_file("pack.mcmeta", opts).unwrap();
            zip.write_all(
                &serde_json::to_vec(&serde_json::json!({
                    "pack": {
                        "description": "Lazy web zip pack",
                        "min_format": [bbb_protocol::MC_RESOURCE_PACK_FORMAT.major, bbb_protocol::MC_RESOURCE_PACK_FORMAT.minor],
                        "max_format": [bbb_protocol::MC_RESOURCE_PACK_FORMAT.major, bbb_protocol::MC_RESOURCE_PACK_FORMAT.minor]
                    }
                }))
                .unwrap(),
            )
            .unwrap();
            zip.start_file("version.json", opts).unwrap();
            zip.write_all(
                &serde_json::to_vec(&serde_json::json!({
                    "id": MC_VERSION,
                    "name": MC_VERSION,
                    "protocol_version": bbb_protocol::PROTOCOL_VERSION,
                }))
                .unwrap(),
            )
            .unwrap();
            zip.start_file("assets/minecraft/sounds.json", opts)
                .unwrap();
            zip.write_all(br#"{"custom.chime":{"sounds":["custom/chime"]}}"#)
                .unwrap();
            zip.start_file("assets/minecraft/sounds/custom/chime.ogg", opts)
                .unwrap();
            zip.write_all(b"OggS-lazy-zip-payload").unwrap();
            zip.finish().unwrap();
        }

        let zip_bytes: Arc<[u8]> = Arc::from(cursor.into_inner());
        let roots = PackRoots::from_web_bundle(Arc::clone(&zip_bytes), None, None).unwrap();
        let catalog = roots.load_required_sound_catalog().unwrap();
        let ogg_path = catalog.event("minecraft:custom.chime").unwrap().sounds[0]
            .ogg_path
            .clone()
            .unwrap();
        assert!(
            !crate::virtual_store::has_virtual_file(&ogg_path),
            "virtual ZIP .ogg must not be eagerly materialized into virtual_store"
        );
        let bytes = SoundEntry::read_lazy_virtual_ogg(&ogg_path, 1024 * 1024)
            .expect("expected registered lazy virtual .ogg")
            .unwrap();
        assert_eq!(bytes, b"OggS-lazy-zip-payload");

        let weak_inner = Arc::downgrade(&zip_bytes);
        drop(zip_bytes);
        drop(catalog);
        drop(roots);
        crate::virtual_store::remove_virtual_prefix(Path::new("/__bbb_mem__/vanilla"));
        assert_eq!(
            weak_inner.strong_count(),
            0,
            "lazy virtual ZIP entry must not pin unmounted Arc<[u8]> memory"
        );
        assert!(SoundEntry::read_lazy_virtual_ogg(&ogg_path, 1024 * 1024).is_none());
    }

    #[test]
    fn rejects_invalid_sound_entry_values() {
        let err = SoundCatalog::from_json_bytes(
            "minecraft",
            std::env::temp_dir(),
            br#"{"bad.event":{"sounds":[{"name":"bad","volume":0.0}]}}"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("bad.event"));
    }

    #[test]
    fn native_zip_sound_materialization_deduplicates_and_rejects_budget_overflow() {
        use std::io::Write;
        use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

        let root = unique_temp_dir("native-zip-sound-budget");
        std::fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("pack.zip");
        {
            let file = std::fs::File::create(&archive_path).unwrap();
            let mut zip = ZipWriter::new(file);
            let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
            zip.start_file("pack.mcmeta", opts).unwrap();
            zip.write_all(
                br#"{"pack":{"description":"sound budget","min_format":[84,0],"max_format":[84,0]}}"#,
            )
            .unwrap();
            zip.start_file("assets/minecraft/sounds.json", opts)
                .unwrap();
            zip.write_all(
                br#"{
                  "a":{"sounds":["custom/one","custom/one"]},
                  "b":{"sounds":["custom/two"]}
                }"#,
            )
            .unwrap();
            zip.start_file("assets/minecraft/sounds/custom/one.ogg", opts)
                .unwrap();
            zip.write_all(&vec![0x55u8; 4096]).unwrap();
            zip.start_file("assets/minecraft/sounds/custom/two.ogg", opts)
                .unwrap();
            zip.write_all(&vec![0x66u8; 4096]).unwrap();
            zip.finish().unwrap();
        }

        let cache = root.join("cache");
        let prepared = crate::prepare_pack_archive(&archive_path, &cache).unwrap();
        let pack = crate::SelectedClientPack::server_archive("sounds", 1, 0, prepared).unwrap();
        let stack = PackResourceStack::prepare_selected_client_packs(&[pack]).unwrap();

        // Tight 6000-byte total budget allows `one.ogg` (referenced twice, deduped!) but rejects `two.ogg`.
        let mut tight = SoundCatalog::default();
        let mut tight_budget = SoundMaterializeBudget {
            seen_paths: HashSet::new(),
            materialized_bytes: 0,
            max_entry_bytes: 4096,
            max_total_bytes: 6000,
        };
        let err = tight
            .merge_selected_resource_stack(&stack, &mut tight_budget)
            .unwrap_err();
        assert!(
            format!("{err:#}").contains("materialization limit"),
            "{err:#}"
        );
        assert!(stack.ensure_resource_data_admitted().is_err());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "requires the local Vanilla 26.1 asset object store"]
    fn indexed_vanilla_26_1_sounds_catalog_loads_representative_events() {
        let catalog = PackRoots::from_environment()
            .unwrap()
            .load_required_sound_catalog()
            .unwrap();

        assert_eq!(catalog.len(), 1902);
        let cave = catalog.event("minecraft:ambient.cave").unwrap();
        assert_eq!(cave.subtitle.as_deref(), Some("subtitles.ambient.sound"));
        assert_eq!(cave.sounds[0].name, "minecraft:ambient/cave/cave1");
        assert!(cave.sounds[0].ogg_path.as_ref().unwrap().is_file());
        assert!(cave.sounds[0].ogg_integrity.is_some());
        let music = catalog.event("music.menu").unwrap();
        assert!(music.sounds.iter().any(|sound| sound.stream));
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-client-assets-{label}-{nanos}"))
    }

    fn write_file(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, []).unwrap();
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }
}
