mod random;
pub(crate) mod resolver;

use std::borrow::{Borrow, Cow};
use std::hash::{Hash, Hasher};

use crate::control::AudioCounters;
use anyhow::{Context, Result};
#[cfg(not(target_arch = "wasm32"))]
use bbb_client_assets::materialize_verified_openal_library;
use bbb_client_assets::{JukeboxSongRegistry, PackRoots, SoundCatalog, SoundEventRegistry};
#[cfg(not(target_arch = "wasm32"))]
use bbb_client_audio::NativeAudioRuntime as AudioPlaybackBackend;
#[cfg(target_arch = "wasm32")]
use bbb_client_audio::WebAudioRuntime as AudioPlaybackBackend;
use bbb_client_audio::{
    AudioCategory, AudioChannelObservation, AudioCommand, AudioDeviceObservation,
    AudioListenerState, AudioPlaybackEvent, AudioPreloadObservation, AudioPreloadRequest,
    AudioVolumeSettings, EntitySoundPosition, TickEntitySoundPositionsCommand,
    TickableEntitySoundUpdate,
};
use bbb_client_world::{
    AmbientLoopSoundState, FxHashMap, FxHashSet, JukeboxLevelEventState, LocalSoundEventState,
    SoundEntityEventState, SoundEventState, StopSoundEventState,
};

use resolver::{AudioCommandResolver, AudioResolveError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MusicAudioSnapshot<'a> {
    pub(crate) commands_submitted: u64,
    pub(crate) resolve_failures: u64,
    pub(crate) submit_failures: u64,
    pub(crate) active_music_event_id: Option<Cow<'a, str>>,
    pub(crate) active_music_sound_name: Option<Cow<'a, str>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TickableSoundKey {
    entity_id: i32,
    event_id: String,
}

trait TickableSoundKeyLookup {
    fn key_parts(&self) -> (i32, &str);
}

impl TickableSoundKeyLookup for TickableSoundKey {
    fn key_parts(&self) -> (i32, &str) {
        (self.entity_id, self.event_id.as_str())
    }
}

struct TickableSoundKeyRef<'a>(i32, &'a str);

impl TickableSoundKeyLookup for TickableSoundKeyRef<'_> {
    fn key_parts(&self) -> (i32, &str) {
        (self.0, self.1)
    }
}

impl<'a> Borrow<dyn TickableSoundKeyLookup + 'a> for TickableSoundKey {
    fn borrow(&self) -> &(dyn TickableSoundKeyLookup + 'a) {
        self
    }
}

impl PartialEq for dyn TickableSoundKeyLookup + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.key_parts() == other.key_parts()
    }
}

impl Eq for dyn TickableSoundKeyLookup + '_ {}

impl Hash for TickableSoundKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (entity_id, event_id) = self.key_parts();
        entity_id.hash(state);
        event_id.hash(state);
    }
}

impl Hash for dyn TickableSoundKeyLookup + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (entity_id, event_id) = self.key_parts();
        entity_id.hash(state);
        event_id.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct CachedTickableSound {
    entry_volume: f32,
    entry_pitch: f32,
    category: AudioCategory,
}

pub(crate) trait AudioEventSink {
    fn counters(&self) -> AudioCounters;
    fn music_audio_snapshot(&self) -> MusicAudioSnapshot<'_> {
        let counters = self.counters();
        MusicAudioSnapshot {
            commands_submitted: counters.commands_submitted,
            resolve_failures: counters.resolve_failures,
            submit_failures: counters.submit_failures,
            active_music_event_id: counters.active_music_event_id.map(Cow::Owned),
            active_music_sound_name: counters.active_music_sound_name.map(Cow::Owned),
        }
    }
    fn set_sound_event_registry(&mut self, registry: SoundEventRegistry);
    fn set_jukebox_song_registry(&mut self, registry: JukeboxSongRegistry);
    fn play_local_sound(&mut self, state: &LocalSoundEventState);
    fn play_positioned_sound(&mut self, state: &SoundEventState);
    fn play_entity_sound(&mut self, state: &SoundEntityEventState, position: Option<[f64; 3]>);
    fn play_jukebox_song(&mut self, state: &JukeboxLevelEventState);
    fn stop_jukebox_song(&mut self, state: &JukeboxLevelEventState);
    fn stop_sound(&mut self, state: &StopSoundEventState);
    fn set_music_gain(&mut self, gain: f32);
    fn sync_ambient_loops(&mut self, loops: &[AmbientLoopSoundState]);
    fn stop_all_sounds(&mut self) {
        self.stop_sound(&StopSoundEventState {
            source: None,
            name: None,
        });
    }
    fn stop_music(&mut self) {
        self.stop_sound(&StopSoundEventState {
            source: Some(
                bbb_protocol::packets::SoundSource::Music
                    .as_str()
                    .to_string(),
            ),
            name: None,
        });
    }
    fn tick_entity_sound_positions(
        &mut self,
        listener: Option<AudioListenerState>,
        entity_position: &dyn Fn(i32) -> Option<[f64; 3]>,
        tickable_sounds: &[bbb_client_world::TickableEntitySoundState],
    );
}

pub(crate) struct NativeAudioRuntime {
    catalog: SoundCatalog,
    registry: SoundEventRegistry,
    jukebox_registry: JukeboxSongRegistry,
    volume_settings: AudioVolumeSettings,
    playback: AudioPlaybackBackend,
    counters: AudioCounters,
    // Backend-only playback identity + cached static sounds.json modifiers.
    // Canonical ownership remains in the immutable world generation; this map
    // prevents duplicate starts, avoids 20 Hz sounds.json re-resolution, and
    // notices event-id switches or resource/device reload recovery.
    active_tickable_sounds: FxHashMap<TickableSoundKey, CachedTickableSound>,
    scratch_next_tickable_sounds: FxHashMap<TickableSoundKey, CachedTickableSound>,
    scratch_tick_entity_ids: FxHashSet<i32>,
    scratch_tick_entities: Vec<EntitySoundPosition>,
    scratch_tick_updates: Vec<TickableEntitySoundUpdate>,
    /// Lockstep replay: audible changes as JSON lines, plus the last ambient
    /// loop set so only its changes are logged.
    #[cfg(not(target_arch = "wasm32"))]
    command_log: Option<(std::io::LineWriter<std::fs::File>, Vec<String>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SoundSubtitleEvent {
    pub(crate) subtitle_key: String,
    pub(crate) position: [f64; 3],
    pub(crate) range: f64,
}

pub(crate) struct NativeAudioReload {
    catalog: SoundCatalog,
    preloads: Vec<AudioPreloadRequest>,
}

impl NativeAudioRuntime {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn load(
        roots: &PackRoots,
        preferred_device: &str,
        directional_audio: bool,
        volume_settings: AudioVolumeSettings,
    ) -> Result<Self> {
        let catalog = load_required_native_sound_catalog(roots)?;
        let registry = SoundEventRegistry::vanilla_26_1();
        let jukebox_registry = JukeboxSongRegistry::vanilla_26_1();
        let openal_library = materialize_verified_openal_library(&roots.vanilla_root)
            .context("materialize verified Vanilla OpenAL library")?;
        let mut playback = AudioPlaybackBackend::new_with_device_preference(
            openal_library,
            preferred_device,
            directional_audio,
            catalog.resource_generation(),
        )
        .context("initialize OpenAL audio runtime")?;
        playback.request_preload(audio_preload_requests(&catalog));
        let mut counters = AudioCounters {
            enabled: true,
            catalog_events: catalog.len(),
            registry_entries: registry.len(),
            ..AudioCounters::default()
        };
        if let Some(device) = playback.audio_device_observation() {
            project_audio_device_observation(&mut counters, device);
        }
        Ok(Self {
            catalog,
            registry,
            jukebox_registry,
            volume_settings,
            playback,
            counters,
            active_tickable_sounds: FxHashMap::default(),
            scratch_next_tickable_sounds: FxHashMap::default(),
            scratch_tick_entity_ids: FxHashSet::default(),
            scratch_tick_entities: Vec::new(),
            scratch_tick_updates: Vec::new(),
            command_log: None,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn load_web(
        roots: &PackRoots,
        directional_audio: bool,
        volume_settings: AudioVolumeSettings,
    ) -> Result<Self> {
        let catalog = load_required_native_sound_catalog(roots)?;
        let registry = SoundEventRegistry::vanilla_26_1();
        let jukebox_registry = JukeboxSongRegistry::vanilla_26_1();
        let playback =
            AudioPlaybackBackend::new_web(directional_audio, catalog.resource_generation())
                .context("initialize WebAudio runtime")?;
        let mut counters = AudioCounters {
            enabled: true,
            catalog_events: catalog.len(),
            registry_entries: registry.len(),
            ..AudioCounters::default()
        };
        if let Some(device) = playback.audio_device_observation() {
            project_audio_device_observation(&mut counters, device);
        }
        Ok(Self {
            catalog,
            registry,
            jukebox_registry,
            volume_settings,
            playback,
            counters,
            active_tickable_sounds: FxHashMap::default(),
            scratch_next_tickable_sounds: FxHashMap::default(),
            scratch_tick_entity_ids: FxHashSet::default(),
            scratch_tick_entities: Vec::new(),
            scratch_tick_updates: Vec::new(),
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn resume_web_context(&mut self) {
        self.playback.resume_context();
    }

    pub(crate) fn counters(&self) -> AudioCounters {
        let mut counters = self.counters.clone();
        counters.active_music_event_id = self.playback.active_music_event_id().map(str::to_string);
        counters.active_music_sound_name =
            self.playback.active_music_sound_name().map(str::to_string);
        let observation = self.playback.residency_observation();
        counters.sound_cache_buffers = observation.static_buffers;
        counters.sound_cache_bytes = observation.static_decoded_bytes;
        counters.sound_cache_peak_bytes = observation.peak_static_decoded_bytes;
        counters.sound_cache_hits = observation.static_cache_hits;
        counters.sound_cache_misses = observation.static_cache_misses;
        counters.sound_cache_evictions = observation.static_cache_evictions;
        counters.sound_admission_rejections = observation.audio_admission_rejections;
        counters.sound_streaming_starts = observation.streaming_starts;
        project_audio_preload_observation(&mut counters, self.playback.preload_observation());
        project_audio_channel_observation(&mut counters, self.playback.channel_observation());
        counters
    }

    pub(crate) fn begin_recording(
        &mut self,
        samples: std::sync::mpsc::SyncSender<Vec<i16>>,
    ) -> Result<bbb_client_clock::Instant> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.playback.begin_recording(samples)
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = samples;
            anyhow::bail!("video recording is not supported on wasm32")
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn poll_recording(&mut self) -> Result<()> {
        self.playback.poll_recording()
    }

    pub(crate) fn end_recording(&mut self) -> Result<u64> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.playback.end_recording()
        }
        #[cfg(target_arch = "wasm32")]
        {
            Ok(0)
        }
    }

    /// `SoundEngine.tick` checks device changes before advancing any sound.
    /// A successful library reload stops every backend handle, so the native
    /// duplicate-suppression set must be cleared and rebuilt from canonical
    /// world tickable state on the next audio tick.
    pub(crate) fn poll_audio_device(&mut self) {
        let poll = self.playback.poll_audio_device();
        if poll.observation_changed {
            if let Some(device) = self.playback.audio_device_observation() {
                project_audio_device_observation(&mut self.counters, device);
            }
        }
        if let Some(reason) = poll.reload_reason {
            reset_tickable_sound_identity_after_device_reload(
                &mut self.active_tickable_sounds,
                true,
            );
            // Device reload clears residency and cancels preload; re-issue `"preload": true`
            // requests for the active catalog just like Vanilla `SoundEngine.loadLibrary`.
            self.playback
                .request_preload(audio_preload_requests(&self.catalog));
            tracing::info!(reason = reason.as_str(), "reloaded native audio device");
        }
        if let Some(error) = poll.error {
            tracing::warn!(%error, "audio device poll or reload failed");
        }
        self.playback.poll_audio_preload();
    }

    pub(crate) fn available_sound_ids(&self) -> Vec<String> {
        self.catalog.events.keys().cloned().collect()
    }

    pub(crate) fn drain_subtitle_events(&mut self) -> Vec<SoundSubtitleEvent> {
        self.playback
            .drain_playback_events()
            .filter_map(|event| subtitle_event_from_playback(&self.catalog, event))
            .collect()
    }

    pub(crate) fn prepare_reload(roots: &PackRoots) -> Result<NativeAudioReload> {
        let catalog = load_required_native_sound_catalog(roots)?;
        let preloads = audio_preload_requests(&catalog);
        Ok(NativeAudioReload { catalog, preloads })
    }

    pub(crate) fn commit_reload(&mut self, reload: NativeAudioReload) {
        self.playback
            .reset_resource_generation(reload.catalog.resource_generation());
        self.playback.request_preload(reload.preloads);
        self.counters.catalog_events = reload.catalog.len();
        self.catalog = reload.catalog;
        self.active_tickable_sounds.clear();
    }

    fn handle_resolved_command(
        &mut self,
        command: std::result::Result<AudioCommand, AudioResolveError>,
    ) -> bool {
        match command {
            Ok(command) => self.submit_command(&command),
            Err(err) => {
                self.counters.resolve_failures += 1;
                self.counters.last_resolve_error = Some(err.to_string());
                tracing::warn!(?err, "failed to resolve audio command");
                false
            }
        }
    }

    /// Lockstep replay: appends every command this runtime submits to `path`.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn log_commands_to(&mut self, path: &std::path::Path) -> Result<()> {
        let file = std::fs::File::create(path)
            .with_context(|| format!("create audio command log {}", path.display()))?;
        self.command_log = Some((std::io::LineWriter::new(file), Vec::new()));
        Ok(())
    }

    fn submit_command(&mut self, command: &AudioCommand) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some((log, loops)) = self.command_log.as_mut() {
            if let Some(record) = audible_record(command, loops) {
                use std::io::Write as _;
                let _ = writeln!(log, "{record}");
            }
        }
        match self.playback.handle_command(command) {
            Ok(()) => {
                self.counters.commands_submitted += 1;
                true
            }
            Err(err) => {
                self.counters.submit_failures += 1;
                self.counters.last_submit_error = Some(err.to_string());
                tracing::warn!(?err, "failed to submit audio command");
                false
            }
        }
    }
}

fn subtitle_event_from_playback(
    catalog: &SoundCatalog,
    event: AudioPlaybackEvent,
) -> Option<SoundSubtitleEvent> {
    let subtitle_key = catalog.events.get(&event.event_id)?.subtitle.clone()?;
    Some(SoundSubtitleEvent {
        subtitle_key,
        position: event.position,
        range: event.range,
    })
}

fn project_audio_channel_observation(
    counters: &mut AudioCounters,
    channels: AudioChannelObservation,
) {
    counters.sound_static_channels_used = channels.static_used;
    counters.sound_static_channels_capacity = channels.static_capacity;
    counters.sound_streaming_channels_used = channels.streaming_used;
    counters.sound_streaming_channels_capacity = channels.streaming_capacity;
    counters.sound_channel_admission_rejections = channels.admission_rejections;
    counters.sound_native_static_buffers = channels.native_static_buffers;
    counters.sound_native_static_bytes = channels.native_static_bytes;
    counters.sound_native_static_evictions = channels.native_static_evictions;
    counters.sound_channel_capacity_origin = Some(channels.capacity_origin.to_owned());
}

fn project_audio_device_observation(counters: &mut AudioCounters, device: AudioDeviceObservation) {
    counters.preferred_device = device.preferred_device;
    counters.current_device = device.current_device;
    counters.default_device = device.default_device;
    counters.available_device_count = device.available_device_count;
    counters.device_reloads = device.reloads;
    counters.device_reload_failures = device.reload_failures;
    counters.device_query_failures = device.query_failures;
    counters.preferred_device_fallbacks = device.preferred_fallbacks;
    counters.last_device_reload_reason = device.last_reload_reason;
    counters.last_device_error = device.last_error;
    counters.requested_directional_audio = device.requested_directional_audio;
    counters.openal_hrtf_extension = device.hrtf_extension;
    counters.openal_hrtf_specifiers = device.hrtf_specifiers;
    counters.openal_hrtf_status = device.hrtf_status;
    counters.openal_output_limiter = device.output_limiter;
    counters.openal_supports_disconnection = device.supports_disconnection;
    counters.openal_device_tracking_mode = Some(device.tracking_mode.to_owned());
}

fn project_audio_preload_observation(
    counters: &mut AudioCounters,
    preload: AudioPreloadObservation,
) {
    counters.sound_preload_generation = preload.resource_generation;
    counters.sound_preload_requested = preload.requested;
    counters.sound_preload_completed = preload.completed;
    counters.sound_preload_failures = preload.failures;
    counters.sound_preload_pending = preload.pending;
    counters.sound_preload_worker_jobs = preload.worker_jobs;
    counters.sound_preload_cancelled_jobs = preload.cancelled_jobs;
    counters.sound_preload_stale_results = preload.stale_results;
    counters.last_sound_preload_error = preload.last_error;
}

fn audio_preload_requests(catalog: &SoundCatalog) -> Vec<AudioPreloadRequest> {
    let resource_generation = catalog.resource_generation();
    catalog
        .preload_files()
        .into_iter()
        .map(|preload| AudioPreloadRequest {
            sound_name: preload.sound_name,
            ogg_path: preload.ogg_path,
            resource_generation,
        })
        .collect()
}

fn load_required_native_sound_catalog(roots: &PackRoots) -> Result<SoundCatalog> {
    roots
        .load_required_sound_catalog()
        .context("load native sound catalog")
}

impl AudioEventSink for NativeAudioRuntime {
    fn counters(&self) -> AudioCounters {
        self.counters()
    }

    fn music_audio_snapshot(&self) -> MusicAudioSnapshot<'_> {
        MusicAudioSnapshot {
            commands_submitted: self.counters.commands_submitted,
            resolve_failures: self.counters.resolve_failures,
            submit_failures: self.counters.submit_failures,
            active_music_event_id: self.playback.active_music_event_id().map(Cow::Borrowed),
            active_music_sound_name: self.playback.active_music_sound_name().map(Cow::Borrowed),
        }
    }

    fn set_sound_event_registry(&mut self, registry: SoundEventRegistry) {
        self.counters.registry_entries = registry.len();
        self.counters.registry_updates += 1;
        self.registry = registry;
    }

    fn set_jukebox_song_registry(&mut self, registry: JukeboxSongRegistry) {
        self.jukebox_registry = registry;
    }

    fn play_local_sound(&mut self, state: &LocalSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                self.volume_settings,
            );
            resolver.play_local_sound(state)
        };
        self.handle_resolved_command(command);
    }

    fn play_positioned_sound(&mut self, state: &SoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                self.volume_settings,
            );
            resolver.play_positioned_sound(state)
        };
        self.handle_resolved_command(command);
    }

    fn play_entity_sound(&mut self, state: &SoundEntityEventState, position: Option<[f64; 3]>) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                self.volume_settings,
            );
            resolver.play_entity_sound_at(state, position)
        };
        let tickable_entry = command.as_ref().ok().and_then(tickable_play_command_entry);
        if self.handle_resolved_command(command) {
            if let Some((key, cached)) = tickable_entry {
                self.active_tickable_sounds.insert(key, cached);
            }
        }
    }

    fn play_jukebox_song(&mut self, state: &JukeboxLevelEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                self.volume_settings,
            );
            resolver.play_jukebox_song(state)
        };
        self.handle_resolved_command(command);
    }

    fn stop_jukebox_song(&mut self, state: &JukeboxLevelEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                self.volume_settings,
            );
            resolver.stop_jukebox_song(state)
        };
        self.submit_command(&command);
    }

    fn stop_sound(&mut self, state: &StopSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                self.volume_settings,
            );
            resolver.stop_sound(state)
        };
        self.submit_command(&command);
    }

    fn set_music_gain(&mut self, gain: f32) {
        self.submit_command(&AudioCommand::SetMusicGain(
            bbb_client_audio::SetMusicGainCommand { gain },
        ));
    }

    fn sync_ambient_loops(&mut self, loops: &[AmbientLoopSoundState]) {
        let resolved = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                self.volume_settings,
            );
            loops
                .iter()
                .map(|state| resolver.ambient_loop_sound(state))
                .collect::<Vec<_>>()
        };
        let mut commands = Vec::with_capacity(resolved.len());
        for command in resolved {
            match command {
                Ok(command) => commands.push(command),
                Err(err) => {
                    self.counters.resolve_failures += 1;
                    self.counters.last_resolve_error = Some(err.to_string());
                    tracing::warn!(?err, "failed to resolve ambient loop sound");
                }
            }
        }
        self.submit_command(&AudioCommand::SyncAmbientLoops(
            bbb_client_audio::SyncAmbientLoopsCommand { loops: commands },
        ));
    }

    fn tick_entity_sound_positions(
        &mut self,
        listener: Option<AudioListenerState>,
        entity_position: &dyn Fn(i32) -> Option<[f64; 3]>,
        tickable_sounds: &[bbb_client_world::TickableEntitySoundState],
    ) {
        self.scratch_next_tickable_sounds.clear();
        self.scratch_tick_updates.clear();
        for state in tickable_sounds {
            let event_id: Cow<'_, str> = match state.event.sound.kind.as_str() {
                "direct" => match state.event.sound.location.as_deref() {
                    Some(location) => Cow::Borrowed(location),
                    None => {
                        let err = AudioResolveError::InvalidSoundHolder {
                            kind: state.event.sound.kind.clone(),
                        };
                        self.counters.resolve_failures += 1;
                        self.counters.last_resolve_error = Some(err.to_string());
                        continue;
                    }
                },
                _ => {
                    let resolver = AudioCommandResolver::with_jukebox_registry(
                        &self.catalog,
                        &self.registry,
                        &self.jukebox_registry,
                        self.volume_settings,
                    );
                    match resolver.event_id_str_for_holder(&state.event.sound) {
                        Ok((event_id, _)) => Cow::Owned(event_id.to_string()),
                        Err(err) => {
                            self.counters.resolve_failures += 1;
                            self.counters.last_resolve_error = Some(err.to_string());
                            continue;
                        }
                    }
                }
            };
            let lookup = TickableSoundKeyRef(state.event.entity_id, event_id.as_ref());
            let existing = self
                .active_tickable_sounds
                .remove_entry(&lookup as &dyn TickableSoundKeyLookup);
            let cached = if let Some((key, cached)) = existing {
                self.scratch_next_tickable_sounds
                    .insert(key, cached.clone());
                cached
            } else if let Some(cached) = self
                .scratch_next_tickable_sounds
                .get(&lookup as &dyn TickableSoundKeyLookup)
                .cloned()
            {
                cached
            } else {
                let start_command = {
                    let resolver = AudioCommandResolver::with_jukebox_registry(
                        &self.catalog,
                        &self.registry,
                        &self.jukebox_registry,
                        self.volume_settings,
                    );
                    resolver.play_entity_sound_at(
                        &state.event,
                        Some([state.position.x, state.position.y, state.position.z]),
                    )
                };
                let started_entry = start_command
                    .as_ref()
                    .ok()
                    .and_then(tickable_play_command_entry);
                let submitted = self.handle_resolved_command(start_command);
                let Some((key, cached)) = started_entry else {
                    continue;
                };
                if submitted {
                    self.scratch_next_tickable_sounds
                        .insert(key, cached.clone());
                }
                cached
            };
            let gain = state.event.volume * cached.entry_volume;
            self.scratch_tick_updates.push(TickableEntitySoundUpdate {
                event_id: event_id.into_owned(),
                entity_id: state.event.entity_id,
                position: [state.position.x, state.position.y, state.position.z],
                channel_gain: self.volume_settings.channel_gain(gain, &cached.category),
                playback_rate: state.event.pitch * cached.entry_pitch,
            });
        }
        self.scratch_tick_entity_ids.clear();
        self.scratch_tick_entity_ids
            .extend(self.playback.entity_sound_ids());
        self.scratch_tick_entities.clear();
        for &entity_id in &self.scratch_tick_entity_ids {
            if let Some(position) = entity_position(entity_id) {
                self.scratch_tick_entities.push(EntitySoundPosition {
                    entity_id,
                    position,
                });
            }
        }
        let command = AudioCommand::TickEntitySoundPositions(TickEntitySoundPositionsCommand {
            listener,
            entities: std::mem::take(&mut self.scratch_tick_entities),
            tickable_sounds: std::mem::take(&mut self.scratch_tick_updates),
        });
        let _ = self.submit_command(&command);
        if let AudioCommand::TickEntitySoundPositions(mut tick_cmd) = command {
            tick_cmd.entities.clear();
            tick_cmd.tickable_sounds.clear();
            self.scratch_tick_entities = tick_cmd.entities;
            self.scratch_tick_updates = tick_cmd.tickable_sounds;
        }
        self.active_tickable_sounds.clear();
        std::mem::swap(
            &mut self.active_tickable_sounds,
            &mut self.scratch_next_tickable_sounds,
        );
    }
}

fn tickable_play_command_entry(
    command: &AudioCommand,
) -> Option<(TickableSoundKey, CachedTickableSound)> {
    let AudioCommand::PlayEntitySound(command) = command else {
        return None;
    };
    command.tickable.then(|| {
        (
            TickableSoundKey {
                entity_id: command.entity_id,
                event_id: command.sound.event_id.clone(),
            },
            CachedTickableSound {
                entry_volume: command.sound.entry_volume,
                entry_pitch: command.sound.entry_pitch,
                category: command.category.clone(),
            },
        )
    })
}

#[cfg(test)]
fn tickable_sound_needs_start(
    active: &FxHashMap<TickableSoundKey, CachedTickableSound>,
    entity_id: i32,
    event_id: &str,
) -> bool {
    let lookup = TickableSoundKeyRef(entity_id, event_id);
    !active.contains_key(&lookup as &dyn TickableSoundKeyLookup)
}

fn reset_tickable_sound_identity_after_device_reload(
    active: &mut FxHashMap<TickableSoundKey, CachedTickableSound>,
    reloaded: bool,
) {
    if reloaded {
        active.clear();
    }
}

/// What a listener could hear change: sound starts and stops, music gain and
/// the ambient loop set. Per-tick source moves are left out.
#[cfg(not(target_arch = "wasm32"))]
fn audible_record(command: &AudioCommand, loops: &mut Vec<String>) -> Option<serde_json::Value> {
    fn round(value: f64) -> f64 {
        (value * 1000.0).round() / 1000.0
    }
    fn play(
        sound: &bbb_client_audio::ResolvedSound,
        category: impl std::fmt::Debug,
        gain: f32,
        rate: f32,
        position: Option<[f64; 3]>,
    ) -> serde_json::Value {
        serde_json::json!({
            "op": "play",
            "event": sound.event_id,
            "sound": sound.sound_name,
            "category": format!("{category:?}"),
            "gain": round(f64::from(gain)),
            "rate": round(f64::from(rate)),
            "pos": position.map(|p| p.map(round)),
        })
    }
    Some(match command {
        AudioCommand::PlayLocalSound(c) => {
            play(&c.sound, &c.category, c.channel_gain, c.playback_rate, None)
        }
        AudioCommand::PlayPositionedSound(c) => play(
            &c.sound,
            &c.category,
            c.channel_gain,
            c.playback_rate,
            Some(c.position),
        ),
        AudioCommand::PlayEntitySound(c) => {
            let mut record = play(
                &c.sound,
                &c.category,
                c.channel_gain,
                c.playback_rate,
                c.position,
            );
            record["entity"] = c.entity_id.into();
            record["looping"] = c.looping.into();
            record
        }
        AudioCommand::PlayJukeboxSong(c) => play(
            &c.sound,
            &c.category,
            c.channel_gain,
            c.playback_rate,
            Some(c.position),
        ),
        AudioCommand::StopJukeboxSong(c) => {
            serde_json::json!({"op": "stop_jukebox", "pos": c.jukebox_pos})
        }
        AudioCommand::StopSound(c) => serde_json::json!({
            "op": "stop",
            "category": c.category.as_ref().map(|category| format!("{category:?}")),
            "name": c.name,
        }),
        AudioCommand::SetMusicGain(c) => {
            serde_json::json!({"op": "music_gain", "gain": round(f64::from(c.gain))})
        }
        AudioCommand::SyncAmbientLoops(c) => {
            let current: Vec<String> = c.loops.iter().map(|l| l.sound.sound_name.clone()).collect();
            if current == *loops {
                return None;
            }
            *loops = current.clone();
            serde_json::json!({"op": "loops", "sounds": current})
        }
        AudioCommand::TickEntitySoundPositions(_) => return None,
    })
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn tickable_sound_owner_requires_start_after_event_switch_or_reload() {
        let cached = CachedTickableSound {
            entry_volume: 1.0,
            entry_pitch: 1.0,
            category: AudioCategory::Neutral,
        };
        let mut active = FxHashMap::default();
        active.insert(
            TickableSoundKey {
                entity_id: 7,
                event_id: "minecraft:entity.bee.loop".to_string(),
            },
            cached,
        );

        assert!(!tickable_sound_needs_start(
            &active,
            7,
            "minecraft:entity.bee.loop"
        ));
        assert!(tickable_sound_needs_start(
            &active,
            7,
            "minecraft:entity.bee.loop_aggressive"
        ));
        assert!(tickable_sound_needs_start(
            &FxHashMap::default(),
            7,
            "minecraft:entity.bee.loop"
        ));
    }

    #[test]
    fn device_reload_invalidates_backend_tickable_identity_only_after_success() {
        let key = TickableSoundKey {
            entity_id: 7,
            event_id: "minecraft:entity.bee.loop".to_owned(),
        };
        let mut active = FxHashMap::default();
        active.insert(
            key.clone(),
            CachedTickableSound {
                entry_volume: 1.0,
                entry_pitch: 1.0,
                category: AudioCategory::Neutral,
            },
        );

        reset_tickable_sound_identity_after_device_reload(&mut active, false);
        assert!(active.contains_key(&key));

        reset_tickable_sound_identity_after_device_reload(&mut active, true);
        assert!(active.is_empty());
    }

    #[test]
    fn audio_channel_diagnostics_project_independent_pool_receipt() {
        let mut counters = AudioCounters {
            enabled: true,
            resolve_failures: 3,
            ..AudioCounters::default()
        };

        project_audio_channel_observation(
            &mut counters,
            AudioChannelObservation {
                static_used: 5,
                streaming_used: 2,
                static_capacity: 248,
                streaming_capacity: 8,
                admission_rejections: 4,
                native_static_buffers: 6,
                native_static_bytes: 96_000,
                native_static_evictions: 2,
                capacity_origin: "kira-logical-main-plus-spatial",
            },
        );

        assert_eq!(counters.sound_static_channels_used, 5);
        assert_eq!(counters.sound_streaming_channels_used, 2);
        assert_eq!(counters.sound_static_channels_capacity, 248);
        assert_eq!(counters.sound_streaming_channels_capacity, 8);
        assert_eq!(counters.sound_channel_admission_rejections, 4);
        assert_eq!(counters.sound_native_static_buffers, 6);
        assert_eq!(counters.sound_native_static_bytes, 96_000);
        assert_eq!(counters.sound_native_static_evictions, 2);
        assert_eq!(
            counters.sound_channel_capacity_origin.as_deref(),
            Some("kira-logical-main-plus-spatial")
        );
        assert_eq!(counters.resolve_failures, 3);
    }

    #[test]
    fn device_observation_projects_cached_receipt_fields() {
        let mut counters = AudioCounters::default();
        project_audio_device_observation(
            &mut counters,
            AudioDeviceObservation {
                preferred_device: Some("USB Headset".to_owned()),
                current_device: Some("Built-in Audio".to_owned()),
                default_device: Some("Built-in Audio".to_owned()),
                available_device_count: 1,
                reloads: 2,
                query_failures: 1,
                requested_directional_audio: true,
                hrtf_extension: true,
                hrtf_specifiers: 2,
                hrtf_status: 1,
                output_limiter: true,
                supports_disconnection: true,
                tracking_mode: "openal-soft-system-events",
                ..AudioDeviceObservation::default()
            },
        );

        assert_eq!(counters.preferred_device.as_deref(), Some("USB Headset"));
        assert_eq!(counters.current_device.as_deref(), Some("Built-in Audio"));
        assert_eq!(counters.available_device_count, 1);
        assert_eq!(counters.device_reloads, 2);
        assert_eq!(counters.device_query_failures, 1);
        assert!(counters.requested_directional_audio);
        assert!(counters.openal_hrtf_extension);
        assert_eq!(counters.openal_hrtf_specifiers, 2);
        assert!(counters.openal_output_limiter);
        assert!(counters.openal_supports_disconnection);
        assert_eq!(
            counters.openal_device_tracking_mode.as_deref(),
            Some("openal-soft-system-events")
        );
    }

    #[test]
    fn preload_plan_stamps_catalog_generation_and_receipt_is_machine_readable() {
        let mut catalog = SoundCatalog::default();
        catalog.events.insert(
            "minecraft:test".to_string(),
            bbb_client_assets::SoundEventDefinition {
                id: "minecraft:test".to_string(),
                replace: false,
                subtitle: None,
                sounds: vec![bbb_client_assets::SoundEntry {
                    name: "minecraft:test/preloaded".to_string(),
                    kind: bbb_client_assets::SoundEntryKind::File,
                    volume: 1.0,
                    pitch: 1.0,
                    weight: 1,
                    stream: false,
                    preload: true,
                    attenuation_distance: 16,
                    ogg_path: Some(PathBuf::from("assets/minecraft/sounds/test/preloaded.ogg")),
                    ogg_integrity: None,
                }],
            },
        );

        assert_eq!(
            audio_preload_requests(&catalog),
            vec![AudioPreloadRequest {
                sound_name: "minecraft:test/preloaded".to_string(),
                ogg_path: PathBuf::from("assets/minecraft/sounds/test/preloaded.ogg"),
                resource_generation: 0,
            }]
        );

        let mut counters = AudioCounters::default();
        project_audio_preload_observation(
            &mut counters,
            AudioPreloadObservation {
                resource_generation: 47,
                requested: 3,
                completed: 2,
                failures: 1,
                pending: 0,
                worker_jobs: 4,
                cancelled_jobs: 1,
                stale_results: 0,
                last_error: Some("preload minecraft:test/bad".to_string()),
            },
        );
        assert_eq!(counters.sound_preload_generation, 47);
        assert_eq!(counters.sound_preload_requested, 3);
        assert_eq!(counters.sound_preload_completed, 2);
        assert_eq!(counters.sound_preload_failures, 1);
        assert_eq!(counters.sound_preload_worker_jobs, 4);
        assert_eq!(
            counters.last_sound_preload_error.as_deref(),
            Some("preload minecraft:test/bad")
        );
    }

    #[test]
    fn audio_runtime_maps_playback_root_event_to_current_subtitle_key() {
        let mut catalog = SoundCatalog::default();
        catalog.events.insert(
            "minecraft:block.note_block.harp".to_string(),
            bbb_client_assets::SoundEventDefinition {
                id: "minecraft:block.note_block.harp".to_string(),
                replace: false,
                subtitle: Some("subtitles.block.note_block.note".to_string()),
                sounds: Vec::new(),
            },
        );
        let playback = AudioPlaybackEvent {
            event_id: "minecraft:block.note_block.harp".to_string(),
            position: [1.0, 2.0, 3.0],
            range: 16.0,
        };

        assert_eq!(
            subtitle_event_from_playback(&catalog, playback),
            Some(SoundSubtitleEvent {
                subtitle_key: "subtitles.block.note_block.note".to_string(),
                position: [1.0, 2.0, 3.0],
                range: 16.0,
            })
        );
        assert_eq!(
            subtitle_event_from_playback(
                &catalog,
                AudioPlaybackEvent {
                    event_id: "minecraft:missing".to_string(),
                    position: [0.0; 3],
                    range: f64::INFINITY,
                },
            ),
            None
        );
    }

    #[test]
    fn native_audio_catalog_loader_errors_before_kira_when_catalog_is_empty() {
        let root = unique_temp_dir("native-audio-empty-catalog");
        std::fs::create_dir_all(root.join("sources").join("26.1"))
            .expect("test source root should exist");
        let roots = PackRoots::from_root(&root).unwrap();

        let err = load_required_native_sound_catalog(&roots).unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("load native sound catalog"));
        assert!(message.contains("required sound catalog is empty"));

        std::fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-client-{label}-{nanos}"))
    }
}
