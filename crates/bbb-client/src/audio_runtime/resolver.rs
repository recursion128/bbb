use bbb_protocol::environment_audio::SoundEventHolderValue as ClientSoundEventHolder;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use bbb_client_assets::{
    JukeboxSongRegistry, SoundCatalog, SoundEntry, SoundEntryKind, SoundEventDefinition,
    SoundEventRegistry,
};
#[cfg(test)]
use bbb_client_audio::TickableEntitySoundUpdate;
use bbb_client_audio::{
    AmbientLoopSoundCommand, AudioCategory, AudioCommand, AudioVolumeSettings,
    PlayEntitySoundCommand, PlayJukeboxSongCommand, PlayLocalSoundCommand,
    PlayPositionedSoundCommand, ResolvedSound, StopJukeboxSongCommand, StopSoundCommand,
};
use bbb_client_world::{
    AmbientLoopSoundState, JukeboxLevelEventAction, JukeboxLevelEventState, LocalSoundEventState,
    SoundEntityEventState, SoundEventState, SoundHolderState, StopSoundEventState,
};
use thiserror::Error;

use super::random::LegacyRandom;

const MAX_SOUND_EVENT_DEPTH: usize = 32;
#[cfg(test)]
static VANILLA_JUKEBOX_REGISTRY: OnceLock<JukeboxSongRegistry> = OnceLock::new();

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AudioResolveError {
    #[error("invalid sound holder state {kind:?}")]
    InvalidSoundHolder { kind: String },
    #[error("unknown sound event registry id {registry_id}")]
    UnknownSoundRegistryId { registry_id: i32 },
    #[error("unknown jukebox song registry id {registry_id}")]
    UnknownJukeboxSongRegistryId { registry_id: i32 },
    #[error("invalid jukebox level event")]
    InvalidJukeboxLevelEvent,
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
    #[error("sound entry {sound_name} in event {event_id} failed indexed integrity: {reason}")]
    InvalidSoundFile {
        event_id: String,
        sound_name: String,
        reason: String,
    },
}

pub struct AudioCommandResolver<'a> {
    catalog: &'a SoundCatalog,
    registry: &'a SoundEventRegistry,
    jukebox_registry: &'a JukeboxSongRegistry,
    volume_settings: AudioVolumeSettings,
}

impl<'a> AudioCommandResolver<'a> {
    #[cfg(test)]
    pub fn new(catalog: &'a SoundCatalog, registry: &'a SoundEventRegistry) -> Self {
        Self::with_jukebox_registry(
            catalog,
            registry,
            vanilla_jukebox_registry(),
            AudioVolumeSettings::default(),
        )
    }

    #[cfg(test)]
    pub fn with_volume_settings(
        catalog: &'a SoundCatalog,
        registry: &'a SoundEventRegistry,
        volume_settings: AudioVolumeSettings,
    ) -> Self {
        Self::with_jukebox_registry(
            catalog,
            registry,
            vanilla_jukebox_registry(),
            volume_settings,
        )
    }

    pub fn with_jukebox_registry(
        catalog: &'a SoundCatalog,
        registry: &'a SoundEventRegistry,
        jukebox_registry: &'a JukeboxSongRegistry,
        volume_settings: AudioVolumeSettings,
    ) -> Self {
        Self {
            catalog,
            registry,
            jukebox_registry,
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
                distance_delay: state.distance_delay,
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

    pub fn ambient_loop_sound(
        &self,
        state: &AmbientLoopSoundState,
    ) -> Result<AmbientLoopSoundCommand, AudioResolveError> {
        let event_id = match &state.sound {
            ClientSoundEventHolder::Reference { resource_id } => resource_id.clone(),
            ClientSoundEventHolder::Direct { sound_id, .. } => sound_id.clone(),
        };
        let sound = self.resolve_event_for_seed(&event_id, state.seed)?;
        let gain = if state.gain.is_finite() {
            state.gain.clamp(0.0, 1.0)
        } else {
            0.0
        } * sound.entry_volume;
        Ok(AmbientLoopSoundCommand {
            channel_gain: self
                .volume_settings
                .channel_gain(gain, &AudioCategory::Ambient),
            sound,
        })
    }

    #[cfg(test)]
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
            looping: state.looping,
            linear_attenuation: state.linear_attenuation,
            tickable: state.tickable,
        }))
    }

    #[cfg(test)]
    pub fn tickable_entity_sound_update(
        &self,
        state: &bbb_client_world::TickableEntitySoundState,
    ) -> Result<TickableEntitySoundUpdate, AudioResolveError> {
        let (event_id, _) = self.event_id_for_holder(&state.event.sound)?;
        let sound = self.select_event_for_seed(&event_id, state.event.seed)?;
        let category = AudioCategory::from_world_source(&state.event.source);
        let gain = state.event.volume * sound.modifiers.volume;
        Ok(TickableEntitySoundUpdate {
            event_id,
            entity_id: state.event.entity_id,
            position: [state.position.x, state.position.y, state.position.z],
            channel_gain: self.volume_settings.channel_gain(gain, &category),
            playback_rate: state.event.pitch * sound.modifiers.pitch,
        })
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

    pub fn play_jukebox_song(
        &self,
        state: &JukeboxLevelEventState,
    ) -> Result<AudioCommand, AudioResolveError> {
        if state.action != JukeboxLevelEventAction::Start {
            return Err(AudioResolveError::InvalidJukeboxLevelEvent);
        }
        let song_registry_id = state
            .song_registry_id
            .ok_or(AudioResolveError::InvalidJukeboxLevelEvent)?;
        let event_id = self
            .jukebox_registry
            .sound_event_id(song_registry_id)
            .ok_or(AudioResolveError::UnknownJukeboxSongRegistryId {
                registry_id: song_registry_id,
            })?;
        let fixed_range = self
            .jukebox_registry
            .sound_event_fixed_range(song_registry_id);
        let seed = 0;
        let sound = self.resolve_event_for_seed(event_id, seed)?;
        let category = AudioCategory::Records;
        let packet_volume = 4.0;
        let packet_pitch = 1.0;
        let gain = packet_volume * sound.entry_volume;
        let channel_gain = self.volume_settings.channel_gain(gain, &category);
        Ok(AudioCommand::PlayJukeboxSong(PlayJukeboxSongCommand {
            gain,
            channel_gain,
            playback_rate: packet_pitch * sound.entry_pitch,
            packet_volume,
            packet_pitch,
            seed,
            category,
            fixed_range,
            position: [
                f64::from(state.pos.x) + 0.5,
                f64::from(state.pos.y) + 0.5,
                f64::from(state.pos.z) + 0.5,
            ],
            jukebox_pos: [state.pos.x, state.pos.y, state.pos.z],
            sound,
        }))
    }

    pub fn stop_jukebox_song(&self, state: &JukeboxLevelEventState) -> AudioCommand {
        AudioCommand::StopJukeboxSong(StopJukeboxSongCommand {
            jukebox_pos: [state.pos.x, state.pos.y, state.pos.z],
        })
    }

    pub(crate) fn event_id_str_for_holder<'b>(
        &'b self,
        holder: &'b SoundHolderState,
    ) -> Result<(&'b str, Option<f32>), AudioResolveError> {
        match holder.kind.as_str() {
            "direct" => holder
                .location
                .as_deref()
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
                Ok((event_id, holder.fixed_range))
            }
            other => Err(AudioResolveError::InvalidSoundHolder {
                kind: other.to_string(),
            }),
        }
    }

    fn event_id_for_holder(
        &self,
        holder: &SoundHolderState,
    ) -> Result<(String, Option<f32>), AudioResolveError> {
        let (event_id, fixed_range) = self.event_id_str_for_holder(holder)?;
        Ok((event_id.to_string(), fixed_range))
    }

    fn resolve_event_for_seed(
        &self,
        event_id: &str,
        seed: i64,
    ) -> Result<ResolvedSound, AudioResolveError> {
        let sound = self.select_event_for_seed(event_id, seed)?;
        resolved_file_sound(event_id, sound, self.catalog.resource_generation())
    }

    fn select_event_for_seed(
        &self,
        event_id: &str,
        seed: i64,
    ) -> Result<SelectedSound<'_>, AudioResolveError> {
        let mut random = LegacyRandom::new(seed);
        let mut path = Vec::new();
        self.select_event(event_id, &mut random, &mut path, SoundModifiers::default())
    }

    fn select_event(
        &self,
        event_id: &str,
        random: &mut LegacyRandom,
        path: &mut Vec<String>,
        modifiers: SoundModifiers,
    ) -> Result<SelectedSound<'_>, AudioResolveError> {
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
            SoundEntryKind::File => Ok(SelectedSound {
                event_id: &event.id,
                entry,
                modifiers: modifiers.with_entry(entry),
            }),
            SoundEntryKind::Event => {
                self.select_event(&entry.name, random, path, modifiers.with_entry(entry))
            }
        }
    }

    fn select_weighted_entry<'b>(
        &'b self,
        event: &'b SoundEventDefinition,
        random: &mut LegacyRandom,
        path: &mut Vec<String>,
    ) -> Result<&'b SoundEntry, AudioResolveError> {
        let mut weight_memo = bbb_client_world::FxHashMap::<&str, (u64, usize)>::default();
        let mut weights = Vec::with_capacity(event.sounds.len());
        let mut total_weight = 0_u64;
        for entry in &event.sounds {
            let weight = self.entry_weight(entry, path, &mut weight_memo)?;
            weights.push(weight);
            total_weight = total_weight.saturating_add(weight);
        }
        if event.sounds.is_empty() || total_weight == 0 {
            return Err(AudioResolveError::EmptySoundEvent {
                event_id: event.id.clone(),
            });
        }

        let bound = i32::try_from(total_weight).unwrap_or(i32::MAX);
        let mut selection = u64::try_from(random.next_i32(bound)).unwrap_or(0);
        for (entry, weight) in event.sounds.iter().zip(weights) {
            if selection < weight {
                return Ok(entry);
            }
            selection -= weight;
        }

        Err(AudioResolveError::EmptySoundEvent {
            event_id: event.id.clone(),
        })
    }

    fn entry_weight<'b>(
        &'b self,
        entry: &'b SoundEntry,
        path: &mut Vec<String>,
        memo: &mut bbb_client_world::FxHashMap<&'b str, (u64, usize)>,
    ) -> Result<u64, AudioResolveError> {
        match entry.kind {
            SoundEntryKind::File => Ok(u64::try_from(entry.weight).unwrap_or(0)),
            SoundEntryKind::Event => self
                .event_total_weight(&entry.name, path, memo)
                .map(|(weight, _)| weight),
        }
    }

    fn event_total_weight<'b>(
        &'b self,
        event_id: &'b str,
        path: &mut Vec<String>,
        memo: &mut bbb_client_world::FxHashMap<&'b str, (u64, usize)>,
    ) -> Result<(u64, usize), AudioResolveError> {
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
        if let Some(&(weight, subtree_depth)) = memo.get(event_id) {
            if path.len().saturating_add(subtree_depth) >= MAX_SOUND_EVENT_DEPTH {
                return Err(AudioResolveError::SoundEventReferenceDepthExceeded {
                    event_id: event_id.to_string(),
                });
            }
            return Ok((weight, subtree_depth));
        }
        let Some(event) = self.catalog.event(event_id) else {
            return Ok((0, 0));
        };

        path.push(event_id.to_string());
        let mut total = 0_u64;
        let mut max_child_depth = 0_usize;
        for entry in &event.sounds {
            let (weight, child_depth) = match entry.kind {
                SoundEntryKind::File => (u64::try_from(entry.weight).unwrap_or(0), 0),
                SoundEntryKind::Event => {
                    let (w, d) = self.event_total_weight(&entry.name, path, memo)?;
                    (w, d.saturating_add(1))
                }
            };
            total = total.saturating_add(weight);
            max_child_depth = max_child_depth.max(child_depth);
        }
        path.pop();
        memo.insert(&event.id, (total, max_child_depth));
        Ok((total, max_child_depth))
    }
}

#[cfg(test)]
fn vanilla_jukebox_registry() -> &'static JukeboxSongRegistry {
    VANILLA_JUKEBOX_REGISTRY.get_or_init(JukeboxSongRegistry::vanilla_26_1)
}

const MAX_VERIFIED_SOUND_CACHE_ENTRIES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
struct VerifiedSoundKey {
    path: PathBuf,
    sha1: String,
    size: u64,
}

trait VerifiedSoundKeyLookup {
    fn key_parts(&self) -> (&std::path::Path, &str, u64);
}

impl VerifiedSoundKeyLookup for VerifiedSoundKey {
    fn key_parts(&self) -> (&std::path::Path, &str, u64) {
        (self.path.as_path(), self.sha1.as_str(), self.size)
    }
}

struct VerifiedSoundKeyRef<'a>(&'a std::path::Path, &'a str, u64);

impl VerifiedSoundKeyLookup for VerifiedSoundKeyRef<'_> {
    fn key_parts(&self) -> (&std::path::Path, &str, u64) {
        (self.0, self.1, self.2)
    }
}

impl<'a> std::borrow::Borrow<dyn VerifiedSoundKeyLookup + 'a> for VerifiedSoundKey {
    fn borrow(&self) -> &(dyn VerifiedSoundKeyLookup + 'a) {
        self
    }
}

impl PartialEq for dyn VerifiedSoundKeyLookup + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.key_parts() == other.key_parts()
    }
}

impl Eq for dyn VerifiedSoundKeyLookup + '_ {}

impl std::hash::Hash for VerifiedSoundKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let (path, sha1, size) = self.key_parts();
        path.hash(state);
        sha1.hash(state);
        size.hash(state);
    }
}

impl std::hash::Hash for dyn VerifiedSoundKeyLookup + '_ {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let (path, sha1, size) = self.key_parts();
        path.hash(state);
        sha1.hash(state);
        size.hash(state);
    }
}

struct VerifiedSoundCache {
    generation: u64,
    verified: bbb_client_world::FxHashSet<VerifiedSoundKey>,
}

fn verified_sound_cache() -> &'static RwLock<VerifiedSoundCache> {
    static CACHE: OnceLock<RwLock<VerifiedSoundCache>> = OnceLock::new();
    CACHE.get_or_init(|| {
        RwLock::new(VerifiedSoundCache {
            generation: 0,
            verified: bbb_client_world::FxHashSet::default(),
        })
    })
}

fn is_sound_integrity_verified(
    resource_generation: u64,
    ogg_path: &std::path::Path,
    integrity: &bbb_client_assets::ResourceIntegrity,
) -> bool {
    let Ok(cache) = verified_sound_cache().read() else {
        return false;
    };
    if cache.generation != resource_generation {
        return false;
    }
    let lookup = VerifiedSoundKeyRef(ogg_path, integrity.sha1.as_str(), integrity.size);
    cache
        .verified
        .contains(&lookup as &dyn VerifiedSoundKeyLookup)
}

fn mark_sound_integrity_verified(
    resource_generation: u64,
    ogg_path: &std::path::Path,
    integrity: &bbb_client_assets::ResourceIntegrity,
) {
    let Ok(mut cache) = verified_sound_cache().write() else {
        return;
    };
    if cache.generation != resource_generation
        || cache.verified.len() >= MAX_VERIFIED_SOUND_CACHE_ENTRIES
    {
        cache.generation = resource_generation;
        cache.verified.clear();
    }
    cache.verified.insert(VerifiedSoundKey {
        path: ogg_path.to_path_buf(),
        sha1: integrity.sha1.clone(),
        size: integrity.size,
    });
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
    fn with_entry(self, entry: &SoundEntry) -> Self {
        Self {
            volume: self.volume * entry.volume,
            pitch: self.pitch * entry.pitch,
            stream: self.stream || entry.stream,
        }
    }
}

struct SelectedSound<'a> {
    event_id: &'a str,
    entry: &'a SoundEntry,
    modifiers: SoundModifiers,
}

fn resolved_file_sound(
    root_event_id: &str,
    sound: SelectedSound<'_>,
    resource_generation: u64,
) -> Result<ResolvedSound, AudioResolveError> {
    let SelectedSound {
        event_id,
        entry,
        modifiers,
    } = sound;
    let ogg_path =
        entry
            .ogg_path
            .clone()
            .ok_or_else(|| AudioResolveError::MissingSoundFilePath {
                event_id: event_id.to_string(),
                sound_name: entry.name.clone(),
            })?;
    if let Some(integrity) = &entry.ogg_integrity {
        #[cfg(target_arch = "wasm32")]
        let should_verify_now = !ogg_path
            .to_str()
            .is_some_and(|s| s.starts_with("/__bbb_mem__/") && s.contains("/objects/"))
            || bbb_client_assets::open_resource_file(&ogg_path).is_ok();
        #[cfg(not(target_arch = "wasm32"))]
        let should_verify_now = true;
        if should_verify_now
            && !is_sound_integrity_verified(resource_generation, &ogg_path, integrity)
        {
            integrity
                .verify(&ogg_path, "indexed sound object")
                .map_err(|error| AudioResolveError::InvalidSoundFile {
                    event_id: event_id.to_string(),
                    sound_name: entry.name.clone(),
                    reason: format!("{error:#}"),
                })?;
            mark_sound_integrity_verified(resource_generation, &ogg_path, integrity);
        }
    }
    Ok(ResolvedSound {
        event_id: root_event_id.to_string(),
        sound_name: entry.name.clone(),
        ogg_path,
        resource_generation,
        stream: modifiers.stream,
        preload: entry.preload,
        attenuation_distance: entry.attenuation_distance,
        entry_volume: modifiers.volume,
        entry_pitch: modifiers.pitch,
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
                distance_delay: true,
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
        assert!(play.distance_delay);
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
    fn resolves_environment_loop_holder_with_fade_gain_and_stable_seed() {
        let assets_dir = unique_assets_dir("ambient-loop");
        let catalog = test_catalog(
            &assets_dir,
            br#"{
              "ambient.loop": {
                "sounds": [
                  {
                    "name": "ambient/loop",
                    "volume": 0.8
                  }
                ]
              }
            }"#,
        );
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::new(&catalog, &registry);

        let command = resolver
            .ambient_loop_sound(&AmbientLoopSoundState {
                sound: ClientSoundEventHolder::Reference {
                    resource_id: "minecraft:ambient.loop".to_string(),
                },
                gain: 0.25,
                seed: 19,
            })
            .unwrap();

        assert_eq!(command.sound.event_id, "minecraft:ambient.loop");
        assert_eq!(command.sound.sound_name, "minecraft:ambient/loop");
        assert_near(command.channel_gain, 0.2);
    }

    #[test]
    fn resolves_environment_animate_audio_commands() {
        let assets_dir = unique_assets_dir("underwater-ambient");
        let catalog = test_catalog(
            &assets_dir,
            br#"{
              "ambient.underwater.enter": {"sounds": ["ambient/underwater/enter"]},
              "ambient.underwater.exit": {"sounds": ["ambient/underwater/exit"]},
              "ambient.underwater.loop": {"sounds": ["ambient/underwater/loop"]},
              "ambient.underwater.loop.additions": {"sounds": ["ambient/underwater/addition"]},
              "ambient.underwater.loop.additions.rare": {"sounds": ["ambient/underwater/addition_rare"]},
              "ambient.underwater.loop.additions.ultra_rare": {"sounds": ["ambient/underwater/addition_ultra_rare"]},
              "block.bubble_column.upwards_inside": {"sounds": ["block/bubble_column/upwards_inside"]},
              "block.bubble_column.whirlpool_inside": {"sounds": ["block/bubble_column/whirlpool_inside"]},
              "block.bubble_column.upwards_ambient": {"sounds": ["block/bubble_column/upwards_ambient"]},
              "block.bubble_column.whirlpool_ambient": {"sounds": ["block/bubble_column/whirlpool_ambient"]},
              "block.water.ambient": {"sounds": ["block/water/ambient"]},
              "block.lava.pop": {"sounds": ["block/lava/pop"]},
              "block.lava.ambient": {"sounds": ["block/lava/ambient"]},
              "block.furnace.fire_crackle": {"sounds": ["block/furnace/fire_crackle"]},
              "block.blastfurnace.fire_crackle": {"sounds": ["block/blastfurnace/fire_crackle"]},
              "block.smoker.smoke": {"sounds": ["block/smoker/smoke"]},
              "block.campfire.crackle": {"sounds": ["block/campfire/crackle"]},
              "block.fire.ambient": {"sounds": ["block/fire/ambient"]},
              "block.respawn_anchor.ambient": {"sounds": ["block/respawn_anchor/ambient"]},
              "block.portal.ambient": {"sounds": ["block/portal/ambient"]},
              "block.dried_ghast.ambient": {"sounds": ["block/dried_ghast/ambient"]},
              "block.dried_ghast.ambient_water": {"sounds": ["block/dried_ghast/ambient_water"]}
            }"#,
        );
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::new(&catalog, &registry);
        let direct_holder = |sound_id: &str| SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(sound_id.to_string()),
            fixed_range: None,
        };

        for sound_id in [
            "minecraft:ambient.underwater.enter",
            "minecraft:ambient.underwater.exit",
        ] {
            let command = resolver
                .play_positioned_sound(&SoundEventState {
                    sound: direct_holder(sound_id),
                    source: "ambient".to_string(),
                    position: Vec3d::default(),
                    volume: 1.0,
                    pitch: 1.0,
                    seed: 1,
                    distance_delay: false,
                })
                .unwrap();
            let AudioCommand::PlayPositionedSound(command) = command else {
                panic!("expected positioned underwater transition");
            };
            assert_eq!(command.category, AudioCategory::Ambient);
            assert_eq!(command.sound.event_id, sound_id);
        }

        for sound_id in [
            "minecraft:ambient.underwater.loop.additions",
            "minecraft:ambient.underwater.loop.additions.rare",
            "minecraft:ambient.underwater.loop.additions.ultra_rare",
        ] {
            let command = resolver
                .play_local_sound(&LocalSoundEventState {
                    sound: direct_holder(sound_id),
                    source: "ambient".to_string(),
                    volume: 1.0,
                    pitch: 1.0,
                    seed: 2,
                })
                .unwrap();
            let AudioCommand::PlayLocalSound(command) = command else {
                panic!("expected local underwater addition");
            };
            assert_eq!(command.category, AudioCategory::Ambient);
            assert_eq!(command.sound.event_id, sound_id);
        }

        let command = resolver
            .ambient_loop_sound(&AmbientLoopSoundState {
                sound: ClientSoundEventHolder::Reference {
                    resource_id: "minecraft:ambient.underwater.loop".to_string(),
                },
                gain: 0.5,
                seed: 3,
            })
            .unwrap();
        assert_eq!(command.sound.event_id, "minecraft:ambient.underwater.loop");
        assert_near(command.channel_gain, 0.5);

        for (sound_id, source, category) in [
            (
                "minecraft:block.bubble_column.upwards_inside",
                "player",
                AudioCategory::Players,
            ),
            (
                "minecraft:block.bubble_column.whirlpool_inside",
                "player",
                AudioCategory::Players,
            ),
            (
                "minecraft:block.bubble_column.upwards_ambient",
                "block",
                AudioCategory::Blocks,
            ),
            (
                "minecraft:block.bubble_column.whirlpool_ambient",
                "block",
                AudioCategory::Blocks,
            ),
        ] {
            let command = resolver
                .play_positioned_sound(&SoundEventState {
                    sound: direct_holder(sound_id),
                    source: source.to_string(),
                    position: Vec3d::default(),
                    volume: 0.3,
                    pitch: 0.95,
                    seed: 4,
                    distance_delay: false,
                })
                .unwrap();
            let AudioCommand::PlayPositionedSound(command) = command else {
                panic!("expected positioned bubble-column sound");
            };
            assert_eq!(command.category, category);
            assert_eq!(command.sound.event_id, sound_id);
        }

        for sound_id in [
            "minecraft:block.water.ambient",
            "minecraft:block.lava.pop",
            "minecraft:block.lava.ambient",
        ] {
            let command = resolver
                .play_positioned_sound(&SoundEventState {
                    sound: direct_holder(sound_id),
                    source: "ambient".to_string(),
                    position: Vec3d::default(),
                    volume: 0.3,
                    pitch: 0.95,
                    seed: 5,
                    distance_delay: false,
                })
                .unwrap();
            let AudioCommand::PlayPositionedSound(command) = command else {
                panic!("expected positioned fluid sound");
            };
            assert_eq!(command.category, AudioCategory::Ambient);
            assert_eq!(command.sound.event_id, sound_id);
        }

        for sound_id in [
            "minecraft:block.furnace.fire_crackle",
            "minecraft:block.blastfurnace.fire_crackle",
            "minecraft:block.smoker.smoke",
            "minecraft:block.campfire.crackle",
            "minecraft:block.fire.ambient",
            "minecraft:block.respawn_anchor.ambient",
            "minecraft:block.portal.ambient",
            "minecraft:block.dried_ghast.ambient",
            "minecraft:block.dried_ghast.ambient_water",
        ] {
            let command = resolver
                .play_positioned_sound(&SoundEventState {
                    sound: direct_holder(sound_id),
                    source: "block".to_string(),
                    position: Vec3d::default(),
                    volume: 0.3,
                    pitch: 0.95,
                    seed: 5,
                    distance_delay: false,
                })
                .unwrap();
            let AudioCommand::PlayPositionedSound(command) = command else {
                panic!("expected positioned block sound");
            };
            assert_eq!(command.category, AudioCategory::Blocks);
            assert_eq!(command.sound.event_id, sound_id);
        }
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
                distance_delay: false,
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
                distance_delay: false,
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
                distance_delay: false,
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
                looping: false,
                linear_attenuation: true,
                tickable: false,
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
    fn resolves_tickable_entity_sound_updates_with_current_gain_pitch_and_position() {
        let assets_dir = unique_assets_dir("tickable-entity-update");
        let mut catalog = test_catalog(
            &assets_dir,
            br#"{
              "entity.guardian.attack": {
                "sounds": [{"name": "mob/guardian/attack_loop", "volume": 0.8, "pitch": 1.1}]
              }
            }"#,
        );
        // An already playing source can still be updated when its Ogg is no longer on disk.
        let guardian = catalog
            .events
            .get_mut("minecraft:entity.guardian.attack")
            .unwrap();
        guardian.sounds[0].ogg_integrity = Some(bbb_client_assets::ResourceIntegrity {
            size: 4,
            sha1: "0".repeat(40),
        });
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::new(&catalog, &registry);
        let update = resolver
            .tickable_entity_sound_update(&bbb_client_world::TickableEntitySoundState {
                event: SoundEntityEventState {
                    sound: SoundHolderState {
                        kind: "direct".to_string(),
                        registry_id: None,
                        location: Some("minecraft:entity.guardian.attack".to_string()),
                        fixed_range: None,
                    },
                    source: "hostile".to_string(),
                    entity_id: 42,
                    volume: 0.25,
                    pitch: 1.2,
                    seed: 7,
                    looping: true,
                    linear_attenuation: false,
                    tickable: true,
                },
                position: Vec3d {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0,
                },
            })
            .unwrap();

        assert_eq!(update.event_id, "minecraft:entity.guardian.attack");
        assert_eq!(update.entity_id, 42);
        assert_eq!(update.position, [4.0, 5.0, 6.0]);
        assert_near(update.channel_gain, 0.2);
        assert_near(update.playback_rate, 1.32);
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
                distance_delay: false,
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
    fn resolves_jukebox_song_as_record_positioned_sound() {
        let assets_dir = unique_assets_dir("jukebox-song");
        let catalog = test_catalog(
            &assets_dir,
            br#"{
              "music_disc.cat": {
                "sounds": [
                  {
                    "name": "records/cat",
                    "stream": true,
                    "attenuation_distance": 16
                  }
                ]
              }
            }"#,
        );
        let registry = SoundEventRegistry::default();
        let jukebox_registry =
            JukeboxSongRegistry::from_sound_event_ids([Some("minecraft:music_disc.cat")]);
        let resolver = AudioCommandResolver::with_jukebox_registry(
            &catalog,
            &registry,
            &jukebox_registry,
            AudioVolumeSettings::default(),
        );

        let command = resolver
            .play_jukebox_song(&JukeboxLevelEventState {
                action: JukeboxLevelEventAction::Start,
                pos: bbb_client_world::BlockPos { x: 4, y: 64, z: -7 },
                song_registry_id: Some(0),
                stopped_existing: false,
            })
            .unwrap();

        let AudioCommand::PlayJukeboxSong(play) = command else {
            panic!("expected jukebox song command");
        };
        assert_eq!(play.category, AudioCategory::Records);
        assert_eq!(play.position, [4.5, 64.5, -6.5]);
        assert_eq!(play.jukebox_pos, [4, 64, -7]);
        assert_eq!(play.sound.event_id, "minecraft:music_disc.cat");
        assert_eq!(play.sound.sound_name, "minecraft:records/cat");
        assert!(play.sound.stream);
        assert_eq!(play.packet_volume, 4.0);
        assert_eq!(play.packet_pitch, 1.0);
        assert_eq!(play.fixed_range, None);
        assert_near(play.gain, 4.0);
        assert_near(play.playback_rate, 1.0);
    }

    #[test]
    fn rejects_unknown_jukebox_song_registry_id() {
        let assets_dir = unique_assets_dir("unknown-jukebox-song");
        let catalog = test_catalog(&assets_dir, br#"{}"#);
        let registry = SoundEventRegistry::default();
        let jukebox_registry = JukeboxSongRegistry::default();
        let resolver = AudioCommandResolver::with_jukebox_registry(
            &catalog,
            &registry,
            &jukebox_registry,
            AudioVolumeSettings::default(),
        );

        let err = resolver
            .play_jukebox_song(&JukeboxLevelEventState {
                action: JukeboxLevelEventAction::Start,
                pos: bbb_client_world::BlockPos { x: 0, y: 0, z: 0 },
                song_registry_id: Some(42),
                stopped_existing: false,
            })
            .unwrap_err();

        assert_eq!(
            err,
            AudioResolveError::UnknownJukeboxSongRegistryId { registry_id: 42 }
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
                looping: false,
                linear_attenuation: true,
                tickable: false,
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

    #[test]
    #[ignore = "requires the local Vanilla 26.1 asset object store"]
    fn real_indexed_sound_resolves_and_opens_through_the_bounded_decoder() {
        let roots = bbb_client_assets::PackRoots::from_environment().unwrap();
        let catalog = roots.load_required_sound_catalog().unwrap();
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::new(&catalog, &registry);

        let sound = resolver
            .resolve_event_for_seed("minecraft:ui.button.click", 0)
            .unwrap();
        assert_eq!(sound.sound_name, "minecraft:random/click_stereo");
        assert_eq!(sound.resource_generation, 0);
        let probe = bbb_client_audio::probe_bounded_audio_file(&sound.ogg_path).unwrap();
        assert!(probe.sample_rate > 0);
        assert!(probe.frames > 0);
    }

    #[test]
    fn indexed_sound_integrity_is_checked_only_when_the_entry_is_resolved() {
        let assets = unique_assets_dir("indexed-integrity");
        let ogg = assets.join("sounds/random/test.ogg");
        std::fs::create_dir_all(ogg.parent().unwrap()).unwrap();
        std::fs::write(&ogg, b"bad").unwrap();
        let mut catalog = test_catalog(
            &assets,
            br#"{"test":{"sounds":["random/test"]},"unused":{"sounds":[]}}"#,
        );
        catalog.events.get_mut("minecraft:test").unwrap().sounds[0].ogg_integrity =
            Some(bbb_client_assets::ResourceIntegrity {
                size: 4,
                sha1: "0".repeat(40),
            });
        let registry = SoundEventRegistry::default();
        let resolver = AudioCommandResolver::new(&catalog, &registry);

        let error = resolver
            .resolve_event_for_seed("minecraft:test", 0)
            .unwrap_err();
        assert!(matches!(error, AudioResolveError::InvalidSoundFile { .. }));
        assert!(resolver
            .resolve_event_for_seed("minecraft:unused", 0)
            .is_err());
        std::fs::remove_dir_all(assets.parent().unwrap().parent().unwrap()).unwrap();
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
            .join(format!("bbb-client-audio-{label}-{nanos}"))
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
