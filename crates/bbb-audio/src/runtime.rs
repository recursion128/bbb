use std::collections::{HashMap, HashSet, VecDeque};

#[cfg(feature = "device")]
use std::path::{Path, PathBuf};

#[cfg(feature = "device")]
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use kira::{
    listener::ListenerHandle,
    sound::{static_sound::StaticSoundHandle, streaming::StreamingSoundHandle, PlaybackState},
    track::{MainTrackBuilder, SpatialTrackBuilder, SpatialTrackHandle},
    AudioManager, AudioManagerSettings, Capacities, Decibels, DefaultBackend, Tween,
};

use crate::{
    preload::{AudioPreloadObservation, AudioPreloadRequest, AudioPreloadWorker},
    residency::{AudioDecodeError, AudioResidency},
    AudioCategory, AudioCommand, AudioListenerState, AudioResidencyObservation,
    EntitySoundPosition, PlayPositionedSoundCommand, ResolvedSound, StopJukeboxSongCommand,
    StopSoundCommand, TickEntitySoundPositionsCommand, TickableEntitySoundUpdate,
};

#[cfg(feature = "device")]
use crate::openal::{OpenAlBackend, OpenAlSoundSource, OpenAlSourceSettings};

#[cfg(feature = "device")]
use crate::device::{
    audio_device_reload_reason, initial_audio_devices, is_sound_device_default,
    AudioDeviceInventory, AudioDeviceObservation, AudioDevicePoll, AudioDeviceReloadReason,
    AudioDeviceTracker,
};

const MIN_SPATIAL_DISTANCE: f32 = 1.0;
const DISTANCE_DELAY_THRESHOLD_SQR: f64 = 100.0;
const DISTANCE_DELAY_TICKS_PER_BLOCK: f64 = 0.5;
const MAX_PLAYBACK_EVENTS: usize = 1024;
const MINIMUM_SOURCE_LIFETIME_TICKS: u64 = 20;
// Kira exposes mixer-slot limits rather than OpenAL's ALC_MONO_SOURCES. Keep
// the pre-existing aggregate of 256 slots as an explicitly backend-logical
// source contract until an OpenAL capability owner replaces this origin. The
// physical Kira paths are each widened to that aggregate so only the Vanilla
// static/streaming admission pools below decide which class is saturated.
const KIRA_LOGICAL_MONO_SOURCES: usize = 256;
const KIRA_CHANNEL_CAPACITY_ORIGIN: &str = "kira-logical-main-plus-spatial";
#[cfg(feature = "device")]
const DEVICE_TICK_INTERVAL: Duration = Duration::from_millis(50);

pub struct KiraAudioRuntime {
    manager: Option<AudioManager<DefaultBackend>>,
    listener: Option<ListenerHandle>,
    listener_position: [f64; 3],
    delayed_positioned: Vec<KiraDelayedPositionedSound>,
    playing: Vec<KiraPlayingSound>,
    music_gain: f32,
    residency: AudioResidency,
    channel_limits: AudioChannelLimits,
    channel_capacity_origin: &'static str,
    channel_admission_rejections: u64,
    tick_count: u64,
    playback_events: VecDeque<AudioPlaybackEvent>,
    listener_orientation: [f32; 4],
    preload_worker: AudioPreloadWorker,
    preload_observation: AudioPreloadObservation,
    preload_token: Option<u64>,
    #[cfg(feature = "device")]
    device: Option<KiraAudioDeviceRuntime>,
    #[cfg(feature = "device")]
    openal: Option<OpenAlBackend>,
}

#[cfg(feature = "device")]
struct KiraAudioDeviceRuntime {
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

#[derive(Debug, Clone, PartialEq)]
pub struct AudioPlaybackEvent {
    pub event_id: String,
    pub position: [f64; 3],
    pub range: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioChannelObservation {
    pub static_used: usize,
    pub streaming_used: usize,
    pub static_capacity: usize,
    pub streaming_capacity: usize,
    pub admission_rejections: u64,
    pub native_static_buffers: usize,
    pub native_static_bytes: u64,
    pub native_static_evictions: u64,
    /// Exact capability owner for the active backend.
    pub capacity_origin: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AudioChannelLimits {
    static_capacity: usize,
    streaming_capacity: usize,
}

#[derive(Debug, Clone)]
struct KiraDelayedPositionedSound {
    command: PlayPositionedSoundCommand,
    remaining_ticks: u32,
}

struct KiraPlayingSound {
    event_id: String,
    category: AudioCategory,
    entity_id: Option<i32>,
    jukebox_pos: Option<[i32; 3]>,
    handle: KiraSoundHandle,
    track: Option<SpatialTrackHandle>,
    tickable: bool,
    ambient_loop: bool,
    base_channel_gain: f32,
    minimum_delete_tick: u64,
}

enum KiraSoundHandle {
    Static(StaticSoundHandle),
    Streaming(StreamingSoundHandle<AudioDecodeError>),
    #[cfg(feature = "device")]
    OpenAl {
        source: Option<OpenAlSoundSource>,
        streaming: bool,
    },
}

impl KiraAudioRuntime {
    pub fn new() -> Result<Self> {
        Self::new_with_resource_generation(0)
    }

    pub fn new_with_resource_generation(resource_generation: u64) -> Result<Self> {
        let manager = AudioManager::<DefaultBackend>::new(kira_audio_manager_settings())
            .map_err(|_| anyhow!("initialize Kira audio manager"))?;
        Self::from_manager(manager, resource_generation, None)
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
        let device = KiraAudioDeviceRuntime {
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
            next_poll_at: Instant::now(),
        };
        Self::from_parts(
            None,
            None,
            resource_generation,
            Some(device),
            Some(openal),
            AudioChannelLimits::from_mono_sources(mono_sources),
            "openal-alc-mono-sources",
        )
    }

    fn from_manager(
        mut manager: AudioManager<DefaultBackend>,
        resource_generation: u64,
        #[cfg(feature = "device")] device: Option<KiraAudioDeviceRuntime>,
        #[cfg(not(feature = "device"))] _device: Option<()>,
    ) -> Result<Self> {
        let listener = manager
            .add_listener([0.0, 0.0, 0.0], identity_quaternion())
            .map_err(|_| anyhow!("initialize Kira audio listener"))?;
        Self::from_parts(
            Some(manager),
            Some(listener),
            resource_generation,
            #[cfg(feature = "device")]
            device,
            #[cfg(not(feature = "device"))]
            _device,
            #[cfg(feature = "device")]
            None,
            AudioChannelLimits::from_mono_sources(KIRA_LOGICAL_MONO_SOURCES),
            KIRA_CHANNEL_CAPACITY_ORIGIN,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        manager: Option<AudioManager<DefaultBackend>>,
        listener: Option<ListenerHandle>,
        resource_generation: u64,
        #[cfg(feature = "device")] device: Option<KiraAudioDeviceRuntime>,
        #[cfg(not(feature = "device"))] _device: Option<()>,
        #[cfg(feature = "device")] openal: Option<OpenAlBackend>,
        channel_limits: AudioChannelLimits,
        channel_capacity_origin: &'static str,
    ) -> Result<Self> {
        let preload_worker = AudioPreloadWorker::spawn()
            .map_err(|error| anyhow!("initialize audio preload worker: {error}"))?;
        Ok(Self {
            manager,
            listener,
            listener_position: [0.0, 0.0, 0.0],
            delayed_positioned: Vec::new(),
            playing: Vec::new(),
            music_gain: 1.0,
            residency: AudioResidency::new(resource_generation),
            channel_limits,
            channel_capacity_origin,
            channel_admission_rejections: 0,
            tick_count: 0,
            playback_events: VecDeque::new(),
            listener_orientation: identity_quaternion(),
            preload_worker,
            preload_observation: AudioPreloadObservation {
                resource_generation,
                ..AudioPreloadObservation::default()
            },
            preload_token: None,
            #[cfg(feature = "device")]
            device,
            #[cfg(feature = "device")]
            openal,
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
        let (static_used, streaming_used) = active_channel_counts(
            self.playing
                .iter()
                .map(|sound| (sound.handle.is_streaming(), sound.handle.holds_channel())),
        );
        #[cfg(feature = "device")]
        let (native_static_buffers, native_static_bytes, native_static_evictions) = self
            .openal
            .as_ref()
            .map(OpenAlBackend::static_cache_observation)
            .unwrap_or_default();
        #[cfg(not(feature = "device"))]
        let (native_static_buffers, native_static_bytes, native_static_evictions) = (0, 0, 0);
        AudioChannelObservation {
            static_used,
            streaming_used,
            static_capacity: self.channel_limits.static_capacity,
            streaming_capacity: self.channel_limits.streaming_capacity,
            admission_rejections: self.channel_admission_rejections,
            native_static_buffers,
            native_static_bytes,
            native_static_evictions,
            capacity_origin: self.channel_capacity_origin,
        }
    }

    #[cfg(feature = "device")]
    pub fn audio_device_observation(&self) -> Option<AudioDeviceObservation> {
        let device = self.device.as_ref()?;
        let openal = self.openal.as_ref()?.observation();
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
    /// The per-event fast path only probes bounded channels maintained by Kira
    /// and the one background tracker; host enumeration stays off-thread.
    #[cfg(feature = "device")]
    pub fn poll_audio_device(&mut self) -> AudioDevicePoll {
        let Some(mut device) = self.device.take() else {
            return AudioDevicePoll::default();
        };
        let now = Instant::now();
        if now < device.next_poll_at {
            self.device = Some(device);
            return AudioDevicePoll::default();
        }
        device.next_poll_at = now + DEVICE_TICK_INTERVAL;
        let mut disconnected = false;
        let mut observation_changed = false;
        if self
            .openal
            .as_ref()
            .is_some_and(OpenAlBackend::is_disconnected)
        {
            disconnected = true;
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
                    self.device = Some(device);
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
            self.device = Some(device);
            return AudioDevicePoll {
                observation_changed,
                ..AudioDevicePoll::default()
            };
        }
        let inventory = next_inventory.unwrap_or_else(|| {
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
            self.device = Some(device);
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
                self.device = Some(device);
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
                self.device = Some(device);
                AudioDevicePoll {
                    reload_reason: None,
                    error: Some(error),
                    observation_changed: true,
                }
            }
        }
    }

    #[cfg(feature = "device")]
    fn replace_openal_backend(&mut self, backend: OpenAlBackend) -> Result<()> {
        let previous_backend = self.openal.take();
        for sound in &mut self.playing {
            sound.handle.stop();
        }
        self.playing.clear();
        if let Err(error) = backend.set_listener([0.0; 3], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]) {
            self.openal = previous_backend;
            return Err(error);
        }
        // The listener call restores the new context after stopping old
        // sources. Destroying the old owner now leaves the new context current.
        drop(previous_backend);
        self.delayed_positioned.clear();
        self.playback_events.clear();
        let resource_generation = self.residency.active_generation();
        self.cancel_audio_preload(resource_generation);
        self.residency.reset_generation(resource_generation);
        self.listener_position = [0.0; 3];
        self.listener_orientation = identity_quaternion();
        self.channel_limits =
            AudioChannelLimits::from_mono_sources(backend.observation().mono_sources);
        self.channel_capacity_origin = backend.observation().capacity_origin;
        self.openal = Some(backend);
        Ok(())
    }

    /// Returns the most recently started music-category instance retained by
    /// the Vanilla-compatible lifetime owner. A stopped backend handle remains
    /// active to simulation until its minimum delete tick is processed.
    pub fn active_music_event_id(&self) -> Option<&str> {
        latest_retained_music_event_id(
            self.playing
                .iter()
                .map(|sound| (sound.event_id.as_str(), &sound.category)),
        )
    }

    pub fn drain_playback_events(&mut self) -> impl Iterator<Item = AudioPlaybackEvent> + '_ {
        self.playback_events.drain(..)
    }

    pub fn reset_resource_generation(&mut self, resource_generation: u64) {
        self.delayed_positioned.clear();
        self.playback_events.clear();
        for sound in &mut self.playing {
            sound.handle.stop();
        }
        self.playing.clear();
        #[cfg(feature = "device")]
        if let Some(openal) = self.openal.as_mut() {
            openal.clear_static_buffers();
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

    pub fn handle_command(&mut self, command: &AudioCommand) -> Result<()> {
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
        self.retain_active_sounds();
        push_bounded_playback_event(&mut self.playback_events, local_playback_event(sound));
        if !sound_should_start(channel_gain, &category) {
            return Ok(());
        }
        if !self.try_admit_channel(sound.stream) {
            return Ok(());
        }
        let effective_gain = effective_channel_gain(&category, channel_gain, self.music_gain);
        let volume = channel_decibels_from_gain(effective_gain);
        let playback_rate = channel_playback_rate(playback_rate);
        #[cfg(feature = "device")]
        let handle = if let Some(openal) = self.openal.as_mut() {
            let settings = OpenAlSourceSettings {
                position: [0.0, 0.0, 0.0],
                pitch: playback_rate,
                gain: effective_gain,
                looping: looping && !sound.stream,
                relative: true,
                max_distance: None,
            };
            let source = if sound.stream {
                let decoder = self.residency.openal_streaming_sound(sound, looping)?;
                openal.play_stream(decoder, settings)?
            } else {
                let residency = &mut self.residency;
                openal.play_static(
                    sound.resource_generation,
                    &sound.ogg_path,
                    || residency.openal_static_sound(sound),
                    settings,
                )?
            };
            KiraSoundHandle::OpenAl {
                source: Some(source),
                streaming: sound.stream,
            }
        } else if sound.stream {
            let data = self
                .residency
                .streaming_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Streaming(
                self.manager
                    .as_mut()
                    .expect("Kira backend exists when OpenAL is absent")
                    .play(data)?,
            )
        } else {
            let data = self
                .residency
                .static_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Static(
                self.manager
                    .as_mut()
                    .expect("Kira backend exists when OpenAL is absent")
                    .play(data)?,
            )
        };
        #[cfg(not(feature = "device"))]
        let handle = if sound.stream {
            let data = self
                .residency
                .streaming_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Streaming(
                self.manager
                    .as_mut()
                    .expect("Kira backend exists")
                    .play(data)?,
            )
        } else {
            let data = self
                .residency
                .static_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Static(
                self.manager
                    .as_mut()
                    .expect("Kira backend exists")
                    .play(data)?,
            )
        };
        self.playing.push(KiraPlayingSound {
            event_id: sound.event_id.clone(),
            category,
            entity_id: None,
            jukebox_pos: None,
            handle,
            track: None,
            tickable: false,
            ambient_loop,
            base_channel_gain: channel_gain,
            minimum_delete_tick: self
                .tick_count
                .saturating_add(MINIMUM_SOURCE_LIFETIME_TICKS),
        });
        Ok(())
    }

    fn play_positioned_sound(&mut self, command: &PlayPositionedSoundCommand) -> Result<()> {
        self.retain_active_sounds();
        if let Some(delayed) = delayed_positioned_sound_for_command(command, self.listener_position)
        {
            self.delayed_positioned.push(delayed);
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
        self.retain_active_sounds();
        push_bounded_playback_event(
            &mut self.playback_events,
            spatial_playback_event(sound, position, gain, linear_attenuation),
        );
        if !tickable && !sound_should_start(channel_gain, &category) {
            return Ok(());
        }
        if !self.try_admit_channel(sound.stream) {
            return Ok(());
        }
        let mut track = if linear_attenuation && !self.uses_openal() {
            Some(
                self.manager
                    .as_mut()
                    .expect("Kira backend exists when OpenAL is absent")
                    .add_spatial_sub_track(
                        self.listener
                            .as_ref()
                            .expect("Kira listener exists with Kira backend"),
                        audio_position(position),
                        SpatialTrackBuilder::new()
                            .persist_until_sounds_finish(true)
                            .distances((
                                MIN_SPATIAL_DISTANCE,
                                spatial_max_distance(sound, gain, fixed_range),
                            )),
                    )?,
            )
        } else {
            None
        };
        let volume = channel_decibels_from_gain(channel_gain);
        let playback_rate = channel_playback_rate(playback_rate);
        #[cfg(feature = "device")]
        let handle = if let Some(openal) = self.openal.as_mut() {
            let settings = OpenAlSourceSettings {
                position: audio_position(position),
                pitch: playback_rate,
                gain: channel_gain,
                looping: looping && !sound.stream,
                relative: false,
                max_distance: linear_attenuation
                    .then(|| spatial_max_distance(sound, gain, fixed_range)),
            };
            let source = if sound.stream {
                let decoder = self.residency.openal_streaming_sound(sound, looping)?;
                openal.play_stream(decoder, settings)?
            } else {
                let residency = &mut self.residency;
                openal.play_static(
                    sound.resource_generation,
                    &sound.ogg_path,
                    || residency.openal_static_sound(sound),
                    settings,
                )?
            };
            KiraSoundHandle::OpenAl {
                source: Some(source),
                streaming: sound.stream,
            }
        } else if sound.stream {
            let data = self
                .residency
                .streaming_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Streaming(if let Some(track) = track.as_mut() {
                track.play(data)?
            } else {
                self.manager
                    .as_mut()
                    .expect("Kira backend exists when OpenAL is absent")
                    .play(data)?
            })
        } else {
            let data = self
                .residency
                .static_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Static(if let Some(track) = track.as_mut() {
                track.play(data)?
            } else {
                self.manager
                    .as_mut()
                    .expect("Kira backend exists when OpenAL is absent")
                    .play(data)?
            })
        };
        #[cfg(not(feature = "device"))]
        let handle = if sound.stream {
            let data = self
                .residency
                .streaming_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Streaming(if let Some(track) = track.as_mut() {
                track.play(data)?
            } else {
                self.manager
                    .as_mut()
                    .expect("Kira backend exists")
                    .play(data)?
            })
        } else {
            let data = self
                .residency
                .static_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Static(if let Some(track) = track.as_mut() {
                track.play(data)?
            } else {
                self.manager
                    .as_mut()
                    .expect("Kira backend exists")
                    .play(data)?
            })
        };
        self.playing.push(KiraPlayingSound {
            event_id: sound.event_id.clone(),
            category,
            entity_id,
            jukebox_pos,
            handle,
            track,
            tickable,
            ambient_loop: false,
            base_channel_gain: channel_gain,
            minimum_delete_tick: self
                .tick_count
                .saturating_add(MINIMUM_SOURCE_LIFETIME_TICKS),
        });
        Ok(())
    }

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
        stop_delayed_positioned_sounds(&mut self.delayed_positioned, command);
        if stop_command_is_stop_all(command) {
            for sound in &mut self.playing {
                sound.handle.stop();
            }
            self.playing.clear();
            return;
        }
        for sound in &mut self.playing {
            if sound_matches_stop(sound, command) {
                sound.handle.stop();
            }
        }
    }

    fn stop_jukebox_song(&mut self, command: &StopJukeboxSongCommand) {
        for sound in &mut self.playing {
            if sound.jukebox_pos == Some(command.jukebox_pos) {
                sound.handle.stop();
            }
        }
    }

    fn set_music_gain(&mut self, gain: f32) {
        self.music_gain = normalized_gain(gain);
        for sound in &mut self.playing {
            if sound.category == AudioCategory::Music {
                sound.handle.set_volume(effective_channel_gain(
                    &sound.category,
                    sound.base_channel_gain,
                    self.music_gain,
                ));
            }
        }
    }

    fn sync_ambient_loops(&mut self, desired: &[crate::AmbientLoopSoundCommand]) -> Result<()> {
        self.retain_active_sounds();
        let plan = ambient_loop_sync_plan(
            self.playing
                .iter()
                .filter(|sound| sound.ambient_loop)
                .map(|sound| sound.event_id.as_str()),
            desired,
        );
        for sound in &mut self.playing {
            if !sound.ambient_loop {
                continue;
            }
            let Some(desired_index) = plan
                .updates
                .iter()
                .find_map(|(event_id, index)| (event_id == &sound.event_id).then_some(*index))
            else {
                sound.handle.stop();
                continue;
            };
            let desired = &desired[desired_index];
            sound.base_channel_gain = desired.channel_gain;
            sound.handle.set_volume(desired.channel_gain);
        }
        self.retain_active_sounds();
        for desired_index in plan.starts {
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
        self.tick_count = self.tick_count.saturating_add(1);
        if let Some(listener) = command.listener {
            self.listener_position = listener.position;
            self.listener_orientation = listener_orientation(listener);
            #[cfg(feature = "device")]
            if let Some(openal) = self.openal.as_ref() {
                let (forward, up) = listener_vectors(listener);
                openal.set_listener(audio_position(listener.position), forward, up)?;
            } else {
                let kira_listener = self
                    .listener
                    .as_mut()
                    .expect("Kira listener exists when OpenAL is absent");
                kira_listener.set_position(audio_position(listener.position), Tween::default());
                kira_listener.set_orientation(self.listener_orientation, Tween::default());
            }
            #[cfg(not(feature = "device"))]
            {
                let kira_listener = self.listener.as_mut().expect("Kira listener exists");
                kira_listener.set_position(audio_position(listener.position), Tween::default());
                kira_listener.set_orientation(self.listener_orientation, Tween::default());
            }
        }
        for sound in &mut self.playing {
            sound.handle.update_stream()?;
        }
        let active_entity_ids = active_entity_sound_ids(&command.entities);
        let active_tickable_sounds = active_tickable_entity_sounds(&command.tickable_sounds);
        for sound in &mut self.playing {
            if entity_bound_sound_is_missing(sound.entity_id, &active_entity_ids) {
                sound.handle.stop();
            }
            if tickable_entity_sound_is_missing(sound, &active_tickable_sounds) {
                sound.handle.stop();
            }
        }
        for entity in &command.entities {
            self.update_entity_sound_position(*entity);
        }
        for sound in &command.tickable_sounds {
            self.update_tickable_entity_sound(sound);
        }
        self.retain_active_sounds();
        for command in advance_delayed_positioned_sounds(&mut self.delayed_positioned) {
            self.play_positioned_sound_now(&command)?;
        }
        for sound in &mut self.playing {
            sound.handle.release_stopped_channel();
        }
        Ok(())
    }

    fn update_entity_sound_position(&mut self, entity: EntitySoundPosition) {
        for sound in &mut self.playing {
            if sound.entity_id == Some(entity.entity_id) {
                if let Some(track) = &mut sound.track {
                    track.set_position(audio_position(entity.position), Tween::default());
                }
                sound.handle.set_position(audio_position(entity.position));
            }
        }
    }

    fn update_tickable_entity_sound(&mut self, update: &TickableEntitySoundUpdate) {
        for sound in &mut self.playing {
            if sound.tickable
                && sound.entity_id == Some(update.entity_id)
                && sound.event_id == update.event_id
            {
                if let Some(track) = &mut sound.track {
                    track.set_position(audio_position(update.position), Tween::default());
                }
                sound.handle.set_position(audio_position(update.position));
                sound.handle.set_volume(update.channel_gain);
                sound.base_channel_gain = update.channel_gain;
                sound.handle.set_playback_rate(update.playback_rate);
            }
        }
    }

    fn retain_active_sounds(&mut self) {
        let tick_count = self.tick_count;
        self.playing
            .retain(|sound| sound.is_retained_at(tick_count));
    }

    fn try_admit_channel(&mut self, streaming: bool) -> bool {
        let (static_used, streaming_used) = active_channel_counts(
            self.playing
                .iter()
                .map(|sound| (sound.handle.is_streaming(), sound.handle.holds_channel())),
        );
        if self
            .channel_limits
            .admits(streaming, static_used, streaming_used)
        {
            true
        } else {
            self.channel_admission_rejections = self.channel_admission_rejections.saturating_add(1);
            false
        }
    }

    fn uses_openal(&self) -> bool {
        #[cfg(feature = "device")]
        {
            self.openal.is_some()
        }
        #[cfg(not(feature = "device"))]
        {
            false
        }
    }
}

fn kira_capacities() -> Capacities {
    Capacities {
        sub_track_capacity: KIRA_LOGICAL_MONO_SOURCES,
        ..Capacities::default()
    }
}

fn kira_main_track_builder() -> MainTrackBuilder {
    MainTrackBuilder::default().sound_capacity(KIRA_LOGICAL_MONO_SOURCES)
}

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

impl KiraSoundHandle {
    fn stop(&mut self) {
        match self {
            Self::Static(handle) => handle.stop(Tween::default()),
            Self::Streaming(handle) => handle.stop(Tween::default()),
            #[cfg(feature = "device")]
            Self::OpenAl { source, .. } => {
                if let Some(source) = source {
                    source.stop();
                }
            }
        }
    }

    fn state(&self) -> PlaybackState {
        match self {
            Self::Static(handle) => handle.state(),
            Self::Streaming(handle) => handle.state(),
            #[cfg(feature = "device")]
            Self::OpenAl { source, .. } => {
                if source.as_ref().is_some_and(OpenAlSoundSource::is_active) {
                    PlaybackState::Playing
                } else {
                    PlaybackState::Stopped
                }
            }
        }
    }

    fn is_streaming(&self) -> bool {
        match self {
            Self::Streaming(_) => true,
            Self::Static(_) => false,
            #[cfg(feature = "device")]
            Self::OpenAl { streaming, .. } => *streaming,
        }
    }

    fn is_active(&self) -> bool {
        self.state() != PlaybackState::Stopped
    }

    fn holds_channel(&self) -> bool {
        match self {
            Self::Static(_) | Self::Streaming(_) => self.is_active(),
            #[cfg(feature = "device")]
            Self::OpenAl { source, .. } => source.is_some(),
        }
    }

    fn set_volume(&mut self, gain: f32) {
        let volume = channel_decibels_from_gain(gain);
        match self {
            Self::Static(handle) => handle.set_volume(volume, Tween::default()),
            Self::Streaming(handle) => handle.set_volume(volume, Tween::default()),
            #[cfg(feature = "device")]
            Self::OpenAl { source, .. } => {
                if let Some(source) = source {
                    source.set_volume(normalized_gain(gain));
                }
            }
        }
    }

    fn set_playback_rate(&mut self, playback_rate: f32) {
        let playback_rate = f64::from(channel_playback_rate(playback_rate));
        match self {
            Self::Static(handle) => handle.set_playback_rate(playback_rate, Tween::default()),
            Self::Streaming(handle) => handle.set_playback_rate(playback_rate, Tween::default()),
            #[cfg(feature = "device")]
            Self::OpenAl { source, .. } => {
                if let Some(source) = source {
                    source.set_playback_rate(playback_rate as f32);
                }
            }
        }
    }

    fn set_position(&mut self, position: [f32; 3]) {
        #[cfg(feature = "device")]
        if let Self::OpenAl { source, .. } = self {
            if let Some(source) = source {
                source.set_position(position);
            }
        }
        #[cfg(not(feature = "device"))]
        let _ = position;
    }

    fn update_stream(&mut self) -> Result<()> {
        #[cfg(feature = "device")]
        if let Self::OpenAl { source, .. } = self {
            if let Some(source) = source {
                source.update_stream()?;
            }
        }
        Ok(())
    }

    /// Mirrors `ChannelAccess.scheduleTick`: the instance keeps its handle
    /// shell until `soundDeleteTime`, but a stopped native channel returns to
    /// its static/streaming pool at the end of the current sound tick.
    fn release_stopped_channel(&mut self) {
        #[cfg(feature = "device")]
        if let Self::OpenAl { source, .. } = self {
            if source.as_ref().is_some_and(|source| !source.is_active()) {
                source.take();
            }
        }
    }
}

impl KiraPlayingSound {
    fn is_retained_at(&self, tick_count: u64) -> bool {
        self.handle.is_active() || stopped_sound_is_retained(tick_count, self.minimum_delete_tick)
    }
}

fn stopped_sound_is_retained(tick_count: u64, minimum_delete_tick: u64) -> bool {
    tick_count < minimum_delete_tick
}

fn sound_matches_stop(sound: &KiraPlayingSound, command: &StopSoundCommand) -> bool {
    stop_filter_matches(&sound.event_id, &sound.category, command)
}

fn stop_command_is_stop_all(command: &StopSoundCommand) -> bool {
    command.category.is_none() && command.name.is_none()
}

fn latest_retained_music_event_id<'a>(
    sounds: impl DoubleEndedIterator<Item = (&'a str, &'a AudioCategory)>,
) -> Option<&'a str> {
    sounds
        .rev()
        .find_map(|(event_id, category)| (category == &AudioCategory::Music).then_some(event_id))
}

fn active_channel_counts(sounds: impl IntoIterator<Item = (bool, bool)>) -> (usize, usize) {
    sounds.into_iter().filter(|(_, active)| *active).fold(
        (0usize, 0usize),
        |(static_used, streaming_used), (streaming, _)| {
            if streaming {
                (static_used, streaming_used.saturating_add(1))
            } else {
                (static_used.saturating_add(1), streaming_used)
            }
        },
    )
}

impl AudioChannelLimits {
    /// Vanilla 26.1 `Library.init`: streaming=floor(sqrt(total)).clamp(2, 8),
    /// static=(total-streaming).clamp(8, 255).
    fn from_mono_sources(mono_sources: usize) -> Self {
        let streaming_capacity = (mono_sources as f32).sqrt() as usize;
        let streaming_capacity = streaming_capacity.clamp(2, 8);
        let static_capacity = mono_sources
            .saturating_sub(streaming_capacity)
            .clamp(8, 255);
        Self {
            static_capacity,
            streaming_capacity,
        }
    }

    fn admits(self, streaming: bool, static_used: usize, streaming_used: usize) -> bool {
        if streaming {
            streaming_used < self.streaming_capacity
        } else {
            static_used < self.static_capacity
        }
    }
}

fn stop_delayed_positioned_sounds(
    delayed: &mut Vec<KiraDelayedPositionedSound>,
    command: &StopSoundCommand,
) {
    if stop_command_is_stop_all(command) {
        delayed.clear();
    }
}

fn delayed_positioned_sound_for_command(
    command: &PlayPositionedSoundCommand,
    listener_position: [f64; 3],
) -> Option<KiraDelayedPositionedSound> {
    vanilla_distance_delay_ticks(command.distance_delay, listener_position, command.position).map(
        |remaining_ticks| KiraDelayedPositionedSound {
            command: command.clone(),
            remaining_ticks,
        },
    )
}

fn advance_delayed_positioned_sounds(
    delayed: &mut Vec<KiraDelayedPositionedSound>,
) -> Vec<PlayPositionedSoundCommand> {
    let mut ready = Vec::new();
    let mut pending = Vec::with_capacity(delayed.len());
    for mut sound in delayed.drain(..) {
        if sound.remaining_ticks <= 1 {
            ready.push(sound.command);
        } else {
            sound.remaining_ticks -= 1;
            pending.push(sound);
        }
    }
    *delayed = pending;
    ready
}

fn active_entity_sound_ids(entities: &[EntitySoundPosition]) -> HashSet<i32> {
    entities.iter().map(|entity| entity.entity_id).collect()
}

fn active_tickable_entity_sounds(sounds: &[TickableEntitySoundUpdate]) -> HashSet<(i32, &str)> {
    sounds
        .iter()
        .map(|sound| (sound.entity_id, sound.event_id.as_str()))
        .collect()
}

fn tickable_entity_sound_is_missing(
    sound: &KiraPlayingSound,
    active_sounds: &HashSet<(i32, &str)>,
) -> bool {
    sound.tickable
        && sound
            .entity_id
            .is_some_and(|entity_id| !active_sounds.contains(&(entity_id, sound.event_id.as_str())))
}

fn entity_bound_sound_is_missing(entity_id: Option<i32>, active_entity_ids: &HashSet<i32>) -> bool {
    entity_id.is_some_and(|entity_id| !active_entity_ids.contains(&entity_id))
}

fn stop_filter_matches(
    event_id: &str,
    category: &AudioCategory,
    command: &StopSoundCommand,
) -> bool {
    let category_matches = command
        .category
        .as_ref()
        .map_or(true, |stop_category| stop_category == category);
    let name_matches = command
        .name
        .as_deref()
        .map_or(true, |name| name == event_id);
    category_matches && name_matches
}

fn channel_decibels_from_gain(gain: f32) -> Decibels {
    if !gain.is_finite() || gain <= 0.0 {
        Decibels::SILENCE
    } else {
        let gain = gain.min(1.0);
        Decibels((20.0 * gain.log10()).max(Decibels::SILENCE.0))
    }
}

fn normalized_gain(gain: f32) -> f32 {
    if gain.is_finite() {
        gain.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn effective_channel_gain(
    category: &AudioCategory,
    base_channel_gain: f32,
    music_gain: f32,
) -> f32 {
    if category == &AudioCategory::Music {
        base_channel_gain * normalized_gain(music_gain)
    } else {
        base_channel_gain
    }
}

#[derive(Debug, PartialEq, Eq)]
struct AmbientLoopSyncPlan {
    updates: Vec<(String, usize)>,
    starts: Vec<usize>,
}

fn ambient_loop_sync_plan<'a>(
    active_event_ids: impl IntoIterator<Item = &'a str>,
    desired: &[crate::AmbientLoopSoundCommand],
) -> AmbientLoopSyncPlan {
    let mut remaining = desired
        .iter()
        .enumerate()
        .map(|(index, sound)| (sound.sound.event_id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut updates = Vec::new();
    for active in active_event_ids {
        if let Some(index) = remaining.remove(active) {
            updates.push((active.to_string(), index));
        }
    }
    let mut starts = remaining.into_values().collect::<Vec<_>>();
    starts.sort_unstable();
    AmbientLoopSyncPlan { updates, starts }
}

fn channel_playback_rate(playback_rate: f32) -> f32 {
    if playback_rate.is_finite() {
        playback_rate.clamp(0.5, 2.0)
    } else {
        1.0
    }
}

fn sound_should_start(gain: f32, category: &AudioCategory) -> bool {
    category == &AudioCategory::Music || java_clamped_volume(gain) != 0.0
}

fn java_clamped_volume(gain: f32) -> f32 {
    if gain < 0.0 {
        0.0
    } else if gain > 1.0 {
        1.0
    } else {
        gain
    }
}

fn spatial_max_distance(sound: &ResolvedSound, gain: f32, fixed_range: Option<f32>) -> f32 {
    if let Some(range) = fixed_range {
        if range.is_finite() {
            return range.max(MIN_SPATIAL_DISTANCE + f32::EPSILON);
        }
    }
    let gain = if gain.is_finite() { gain } else { 0.0 };
    (sound.attenuation_distance as f32 * gain.max(1.0)).max(MIN_SPATIAL_DISTANCE + f32::EPSILON)
}

fn local_playback_event(sound: &ResolvedSound) -> AudioPlaybackEvent {
    AudioPlaybackEvent {
        event_id: sound.event_id.clone(),
        position: [0.0, 0.0, 0.0],
        range: f64::INFINITY,
    }
}

fn spatial_playback_event(
    sound: &ResolvedSound,
    position: [f64; 3],
    gain: f32,
    linear_attenuation: bool,
) -> AudioPlaybackEvent {
    let range = if linear_attenuation {
        let subtitle_gain = if gain.is_nan() {
            f64::NAN
        } else {
            f64::from(gain.max(1.0))
        };
        f64::from(sound.attenuation_distance) * subtitle_gain
    } else {
        f64::INFINITY
    };
    AudioPlaybackEvent {
        event_id: sound.event_id.clone(),
        position,
        range,
    }
}

fn push_bounded_playback_event(
    events: &mut VecDeque<AudioPlaybackEvent>,
    event: AudioPlaybackEvent,
) {
    if events.len() == MAX_PLAYBACK_EVENTS {
        events.pop_front();
    }
    events.push_back(event);
}

fn vanilla_distance_delay_ticks(
    distance_delay: bool,
    listener_position: [f64; 3],
    sound_position: [f64; 3],
) -> Option<u32> {
    if !distance_delay {
        return None;
    }
    let distance_sqr = distance_squared(listener_position, sound_position);
    if distance_sqr <= DISTANCE_DELAY_THRESHOLD_SQR {
        return None;
    }
    Some((distance_sqr.sqrt() * DISTANCE_DELAY_TICKS_PER_BLOCK) as u32)
}

fn distance_squared(lhs: [f64; 3], rhs: [f64; 3]) -> f64 {
    let dx = lhs[0] - rhs[0];
    let dy = lhs[1] - rhs[1];
    let dz = lhs[2] - rhs[2];
    dx * dx + dy * dy + dz * dz
}

fn audio_position(position: [f64; 3]) -> [f32; 3] {
    [position[0] as f32, position[1] as f32, position[2] as f32]
}

fn identity_quaternion() -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}

fn listener_orientation(listener: AudioListenerState) -> [f32; 4] {
    let yaw = (180.0 - listener.y_rot).to_radians();
    let pitch = listener.x_rot.to_radians();
    quat_mul(quat_from_yaw(yaw), quat_from_pitch(pitch))
}

#[cfg(feature = "device")]
fn listener_vectors(listener: AudioListenerState) -> ([f32; 3], [f32; 3]) {
    let yaw = listener.y_rot.to_radians();
    let pitch = listener.x_rot.to_radians();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    (
        [-sin_yaw * cos_pitch, -sin_pitch, cos_yaw * cos_pitch],
        [-sin_yaw * sin_pitch, cos_pitch, cos_yaw * sin_pitch],
    )
}

fn quat_from_yaw(yaw: f32) -> [f32; 4] {
    let half = yaw * 0.5;
    [0.0, half.sin(), 0.0, half.cos()]
}

fn quat_from_pitch(pitch: f32) -> [f32; 4] {
    let half = pitch * 0.5;
    [half.sin(), 0.0, 0.0, half.cos()]
}

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
    fn channel_volume_clamps_above_identity_without_changing_attenuation() {
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
    fn channel_diagnostics_count_only_held_pool_slots_by_decode_mode() {
        assert_eq!(
            active_channel_counts([
                (false, true),
                (false, false),
                (true, true),
                (true, true),
                (true, false),
            ]),
            (1, 2)
        );
    }

    #[test]
    fn channel_limits_match_vanilla_formula_and_isolate_pools() {
        assert_eq!(
            AudioChannelLimits::from_mono_sources(255),
            AudioChannelLimits {
                static_capacity: 247,
                streaming_capacity: 8,
            }
        );
        assert_eq!(
            AudioChannelLimits::from_mono_sources(30),
            AudioChannelLimits {
                static_capacity: 25,
                streaming_capacity: 5,
            }
        );
        assert_eq!(
            AudioChannelLimits::from_mono_sources(2),
            AudioChannelLimits {
                static_capacity: 8,
                streaming_capacity: 2,
            }
        );

        let limits = AudioChannelLimits::from_mono_sources(KIRA_LOGICAL_MONO_SOURCES);
        assert_eq!(limits.static_capacity, 248);
        assert_eq!(limits.streaming_capacity, 8);
        assert!(!limits.admits(false, 248, 0));
        assert!(limits.admits(true, 248, 0));
        assert!(!limits.admits(true, 0, 8));
        assert!(limits.admits(false, 0, 8));
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
                        samples: vec![0; 48_000],
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
        let mut handle = KiraSoundHandle::OpenAl {
            source: Some(source),
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
    #[cfg(not(feature = "device"))]
    fn kira_capacity_has_headroom_for_both_logical_pools() {
        let mut manager =
            AudioManager::<DefaultBackend>::new(kira_audio_manager_settings()).unwrap();

        assert_eq!(manager.main_track().sound_capacity(), 256);
        assert_eq!(manager.sub_track_capacity(), 256);
    }

    #[test]
    fn stopped_sound_lifetime_matches_vanilla_twenty_tick_boundary() {
        let minimum_delete_tick = MINIMUM_SOURCE_LIFETIME_TICKS;

        assert!(stopped_sound_is_retained(0, minimum_delete_tick));
        assert!(stopped_sound_is_retained(19, minimum_delete_tick));
        assert!(!stopped_sound_is_retained(20, minimum_delete_tick));
        assert!(!stopped_sound_is_retained(21, minimum_delete_tick));
    }

    #[test]
    #[cfg(not(feature = "device"))]
    fn stopped_instance_shell_remains_projected_until_twentieth_tick() {
        use std::sync::Arc;

        use kira::{
            sound::static_sound::{StaticSoundData, StaticSoundSettings},
            Frame,
        };

        let mut runtime = KiraAudioRuntime::new().unwrap();
        let data = StaticSoundData {
            sample_rate: 1,
            frames: Arc::from(vec![Frame::ZERO; 128]),
            settings: StaticSoundSettings::default(),
            slice: None,
        };
        let handle = runtime
            .manager
            .as_mut()
            .unwrap()
            .play(data.clone())
            .unwrap();
        runtime
            .manager
            .as_mut()
            .unwrap()
            .backend_mut()
            .on_start_processing();
        runtime.manager.as_mut().unwrap().backend_mut().process();
        runtime.playing.push(KiraPlayingSound {
            event_id: "minecraft:music.test".to_owned(),
            category: AudioCategory::Music,
            entity_id: None,
            jukebox_pos: None,
            handle: KiraSoundHandle::Static(handle),
            track: None,
            tickable: false,
            ambient_loop: false,
            base_channel_gain: 1.0,
            minimum_delete_tick: MINIMUM_SOURCE_LIFETIME_TICKS,
        });
        runtime.stop_sounds(&StopSoundCommand {
            category: None,
            name: Some("minecraft:music.test".to_owned()),
        });
        runtime
            .manager
            .as_mut()
            .unwrap()
            .backend_mut()
            .on_start_processing();
        runtime.manager.as_mut().unwrap().backend_mut().process();
        assert!(!runtime.playing[0].handle.is_active());

        let empty_tick = TickEntitySoundPositionsCommand {
            listener: None,
            entities: Vec::new(),
            tickable_sounds: Vec::new(),
        };
        for expected_tick in 1..MINIMUM_SOURCE_LIFETIME_TICKS {
            runtime.tick_entity_sound_positions(&empty_tick).unwrap();
            assert_eq!(runtime.tick_count, expected_tick);
            assert_eq!(
                runtime.active_music_event_id(),
                Some("minecraft:music.test")
            );
        }
        runtime.tick_entity_sound_positions(&empty_tick).unwrap();
        assert_eq!(runtime.tick_count, MINIMUM_SOURCE_LIFETIME_TICKS);
        assert_eq!(runtime.active_music_event_id(), None);
        assert!(runtime.playing.is_empty());

        let stop_all_handle = runtime.manager.as_mut().unwrap().play(data).unwrap();
        runtime
            .manager
            .as_mut()
            .unwrap()
            .backend_mut()
            .on_start_processing();
        runtime.manager.as_mut().unwrap().backend_mut().process();
        runtime.playing.push(KiraPlayingSound {
            event_id: "minecraft:music.stop_all".to_owned(),
            category: AudioCategory::Music,
            entity_id: None,
            jukebox_pos: None,
            handle: KiraSoundHandle::Static(stop_all_handle),
            track: None,
            tickable: false,
            ambient_loop: false,
            base_channel_gain: 1.0,
            minimum_delete_tick: runtime
                .tick_count
                .saturating_add(MINIMUM_SOURCE_LIFETIME_TICKS),
        });
        runtime.stop_sounds(&StopSoundCommand {
            category: None,
            name: None,
        });
        assert!(runtime.playing.is_empty());
    }

    #[test]
    #[cfg(not(feature = "device"))]
    fn resource_generation_reset_preserves_engine_clock_and_diagnostics() {
        let mut runtime = KiraAudioRuntime::new_with_resource_generation(4).unwrap();
        runtime.tick_count = 17;
        runtime.channel_admission_rejections = 3;

        runtime.reset_resource_generation(5);

        assert_eq!(runtime.tick_count, 17);
        assert_eq!(runtime.channel_admission_rejections, 3);
        assert_eq!(runtime.residency.active_generation(), 5);
    }

    #[test]
    fn playback_rate_clamps_to_vanilla_pitch_range() {
        assert_eq!(channel_playback_rate(0.25), 0.5);
        assert_eq!(channel_playback_rate(1.25), 1.25);
        assert_eq!(channel_playback_rate(2.88), 2.0);
        assert_eq!(channel_playback_rate(f32::NAN), 1.0);
    }

    #[test]
    fn non_music_zero_volume_sounds_do_not_start() {
        assert!(!sound_should_start(0.0, &AudioCategory::Blocks));
        assert!(!sound_should_start(-1.0, &AudioCategory::Blocks));
        assert!(sound_should_start(0.01, &AudioCategory::Blocks));
        assert!(sound_should_start(0.0, &AudioCategory::Music));
        assert!(sound_should_start(f32::NAN, &AudioCategory::Blocks));
    }

    #[test]
    fn spatial_max_distance_matches_vanilla_gain_scaling() {
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

        assert_eq!(spatial_max_distance(&sound, 0.5, None), 16.0);
        assert_eq!(spatial_max_distance(&sound, 1.0, None), 16.0);
        assert_eq!(spatial_max_distance(&sound, 2.0, None), 32.0);
        assert_eq!(spatial_max_distance(&sound, f32::NAN, None), 16.0);
    }

    #[test]
    fn fixed_range_overrides_sound_attenuation_and_gain_scaling() {
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

        assert_eq!(spatial_max_distance(&sound, 0.5, Some(24.0)), 24.0);
        assert_eq!(spatial_max_distance(&sound, 2.0, Some(24.0)), 24.0);
        assert_eq!(
            spatial_max_distance(&sound, 1.0, Some(0.0)),
            MIN_SPATIAL_DISTANCE + f32::EPSILON
        );
        assert_eq!(spatial_max_distance(&sound, 2.0, Some(f32::NAN)), 32.0);
    }

    #[test]
    fn distance_delay_ticks_match_vanilla_threshold_and_flooring() {
        assert_eq!(
            vanilla_distance_delay_ticks(false, [0.0, 0.0, 0.0], [40.0, 0.0, 0.0]),
            None
        );
        assert_eq!(
            vanilla_distance_delay_ticks(true, [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]),
            None
        );
        assert_eq!(
            vanilla_distance_delay_ticks(true, [0.0, 0.0, 0.0], [10.1, 0.0, 0.0]),
            Some(5)
        );
        assert_eq!(
            vanilla_distance_delay_ticks(true, [0.0, 0.0, 0.0], [40.0, 0.0, 0.0]),
            Some(20)
        );
    }

    #[test]
    fn delayed_positioned_sounds_release_after_remaining_ticks() {
        let command = PlayPositionedSoundCommand {
            sound: test_resolved_sound("minecraft:block.cobweb.place"),
            category: AudioCategory::Blocks,
            position: [40.0, 0.0, 0.0],
            packet_volume: 1.0,
            packet_pitch: 1.0,
            gain: 1.0,
            channel_gain: 1.0,
            playback_rate: 1.0,
            seed: 0,
            fixed_range: None,
            distance_delay: true,
        };
        let mut delayed = vec![KiraDelayedPositionedSound {
            command: command.clone(),
            remaining_ticks: 2,
        }];

        assert!(advance_delayed_positioned_sounds(&mut delayed).is_empty());
        assert_eq!(delayed.len(), 1);
        assert_eq!(delayed[0].remaining_ticks, 1);
        assert_eq!(
            advance_delayed_positioned_sounds(&mut delayed),
            vec![command]
        );
        assert!(delayed.is_empty());
    }

    #[test]
    fn delayed_positioned_sound_for_command_preserves_playback_command() {
        let command = PlayPositionedSoundCommand {
            sound: test_resolved_sound("minecraft:block.cobweb.place"),
            category: AudioCategory::Blocks,
            position: [40.0, 0.0, 0.0],
            packet_volume: 1.0,
            packet_pitch: 1.0,
            gain: 1.0,
            channel_gain: 1.0,
            playback_rate: 1.0,
            seed: 99,
            fixed_range: Some(24.0),
            distance_delay: true,
        };

        let delayed = delayed_positioned_sound_for_command(&command, [0.0, 0.0, 0.0]).unwrap();

        assert_eq!(delayed.remaining_ticks, 20);
        assert_eq!(delayed.command, command);
        assert!(delayed_positioned_sound_for_command(&delayed.command, [35.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn stop_sound_only_stop_all_clears_delayed_positioned_sounds() {
        let mut delayed = vec![
            KiraDelayedPositionedSound {
                command: PlayPositionedSoundCommand {
                    sound: test_resolved_sound("minecraft:block.cobweb.place"),
                    category: AudioCategory::Blocks,
                    position: [40.0, 0.0, 0.0],
                    packet_volume: 1.0,
                    packet_pitch: 1.0,
                    gain: 1.0,
                    channel_gain: 1.0,
                    playback_rate: 1.0,
                    seed: 0,
                    fixed_range: None,
                    distance_delay: true,
                },
                remaining_ticks: 20,
            },
            KiraDelayedPositionedSound {
                command: PlayPositionedSoundCommand {
                    sound: test_resolved_sound("minecraft:entity.ghast.warn"),
                    category: AudioCategory::Hostile,
                    position: [40.0, 0.0, 0.0],
                    packet_volume: 1.0,
                    packet_pitch: 1.0,
                    gain: 1.0,
                    channel_gain: 1.0,
                    playback_rate: 1.0,
                    seed: 0,
                    fixed_range: None,
                    distance_delay: true,
                },
                remaining_ticks: 20,
            },
        ];
        let stop_blocks = StopSoundCommand {
            category: Some(AudioCategory::Blocks),
            name: None,
        };

        stop_delayed_positioned_sounds(&mut delayed, &stop_blocks);

        assert_eq!(delayed.len(), 2);
        assert!(!stop_command_is_stop_all(&stop_blocks));

        let stop_all = StopSoundCommand {
            category: None,
            name: None,
        };
        assert!(stop_command_is_stop_all(&stop_all));
        stop_delayed_positioned_sounds(&mut delayed, &stop_all);
        assert!(delayed.is_empty());
    }

    #[test]
    fn entity_bound_sound_is_missing_when_entity_position_is_absent() {
        let active = active_entity_sound_ids(&[
            EntitySoundPosition {
                entity_id: 123,
                position: [1.0, 2.0, 3.0],
            },
            EntitySoundPosition {
                entity_id: 456,
                position: [4.0, 5.0, 6.0],
            },
        ]);

        assert!(!entity_bound_sound_is_missing(None, &active));
        assert!(!entity_bound_sound_is_missing(Some(123), &active));
        assert!(entity_bound_sound_is_missing(Some(404), &active));
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

    #[test]
    fn stop_filter_matches_source_name_and_all() {
        let event_id = "minecraft:music.menu";
        let category = AudioCategory::Music;

        assert!(stop_filter_matches(
            event_id,
            &category,
            &StopSoundCommand {
                category: Some(AudioCategory::Music),
                name: Some(event_id.to_string()),
            },
        ));
        assert!(stop_filter_matches(
            event_id,
            &category,
            &StopSoundCommand {
                category: Some(AudioCategory::Music),
                name: None,
            },
        ));
        assert!(stop_filter_matches(
            event_id,
            &category,
            &StopSoundCommand {
                category: None,
                name: None,
            },
        ));
        assert!(!stop_filter_matches(
            event_id,
            &category,
            &StopSoundCommand {
                category: Some(AudioCategory::Blocks),
                name: Some(event_id.to_string()),
            },
        ));
        assert!(!stop_filter_matches(
            event_id,
            &category,
            &StopSoundCommand {
                category: Some(AudioCategory::Music),
                name: Some("minecraft:music.game".to_string()),
            },
        ));
    }

    #[test]
    fn backend_music_identity_retains_stopped_instance_until_delete_tick() {
        let music = AudioCategory::Music;
        let blocks = AudioCategory::Blocks;
        let retained = [
            ("minecraft:music.game", &music),
            ("minecraft:block.note_block.harp", &blocks),
            ("minecraft:music.dragon", &music),
        ];
        assert_eq!(
            latest_retained_music_event_id(retained.into_iter()),
            Some("minecraft:music.dragon")
        );

        // `playing` retains an explicitly stopped or naturally completed
        // instance through tick 19, so the newest retained music identity
        // still wins until tick 20 removes it.
        let dragon_stopped_but_retained = [
            ("minecraft:music.game", &music),
            ("minecraft:music.dragon", &music),
        ];
        assert_eq!(
            latest_retained_music_event_id(dragon_stopped_but_retained.into_iter()),
            Some("minecraft:music.dragon")
        );
        let removed_at_delete_tick = std::iter::empty::<(&str, &AudioCategory)>();
        assert_eq!(latest_retained_music_event_id(removed_at_delete_tick), None);
    }

    #[test]
    fn music_gain_clamps_and_only_scales_music_category_handles() {
        assert_eq!(normalized_gain(-1.0), 0.0);
        assert_eq!(normalized_gain(0.25), 0.25);
        assert_eq!(normalized_gain(2.0), 1.0);
        assert_eq!(normalized_gain(f32::NAN), 0.0);
        assert_eq!(
            effective_channel_gain(&AudioCategory::Music, 0.8, 0.25),
            0.2
        );
        assert_eq!(
            effective_channel_gain(&AudioCategory::Ambient, 0.8, 0.25),
            0.8
        );
    }

    #[test]
    fn ambient_loop_sync_updates_retained_stops_omitted_and_starts_new() {
        let desired = [
            crate::AmbientLoopSoundCommand {
                sound: test_resolved_sound("minecraft:ambient.keep"),
                channel_gain: 0.75,
            },
            crate::AmbientLoopSoundCommand {
                sound: test_resolved_sound("minecraft:ambient.new"),
                channel_gain: 0.25,
            },
        ];

        let plan = ambient_loop_sync_plan(
            ["minecraft:ambient.old", "minecraft:ambient.keep"],
            &desired,
        );

        assert_eq!(
            plan,
            AmbientLoopSyncPlan {
                updates: vec![("minecraft:ambient.keep".to_string(), 0)],
                starts: vec![1],
            }
        );
    }

    #[test]
    fn playback_event_local_and_spatial_ranges_match_vanilla_subtitle_listener() {
        let sound = test_resolved_sound("minecraft:block.note_block.harp");

        assert_eq!(
            local_playback_event(&sound),
            AudioPlaybackEvent {
                event_id: sound.event_id.clone(),
                position: [0.0, 0.0, 0.0],
                range: f64::INFINITY,
            }
        );
        assert_eq!(
            spatial_playback_event(&sound, [1.0, 2.0, 3.0], 0.25, true).range,
            16.0
        );
        assert_eq!(
            spatial_playback_event(&sound, [1.0, 2.0, 3.0], 2.5, true).range,
            40.0
        );
        assert_eq!(
            spatial_playback_event(&sound, [1.0, 2.0, 3.0], 1.0, false).range,
            f64::INFINITY
        );
    }

    #[test]
    fn playback_event_fifo_drops_oldest_at_its_explicit_bound() {
        let mut events = VecDeque::new();
        for index in 0..=MAX_PLAYBACK_EVENTS {
            push_bounded_playback_event(
                &mut events,
                AudioPlaybackEvent {
                    event_id: format!("minecraft:test.{index}"),
                    position: [f64::from(index as u32), 0.0, 0.0],
                    range: 16.0,
                },
            );
        }

        assert_eq!(events.len(), MAX_PLAYBACK_EVENTS);
        assert_eq!(
            events.front().map(|event| event.event_id.as_str()),
            Some("minecraft:test.1")
        );
        assert_eq!(
            events.back().map(|event| event.event_id.as_str()),
            Some("minecraft:test.1024")
        );
    }

    #[cfg(not(feature = "device"))]
    #[test]
    fn preload_completion_installs_current_generation_without_a_playback_decode_miss() {
        let path = unique_preload_path("runtime-install");
        crate::preload::write_test_ogg(&path);
        let mut runtime = KiraAudioRuntime::new_with_resource_generation(11).unwrap();
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
        runtime.residency.static_sound(&sound).unwrap();
        let residency = runtime.residency_observation();
        assert_eq!(residency.static_buffers, 1);
        assert_eq!(residency.static_cache_hits, 1);
        assert_eq!(residency.static_cache_misses, 0);
        std::fs::remove_file(path).unwrap();
    }

    #[cfg(not(feature = "device"))]
    #[test]
    fn generation_reset_cancels_pending_preload_and_rejects_old_requests() {
        let mut runtime = KiraAudioRuntime::new_with_resource_generation(3).unwrap();
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

    fn test_resolved_sound(event_id: &str) -> ResolvedSound {
        ResolvedSound {
            event_id: event_id.to_string(),
            sound_name: event_id
                .strip_prefix("minecraft:")
                .unwrap_or(event_id)
                .replace('.', "/"),
            ogg_path: format!("sounds/{event_id}.ogg").into(),
            resource_generation: 0,
            stream: false,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        }
    }

    #[cfg(not(feature = "device"))]
    fn unique_preload_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-audio-runtime-{label}-{nanos}.ogg"))
    }
}
