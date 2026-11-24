#[cfg(feature = "device")]
mod recording;

#[cfg(feature = "device")]
use std::collections::HashSet;
#[cfg(feature = "device")]
use std::path::{Path, PathBuf};

#[cfg(feature = "device")]
use bbb_client_clock::Instant;
#[cfg(feature = "device")]
use std::time::Duration;

#[cfg(feature = "device")]
use anyhow::Context;
use anyhow::{anyhow, Result};
#[cfg(test)]
use kira::{
    listener::ListenerHandle,
    sound::{static_sound::StaticSoundHandle, streaming::StreamingSoundHandle, PlaybackState},
    track::{MainTrackBuilder, SpatialTrackBuilder, SpatialTrackHandle},
    AudioManager, AudioManagerSettings, Capacities, Decibels, DefaultBackend, Tween,
};

#[cfg(test)]
use crate::residency::AudioDecodeError;
#[cfg(feature = "device")]
use crate::scheduler::{listener_vectors, normalized_gain};
#[cfg(test)]
use crate::scheduler::{spatial_max_distance, MIN_SPATIAL_DISTANCE};
#[cfg(test)]
use crate::AudioListenerState;
use crate::{
    preload::{AudioPreloadObservation, AudioPreloadRequest, AudioPreloadWorker},
    residency::AudioResidency,
    scheduler::{
        audio_position, channel_playback_rate, AudioChannelLimits, VoiceHandle, VoiceScheduler,
    },
    AudioCategory, AudioCommand, AudioResidencyObservation, PlayPositionedSoundCommand,
    ResolvedSound, StopJukeboxSongCommand, StopSoundCommand, TickEntitySoundPositionsCommand,
};

#[cfg(feature = "device")]
use crate::decode_worker::{AsyncOpenAlStreamHandle, AudioDecodeWorker, StreamChunkStatus};
#[cfg(feature = "device")]
use crate::openal::{OpenAlBackend, OpenAlSoundSource, OpenAlSourceSettings};

#[cfg(feature = "device")]
use crate::device::{
    audio_device_reload_reason, initial_audio_devices, is_sound_device_default,
    AudioDeviceInventory, AudioDeviceObservation, AudioDevicePoll, AudioDeviceReloadReason,
    AudioDeviceTracker,
};

#[cfg(test)]
// Kira exposes mixer-slot limits rather than OpenAL's ALC_MONO_SOURCES. Keep
// the pre-existing aggregate of 256 slots as an explicitly backend-logical
// source contract until an OpenAL capability owner replaces this origin.
const KIRA_LOGICAL_MONO_SOURCES: usize = 256;
#[cfg(test)]
const KIRA_CHANNEL_CAPACITY_ORIGIN: &str = "kira-logical-main-plus-spatial";
#[cfg(feature = "device")]
const DEVICE_TICK_INTERVAL: Duration = Duration::from_millis(50);

pub struct NativeAudioRuntime {
    // Drop `scheduler` before `backend` so every live OpenAL voice (and any
    // attached recording source / static buffer handle) releases its handles
    // before the OpenAL context and device are destroyed.
    scheduler: VoiceScheduler<PlayingSoundHandle>,
    backend: ActiveAudioBackend,
    residency: AudioResidency,
    channel_capacity_origin: &'static str,
    preload_worker: AudioPreloadWorker,
    preload_observation: AudioPreloadObservation,
    preload_token: Option<u64>,
}

impl Drop for NativeAudioRuntime {
    fn drop(&mut self) {
        self.scheduler.stop_and_clear_playing();
    }
}

enum ActiveAudioBackend {
    #[cfg(test)]
    Kira(Box<KiraBackendState>),
    #[cfg(feature = "device")]
    OpenAl(Box<OpenAlBackendState>),
}

#[cfg(test)]
struct KiraBackendState {
    manager: AudioManager<DefaultBackend>,
    listener: ListenerHandle,
}

#[cfg(feature = "device")]
struct OpenAlBackendState {
    backend: OpenAlBackend,
    device: Option<OpenAlDeviceRuntime>,
    decode_worker: AudioDecodeWorker,
    in_flight_static: HashSet<(u64, PathBuf)>,
    recording: Option<recording::RecordingAudio>,
    recording_error: Option<String>,
}

#[cfg(feature = "device")]
struct OpenAlDeviceRuntime {
    openal_library_path: PathBuf,
    use_hrtf: bool,
    preferred_device: String,
    current_device: Option<String>,
    inventory: AudioDeviceInventory,
    tracker: AudioDeviceTracker,
    reloads: u64,
    reload_failures: u64,
    query_failures: u64,
    preferred_fallbacks: u64,
    last_reload_reason: Option<AudioDeviceReloadReason>,
    last_error: Option<String>,
    next_poll_at: Instant,
}

pub use crate::command::{AudioChannelObservation, AudioPlaybackEvent};

enum PlayingSoundHandle {
    #[cfg(test)]
    Kira {
        sound: KiraPlaybackHandle,
        track: Option<Box<SpatialTrackHandle>>,
    },
    #[cfg(feature = "device")]
    OpenAl {
        state: OpenAlVoiceState,
        streaming: bool,
    },
}

#[cfg(feature = "device")]
enum OpenAlVoiceState {
    PendingStatic {
        generation: u64,
        path: PathBuf,
        settings: OpenAlSourceSettings,
    },
    PendingStream {
        generation: u64,
        stream: AsyncOpenAlStreamHandle,
        settings: OpenAlSourceSettings,
    },
    Active(OpenAlSoundSource),
    Stopped,
}

#[cfg(test)]
enum KiraPlaybackHandle {
    Static(StaticSoundHandle),
    Streaming(StreamingSoundHandle<AudioDecodeError>),
}

impl NativeAudioRuntime {
    #[cfg(test)]
    pub fn new() -> Result<Self> {
        Self::new_with_resource_generation(0)
    }

    #[cfg(test)]
    pub fn new_with_resource_generation(resource_generation: u64) -> Result<Self> {
        let mut manager = AudioManager::<DefaultBackend>::new(kira_audio_manager_settings())
            .map_err(|_| anyhow!("initialize Kira audio manager"))?;
        let listener = manager
            .add_listener([0.0, 0.0, 0.0], identity_quaternion())
            .map_err(|_| anyhow!("initialize Kira audio listener"))?;
        Self::with_backend(
            ActiveAudioBackend::Kira(Box::new(KiraBackendState { manager, listener })),
            resource_generation,
            AudioChannelLimits::from_mono_sources(KIRA_LOGICAL_MONO_SOURCES),
            KIRA_CHANNEL_CAPACITY_ORIGIN,
        )
    }

    #[cfg(feature = "device")]
    pub fn new_with_device_preference(
        openal_library_path: impl AsRef<Path>,
        preferred_device: impl Into<String>,
        use_hrtf: bool,
        resource_generation: u64,
    ) -> Result<Self> {
        let preferred_device = preferred_device.into();
        let (inventory, initial_query_error) = initial_audio_devices(openal_library_path.as_ref());
        let (openal, current_device, preferred_fallback) =
            start_openal_backend(openal_library_path.as_ref(), &preferred_device, use_hrtf)?;
        let mono_sources = openal.observation().mono_sources;
        let device = OpenAlDeviceRuntime {
            openal_library_path: openal_library_path.as_ref().to_path_buf(),
            use_hrtf,
            preferred_device,
            current_device,
            inventory,
            tracker: AudioDeviceTracker::spawn(openal_library_path.as_ref().to_path_buf())?,
            reloads: 0,
            reload_failures: 0,
            query_failures: u64::from(initial_query_error.is_some()),
            preferred_fallbacks: u64::from(preferred_fallback),
            last_reload_reason: None,
            last_error: initial_query_error,
            next_poll_at: bbb_client_clock::now(),
        };
        Self::from_openal_backend(
            openal,
            resource_generation,
            Some(device),
            AudioChannelLimits::from_mono_sources(mono_sources),
            "openal-alc-mono-sources",
        )
    }

    #[cfg(feature = "device")]
    fn from_openal_backend(
        backend: OpenAlBackend,
        resource_generation: u64,
        device: Option<OpenAlDeviceRuntime>,
        channel_limits: AudioChannelLimits,
        channel_capacity_origin: &'static str,
    ) -> Result<Self> {
        let decode_worker = AudioDecodeWorker::spawn()
            .map_err(|error| anyhow!("initialize audio decode worker: {error}"))?;
        Self::with_backend(
            ActiveAudioBackend::OpenAl(Box::new(OpenAlBackendState {
                backend,
                device,
                decode_worker,
                in_flight_static: HashSet::new(),
                recording: None,
                recording_error: None,
            })),
            resource_generation,
            channel_limits,
            channel_capacity_origin,
        )
    }

    fn with_backend(
        backend: ActiveAudioBackend,
        resource_generation: u64,
        channel_limits: AudioChannelLimits,
        channel_capacity_origin: &'static str,
    ) -> Result<Self> {
        let preload_worker = AudioPreloadWorker::spawn()
            .map_err(|error| anyhow!("initialize audio preload worker: {error}"))?;
        Ok(Self {
            backend,
            scheduler: VoiceScheduler::new(channel_limits),
            residency: AudioResidency::new(resource_generation),
            channel_capacity_origin,
            preload_worker,
            preload_observation: AudioPreloadObservation {
                resource_generation,
                ..AudioPreloadObservation::default()
            },
            preload_token: None,
        })
    }

    pub fn residency_observation(&self) -> AudioResidencyObservation {
        self.residency.observation()
    }

    pub fn preload_observation(&self) -> AudioPreloadObservation {
        self.preload_observation.clone()
    }

    /// Queue the current generation's `preload:true` complete buffers on the
    /// single latest-wins decode owner. The caller only publishes immutable
    /// paths; file open and decode remain off the application frame.
    pub fn request_preload(&mut self, mut requests: Vec<AudioPreloadRequest>) {
        let generation = self.residency.active_generation();
        let mut failures = 0_usize;
        let mut last_error = None;
        requests.retain(|request| {
            if request.resource_generation == generation {
                true
            } else {
                failures = failures.saturating_add(1);
                last_error = Some(format!(
                    "sound preload generation {} does not match active generation {generation}",
                    request.resource_generation
                ));
                false
            }
        });
        requests.sort_by(|left, right| {
            left.sound_name
                .cmp(&right.sound_name)
                .then_with(|| left.ogg_path.cmp(&right.ogg_path))
        });
        requests.dedup_by(|left, right| left.sound_name == right.sound_name);

        let cancelled = usize::from(self.preload_observation.pending > 0);
        let worker_jobs = self.preload_observation.worker_jobs;
        let cancelled_jobs = self
            .preload_observation
            .cancelled_jobs
            .saturating_add(cancelled as u64);
        let stale_results = self.preload_observation.stale_results;
        self.preload_worker.cancel();
        self.preload_token = None;
        let requested = requests.len().saturating_add(failures);
        self.preload_observation = AudioPreloadObservation {
            resource_generation: generation,
            requested,
            completed: 0,
            failures,
            pending: requests.len(),
            worker_jobs,
            cancelled_jobs,
            stale_results,
            last_error,
        };
        if requests.is_empty() {
            return;
        }
        let token = self.preload_worker.submit(generation, requests);
        self.preload_token = Some(token);
        self.preload_observation.worker_jobs =
            self.preload_observation.worker_jobs.saturating_add(1);
    }

    /// Non-blockingly install a completed batch if it still belongs to the
    /// active resource generation. Returns true only when the receipt changed.
    pub fn poll_audio_preload(&mut self) -> bool {
        #[cfg(feature = "device")]
        self.drain_completed_decodes();
        let Some(batch) = self.preload_worker.take_result() else {
            return false;
        };
        if self.preload_token != Some(batch.token)
            || batch.resource_generation != self.residency.active_generation()
        {
            self.preload_observation.stale_results =
                self.preload_observation.stale_results.saturating_add(1);
            return true;
        }
        self.preload_token = None;
        self.preload_observation.pending = 0;
        self.preload_observation.requested = self.preload_observation.requested.max(
            batch
                .requested
                .saturating_add(self.preload_observation.failures),
        );
        self.preload_observation.failures = self
            .preload_observation
            .failures
            .saturating_add(batch.failures);
        self.preload_observation.last_error = batch
            .last_error
            .or_else(|| self.preload_observation.last_error.take());
        match self.residency.install_preloaded(batch.residency) {
            Ok(installed) => {
                self.preload_observation.completed = installed
                    .installed
                    .saturating_add(installed.already_resident);
                self.preload_observation.failures = self
                    .preload_observation
                    .failures
                    .saturating_add(installed.failures);
                if installed.last_error.is_some() {
                    self.preload_observation.last_error = installed.last_error;
                }
                debug_assert_eq!(
                    batch.decoded,
                    self.preload_observation
                        .completed
                        .saturating_add(installed.failures)
                );
            }
            Err(error) => {
                self.preload_observation.stale_results =
                    self.preload_observation.stale_results.saturating_add(1);
                self.preload_observation.failures = self
                    .preload_observation
                    .failures
                    .saturating_add(batch.decoded);
                self.preload_observation.last_error = Some(error.to_string());
            }
        }
        true
    }

    pub fn channel_observation(&self) -> AudioChannelObservation {
        let (static_used, streaming_used) = self.scheduler.channel_counts();
        let (native_static_buffers, native_static_bytes, native_static_evictions) =
            match &self.backend {
                #[cfg(test)]
                ActiveAudioBackend::Kira(_) => (0, 0, 0),
                #[cfg(feature = "device")]
                ActiveAudioBackend::OpenAl(openal) => openal.backend.static_cache_observation(),
            };
        AudioChannelObservation {
            static_used,
            streaming_used,
            static_capacity: self.scheduler.channel_limits.static_capacity,
            streaming_capacity: self.scheduler.channel_limits.streaming_capacity,
            admission_rejections: self.scheduler.channel_admission_rejections,
            native_static_buffers,
            native_static_bytes,
            native_static_evictions,
            capacity_origin: self.channel_capacity_origin,
        }
    }

    #[cfg(feature = "device")]
    fn openal_backend_mut(backend: &mut ActiveAudioBackend) -> Option<&mut OpenAlBackendState> {
        #[cfg(test)]
        {
            match backend {
                ActiveAudioBackend::Kira(_) => None,
                ActiveAudioBackend::OpenAl(openal_state) => Some(openal_state),
            }
        }
        #[cfg(not(test))]
        {
            let ActiveAudioBackend::OpenAl(openal_state) = backend;
            Some(openal_state)
        }
    }

    #[cfg(feature = "device")]
    pub fn audio_device_observation(&self) -> Option<AudioDeviceObservation> {
        #[cfg(test)]
        let openal_state = match &self.backend {
            ActiveAudioBackend::Kira(_) => return None,
            ActiveAudioBackend::OpenAl(openal_state) => openal_state,
        };
        #[cfg(not(test))]
        let ActiveAudioBackend::OpenAl(openal_state) = &self.backend;
        let device = openal_state.device.as_ref()?;
        let openal = openal_state.backend.observation();
        Some(AudioDeviceObservation {
            preferred_device: (!is_sound_device_default(&device.preferred_device))
                .then(|| device.preferred_device.clone()),
            current_device: device.current_device.clone(),
            default_device: device.inventory.snapshot.default_device.clone(),
            available_device_count: device.inventory.snapshot.all_devices.len(),
            reloads: device.reloads,
            reload_failures: device.reload_failures,
            query_failures: device.query_failures,
            preferred_fallbacks: device.preferred_fallbacks,
            last_reload_reason: device
                .last_reload_reason
                .map(AudioDeviceReloadReason::as_str)
                .map(str::to_owned),
            last_error: device.last_error.clone(),
            requested_directional_audio: openal.requested_hrtf,
            hrtf_extension: openal.hrtf_extension,
            hrtf_specifiers: openal.hrtf_specifiers,
            hrtf_status: openal.hrtf_status,
            output_limiter: openal.output_limiter,
            supports_disconnection: openal.supports_disconnection,
            tracking_mode: device.tracker.tracking_mode(),
        })
    }

    /// Consume the newest asynchronously queried device list and apply
    /// `SoundEngine.shouldChangeDevice` before the next native audio tick.
    /// The per-event fast path only probes bounded channels and the one
    /// background tracker; host enumeration stays off-thread.
    #[cfg(feature = "device")]
    pub fn poll_audio_device(&mut self) -> AudioDevicePoll {
        self.drain_completed_decodes();
        let (mut device, mut disconnected) = match &mut self.backend {
            #[cfg(test)]
            ActiveAudioBackend::Kira(_) => return AudioDevicePoll::default(),
            ActiveAudioBackend::OpenAl(openal_state) => {
                let Some(device) = openal_state.device.take() else {
                    return AudioDevicePoll::default();
                };
                (device, openal_state.backend.is_disconnected())
            }
        };
        let now = bbb_client_clock::now();
        if now < device.next_poll_at {
            self.restore_openal_device(device);
            return AudioDevicePoll::default();
        }
        device.next_poll_at = now + DEVICE_TICK_INTERVAL;
        let mut observation_changed = false;
        if disconnected {
            device.last_error = Some("OpenAL device disconnected".to_owned());
            observation_changed = true;
        }

        let update = if disconnected {
            Some(device.tracker.force_refresh())
        } else {
            device.tracker.latest()
        };
        let mut poll_error = None;
        let next_inventory = match update {
            Some(Ok(inventory)) => Some(inventory),
            Some(Err(error)) => {
                device.query_failures = device.query_failures.saturating_add(1);
                device.last_error = Some(error.clone());
                if !disconnected {
                    self.restore_openal_device(device);
                    return AudioDevicePoll {
                        reload_reason: None,
                        error: Some(error),
                        observation_changed: true,
                    };
                }
                poll_error = Some(error);
                None
            }
            None => None,
        };

        if !disconnected && next_inventory.is_none() {
            self.restore_openal_device(device);
            return AudioDevicePoll {
                observation_changed,
                ..AudioDevicePoll::default()
            };
        }
        let inventory = next_inventory.unwrap_or_else(|| {
            disconnected = true;
            std::mem::replace(&mut device.inventory, AudioDeviceInventory::empty())
        });
        let reason = audio_device_reload_reason(
            disconnected,
            &device.preferred_device,
            device.current_device.as_deref(),
            &device.inventory.snapshot,
            &inventory.snapshot,
        );
        let Some(reason) = reason else {
            observation_changed |= inventory.snapshot != device.inventory.snapshot;
            device.inventory = inventory;
            self.restore_openal_device(device);
            return AudioDevicePoll {
                observation_changed,
                ..AudioDevicePoll::default()
            };
        };

        match start_openal_backend(
            &device.openal_library_path,
            &device.preferred_device,
            device.use_hrtf,
        )
        .and_then(|(backend, current_device, preferred_fallback)| {
            self.replace_openal_backend(backend)?;
            Ok((current_device, preferred_fallback))
        }) {
            Ok((current_device, preferred_fallback)) => {
                device.current_device = current_device;
                device.inventory = inventory;
                device.reloads = device.reloads.saturating_add(1);
                device.preferred_fallbacks = device
                    .preferred_fallbacks
                    .saturating_add(u64::from(preferred_fallback));
                device.last_reload_reason = Some(reason);
                device.last_error = poll_error.clone();
                self.restore_openal_device(device);
                AudioDevicePoll {
                    reload_reason: Some(reason),
                    error: poll_error,
                    observation_changed: true,
                }
            }
            Err(error) => {
                let error = format!("{error:#}");
                device.reload_failures = device.reload_failures.saturating_add(1);
                device.last_error = Some(error.clone());
                device.inventory = inventory;
                self.restore_openal_device(device);
                AudioDevicePoll {
                    reload_reason: None,
                    error: Some(error),
                    observation_changed: true,
                }
            }
        }
    }

    #[cfg(feature = "device")]
    fn restore_openal_device(&mut self, device: OpenAlDeviceRuntime) {
        match &mut self.backend {
            #[cfg(test)]
            ActiveAudioBackend::Kira(_) => {}
            ActiveAudioBackend::OpenAl(openal_state) => {
                openal_state.device = Some(device);
            }
        }
    }

    #[cfg(feature = "device")]
    fn replace_openal_backend(&mut self, backend: OpenAlBackend) -> Result<()> {
        let _ = self.poll_recording();
        self.scheduler.stop_and_clear_playing();
        let (mono_sources, capacity_origin) = match &mut self.backend {
            #[cfg(test)]
            ActiveAudioBackend::Kira(_) => {
                anyhow::bail!("OpenAL reload requires an active OpenAL backend");
            }
            ActiveAudioBackend::OpenAl(openal_state) => {
                openal_state.decode_worker.cancel_all();
                openal_state.in_flight_static.clear();
                if let Some(recording) = openal_state.recording.as_mut() {
                    recording.backend.clear_static_buffers();
                    if let Err(error) =
                        recording
                            .backend
                            .set_listener([0.0; 3], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0])
                    {
                        recording.failure = Some(format!("{error:#}"));
                    }
                }
                backend.set_listener([0.0; 3], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0])?;
                let obs = backend.observation();
                let limits = (obs.mono_sources, obs.capacity_origin);
                // `set_listener` made the new context current after stopping old
                // sources; replacing `openal_state.backend` now drops the old
                // context while leaving the new context current.
                openal_state.backend = backend;
                limits
            }
        };
        self.scheduler.delayed_positioned.clear();
        self.scheduler.playback_events.clear();
        self.scheduler.music_gain = 1.0;
        let resource_generation = self.residency.active_generation();
        self.cancel_audio_preload(resource_generation);
        self.residency.reset_generation(resource_generation);
        self.scheduler.listener_position = [0.0; 3];
        self.scheduler.channel_limits = AudioChannelLimits::from_mono_sources(mono_sources);
        self.channel_capacity_origin = capacity_origin;
        Ok(())
    }

    /// Returns the most recently started music-category instance retained by
    /// the Vanilla-compatible lifetime owner. A stopped backend handle remains
    /// active to simulation until its minimum delete tick is processed.
    pub fn active_music_event_id(&self) -> Option<&str> {
        self.scheduler.active_music_event_id()
    }

    /// Returns the actual resolved file identity from the same retained music
    /// instance as `active_music_event_id`.
    pub fn active_music_sound_name(&self) -> Option<&str> {
        self.scheduler.active_music_sound_name()
    }

    pub fn drain_playback_events(&mut self) -> impl Iterator<Item = AudioPlaybackEvent> + '_ {
        self.scheduler.drain_playback_events()
    }

    pub fn reset_resource_generation(&mut self, resource_generation: u64) {
        #[cfg(feature = "device")]
        let _ = self.poll_recording();
        self.scheduler.reset_generation();
        #[cfg(feature = "device")]
        match &mut self.backend {
            #[cfg(test)]
            ActiveAudioBackend::Kira(_) => {}
            ActiveAudioBackend::OpenAl(openal_state) => {
                openal_state.decode_worker.cancel_all();
                openal_state.in_flight_static.clear();
                if let Some(recording) = openal_state.recording.as_mut() {
                    recording.backend.clear_static_buffers();
                }
                openal_state.backend.clear_static_buffers();
            }
        }
        self.cancel_audio_preload(resource_generation);
        self.residency.reset_generation(resource_generation);
    }

    fn cancel_audio_preload(&mut self, resource_generation: u64) {
        if self.preload_observation.pending > 0 {
            self.preload_observation.cancelled_jobs =
                self.preload_observation.cancelled_jobs.saturating_add(1);
        }
        self.preload_worker.cancel();
        self.preload_token = None;
        self.preload_observation.resource_generation = resource_generation;
        self.preload_observation.requested = 0;
        self.preload_observation.completed = 0;
        self.preload_observation.failures = 0;
        self.preload_observation.pending = 0;
        self.preload_observation.last_error = None;
    }

    #[cfg(feature = "device")]
    pub(super) fn drain_completed_decodes(&mut self) {
        let Some(openal_state) = Self::openal_backend_mut(&mut self.backend) else {
            return;
        };
        let current_epoch = openal_state.decode_worker.epoch();
        let active_generation = self.residency.active_generation();
        let completed_static = openal_state.decode_worker.drain_completed_static();
        for completion in completed_static {
            if completion.epoch != current_epoch || completion.generation != active_generation {
                continue;
            }
            openal_state
                .in_flight_static
                .remove(&(completion.generation, completion.path.clone()));
            match completion.result {
                Ok(decoded) => {
                    for voice in &mut self.scheduler.playing {
                        let state = match &mut voice.handle {
                            #[cfg(test)]
                            PlayingSoundHandle::Kira { .. } => continue,
                            PlayingSoundHandle::OpenAl { state, .. } => state,
                        };
                        let OpenAlVoiceState::PendingStatic {
                            generation,
                            path,
                            settings,
                        } = state
                        else {
                            continue;
                        };
                        if *generation != completion.generation || *path != completion.path {
                            continue;
                        }
                        let settings = *settings;
                        match openal_state.backend.play_static_pcm(
                            completion.generation,
                            &completion.path,
                            decoded.clone(),
                            settings,
                        ) {
                            Ok(mut source) => {
                                attach_recording_if_active(openal_state, &mut source);
                                *state = OpenAlVoiceState::Active(source);
                            }
                            Err(_) => {
                                *state = OpenAlVoiceState::Stopped;
                            }
                        }
                    }
                }
                Err(_) => {
                    self.residency.record_admission_rejection();
                    for voice in &mut self.scheduler.playing {
                        let state = match &mut voice.handle {
                            #[cfg(test)]
                            PlayingSoundHandle::Kira { .. } => continue,
                            PlayingSoundHandle::OpenAl { state, .. } => state,
                        };
                        let matches_key = matches!(
                            state,
                            OpenAlVoiceState::PendingStatic {
                                generation,
                                path,
                                ..
                            } if *generation == completion.generation && *path == completion.path
                        );
                        if matches_key {
                            *state = OpenAlVoiceState::Stopped;
                        }
                    }
                }
            }
        }

        for voice in &mut self.scheduler.playing {
            let state = match &mut voice.handle {
                #[cfg(test)]
                PlayingSoundHandle::Kira { .. } => continue,
                PlayingSoundHandle::OpenAl { state, .. } => state,
            };
            let action = match state {
                OpenAlVoiceState::PendingStream {
                    generation, stream, ..
                } => {
                    if *generation != active_generation {
                        Some(Err(false))
                    } else {
                        match stream.peek_status() {
                            StreamChunkStatus::Waiting => None,
                            StreamChunkStatus::Ready => Some(Ok(())),
                            StreamChunkStatus::FinishedEmpty => Some(Err(false)),
                            StreamChunkStatus::Error(_) => Some(Err(true)),
                        }
                    }
                }
                _ => None,
            };
            match action {
                None => {}
                Some(Err(record_rejection)) => {
                    if record_rejection {
                        self.residency.record_admission_rejection();
                    }
                    *state = OpenAlVoiceState::Stopped;
                }
                Some(Ok(())) => {
                    let previous = std::mem::replace(state, OpenAlVoiceState::Stopped);
                    if let OpenAlVoiceState::PendingStream {
                        stream, settings, ..
                    } = previous
                    {
                        if let Ok(mut source) =
                            openal_state.backend.play_stream_async(stream, settings)
                        {
                            attach_recording_if_active(openal_state, &mut source);
                            *state = OpenAlVoiceState::Active(source);
                        }
                    }
                }
            }
        }
    }

    #[cfg(feature = "device")]
    pub(super) fn flush_pending_decodes(&mut self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        loop {
            self.drain_completed_decodes();
            let has_pending = self
                .scheduler
                .playing
                .iter()
                .any(|voice| match &voice.handle {
                    #[cfg(test)]
                    PlayingSoundHandle::Kira { .. } => false,
                    PlayingSoundHandle::OpenAl { state, .. } => matches!(
                        state,
                        OpenAlVoiceState::PendingStatic { .. }
                            | OpenAlVoiceState::PendingStream { .. }
                    ),
                });
            if !has_pending {
                return;
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return;
            };
            match &self.backend {
                #[cfg(test)]
                ActiveAudioBackend::Kira(_) => return,
                ActiveAudioBackend::OpenAl(openal_state) => {
                    openal_state
                        .decode_worker
                        .wait_for_progress(remaining.min(Duration::from_millis(50)));
                }
            }
        }
    }

    pub fn handle_command(&mut self, command: &AudioCommand) -> Result<()> {
        #[cfg(feature = "device")]
        {
            self.drain_completed_decodes();
            let _ = self.poll_recording();
        }
        match command {
            AudioCommand::PlayLocalSound(command) => self.play_local_sound(
                &command.sound,
                command.category.clone(),
                command.channel_gain,
                command.playback_rate,
            ),
            AudioCommand::PlayPositionedSound(command) => self.play_positioned_sound(command),
            AudioCommand::PlayEntitySound(command) => self.play_spatial_sound(
                &command.sound,
                command.category.clone(),
                Some(command.entity_id),
                command.position.unwrap_or([0.0, 0.0, 0.0]),
                command.gain,
                command.channel_gain,
                command.playback_rate,
                command.fixed_range,
                None,
                command.looping,
                command.linear_attenuation,
                command.tickable,
            ),
            AudioCommand::PlayJukeboxSong(command) => self.play_jukebox_song(
                &command.sound,
                command.category.clone(),
                command.position,
                command.jukebox_pos,
                command.gain,
                command.channel_gain,
                command.playback_rate,
                command.fixed_range,
            ),
            AudioCommand::StopJukeboxSong(command) => {
                self.stop_jukebox_song(command);
                Ok(())
            }
            AudioCommand::StopSound(command) => {
                self.stop_sounds(command);
                Ok(())
            }
            AudioCommand::SetMusicGain(command) => {
                self.set_music_gain(command.gain);
                Ok(())
            }
            AudioCommand::SyncAmbientLoops(command) => self.sync_ambient_loops(&command.loops),
            AudioCommand::TickEntitySoundPositions(command) => {
                self.tick_entity_sound_positions(command)
            }
        }
    }

    fn play_local_sound(
        &mut self,
        sound: &ResolvedSound,
        category: AudioCategory,
        channel_gain: f32,
        playback_rate: f32,
    ) -> Result<()> {
        self.play_local_sound_with_role(sound, category, channel_gain, playback_rate, false, false)
    }

    fn play_local_sound_with_role(
        &mut self,
        sound: &ResolvedSound,
        category: AudioCategory,
        channel_gain: f32,
        playback_rate: f32,
        looping: bool,
        ambient_loop: bool,
    ) -> Result<()> {
        let Some(launch) =
            self.scheduler
                .prepare_local_launch(sound, &category, channel_gain, playback_rate)
        else {
            return Ok(());
        };
        let handle = match &mut self.backend {
            #[cfg(test)]
            ActiveAudioBackend::Kira(kira) => play_kira_sound(
                kira,
                &mut self.residency,
                sound,
                channel_decibels_from_gain(launch.effective_gain),
                launch.playback_rate,
                looping,
                None,
            )?,
            #[cfg(feature = "device")]
            ActiveAudioBackend::OpenAl(openal_state) => {
                let settings = OpenAlSourceSettings {
                    position: launch.position,
                    pitch: launch.playback_rate,
                    gain: launch.effective_gain,
                    looping: looping && !sound.stream,
                    relative: true,
                    max_distance: None,
                };
                play_openal_sound(openal_state, &mut self.residency, sound, looping, settings)?
            }
        };
        self.scheduler.push_voice(
            sound,
            category,
            None,
            launch.position,
            None,
            handle,
            false,
            ambient_loop,
            channel_gain,
            launch.playback_rate,
        );
        Ok(())
    }

    fn play_positioned_sound(&mut self, command: &PlayPositionedSoundCommand) -> Result<()> {
        if self.scheduler.enqueue_if_delayed(command) {
            return Ok(());
        }
        self.play_positioned_sound_now(command)
    }

    fn play_positioned_sound_now(&mut self, command: &PlayPositionedSoundCommand) -> Result<()> {
        self.play_spatial_sound(
            &command.sound,
            command.category.clone(),
            None,
            command.position,
            command.gain,
            command.channel_gain,
            command.playback_rate,
            command.fixed_range,
            None,
            false,
            true,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn play_spatial_sound(
        &mut self,
        sound: &ResolvedSound,
        category: AudioCategory,
        entity_id: Option<i32>,
        position: [f64; 3],
        gain: f32,
        channel_gain: f32,
        playback_rate: f32,
        fixed_range: Option<f32>,
        jukebox_pos: Option<[i32; 3]>,
        looping: bool,
        linear_attenuation: bool,
        tickable: bool,
    ) -> Result<()> {
        let Some(launch) = self.scheduler.prepare_spatial_launch(
            sound,
            &category,
            position,
            gain,
            channel_gain,
            playback_rate,
            fixed_range,
            linear_attenuation,
            tickable,
        ) else {
            return Ok(());
        };
        let handle = match &mut self.backend {
            #[cfg(test)]
            ActiveAudioBackend::Kira(kira) => {
                let track = if linear_attenuation {
                    Some(
                        kira.manager.add_spatial_sub_track(
                            &kira.listener,
                            launch.position,
                            SpatialTrackBuilder::new()
                                .persist_until_sounds_finish(true)
                                .distances((MIN_SPATIAL_DISTANCE, launch.max_distance)),
                        )?,
                    )
                } else {
                    None
                };
                play_kira_sound(
                    kira,
                    &mut self.residency,
                    sound,
                    channel_decibels_from_gain(launch.effective_gain),
                    launch.playback_rate,
                    looping,
                    track,
                )?
            }
            #[cfg(feature = "device")]
            ActiveAudioBackend::OpenAl(openal_state) => {
                let settings = OpenAlSourceSettings {
                    position: launch.position,
                    pitch: launch.playback_rate,
                    gain: launch.effective_gain,
                    looping: looping && !sound.stream,
                    relative: false,
                    max_distance: linear_attenuation.then_some(launch.max_distance),
                };
                play_openal_sound(openal_state, &mut self.residency, sound, looping, settings)?
            }
        };
        self.scheduler.push_voice(
            sound,
            category,
            entity_id,
            launch.position,
            jukebox_pos,
            handle,
            tickable,
            false,
            channel_gain,
            launch.playback_rate,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn play_jukebox_song(
        &mut self,
        sound: &ResolvedSound,
        category: AudioCategory,
        position: [f64; 3],
        jukebox_pos: [i32; 3],
        gain: f32,
        channel_gain: f32,
        playback_rate: f32,
        fixed_range: Option<f32>,
    ) -> Result<()> {
        self.stop_jukebox_song(&StopJukeboxSongCommand { jukebox_pos });
        self.play_spatial_sound(
            sound,
            category,
            None,
            position,
            gain,
            channel_gain,
            playback_rate,
            fixed_range,
            Some(jukebox_pos),
            false,
            true,
            false,
        )
    }

    fn stop_sounds(&mut self, command: &StopSoundCommand) {
        self.scheduler.stop_sounds(command);
    }

    fn stop_jukebox_song(&mut self, command: &StopJukeboxSongCommand) {
        self.scheduler.stop_jukebox_song(command);
    }

    fn set_music_gain(&mut self, gain: f32) {
        self.scheduler.set_music_gain(gain);
    }

    fn sync_ambient_loops(&mut self, desired: &[crate::AmbientLoopSoundCommand]) -> Result<()> {
        let starts = self.scheduler.plan_and_update_ambient_loops(desired);
        for desired_index in starts {
            let desired = &desired[desired_index];
            self.play_local_sound_with_role(
                &desired.sound,
                AudioCategory::Ambient,
                desired.channel_gain,
                1.0,
                true,
                true,
            )?;
        }
        Ok(())
    }

    fn tick_entity_sound_positions(
        &mut self,
        command: &TickEntitySoundPositionsCommand,
    ) -> Result<()> {
        if let Some(listener) = command.listener {
            self.scheduler.listener_position = listener.position;
            let pos = audio_position(listener.position);
            match &mut self.backend {
                #[cfg(test)]
                ActiveAudioBackend::Kira(kira) => {
                    let orientation = listener_orientation(listener);
                    kira.listener.set_position(pos, Tween::default());
                    kira.listener.set_orientation(orientation, Tween::default());
                }
                #[cfg(feature = "device")]
                ActiveAudioBackend::OpenAl(openal_state) => {
                    let (forward, up) = listener_vectors(listener);
                    openal_state.backend.set_listener(pos, forward, up)?;
                    if let Some(recording) = openal_state.recording.as_mut() {
                        if let Err(error) = recording.backend.set_listener(pos, forward, up) {
                            recording.failure = Some(format!("{error:#}"));
                        }
                        openal_state.backend.restore_current()?;
                    }
                }
            }
        }
        let ready_delayed = self
            .scheduler
            .step_entity_and_delayed_voices(command, |handle| handle.update_stream())?;
        for delayed_command in ready_delayed {
            self.play_positioned_sound_now(&delayed_command)?;
        }
        for sound in &mut self.scheduler.playing {
            sound.handle.release_stopped_channel();
        }
        Ok(())
    }

    pub fn entity_sound_ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.scheduler.entity_sound_ids()
    }
}

#[cfg(test)]
#[inline]
fn play_kira_sound(
    kira: &mut KiraBackendState,
    residency: &mut AudioResidency,
    sound: &ResolvedSound,
    volume: Decibels,
    playback_rate: f32,
    looping: bool,
    mut track: Option<SpatialTrackHandle>,
) -> Result<PlayingSoundHandle> {
    let playback = if sound.stream {
        let data = residency
            .streaming_sound(sound)?
            .volume(volume)
            .playback_rate(playback_rate as f64);
        let data = if looping { data.loop_region(..) } else { data };
        KiraPlaybackHandle::Streaming(if let Some(track) = track.as_mut() {
            track.play(data)?
        } else {
            kira.manager.play(data)?
        })
    } else {
        let data = residency
            .static_sound(sound)?
            .volume(volume)
            .playback_rate(playback_rate as f64);
        let data = if looping { data.loop_region(..) } else { data };
        KiraPlaybackHandle::Static(if let Some(track) = track.as_mut() {
            track.play(data)?
        } else {
            kira.manager.play(data)?
        })
    };
    Ok(PlayingSoundHandle::Kira {
        sound: playback,
        track: track.map(Box::new),
    })
}

#[cfg(test)]
fn kira_capacities() -> Capacities {
    Capacities {
        sub_track_capacity: KIRA_LOGICAL_MONO_SOURCES,
        ..Capacities::default()
    }
}

#[cfg(test)]
fn kira_main_track_builder() -> MainTrackBuilder {
    MainTrackBuilder::default().sound_capacity(KIRA_LOGICAL_MONO_SOURCES)
}

#[cfg(test)]
fn kira_audio_manager_settings() -> AudioManagerSettings<DefaultBackend> {
    AudioManagerSettings {
        capacities: kira_capacities(),
        main_track_builder: kira_main_track_builder(),
        ..AudioManagerSettings::default()
    }
}

#[cfg(feature = "device")]
fn start_openal_backend(
    openal_library_path: &Path,
    preferred_device: &str,
    use_hrtf: bool,
) -> Result<(OpenAlBackend, Option<String>, bool)> {
    let preferred = (!is_sound_device_default(preferred_device)).then_some(preferred_device);
    let backend = OpenAlBackend::open(openal_library_path, preferred, use_hrtf)?;
    let fallback = preferred
        .is_some_and(|preferred| backend.observation().current_device.as_str() != preferred);
    let current_device = Some(backend.observation().current_device.clone());
    Ok((backend, current_device, fallback))
}

#[cfg(feature = "device")]
fn attach_recording_if_active(
    openal_state: &mut OpenAlBackendState,
    source: &mut OpenAlSoundSource,
) {
    if let Some(recording) = openal_state.recording.as_mut() {
        if let Err(error) = source
            .attach_recording_with_worker(&mut recording.backend, Some(&openal_state.decode_worker))
        {
            recording.failure = Some(format!("{error:#}"));
        }
        let _ = openal_state.backend.restore_current();
    }
}

#[cfg(feature = "device")]
fn play_openal_sound(
    openal_state: &mut OpenAlBackendState,
    residency: &mut AudioResidency,
    sound: &ResolvedSound,
    looping: bool,
    settings: OpenAlSourceSettings,
) -> Result<PlayingSoundHandle> {
    if sound.stream {
        residency.record_streaming_start(sound)?;
        let stream = openal_state
            .decode_worker
            .open_stream(sound.ogg_path.clone(), looping);
        return Ok(PlayingSoundHandle::OpenAl {
            state: OpenAlVoiceState::PendingStream {
                generation: sound.resource_generation,
                stream,
                settings,
            },
            streaming: true,
        });
    }

    residency.check_generation(sound.resource_generation)?;
    if openal_state
        .backend
        .has_static_buffer(sound.resource_generation, &sound.ogg_path)
    {
        residency.record_static_cache_hit();
        let mut source = openal_state.backend.play_static(
            sound.resource_generation,
            &sound.ogg_path,
            || unreachable!("native OpenAL static-buffer cache hit must not decode"),
            settings,
        )?;
        attach_recording_if_active(openal_state, &mut source);
        return Ok(PlayingSoundHandle::OpenAl {
            state: OpenAlVoiceState::Active(source),
            streaming: false,
        });
    }

    if let Some(pcm) = residency.take_openal_static_sound_if_cached(sound)? {
        let mut source = openal_state.backend.play_static_pcm(
            sound.resource_generation,
            &sound.ogg_path,
            crate::residency::DecodedStaticAudio {
                sample_rate: pcm.sample_rate,
                channels: pcm.channels,
                samples: pcm.samples,
            },
            settings,
        )?;
        attach_recording_if_active(openal_state, &mut source);
        return Ok(PlayingSoundHandle::OpenAl {
            state: OpenAlVoiceState::Active(source),
            streaming: false,
        });
    }

    let key = (sound.resource_generation, sound.ogg_path.clone());
    if openal_state.in_flight_static.insert(key.clone()) {
        residency.record_static_cache_miss();
        if !openal_state
            .decode_worker
            .submit_static(key.0, key.1.clone())
        {
            openal_state.in_flight_static.remove(&key);
            residency.record_admission_rejection();
            return Ok(PlayingSoundHandle::OpenAl {
                state: OpenAlVoiceState::Stopped,
                streaming: false,
            });
        }
    }
    Ok(PlayingSoundHandle::OpenAl {
        state: OpenAlVoiceState::PendingStatic {
            generation: sound.resource_generation,
            path: sound.ogg_path.clone(),
            settings,
        },
        streaming: false,
    })
}

impl VoiceHandle for PlayingSoundHandle {
    fn stop(&mut self) {
        match self {
            #[cfg(test)]
            Self::Kira { sound, .. } => match sound {
                KiraPlaybackHandle::Static(handle) => handle.stop(Tween::default()),
                KiraPlaybackHandle::Streaming(handle) => handle.stop(Tween::default()),
            },
            #[cfg(feature = "device")]
            Self::OpenAl { state, .. } => match state {
                OpenAlVoiceState::Active(source) => source.stop(),
                OpenAlVoiceState::PendingStatic { .. } | OpenAlVoiceState::PendingStream { .. } => {
                    *state = OpenAlVoiceState::Stopped
                }
                OpenAlVoiceState::Stopped => {}
            },
        }
    }

    fn is_streaming(&self) -> bool {
        match self {
            #[cfg(test)]
            Self::Kira { sound, .. } => matches!(sound, KiraPlaybackHandle::Streaming(_)),
            #[cfg(feature = "device")]
            Self::OpenAl { streaming, .. } => *streaming,
        }
    }

    fn is_active(&self) -> bool {
        match self {
            #[cfg(test)]
            Self::Kira { sound, .. } => {
                let state = match sound {
                    KiraPlaybackHandle::Static(handle) => handle.state(),
                    KiraPlaybackHandle::Streaming(handle) => handle.state(),
                };
                state != PlaybackState::Stopped
            }
            #[cfg(feature = "device")]
            Self::OpenAl { state, .. } => match state {
                OpenAlVoiceState::PendingStatic { .. } | OpenAlVoiceState::PendingStream { .. } => {
                    true
                }
                OpenAlVoiceState::Active(source) => source.is_active(),
                OpenAlVoiceState::Stopped => false,
            },
        }
    }

    fn holds_channel(&self) -> bool {
        match self {
            #[cfg(test)]
            Self::Kira { .. } => self.is_active(),
            #[cfg(feature = "device")]
            Self::OpenAl { state, .. } => !matches!(state, OpenAlVoiceState::Stopped),
        }
    }

    fn set_volume(&mut self, gain: f32) {
        match self {
            #[cfg(test)]
            Self::Kira { sound, .. } => {
                let volume = channel_decibels_from_gain(gain);
                match sound {
                    KiraPlaybackHandle::Static(handle) => {
                        handle.set_volume(volume, Tween::default())
                    }
                    KiraPlaybackHandle::Streaming(handle) => {
                        handle.set_volume(volume, Tween::default())
                    }
                }
            }
            #[cfg(feature = "device")]
            Self::OpenAl { state, .. } => {
                let gain = normalized_gain(gain);
                match state {
                    OpenAlVoiceState::PendingStatic { settings, .. }
                    | OpenAlVoiceState::PendingStream { settings, .. } => settings.gain = gain,
                    OpenAlVoiceState::Active(source) => source.set_volume(gain),
                    OpenAlVoiceState::Stopped => {}
                }
            }
        }
    }

    fn set_playback_rate(&mut self, playback_rate: f32) {
        let playback_rate = channel_playback_rate(playback_rate);
        match self {
            #[cfg(test)]
            Self::Kira { sound, .. } => {
                let playback_rate = f64::from(playback_rate);
                match sound {
                    KiraPlaybackHandle::Static(handle) => {
                        handle.set_playback_rate(playback_rate, Tween::default())
                    }
                    KiraPlaybackHandle::Streaming(handle) => {
                        handle.set_playback_rate(playback_rate, Tween::default())
                    }
                }
            }
            #[cfg(feature = "device")]
            Self::OpenAl { state, .. } => match state {
                OpenAlVoiceState::PendingStatic { settings, .. }
                | OpenAlVoiceState::PendingStream { settings, .. } => {
                    settings.pitch = playback_rate;
                }
                OpenAlVoiceState::Active(source) => source.set_playback_rate(playback_rate),
                OpenAlVoiceState::Stopped => {}
            },
        }
    }

    fn set_position(&mut self, position: [f32; 3]) {
        match self {
            #[cfg(test)]
            Self::Kira { track, .. } => {
                if let Some(track) = track {
                    track.set_position(position, Tween::default());
                }
            }
            #[cfg(feature = "device")]
            Self::OpenAl { state, .. } => match state {
                OpenAlVoiceState::PendingStatic { settings, .. }
                | OpenAlVoiceState::PendingStream { settings, .. } => {
                    settings.position = position;
                }
                OpenAlVoiceState::Active(source) => source.set_position(position),
                OpenAlVoiceState::Stopped => {}
            },
        }
    }
}

impl PlayingSoundHandle {
    fn update_stream(&mut self) -> Result<()> {
        match self {
            #[cfg(test)]
            Self::Kira { .. } => Ok(()),
            #[cfg(feature = "device")]
            Self::OpenAl { state, .. } => {
                if let OpenAlVoiceState::Active(source) = state {
                    source.update_stream()?;
                }
                Ok(())
            }
        }
    }

    /// Mirrors `ChannelAccess.scheduleTick`: the instance keeps its handle
    /// shell until `soundDeleteTime`, but a stopped native channel returns to
    /// its static/streaming pool at the end of the current sound tick.
    fn release_stopped_channel(&mut self) {
        match self {
            #[cfg(test)]
            Self::Kira { .. } => {}
            #[cfg(feature = "device")]
            Self::OpenAl { state, .. } => {
                if let OpenAlVoiceState::Active(source) = state {
                    if !source.is_active() {
                        *state = OpenAlVoiceState::Stopped;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
fn channel_decibels_from_gain(gain: f32) -> Decibels {
    if !gain.is_finite() || gain <= 0.0 {
        Decibels::SILENCE
    } else {
        let gain = gain.min(1.0);
        Decibels((20.0 * gain.log10()).max(Decibels::SILENCE.0))
    }
}

#[cfg(test)]
fn identity_quaternion() -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}

#[cfg(test)]
fn listener_orientation(listener: AudioListenerState) -> [f32; 4] {
    let yaw = (180.0 - listener.y_rot).to_radians();
    let pitch = listener.x_rot.to_radians();
    quat_mul(quat_from_yaw(yaw), quat_from_pitch(pitch))
}

#[cfg(test)]
fn quat_from_yaw(yaw: f32) -> [f32; 4] {
    let half = yaw * 0.5;
    [0.0, half.sin(), 0.0, half.cos()]
}

#[cfg(test)]
fn quat_from_pitch(pitch: f32) -> [f32; 4] {
    let half = pitch * 0.5;
    [half.sin(), 0.0, 0.0, half.cos()]
}

#[cfg(test)]
fn quat_mul(lhs: [f32; 4], rhs: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = lhs;
    let [bx, by, bz, bw] = rhs;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "device"))]
    use std::{
        path::PathBuf,
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn kira_decibels_and_spatial_attenuation_match_vanilla() {
        let sound = ResolvedSound {
            event_id: "minecraft:entity.cat.ambient".to_string(),
            sound_name: "minecraft:mob/cat/meow1".to_string(),
            ogg_path: "sounds/mob/cat/meow1.ogg".into(),
            resource_generation: 0,
            stream: false,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        };

        assert_eq!(channel_decibels_from_gain(1.0), Decibels::IDENTITY);
        assert_eq!(channel_decibels_from_gain(2.0), Decibels::IDENTITY);
        assert_eq!(channel_decibels_from_gain(0.0), Decibels::SILENCE);
        assert_eq!(channel_decibels_from_gain(f32::NAN), Decibels::SILENCE);
        assert_eq!(spatial_max_distance(&sound, 2.0, None), 32.0);
    }

    #[test]
    #[cfg(feature = "device")]
    #[ignore = "requires BBB_TEST_OPENAL_LIBRARY and an explicitly selected OpenAL driver"]
    fn stopped_openal_channel_returns_pool_slot_while_instance_shell_remains() {
        let _receipt = crate::openal::OPENAL_RECEIPT_LOCK.lock().unwrap();
        let library = std::env::var_os("BBB_TEST_OPENAL_LIBRARY")
            .expect("BBB_TEST_OPENAL_LIBRARY must name the verified OpenAL library");
        let mut backend = OpenAlBackend::open(&library, None, false).unwrap();
        let source = backend
            .play_static(
                1,
                std::path::Path::new("schedule-tick.ogg"),
                || {
                    Ok(crate::residency::OpenAlStaticSound {
                        sample_rate: 48_000,
                        channels: 1,
                        samples: vec![0; 48_000].into(),
                    })
                },
                OpenAlSourceSettings {
                    position: [0.0; 3],
                    pitch: 1.0,
                    gain: 1.0,
                    looping: true,
                    relative: true,
                    max_distance: None,
                },
            )
            .unwrap();
        let mut handle = PlayingSoundHandle::OpenAl {
            state: OpenAlVoiceState::Active(source),
            streaming: false,
        };
        assert!(handle.holds_channel());
        handle.stop();
        assert!(handle.holds_channel());
        handle.release_stopped_channel();
        assert!(!handle.holds_channel());
        assert!(!handle.is_active());
    }

    #[test]
    fn kira_runtime_owns_single_kira_backend_variant() {
        let runtime = NativeAudioRuntime::new_with_resource_generation(2).unwrap();
        assert!(matches!(runtime.backend, ActiveAudioBackend::Kira(_)));
        assert_eq!(
            runtime.channel_observation().capacity_origin,
            KIRA_CHANNEL_CAPACITY_ORIGIN
        );
        #[cfg(feature = "device")]
        {
            let mut runtime = runtime;
            assert!(runtime.audio_device_observation().is_none());
            let (sender, _receiver) = std::sync::mpsc::sync_channel(1);
            assert!(runtime
                .begin_recording(sender)
                .unwrap_err()
                .to_string()
                .contains("native OpenAL playback"));
        }
    }

    #[test]
    #[cfg(not(feature = "device"))]
    fn kira_capacity_has_headroom_for_both_logical_pools() {
        let mut manager =
            AudioManager::<DefaultBackend>::new(kira_audio_manager_settings()).unwrap();

        assert_eq!(manager.main_track().sound_capacity(), 256);
        assert_eq!(manager.sub_track_capacity(), 256);
    }

    #[test]
    #[cfg(not(feature = "device"))]
    fn stopped_instance_shell_remains_projected_until_twentieth_tick() {
        use std::sync::Arc;

        use kira::{
            sound::static_sound::{StaticSoundData, StaticSoundSettings},
            Frame,
        };

        use crate::scheduler::{ScheduledVoice, MINIMUM_SOURCE_LIFETIME_TICKS};

        fn kira_manager_mut(runtime: &mut NativeAudioRuntime) -> &mut AudioManager<DefaultBackend> {
            match &mut runtime.backend {
                ActiveAudioBackend::Kira(kira) => &mut kira.manager,
                #[cfg(feature = "device")]
                ActiveAudioBackend::OpenAl(_) => unreachable!("expected Kira backend"),
            }
        }

        let mut runtime = NativeAudioRuntime::new().unwrap();
        let data = StaticSoundData {
            sample_rate: 1,
            frames: Arc::from(vec![Frame::ZERO; 128]),
            settings: StaticSoundSettings::default(),
            slice: None,
        };
        let handle = kira_manager_mut(&mut runtime).play(data.clone()).unwrap();
        kira_manager_mut(&mut runtime)
            .backend_mut()
            .on_start_processing();
        kira_manager_mut(&mut runtime).backend_mut().process();
        runtime.scheduler.playing.push(ScheduledVoice {
            event_id: "minecraft:music.test".to_owned(),
            sound_name: "minecraft:music/game/test1".to_owned(),
            category: AudioCategory::Music,
            entity_id: None,
            position: [0.0; 3],
            jukebox_pos: None,
            handle: PlayingSoundHandle::Kira {
                sound: KiraPlaybackHandle::Static(handle),
                track: None,
            },
            tickable: false,
            ambient_loop: false,
            base_channel_gain: 1.0,
            playback_rate: 1.0,
            minimum_delete_tick: MINIMUM_SOURCE_LIFETIME_TICKS,
        });
        runtime.stop_sounds(&StopSoundCommand {
            category: None,
            name: Some("minecraft:music.test".to_owned()),
        });
        kira_manager_mut(&mut runtime)
            .backend_mut()
            .on_start_processing();
        kira_manager_mut(&mut runtime).backend_mut().process();
        assert!(!runtime.scheduler.playing[0].handle.is_active());

        let empty_tick = TickEntitySoundPositionsCommand {
            listener: None,
            entities: Vec::new(),
            tickable_sounds: Vec::new(),
        };
        for expected_tick in 1..MINIMUM_SOURCE_LIFETIME_TICKS {
            runtime.tick_entity_sound_positions(&empty_tick).unwrap();
            assert_eq!(runtime.scheduler.tick_count, expected_tick);
            assert_eq!(
                runtime.active_music_event_id(),
                Some("minecraft:music.test")
            );
            assert_eq!(
                runtime.active_music_sound_name(),
                Some("minecraft:music/game/test1")
            );
        }
        runtime.tick_entity_sound_positions(&empty_tick).unwrap();
        assert_eq!(runtime.scheduler.tick_count, MINIMUM_SOURCE_LIFETIME_TICKS);
        assert_eq!(runtime.active_music_event_id(), None);
        assert_eq!(runtime.active_music_sound_name(), None);
        assert!(runtime.scheduler.playing.is_empty());

        let stop_all_handle = kira_manager_mut(&mut runtime).play(data).unwrap();
        kira_manager_mut(&mut runtime)
            .backend_mut()
            .on_start_processing();
        kira_manager_mut(&mut runtime).backend_mut().process();
        runtime.scheduler.playing.push(ScheduledVoice {
            event_id: "minecraft:music.stop_all".to_owned(),
            sound_name: "minecraft:music/game/stop_all1".to_owned(),
            category: AudioCategory::Music,
            entity_id: None,
            position: [0.0; 3],
            jukebox_pos: None,
            handle: PlayingSoundHandle::Kira {
                sound: KiraPlaybackHandle::Static(stop_all_handle),
                track: None,
            },
            tickable: false,
            ambient_loop: false,
            base_channel_gain: 1.0,
            playback_rate: 1.0,
            minimum_delete_tick: runtime
                .scheduler
                .tick_count
                .saturating_add(MINIMUM_SOURCE_LIFETIME_TICKS),
        });
        runtime.stop_sounds(&StopSoundCommand {
            category: None,
            name: None,
        });
        assert!(runtime.scheduler.playing.is_empty());
    }

    #[test]
    #[cfg(not(feature = "device"))]
    fn resource_generation_reset_preserves_engine_clock_and_diagnostics() {
        let mut runtime = NativeAudioRuntime::new_with_resource_generation(4).unwrap();
        runtime.scheduler.tick_count = 17;
        runtime.scheduler.channel_admission_rejections = 3;

        runtime.reset_resource_generation(5);

        assert_eq!(runtime.scheduler.tick_count, 17);
        assert_eq!(runtime.scheduler.channel_admission_rejections, 3);
        assert_eq!(runtime.residency.active_generation(), 5);
    }

    #[test]
    fn listener_orientation_matches_kira_forward_at_minecraft_north() {
        assert_eq!(
            listener_orientation(AudioListenerState {
                position: [0.0, 0.0, 0.0],
                y_rot: 180.0,
                x_rot: 0.0,
            }),
            identity_quaternion()
        );

        let yaw_quarter = listener_orientation(AudioListenerState {
            position: [0.0, 0.0, 0.0],
            y_rot: 90.0,
            x_rot: 0.0,
        });
        let expected = std::f32::consts::FRAC_1_SQRT_2;
        assert!((yaw_quarter[1] - expected).abs() < 0.0001);
        assert!((yaw_quarter[3] - expected).abs() < 0.0001);
    }

    #[cfg(not(feature = "device"))]
    #[test]
    fn preload_completion_installs_current_generation_without_a_playback_decode_miss() {
        let path = unique_preload_path("runtime-install");
        crate::preload::write_test_ogg(&path);
        let mut runtime = NativeAudioRuntime::new_with_resource_generation(11).unwrap();
        runtime.request_preload(vec![AudioPreloadRequest {
            sound_name: "minecraft:test/preloaded".to_string(),
            ogg_path: path.clone(),
            resource_generation: 11,
        }]);

        let deadline = Instant::now() + Duration::from_secs(5);
        while !runtime.poll_audio_preload() {
            assert!(Instant::now() < deadline, "audio preload timed out");
            thread::yield_now();
        }

        let observation = runtime.preload_observation();
        assert_eq!(observation.resource_generation, 11);
        assert_eq!(observation.requested, 1);
        assert_eq!(observation.completed, 1);
        assert_eq!(observation.failures, 0);
        assert_eq!(observation.pending, 0);
        let sound = ResolvedSound {
            event_id: "minecraft:test.event".to_string(),
            sound_name: "minecraft:test/preloaded".to_string(),
            ogg_path: path.clone(),
            resource_generation: 11,
            stream: false,
            preload: true,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        };
        runtime.residency.decoded_static_sound(&sound).unwrap();
        let residency = runtime.residency_observation();
        assert_eq!(residency.static_buffers, 1);
        assert_eq!(residency.static_cache_hits, 1);
        assert_eq!(residency.static_cache_misses, 0);
        std::fs::remove_file(path).unwrap();
    }

    #[cfg(not(feature = "device"))]
    #[test]
    fn generation_reset_cancels_pending_preload_and_rejects_old_requests() {
        let mut runtime = NativeAudioRuntime::new_with_resource_generation(3).unwrap();
        runtime.request_preload(vec![AudioPreloadRequest {
            sound_name: "minecraft:test/missing".to_string(),
            ogg_path: PathBuf::from("missing-generation-three.ogg"),
            resource_generation: 3,
        }]);

        runtime.reset_resource_generation(4);
        runtime.request_preload(vec![AudioPreloadRequest {
            sound_name: "minecraft:test/stale".to_string(),
            ogg_path: PathBuf::from("missing-generation-three.ogg"),
            resource_generation: 3,
        }]);

        let observation = runtime.preload_observation();
        assert_eq!(observation.resource_generation, 4);
        assert_eq!(observation.requested, 1);
        assert_eq!(observation.failures, 1);
        assert_eq!(observation.pending, 0);
        assert_eq!(observation.cancelled_jobs, 1);
        assert!(observation
            .last_error
            .unwrap()
            .contains("active generation 4"));
    }

    #[cfg(not(feature = "device"))]
    fn unique_preload_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-client-audio-runtime-{label}-{nanos}.ogg"))
    }
}
