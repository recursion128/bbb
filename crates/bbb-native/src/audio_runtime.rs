mod effects;
mod random;
pub(crate) mod resolver;

pub(crate) use effects::AudioEffect;

use std::collections::BTreeSet;

use anyhow::{Context, Result};
use bbb_audio::{
    AudioChannelObservation, AudioCommand, AudioDeviceObservation, AudioPlaybackEvent,
    AudioPreloadObservation, AudioPreloadRequest, AudioVolumeSettings, KiraAudioRuntime,
    TickEntitySoundPositionsCommand,
};
use bbb_control::AudioCounters;
use bbb_pack::{
    materialize_verified_openal_library, JukeboxSongRegistry, PackRoots, SoundCatalog,
    SoundEventRegistry,
};
use bbb_world::{
    AmbientLoopSoundState, JukeboxLevelEventState, LocalSoundEventState, SoundEntityEventState,
    SoundEventState, StopSoundEventState,
};

use resolver::{AudioCommandResolver, AudioResolveError};

pub(crate) trait AudioEventSink {
    fn counters(&self) -> AudioCounters;
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
        command: TickEntitySoundPositionsCommand,
        tickable_sounds: &[bbb_world::TickableEntitySoundState],
    );
}

pub(crate) struct NativeAudioRuntime {
    catalog: SoundCatalog,
    registry: SoundEventRegistry,
    jukebox_registry: JukeboxSongRegistry,
    volume_settings: AudioVolumeSettings,
    playback: KiraAudioRuntime,
    counters: AudioCounters,
    // Backend-only playback identity. Canonical ownership remains in the
    // immutable world generation; this set only prevents duplicate starts and
    // notices bee event-id switches or resource reload recovery.
    active_tickable_sounds: BTreeSet<(i32, String)>,
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
        let mut playback = KiraAudioRuntime::new_with_device_preference(
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
            active_tickable_sounds: BTreeSet::new(),
        })
    }

    pub(crate) fn counters(&self) -> AudioCounters {
        let mut counters = self.counters.clone();
        counters.active_music_event_id = self.playback.active_music_event_id().map(str::to_string);
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
            Ok(command) => self.submit_command(command),
            Err(err) => {
                self.counters.resolve_failures += 1;
                self.counters.last_resolve_error = Some(err.to_string());
                tracing::warn!(?err, "failed to resolve audio command");
                false
            }
        }
    }

    fn submit_command(&mut self, command: AudioCommand) -> bool {
        match self.playback.handle_command(&command) {
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
        let tickable_key = command.as_ref().ok().and_then(tickable_play_command_key);
        if self.handle_resolved_command(command) {
            if let Some(key) = tickable_key {
                self.active_tickable_sounds.insert(key);
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
        self.submit_command(command);
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
        self.submit_command(command);
    }

    fn set_music_gain(&mut self, gain: f32) {
        self.submit_command(AudioCommand::SetMusicGain(bbb_audio::SetMusicGainCommand {
            gain,
        }));
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
        self.submit_command(AudioCommand::SyncAmbientLoops(
            bbb_audio::SyncAmbientLoopsCommand { loops: commands },
        ));
    }

    fn tick_entity_sound_positions(
        &mut self,
        mut command: TickEntitySoundPositionsCommand,
        tickable_sounds: &[bbb_world::TickableEntitySoundState],
    ) {
        let updates = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                self.volume_settings,
            );
            tickable_sounds
                .iter()
                .map(|state| resolver.tickable_entity_sound_update(state))
                .collect::<Vec<_>>()
        };
        let mut next_active = BTreeSet::new();
        for (state, update) in tickable_sounds.iter().zip(updates) {
            match update {
                Ok(update) => {
                    let key = (update.entity_id, update.event_id.clone());
                    if !tickable_sound_needs_start(&self.active_tickable_sounds, &key) {
                        next_active.insert(key);
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
                        if self.handle_resolved_command(start_command) {
                            next_active.insert(key);
                        }
                    }
                    command.tickable_sounds.push(update);
                }
                Err(err) => {
                    self.counters.resolve_failures += 1;
                    self.counters.last_resolve_error = Some(err.to_string());
                }
            }
        }
        let _ = self.submit_command(AudioCommand::TickEntitySoundPositions(command));
        self.active_tickable_sounds = next_active;
    }
}

fn tickable_play_command_key(command: &AudioCommand) -> Option<(i32, String)> {
    let AudioCommand::PlayEntitySound(command) = command else {
        return None;
    };
    command
        .tickable
        .then(|| (command.entity_id, command.sound.event_id.clone()))
}

fn tickable_sound_needs_start(active: &BTreeSet<(i32, String)>, key: &(i32, String)) -> bool {
    !active.contains(key)
}

fn reset_tickable_sound_identity_after_device_reload(
    active: &mut BTreeSet<(i32, String)>,
    reloaded: bool,
) {
    if reloaded {
        active.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn every_native_audio_resolution_uses_the_startup_volume_settings() {
        let production = include_str!("audio_runtime.rs")
            .split_once("#[cfg(test)]")
            .expect("audio runtime production/test boundary")
            .0;

        assert!(!production.contains("AudioVolumeSettings::default()"));
        assert_eq!(
            production.matches("self.volume_settings,").count(),
            9,
            "every local, positioned, entity, jukebox, stop, ambient and tickable resolver must use the stored startup settings",
        );
    }

    #[test]
    fn tickable_sound_owner_requires_start_after_event_switch_or_reload() {
        let normal = (7, "minecraft:entity.bee.loop".to_string());
        let aggressive = (7, "minecraft:entity.bee.loop_aggressive".to_string());
        let active = BTreeSet::from([normal.clone()]);

        assert!(!tickable_sound_needs_start(&active, &normal));
        assert!(tickable_sound_needs_start(&active, &aggressive));
        assert!(tickable_sound_needs_start(&BTreeSet::new(), &normal));
    }

    #[test]
    fn device_reload_invalidates_backend_tickable_identity_only_after_success() {
        let key = (7, "minecraft:entity.bee.loop".to_owned());
        let mut active = BTreeSet::from([key.clone()]);

        reset_tickable_sound_identity_after_device_reload(&mut active, false);
        assert_eq!(active, BTreeSet::from([key]));

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
            bbb_pack::SoundEventDefinition {
                id: "minecraft:test".to_string(),
                replace: false,
                subtitle: None,
                sounds: vec![bbb_pack::SoundEntry {
                    name: "minecraft:test/preloaded".to_string(),
                    kind: bbb_pack::SoundEntryKind::File,
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
            bbb_pack::SoundEventDefinition {
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
        std::env::temp_dir().join(format!("bbb-native-{label}-{nanos}"))
    }
}
