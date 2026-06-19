use bbb_pack::{SoundCatalog, SoundEntry, SoundEntryKind, SoundEventDefinition};
use bbb_world::{
    LocalSoundEventState, SoundEntityEventState, SoundEventState, SoundHolderState,
    StopSoundEventState,
};
use thiserror::Error;

use crate::{
    command::{
        AudioCategory, AudioCommand, AudioVolumeSettings, PlayEntitySoundCommand,
        PlayLocalSoundCommand, PlayPositionedSoundCommand, ResolvedSound, StopSoundCommand,
    },
    random::LegacyRandom,
    SoundEventRegistry,
};

const MAX_SOUND_EVENT_DEPTH: usize = 32;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AudioResolveError {
    #[error("invalid sound holder state {kind:?}")]
    InvalidSoundHolder { kind: String },
    #[error("unknown sound event registry id {registry_id}")]
    UnknownSoundRegistryId { registry_id: i32 },
    #[error("missing sound event {event_id}")]
    MissingSoundEvent { event_id: String },
    #[error("empty sound event {event_id}")]
    EmptySoundEvent { event_id: String },
    #[error("sound event reference cycle at {event_id}")]
    SoundEventReferenceCycle { event_id: String },
    #[error("sound event reference depth exceeded at {event_id}")]
    SoundEventReferenceDepthExceeded { event_id: String },
    #[error("sound entry {sound_name} in event {event_id} has no ogg path")]
    MissingSoundFilePath {
        event_id: String,
        sound_name: String,
    },
}

pub struct AudioCommandResolver<'a> {
    catalog: &'a SoundCatalog,
    registry: &'a SoundEventRegistry,
    volume_settings: AudioVolumeSettings,
}

impl<'a> AudioCommandResolver<'a> {
    pub fn new(catalog: &'a SoundCatalog, registry: &'a SoundEventRegistry) -> Self {
        Self::with_volume_settings(catalog, registry, AudioVolumeSettings::default())
    }

    pub fn with_volume_settings(
        catalog: &'a SoundCatalog,
        registry: &'a SoundEventRegistry,
        volume_settings: AudioVolumeSettings,
    ) -> Self {
        Self {
            catalog,
            registry,
            volume_settings,
        }
    }

    pub fn play_positioned_sound(
        &self,
        state: &SoundEventState,
    ) -> Result<AudioCommand, AudioResolveError> {
        let (event_id, fixed_range) = self.event_id_for_holder(&state.sound)?;
        let sound = self.resolve_event_for_seed(&event_id, state.seed)?;
        let category = AudioCategory::from_world_source(&state.source);
        let gain = state.volume * sound.entry_volume;
        let channel_gain = self.volume_settings.channel_gain(gain, &category);
        Ok(AudioCommand::PlayPositionedSound(
            PlayPositionedSoundCommand {
                gain,
                channel_gain,
                playback_rate: state.pitch * sound.entry_pitch,
                packet_volume: state.volume,
                packet_pitch: state.pitch,
                position: [state.position.x, state.position.y, state.position.z],
                seed: state.seed,
                fixed_range,
                category,
                sound,
            },
        ))
    }

    pub fn play_local_sound(
        &self,
        state: &LocalSoundEventState,
    ) -> Result<AudioCommand, AudioResolveError> {
        let (event_id, _) = self.event_id_for_holder(&state.sound)?;
        let sound = self.resolve_event_for_seed(&event_id, state.seed)?;
        let category = AudioCategory::from_world_source(&state.source);
        let gain = state.volume * sound.entry_volume;
        let channel_gain = self.volume_settings.channel_gain(gain, &category);
        Ok(AudioCommand::PlayLocalSound(PlayLocalSoundCommand {
            gain,
            channel_gain,
            playback_rate: state.pitch * sound.entry_pitch,
            packet_volume: state.volume,
            packet_pitch: state.pitch,
            seed: state.seed,
            category,
            sound,
        }))
    }

    pub fn play_entity_sound(
        &self,
        state: &SoundEntityEventState,
    ) -> Result<AudioCommand, AudioResolveError> {
        self.play_entity_sound_at(state, None)
    }

    pub fn play_entity_sound_at(
        &self,
        state: &SoundEntityEventState,
        position: Option<[f64; 3]>,
    ) -> Result<AudioCommand, AudioResolveError> {
        let (event_id, fixed_range) = self.event_id_for_holder(&state.sound)?;
        let sound = self.resolve_event_for_seed(&event_id, state.seed)?;
        let category = AudioCategory::from_world_source(&state.source);
        let gain = state.volume * sound.entry_volume;
        let channel_gain = self.volume_settings.channel_gain(gain, &category);
        Ok(AudioCommand::PlayEntitySound(PlayEntitySoundCommand {
            gain,
            channel_gain,
            playback_rate: state.pitch * sound.entry_pitch,
            packet_volume: state.volume,
            packet_pitch: state.pitch,
            entity_id: state.entity_id,
            position,
            seed: state.seed,
            fixed_range,
            category,
            sound,
        }))
    }

    pub fn stop_sound(&self, state: &StopSoundEventState) -> AudioCommand {
        AudioCommand::StopSound(StopSoundCommand {
            category: state
                .source
                .as_deref()
                .map(AudioCategory::from_world_source),
            name: state.name.clone(),
        })
    }

    fn event_id_for_holder(
        &self,
        holder: &SoundHolderState,
    ) -> Result<(String, Option<f32>), AudioResolveError> {
        match holder.kind.as_str() {
            "direct" => holder
                .location
                .clone()
                .map(|location| (location, holder.fixed_range))
                .ok_or_else(|| AudioResolveError::InvalidSoundHolder {
                    kind: holder.kind.clone(),
                }),
            "reference" => {
                let registry_id =
                    holder
                        .registry_id
                        .ok_or_else(|| AudioResolveError::InvalidSoundHolder {
                            kind: holder.kind.clone(),
                        })?;
                let event_id = self
                    .registry
                    .event_id(registry_id)
                    .ok_or(AudioResolveError::UnknownSoundRegistryId { registry_id })?;
                Ok((event_id.to_string(), holder.fixed_range))
            }
            other => Err(AudioResolveError::InvalidSoundHolder {
                kind: other.to_string(),
            }),
        }
    }

    fn resolve_event_for_seed(
        &self,
        event_id: &str,
        seed: i64,
    ) -> Result<ResolvedSound, AudioResolveError> {
        let mut random = LegacyRandom::new(seed);
        let mut path = Vec::new();
        self.resolve_event(
            event_id,
            event_id,
            &mut random,
            &mut path,
            SoundModifiers::default(),
        )
    }

    fn resolve_event(
        &self,
        root_event_id: &str,
        event_id: &str,
        random: &mut LegacyRandom,
        path: &mut Vec<String>,
        modifiers: SoundModifiers,
    ) -> Result<ResolvedSound, AudioResolveError> {
        if path.iter().any(|seen| seen == event_id) {
            return Err(AudioResolveError::SoundEventReferenceCycle {
                event_id: event_id.to_string(),
            });
        }
        if path.len() >= MAX_SOUND_EVENT_DEPTH {
            return Err(AudioResolveError::SoundEventReferenceDepthExceeded {
                event_id: event_id.to_string(),
            });
        }
        path.push(event_id.to_string());

        let event =
            self.catalog
                .event(event_id)
                .ok_or_else(|| AudioResolveError::MissingSoundEvent {
                    event_id: event_id.to_string(),
                })?;
        let entry = self.select_weighted_entry(event, random, path)?;
        match entry.kind {
            SoundEntryKind::File => Ok(resolved_file_sound(
                root_event_id,
                event_id,
                entry,
                modifiers,
            )?),
            SoundEntryKind::Event => self.resolve_event(
                root_event_id,
                &entry.name,
                random,
                path,
                modifiers.with_event_entry(entry),
            ),
        }
    }

    fn select_weighted_entry<'b>(
        &self,
        event: &'b SoundEventDefinition,
        random: &mut LegacyRandom,
        path: &mut Vec<String>,
    ) -> Result<&'b SoundEntry, AudioResolveError> {
        let mut total_weight = 0;
        for entry in &event.sounds {
            total_weight += self.entry_weight(entry, path)?;
        }
        if event.sounds.is_empty() || total_weight <= 0 {
            return Err(AudioResolveError::EmptySoundEvent {
                event_id: event.id.clone(),
            });
        }

        let mut selection = random.next_i32(total_weight);
        for entry in &event.sounds {
            selection -= self.entry_weight(entry, path)?;
            if selection < 0 {
                return Ok(entry);
            }
        }

        Err(AudioResolveError::EmptySoundEvent {
            event_id: event.id.clone(),
        })
    }

    fn entry_weight(
        &self,
        entry: &SoundEntry,
        path: &mut Vec<String>,
    ) -> Result<i32, AudioResolveError> {
        match entry.kind {
            SoundEntryKind::File => Ok(entry.weight),
            SoundEntryKind::Event => self.event_total_weight(&entry.name, path),
        }
    }

    fn event_total_weight(
        &self,
        event_id: &str,
        path: &mut Vec<String>,
    ) -> Result<i32, AudioResolveError> {
        if path.iter().any(|seen| seen == event_id) {
            return Err(AudioResolveError::SoundEventReferenceCycle {
                event_id: event_id.to_string(),
            });
        }
        if path.len() >= MAX_SOUND_EVENT_DEPTH {
            return Err(AudioResolveError::SoundEventReferenceDepthExceeded {
                event_id: event_id.to_string(),
            });
        }
        let Some(event) = self.catalog.event(event_id) else {
            return Ok(0);
        };

        path.push(event_id.to_string());
        let mut total = 0;
        for entry in &event.sounds {
            total += self.entry_weight(entry, path)?;
        }
        path.pop();
        Ok(total)
    }
}

#[derive(Debug, Clone, Copy)]
struct SoundModifiers {
    volume: f32,
    pitch: f32,
    stream: bool,
}

impl Default for SoundModifiers {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pitch: 1.0,
            stream: false,
        }
    }
}

impl SoundModifiers {
    fn with_event_entry(self, entry: &SoundEntry) -> Self {
        Self {
            volume: self.volume * entry.volume,
            pitch: self.pitch * entry.pitch,
            stream: self.stream || entry.stream,
        }
    }
}

fn resolved_file_sound(
    root_event_id: &str,
    event_id: &str,
    entry: &SoundEntry,
    modifiers: SoundModifiers,
) -> Result<ResolvedSound, AudioResolveError> {
    let ogg_path =
        entry
            .ogg_path
            .clone()
            .ok_or_else(|| AudioResolveError::MissingSoundFilePath {
                event_id: event_id.to_string(),
                sound_name: entry.name.clone(),
            })?;
    Ok(ResolvedSound {
        event_id: root_event_id.to_string(),
        sound_name: entry.name.clone(),
        ogg_path,
        stream: entry.stream || modifiers.stream,
        preload: entry.preload,
        attenuation_distance: entry.attenuation_distance,
        entry_volume: entry.volume * modifiers.volume,
        entry_pitch: entry.pitch * modifiers.pitch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::Vec3d;
    use std::{
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn resolves_direct_positioned_sound_through_event_reference() {
        let assets_dir = unique_assets_dir("direct-positioned");
        let catalog = test_catalog(
            &assets_dir,
            br#"{
              "wrapper": {
                "sounds": [
                  {
                    "name": "entity.cat.ambient",
                    "type": "event"
                  }
                ]
              },
              "entity.cat.ambient": {
                "sounds": [
                  {
                    "name": "mob/cat/meow1",
                    "volume": 0.6,
                    "pitch": 1.2,
                    "stream": true,
                    "preload": true,
                    "attenuation_distance": 32
                  }
                ]
              }
            }"#,
        );
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::new(&catalog, &registry);

        let command = resolver
            .play_positioned_sound(&SoundEventState {
                sound: SoundHolderState {
                    kind: "direct".to_string(),
                    registry_id: None,
                    location: Some("minecraft:wrapper".to_string()),
                    fixed_range: Some(24.0),
                },
                source: "block".to_string(),
                position: Vec3d {
                    x: 1.0,
                    y: 2.5,
                    z: -3.0,
                },
                volume: 0.5,
                pitch: 2.0,
                seed: 0,
            })
            .unwrap();

        let AudioCommand::PlayPositionedSound(play) = command else {
            panic!("expected positioned sound command");
        };
        assert_eq!(play.category, AudioCategory::Blocks);
        assert_eq!(play.position, [1.0, 2.5, -3.0]);
        assert_eq!(play.sound.event_id, "minecraft:wrapper");
        assert_eq!(play.sound.sound_name, "minecraft:mob/cat/meow1");
        assert_eq!(
            play.sound.ogg_path,
            assets_dir.join("sounds").join("mob/cat/meow1.ogg")
        );
        assert!(play.sound.stream);
        assert!(play.sound.preload);
        assert_eq!(play.sound.attenuation_distance, 32);
        assert_near(play.gain, 0.3);
        assert_near(play.playback_rate, 2.4);
        assert_eq!(play.fixed_range, Some(24.0));
    }

    #[test]
    fn resolves_direct_local_sound_without_spatial_position() {
        let assets_dir = unique_assets_dir("direct-local");
        let catalog = test_catalog(
            &assets_dir,
            br#"{
              "block.portal.travel": {
                "sounds": [
                  {
                    "name": "portal/travel",
                    "volume": 0.8,
                    "pitch": 1.1
                  }
                ]
              }
            }"#,
        );
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::new(&catalog, &registry);

        let command = resolver
            .play_local_sound(&LocalSoundEventState {
                sound: SoundHolderState {
                    kind: "direct".to_string(),
                    registry_id: None,
                    location: Some("minecraft:block.portal.travel".to_string()),
                    fixed_range: None,
                },
                source: "ambient".to_string(),
                volume: 0.25,
                pitch: 1.0,
                seed: 0,
            })
            .unwrap();

        let AudioCommand::PlayLocalSound(play) = command else {
            panic!("expected local sound command");
        };
        assert_eq!(play.category, AudioCategory::Ambient);
        assert_eq!(play.sound.event_id, "minecraft:block.portal.travel");
        assert_eq!(play.sound.sound_name, "minecraft:portal/travel");
        assert_eq!(
            play.sound.ogg_path,
            assets_dir.join("sounds").join("portal/travel.ogg")
        );
        assert_near(play.gain, 0.2);
        assert_near(play.playback_rate, 1.1);
        assert_eq!(play.packet_volume, 0.25);
        assert_eq!(play.packet_pitch, 1.0);
    }

    #[test]
    fn event_reference_modifiers_wrap_resolved_file_sound() {
        let assets_dir = unique_assets_dir("event-reference-modifiers");
        let catalog = test_catalog(
            &assets_dir,
            br#"{
              "wrapper": {
                "sounds": [
                  {
                    "name": "entity.cat.ambient",
                    "type": "event",
                    "volume": 0.5,
                    "pitch": 0.8,
                    "stream": true
                  }
                ]
              },
              "entity.cat.ambient": {
                "sounds": [
                  {
                    "name": "mob/cat/meow1",
                    "volume": 0.6,
                    "pitch": 1.2,
                    "stream": false,
                    "preload": true,
                    "attenuation_distance": 32
                  }
                ]
              }
            }"#,
        );
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::new(&catalog, &registry);

        let command = resolver
            .play_positioned_sound(&SoundEventState {
                sound: SoundHolderState {
                    kind: "direct".to_string(),
                    registry_id: None,
                    location: Some("minecraft:wrapper".to_string()),
                    fixed_range: None,
                },
                source: "ambient".to_string(),
                position: Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                volume: 2.0,
                pitch: 3.0,
                seed: 0,
            })
            .unwrap();

        let AudioCommand::PlayPositionedSound(play) = command else {
            panic!("expected positioned sound command");
        };
        assert_eq!(play.sound.event_id, "minecraft:wrapper");
        assert_eq!(play.sound.sound_name, "minecraft:mob/cat/meow1");
        assert!(play.sound.stream);
        assert!(play.sound.preload);
        assert_eq!(play.sound.attenuation_distance, 32);
        assert_near(play.sound.entry_volume, 0.3);
        assert_near(play.sound.entry_pitch, 0.96);
        assert_near(play.gain, 0.6);
        assert_near(play.playback_rate, 2.88);
    }

    #[test]
    fn category_volume_settings_affect_channel_gain_not_instance_gain() {
        let assets_dir = unique_assets_dir("category-volume-settings");
        let catalog = test_catalog(
            &assets_dir,
            br#"{
              "entity.cat.ambient": {
                "sounds": [
                  {
                    "name": "mob/cat/meow1",
                    "volume": 0.6
                  }
                ]
              }
            }"#,
        );
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::with_volume_settings(
            &catalog,
            &registry,
            AudioVolumeSettings {
                master: 0.5,
                ambient: 0.25,
                ..AudioVolumeSettings::default()
            },
        );

        let command = resolver
            .play_positioned_sound(&SoundEventState {
                sound: SoundHolderState {
                    kind: "direct".to_string(),
                    registry_id: None,
                    location: Some("minecraft:entity.cat.ambient".to_string()),
                    fixed_range: None,
                },
                source: "ambient".to_string(),
                position: Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                volume: 2.0,
                pitch: 1.0,
                seed: 0,
            })
            .unwrap();

        let AudioCommand::PlayPositionedSound(play) = command else {
            panic!("expected positioned sound command");
        };
        assert_near(play.gain, 1.2);
        assert_near(play.channel_gain, 0.125);
    }

    #[test]
    fn event_reference_selection_uses_referenced_event_weight() {
        let assets_dir = unique_assets_dir("event-reference-weight");
        let catalog = test_catalog(
            &assets_dir,
            br#"{
              "wrapper": {
                "sounds": [
                  {
                    "name": "entity.cat.ambient",
                    "type": "event",
                    "weight": 100
                  },
                  {
                    "name": "random/click",
                    "weight": 1
                  }
                ]
              },
              "entity.cat.ambient": {
                "sounds": [
                  {
                    "name": "mob/cat/meow1",
                    "weight": 1
                  }
                ]
              }
            }"#,
        );
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::new(&catalog, &registry);

        let command = resolver
            .play_positioned_sound(&SoundEventState {
                sound: SoundHolderState {
                    kind: "direct".to_string(),
                    registry_id: None,
                    location: Some("minecraft:wrapper".to_string()),
                    fixed_range: None,
                },
                source: "block".to_string(),
                position: Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                volume: 1.0,
                pitch: 1.0,
                seed: 0,
            })
            .unwrap();

        let AudioCommand::PlayPositionedSound(play) = command else {
            panic!("expected positioned sound command");
        };
        assert_eq!(play.sound.sound_name, "minecraft:random/click");
    }

    #[test]
    fn resolves_reference_entity_sound_with_registry_input() {
        let assets_dir = unique_assets_dir("reference-entity");
        let catalog = test_catalog(
            &assets_dir,
            br#"{
              "entity.cat.ambient": {
                "sounds": ["mob/cat/meow1"]
              }
            }"#,
        );
        let registry = SoundEventRegistry::from_ids(["minecraft:entity.cat.ambient"]);
        let resolver = AudioCommandResolver::new(&catalog, &registry);

        let command = resolver
            .play_entity_sound(&SoundEntityEventState {
                sound: SoundHolderState {
                    kind: "reference".to_string(),
                    registry_id: Some(0),
                    location: None,
                    fixed_range: None,
                },
                source: "neutral".to_string(),
                entity_id: 123,
                volume: 1.0,
                pitch: 0.5,
                seed: 12,
            })
            .unwrap();

        let AudioCommand::PlayEntitySound(play) = command else {
            panic!("expected entity sound command");
        };
        assert_eq!(play.category, AudioCategory::Neutral);
        assert_eq!(play.entity_id, 123);
        assert_eq!(play.sound.event_id, "minecraft:entity.cat.ambient");
        assert_eq!(play.sound.sound_name, "minecraft:mob/cat/meow1");
        assert_near(play.gain, 1.0);
        assert_near(play.playback_rate, 0.5);
    }

    #[test]
    fn resolves_reference_sound_with_vanilla_26_1_registry() {
        let assets_dir = unique_assets_dir("vanilla-registry");
        let catalog = test_catalog(
            &assets_dir,
            br#"{
              "ambient.cave": {
                "sounds": ["ambient/cave/cave1"]
              }
            }"#,
        );
        let registry = SoundEventRegistry::vanilla_26_1();
        let resolver = AudioCommandResolver::new(&catalog, &registry);

        let command = resolver
            .play_positioned_sound(&SoundEventState {
                sound: SoundHolderState {
                    kind: "reference".to_string(),
                    registry_id: Some(7),
                    location: None,
                    fixed_range: None,
                },
                source: "ambient".to_string(),
                position: Vec3d {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                },
                volume: 1.0,
                pitch: 1.0,
                seed: 0,
            })
            .unwrap();

        let AudioCommand::PlayPositionedSound(play) = command else {
            panic!("expected positioned sound command");
        };
        assert_eq!(play.category, AudioCategory::Ambient);
        assert_eq!(play.sound.event_id, "minecraft:ambient.cave");
        assert_eq!(play.sound.sound_name, "minecraft:ambient/cave/cave1");
        assert_eq!(
            play.sound.ogg_path,
            assets_dir.join("sounds").join("ambient/cave/cave1.ogg")
        );
    }

    #[test]
    fn rejects_unknown_sound_registry_id() {
        let assets_dir = unique_assets_dir("unknown-registry");
        let catalog = test_catalog(&assets_dir, br#"{}"#);
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::new(&catalog, &registry);

        let err = resolver
            .play_entity_sound(&SoundEntityEventState {
                sound: SoundHolderState {
                    kind: "reference".to_string(),
                    registry_id: Some(99),
                    location: None,
                    fixed_range: None,
                },
                source: "neutral".to_string(),
                entity_id: 123,
                volume: 1.0,
                pitch: 1.0,
                seed: 0,
            })
            .unwrap_err();

        assert_eq!(
            err,
            AudioResolveError::UnknownSoundRegistryId { registry_id: 99 }
        );
    }

    #[test]
    fn represents_stop_sound_commands() {
        let catalog = SoundCatalog::default();
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::new(&catalog, &registry);

        let command = resolver.stop_sound(&StopSoundEventState {
            source: Some("music".to_string()),
            name: Some("minecraft:music.menu".to_string()),
        });

        assert_eq!(
            command,
            AudioCommand::StopSound(StopSoundCommand {
                category: Some(AudioCategory::Music),
                name: Some("minecraft:music.menu".to_string()),
            })
        );
    }

    fn test_catalog(assets_dir: &Path, bytes: &[u8]) -> SoundCatalog {
        SoundCatalog::from_json_bytes("minecraft", assets_dir, bytes).unwrap()
    }

    fn unique_assets_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("bbb-audio-{label}-{nanos}"))
            .join("assets")
            .join("minecraft")
    }

    fn assert_near(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {actual} near {expected}"
        );
    }
}
