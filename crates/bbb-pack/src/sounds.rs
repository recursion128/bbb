use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::atlas_sources::ResourceLocation;

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

    pub fn from_json_bytes(
        namespace: &str,
        namespace_assets_dir: impl AsRef<Path>,
        bytes: &[u8],
    ) -> Result<Self> {
        ResourceLocation::parse(&format!("{namespace}:_"))?;
        let namespace_assets_dir = namespace_assets_dir.as_ref();
        let raw: BTreeMap<String, RawSoundEventDefinition> = serde_json::from_slice(bytes)?;
        let mut events = BTreeMap::new();
        for (event_path, event) in raw {
            let location = ResourceLocation::parse(&format!("{namespace}:{event_path}"))?;
            let id = location.id();
            events.insert(
                id.clone(),
                event
                    .into_definition(namespace, id, namespace_assets_dir)
                    .with_context(|| format!("parse sound event {event_path:?}"))?,
            );
        }
        Ok(Self { events })
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
    ) -> Result<SoundEventDefinition> {
        let sounds = self
            .sounds
            .into_iter()
            .map(|sound| sound.into_entry(namespace, namespace_assets_dir))
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
    fn into_entry(self, namespace: &str, namespace_assets_dir: &Path) -> Result<SoundEntry> {
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
            ),
            Self::Object(raw) => raw.into_entry(namespace, namespace_assets_dir),
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
    fn into_entry(self, namespace: &str, namespace_assets_dir: &Path) -> Result<SoundEntry> {
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
) -> Result<SoundEntry> {
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
        SoundEntryKind::File => Some(sound_ogg_path(namespace_assets_dir, namespace, &location)),
        SoundEntryKind::Event => None,
    };
    Ok(SoundEntry {
        name,
        kind,
        volume,
        pitch,
        weight,
        stream,
        preload,
        attenuation_distance,
        ogg_path,
    })
}

fn sound_ogg_path(
    namespace_assets_dir: &Path,
    namespace: &str,
    location: &ResourceLocation,
) -> PathBuf {
    let assets_dir = if location.namespace() == namespace {
        namespace_assets_dir.to_path_buf()
    } else {
        namespace_assets_dir
            .parent()
            .unwrap_or(namespace_assets_dir)
            .join(location.namespace())
    };
    assets_dir
        .join("sounds")
        .join(format!("{}.ogg", location.path()))
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
}
