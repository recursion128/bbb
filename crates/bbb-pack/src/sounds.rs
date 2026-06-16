use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::resources::{PackResourceStack, ResourceLocation};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SoundCatalog {
    pub events: BTreeMap<String, SoundEventDefinition>,
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
        let bytes =
            std::fs::read(&path).with_context(|| format!("read sounds {}", path.display()))?;
        Self::from_json_bytes(namespace, namespace_assets_dir, &bytes)
            .with_context(|| format!("parse sounds {}", path.display()))
    }

    pub fn load_resource_stack(stack: &PackResourceStack) -> Result<Self> {
        let mut catalog = Self::default();
        for namespace in stack.namespaces()? {
            let sounds_location = ResourceLocation::new(namespace.clone(), "sounds.json")?;
            for resource in stack.get_resource_stack(&sounds_location) {
                let Ok(bytes) = std::fs::read(&resource.path) else {
                    continue;
                };
                let namespace_assets_dir = resource
                    .path
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("sounds path has no parent"))?;
                if catalog
                    .merge_json_bytes(&namespace, namespace_assets_dir, &bytes, Some(stack))
                    .is_err()
                {
                    continue;
                }
            }
        }
        Ok(catalog)
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
        for (event_path, event) in raw {
            let location = ResourceLocation::new(namespace.to_string(), event_path.clone())?;
            let id = location.id();
            let definition = event
                .into_definition(namespace, id, namespace_assets_dir, stack)
                .with_context(|| format!("parse sound event {event_path:?}"))?;
            self.merge_definition(definition);
        }
        Ok(())
    }

    fn merge_definition(&mut self, definition: SoundEventDefinition) {
        match self.events.get_mut(&definition.id) {
            Some(existing) if !definition.replace => existing.sounds.extend(definition.sounds),
            _ => {
                self.events.insert(definition.id.clone(), definition);
            }
        }
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundEventDefinition {
    pub id: String,
    pub replace: bool,
    pub subtitle: Option<String>,
    pub sounds: Vec<SoundEntry>,
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
    ) -> Result<SoundEventDefinition> {
        let sounds = self
            .sounds
            .into_iter()
            .map(|sound| sound.into_entry(namespace, namespace_assets_dir, stack))
            .filter_map(|entry| entry.transpose())
            .collect::<Result<Vec<_>>>()?;
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
    ) -> Result<Option<SoundEntry>> {
        match self {
            Self::Name(name) => sound_entry(
                name,
                SoundEntryKind::File,
                1.0,
                1.0,
                1,
                false,
                false,
                16,
                namespace,
                namespace_assets_dir,
                stack,
            ),
            Self::Object(raw) => raw.into_entry(namespace, namespace_assets_dir, stack),
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
    ) -> Result<Option<SoundEntry>> {
        sound_entry(
            self.name,
            self.kind.into(),
            self.volume,
            self.pitch,
            self.weight,
            self.stream,
            self.preload,
            self.attenuation_distance,
            namespace,
            namespace_assets_dir,
            stack,
        )
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

fn sound_entry(
    name: String,
    kind: SoundEntryKind,
    volume: f32,
    pitch: f32,
    weight: i32,
    stream: bool,
    preload: bool,
    attenuation_distance: i32,
    namespace: &str,
    namespace_assets_dir: &Path,
    stack: Option<&PackResourceStack>,
) -> Result<Option<SoundEntry>> {
    if !volume.is_finite() || volume <= 0.0 {
        bail!("invalid sound volume {volume}");
    }
    if !pitch.is_finite() || pitch <= 0.0 {
        bail!("invalid sound pitch {pitch}");
    }
    if weight <= 0 {
        bail!("invalid sound weight {weight}");
    }
    let location = ResourceLocation::parse(&name)?;
    let name = location.id();
    let ogg_path = match kind {
        SoundEntryKind::File => {
            match sound_ogg_path(namespace_assets_dir, namespace, &location, stack)? {
                Some(path) => Some(path),
                None => return Ok(None),
            }
        }
        SoundEntryKind::Event => None,
    };
    Ok(Some(SoundEntry {
        name,
        kind,
        volume,
        pitch,
        weight,
        stream,
        preload,
        attenuation_distance,
        ogg_path,
    }))
}

fn sound_ogg_path(
    namespace_assets_dir: &Path,
    namespace: &str,
    location: &ResourceLocation,
    stack: Option<&PackResourceStack>,
) -> Result<Option<PathBuf>> {
    if let Some(resource) = stack.and_then(|stack| {
        sound_resource_location(location)
            .ok()
            .and_then(|id| stack.get_resource(&id))
    }) {
        return Ok(Some(resource.path));
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
    Ok(Some(
        assets_dir
            .join("sounds")
            .join(format!("{}.ogg", location.path())),
    ))
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

        assert_eq!(roots.sounds_definition(), assets_dir.join("sounds.json"));
        assert_eq!(roots.sounds_dir(), assets_dir.join("sounds"));
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
        let custom_assets = base.join("assets").join("custom");
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

        let roots = PackRoots::from_root(&root).unwrap();
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
    #[ignore = "requires generated vanilla 26.1 sound assets in target/assets-26.1"]
    fn generated_vanilla_26_1_sounds_catalog_loads_representative_events() {
        let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("target")
            .join("assets-26.1");
        let catalog = SoundCatalog::load_minecraft_assets_dir(&assets_dir).unwrap();

        assert_eq!(catalog.len(), 1902);
        let cave = catalog.event("minecraft:ambient.cave").unwrap();
        assert_eq!(cave.subtitle.as_deref(), Some("subtitles.ambient.sound"));
        assert_eq!(cave.sounds[0].name, "minecraft:ambient/cave/cave1");
        assert_eq!(
            cave.sounds[0].ogg_path.as_ref().unwrap(),
            &assets_dir.join("sounds").join("ambient/cave/cave1.ogg")
        );
        let music = catalog.event("music.menu").unwrap();
        assert!(music.sounds.iter().any(|sound| sound.stream));
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-pack-{label}-{nanos}"))
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
