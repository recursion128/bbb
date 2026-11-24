//! Browser Web Audio playback, lazy indexed `.ogg` fetch + SHA-1 verification,
//! and off-main-thread native `AudioContext::decodeAudioData` decoding (with
//! `symphonia` software decode for browser-rejected headers).

use std::{
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet, VecDeque},
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{anyhow, Result};
use bbb_client_assets::open_resource_file;
use js_sys::{ArrayBuffer, Float32Array, Promise, Uint8Array};
use sha1::{Digest, Sha1};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, AudioContextState, AudioScheduledSourceNode,
    DistanceModelType, GainNode, PannerNode, PanningModelType, Response,
};

use crate::{
    scheduler::{
        channel_playback_rate, effective_channel_gain, listener_vectors, normalized_gain,
        AudioChannelLimits, ScheduledVoice, VoiceHandle, VoiceScheduler, MIN_SPATIAL_DISTANCE,
    },
    software_ogg::{
        verify_sha1_digest, DecodeSliceStatus, IncrementalOggDecoder, MAX_AUDIO_COMPRESSED_BYTES,
        MAX_AUDIO_DURATION_SECONDS, MAX_AUDIO_SAMPLE_RATE, MAX_STATIC_SOUND_DECODED_BYTES,
        SOFTWARE_DECODE_SLICE_FRAMES,
    },
    AmbientLoopSoundCommand, AudioCategory, AudioChannelObservation, AudioCommand,
    AudioDeviceObservation, AudioDevicePoll, AudioPlaybackEvent, AudioPreloadObservation,
    AudioPreloadRequest, AudioResidencyObservation, PlayPositionedSoundCommand, ResolvedSound,
    StopJukeboxSongCommand, StopSoundCommand, TickEntitySoundPositionsCommand,
};

#[cfg(test)]
use crate::software_ogg::decode_ogg_planar;

const WEB_LOGICAL_MONO_SOURCES: usize = 256;
const WEB_CHANNEL_CAPACITY_ORIGIN: &str = "webaudio-logical-sources";
const AUDIO_CACHE_STORAGE_NAME: &str = "bbb-audio-v1";
const MAX_CONCURRENT_PRELOAD_DECODES: usize = 6;
const INITIAL_AUDIO_FETCH_RESERVE_BYTES: usize = 1024 * 1024;

fn sanitize_finite_vec3(position: [f32; 3]) -> [f32; 3] {
    position.map(|coord| if coord.is_finite() { coord } else { 0.0 })
}

fn sanitize_max_distance(max_distance: f32) -> f64 {
    if max_distance.is_finite() {
        f64::from(max_distance.max(MIN_SPATIAL_DISTANCE + f32::EPSILON))
    } else {
        f64::from(MIN_SPATIAL_DISTANCE + f32::EPSILON)
    }
}

thread_local! {
    static CACHED_AUDIO_STORAGE: RefCell<Option<web_sys::Cache>> = const { RefCell::new(None) };
}

const MAX_WEB_AUDIO_CACHE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_WEB_AUDIO_CACHE_BUFFERS: usize = 4096;
const MAX_WEB_AUDIO_STREAM_CACHE_BUFFERS: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AudioBufferKey {
    generation: u64,
    path: PathBuf,
}

struct CachedAudioBuffer {
    buffer: AudioBuffer,
    decoded_bytes: u64,
    last_used: u64,
}

enum AsyncDecodeError {
    Fetch(String),
    Decode(String),
}

impl AsyncDecodeError {
    fn message(&self) -> &str {
        match self {
            Self::Fetch(msg) | Self::Decode(msg) => msg,
        }
    }
}

struct CompletedAudioDecode {
    key: AudioBufferKey,
    was_preload: bool,
    result: Result<AudioBuffer, AsyncDecodeError>,
}

#[derive(Clone, Copy)]
struct SpatialConfig {
    linear_attenuation: bool,
    max_distance: f32,
}

enum WebSoundHandle {
    Pending {
        key: AudioBufferKey,
        looping: bool,
        streaming: bool,
        spatial: Option<SpatialConfig>,
    },
    DeferredLooping {
        key: AudioBufferKey,
        streaming: bool,
        spatial: Option<SpatialConfig>,
    },
    Active {
        source: AudioBufferSourceNode,
        gain_node: GainNode,
        panner_node: Option<PannerNode>,
        streaming: bool,
        ended: Rc<Cell<bool>>,
        on_ended: Option<Closure<dyn FnMut()>>,
    },
    Stopped,
}

pub struct WebAudioRuntime {
    ctx: Option<AudioContext>,
    directional_audio: bool,
    scheduler: VoiceScheduler<WebSoundHandle>,
    resource_generation: u64,
    buffer_cache: HashMap<AudioBufferKey, CachedAudioBuffer>,
    stream_cache: VecDeque<(AudioBufferKey, AudioBuffer)>,
    use_sequence: u64,
    residency_obs: AudioResidencyObservation,
    preload_obs: AudioPreloadObservation,
    in_flight_decodes: HashMap<AudioBufferKey, bool>,
    pending_preload_keys: HashSet<AudioBufferKey>,
    queued_preload_keys: VecDeque<AudioBufferKey>,
    completed_decodes: Rc<RefCell<VecDeque<CompletedAudioDecode>>>,
}

impl WebAudioRuntime {
    pub fn new_web(directional_audio: bool, resource_generation: u64) -> Result<Self> {
        let ctx = AudioContext::new().ok();
        Ok(Self {
            ctx,
            directional_audio,
            scheduler: VoiceScheduler::new(AudioChannelLimits::from_mono_sources(
                WEB_LOGICAL_MONO_SOURCES,
            )),
            resource_generation,
            buffer_cache: HashMap::new(),
            stream_cache: VecDeque::with_capacity(MAX_WEB_AUDIO_STREAM_CACHE_BUFFERS),
            use_sequence: 0,
            residency_obs: AudioResidencyObservation::default(),
            preload_obs: AudioPreloadObservation {
                resource_generation,
                ..AudioPreloadObservation::default()
            },
            in_flight_decodes: HashMap::new(),
            pending_preload_keys: HashSet::new(),
            queued_preload_keys: VecDeque::new(),
            completed_decodes: Rc::new(RefCell::new(VecDeque::new())),
        })
    }

    pub fn resume_context(&mut self) {
        if self.ctx.is_none() {
            self.ctx = AudioContext::new().ok();
        }
        if let Some(ctx) = &self.ctx {
            if ctx.state() == AudioContextState::Suspended {
                if let Ok(promise) = ctx.resume() {
                    spawn_local(async move {
                        let _ = JsFuture::from(promise).await;
                    });
                }
            }
        }
        self.drain_completed_decodes();
        self.activate_deferred_looping_sounds();
    }

    pub fn active_music_event_id(&self) -> Option<&str> {
        self.scheduler.active_music_event_id()
    }

    pub fn active_music_sound_name(&self) -> Option<&str> {
        self.scheduler.active_music_sound_name()
    }

    pub fn residency_observation(&self) -> AudioResidencyObservation {
        self.residency_obs
    }

    pub fn preload_observation(&self) -> AudioPreloadObservation {
        self.preload_obs.clone()
    }

    pub fn channel_observation(&self) -> AudioChannelObservation {
        let (static_used, streaming_used) = self.scheduler.channel_counts();
        AudioChannelObservation {
            static_used,
            streaming_used,
            static_capacity: self.scheduler.channel_limits.static_capacity,
            streaming_capacity: self.scheduler.channel_limits.streaming_capacity,
            admission_rejections: self.scheduler.channel_admission_rejections,
            native_static_buffers: self.buffer_cache.len(),
            native_static_bytes: self.residency_obs.static_decoded_bytes,
            native_static_evictions: self.residency_obs.static_cache_evictions,
            capacity_origin: WEB_CHANNEL_CAPACITY_ORIGIN,
        }
    }

    pub fn audio_device_observation(&self) -> Option<AudioDeviceObservation> {
        let state = self
            .ctx
            .as_ref()
            .map(|ctx| format!("WebAudio ({:?})", ctx.state()))
            .unwrap_or_else(|| "WebAudio (unavailable)".to_string());
        Some(AudioDeviceObservation {
            preferred_device: None,
            current_device: Some(state.clone()),
            default_device: Some(state),
            available_device_count: usize::from(self.ctx.is_some()),
            requested_directional_audio: self.directional_audio,
            hrtf_extension: self.directional_audio,
            hrtf_specifiers: i32::from(self.directional_audio),
            hrtf_status: i32::from(self.directional_audio),
            output_limiter: true,
            supports_disconnection: false,
            tracking_mode: "webaudio-context",
            ..AudioDeviceObservation::default()
        })
    }

    pub fn poll_audio_device(&mut self) -> AudioDevicePoll {
        self.drain_completed_decodes();
        self.activate_deferred_looping_sounds();
        AudioDevicePoll::default()
    }

    pub fn poll_audio_preload(&mut self) -> bool {
        self.drain_completed_decodes()
    }

    pub fn request_preload(&mut self, mut requests: Vec<AudioPreloadRequest>) {
        if self.ctx.is_none() {
            self.ctx = AudioContext::new().ok();
        }
        let generation = self.resource_generation;
        requests.retain(|req| req.resource_generation == generation);
        requests.sort_by(|a, b| {
            a.sound_name
                .cmp(&b.sound_name)
                .then_with(|| a.ogg_path.cmp(&b.ogg_path))
        });
        requests.dedup_by(|a, b| a.sound_name == b.sound_name);

        let cancelled = u64::from(!self.pending_preload_keys.is_empty());
        self.pending_preload_keys.clear();
        self.queued_preload_keys.clear();
        self.preload_obs = AudioPreloadObservation {
            resource_generation: generation,
            requested: requests.len(),
            completed: 0,
            failures: 0,
            pending: 0,
            worker_jobs: self.preload_obs.worker_jobs + u64::from(!requests.is_empty()),
            cancelled_jobs: self.preload_obs.cancelled_jobs.saturating_add(cancelled),
            stale_results: self.preload_obs.stale_results,
            last_error: None,
        };

        for req in requests {
            let key = AudioBufferKey {
                generation,
                path: req.ogg_path.clone(),
            };
            if self.buffer_cache.contains_key(&key) {
                self.preload_obs.completed += 1;
                continue;
            }
            if self.ctx.is_none() {
                self.preload_obs.failures += 1;
                self.preload_obs.last_error = Some("WebAudio context is unavailable".to_string());
                continue;
            }
            if self.pending_preload_keys.insert(key.clone())
                && !self.in_flight_decodes.contains_key(&key)
            {
                self.queued_preload_keys.push_back(key);
            }
        }
        self.preload_obs.pending = self.pending_preload_keys.len();
        self.pump_preload_queue();
        self.drain_completed_decodes();
    }

    fn pump_preload_queue(&mut self) {
        let mut active_preloads = self
            .in_flight_decodes
            .keys()
            .filter(|key| self.pending_preload_keys.contains(*key))
            .count();
        while active_preloads < MAX_CONCURRENT_PRELOAD_DECODES {
            let Some(key) = self.queued_preload_keys.pop_front() else {
                break;
            };
            if !self.pending_preload_keys.contains(&key) {
                continue;
            }
            if self.buffer_cache.contains_key(&key) {
                self.pending_preload_keys.remove(&key);
                self.preload_obs.completed = self.preload_obs.completed.saturating_add(1);
                self.preload_obs.pending = self.pending_preload_keys.len();
                continue;
            }
            if self.in_flight_decodes.contains_key(&key) {
                active_preloads += 1;
                continue;
            }
            match self.queue_ogg_decode(key.clone(), false, true) {
                Ok(()) => {
                    active_preloads += 1;
                }
                Err(err) => {
                    self.pending_preload_keys.remove(&key);
                    self.preload_obs.failures = self.preload_obs.failures.saturating_add(1);
                    self.preload_obs.last_error = Some(err.to_string());
                    self.preload_obs.pending = self.pending_preload_keys.len();
                }
            }
        }
    }

    pub fn drain_playback_events(&mut self) -> impl Iterator<Item = AudioPlaybackEvent> + '_ {
        self.scheduler.drain_playback_events()
    }

    pub fn reset_resource_generation(&mut self, resource_generation: u64) {
        self.scheduler.reset_generation();
        self.buffer_cache.clear();
        self.stream_cache.clear();
        self.in_flight_decodes.clear();
        self.completed_decodes.borrow_mut().clear();
        self.resource_generation = resource_generation;
        self.residency_obs.static_buffers = 0;
        self.residency_obs.static_decoded_bytes = 0;
        if !self.pending_preload_keys.is_empty() || self.preload_obs.pending > 0 {
            self.preload_obs.cancelled_jobs = self.preload_obs.cancelled_jobs.saturating_add(1);
        }
        self.pending_preload_keys.clear();
        self.queued_preload_keys.clear();
        self.preload_obs.resource_generation = resource_generation;
        self.preload_obs.requested = 0;
        self.preload_obs.completed = 0;
        self.preload_obs.failures = 0;
        self.preload_obs.pending = 0;
        self.preload_obs.last_error = None;
    }

    pub fn entity_sound_ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.scheduler.entity_sound_ids()
    }

    pub fn handle_command(&mut self, command: &AudioCommand) -> Result<()> {
        self.drain_completed_decodes();
        self.activate_deferred_looping_sounds();
        match command {
            AudioCommand::PlayLocalSound(cmd) => self.play_local_sound_with_role(
                &cmd.sound,
                cmd.category.clone(),
                cmd.channel_gain,
                cmd.playback_rate,
                false,
                false,
            ),
            AudioCommand::PlayPositionedSound(cmd) => self.play_positioned_sound(cmd),
            AudioCommand::PlayEntitySound(cmd) => self.play_spatial_sound(
                &cmd.sound,
                cmd.category.clone(),
                Some(cmd.entity_id),
                cmd.position.unwrap_or([0.0, 0.0, 0.0]),
                cmd.gain,
                cmd.channel_gain,
                cmd.playback_rate,
                cmd.fixed_range,
                None,
                cmd.looping,
                cmd.linear_attenuation,
                cmd.tickable,
            ),
            AudioCommand::PlayJukeboxSong(cmd) => {
                self.stop_jukebox_song(&StopJukeboxSongCommand {
                    jukebox_pos: cmd.jukebox_pos,
                });
                self.play_spatial_sound(
                    &cmd.sound,
                    cmd.category.clone(),
                    None,
                    cmd.position,
                    cmd.gain,
                    cmd.channel_gain,
                    cmd.playback_rate,
                    cmd.fixed_range,
                    Some(cmd.jukebox_pos),
                    false,
                    true,
                    false,
                )
            }
            AudioCommand::StopJukeboxSong(cmd) => {
                self.stop_jukebox_song(cmd);
                Ok(())
            }
            AudioCommand::StopSound(cmd) => {
                self.stop_sounds(cmd);
                Ok(())
            }
            AudioCommand::SetMusicGain(cmd) => {
                self.set_music_gain(cmd.gain);
                Ok(())
            }
            AudioCommand::SyncAmbientLoops(cmd) => self.sync_ambient_loops(&cmd.loops),
            AudioCommand::TickEntitySoundPositions(cmd) => self.tick_entity_sound_positions(cmd),
        }
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
        let handle = self.start_or_queue_handle(
            sound,
            launch.effective_gain,
            launch.playback_rate,
            launch.position,
            looping,
            None,
        )?;
        if matches!(handle, WebSoundHandle::Stopped) {
            return Ok(());
        }
        if sound.stream {
            self.residency_obs.streaming_starts =
                self.residency_obs.streaming_starts.saturating_add(1);
        }
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
        let spatial = SpatialConfig {
            linear_attenuation,
            max_distance: launch.max_distance,
        };
        let handle = self.start_or_queue_handle(
            sound,
            launch.effective_gain,
            launch.playback_rate,
            launch.position,
            looping,
            Some(spatial),
        )?;
        if matches!(handle, WebSoundHandle::Stopped) {
            return Ok(());
        }
        if sound.stream {
            self.residency_obs.streaming_starts =
                self.residency_obs.streaming_starts.saturating_add(1);
        }
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

    fn start_or_queue_handle(
        &mut self,
        sound: &ResolvedSound,
        effective_gain: f32,
        playback_rate: f32,
        position: [f32; 3],
        looping: bool,
        spatial: Option<SpatialConfig>,
    ) -> Result<WebSoundHandle> {
        if sound.resource_generation != self.resource_generation {
            self.residency_obs.audio_admission_rejections = self
                .residency_obs
                .audio_admission_rejections
                .saturating_add(1);
            return Err(anyhow!(
                "sound generation {} does not match active generation {}",
                sound.resource_generation,
                self.resource_generation
            ));
        }
        let Some(ctx) = self.ctx.as_ref() else {
            return Ok(WebSoundHandle::Stopped);
        };
        let suspended = ctx.state() != AudioContextState::Running;
        if suspended && !looping {
            // Do not schedule one-shot BufferSourceNodes on a suspended AudioContext:
            // `onended` will not fire while suspended, pinning static channels and
            // blasting accumulated sounds together on the first user gesture.
            return Ok(WebSoundHandle::Stopped);
        }
        let key = AudioBufferKey {
            generation: sound.resource_generation,
            path: sound.ogg_path.clone(),
        };
        if sound.stream {
            if let Some(buffer) = self.lookup_stream_buffer(&key) {
                if suspended {
                    return Ok(WebSoundHandle::DeferredLooping {
                        key,
                        streaming: true,
                        spatial,
                    });
                }
                return self.create_active_nodes(
                    &buffer,
                    effective_gain,
                    playback_rate,
                    position,
                    looping,
                    true,
                    spatial,
                );
            }
            self.queue_ogg_decode(key.clone(), true, false)?;
            return Ok(WebSoundHandle::Pending {
                key,
                looping,
                streaming: true,
                spatial,
            });
        }
        if let Some(buffer) = self.lookup_cached_buffer(&key) {
            self.residency_obs.static_cache_hits =
                self.residency_obs.static_cache_hits.saturating_add(1);
            if suspended {
                return Ok(WebSoundHandle::DeferredLooping {
                    key,
                    streaming: false,
                    spatial,
                });
            }
            return self.create_active_nodes(
                &buffer,
                effective_gain,
                playback_rate,
                position,
                looping,
                false,
                spatial,
            );
        }
        self.residency_obs.static_cache_misses =
            self.residency_obs.static_cache_misses.saturating_add(1);
        self.queue_ogg_decode(key.clone(), false, false)?;
        Ok(WebSoundHandle::Pending {
            key,
            looping,
            streaming: false,
            spatial,
        })
    }

    fn lookup_stream_buffer(&mut self, key: &AudioBufferKey) -> Option<AudioBuffer> {
        let pos = self.stream_cache.iter().position(|(k, _)| k == key)?;
        let (k, buffer) = self.stream_cache.remove(pos)?;
        let cloned = buffer.clone();
        self.stream_cache.push_back((k, buffer));
        Some(cloned)
    }

    fn cache_stream_buffer(&mut self, key: AudioBufferKey, buffer: AudioBuffer) {
        if let Some(pos) = self.stream_cache.iter().position(|(k, _)| k == &key) {
            self.stream_cache.remove(pos);
        }
        while self.stream_cache.len() >= MAX_WEB_AUDIO_STREAM_CACHE_BUFFERS {
            self.stream_cache.pop_front();
        }
        self.stream_cache.push_back((key, buffer));
    }

    fn lookup_cached_buffer(&mut self, key: &AudioBufferKey) -> Option<AudioBuffer> {
        let next_seq = self.use_sequence.wrapping_add(1);
        let entry = self.buffer_cache.get_mut(key)?;
        self.use_sequence = next_seq;
        entry.last_used = next_seq;
        Some(entry.buffer.clone())
    }

    fn queue_ogg_decode(
        &mut self,
        key: AudioBufferKey,
        streaming: bool,
        was_preload: bool,
    ) -> Result<()> {
        if let Some(existing_streaming) = self.in_flight_decodes.get_mut(&key) {
            *existing_streaming |= streaming;
            return Ok(());
        }
        let Some(ctx) = self.ctx.clone() else {
            return Err(anyhow!("WebAudio context is unavailable"));
        };
        let local_bytes = match read_local_ogg_bytes(&key.path) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.residency_obs.audio_admission_rejections = self
                    .residency_obs
                    .audio_admission_rejections
                    .saturating_add(1);
                return Err(err);
            }
        };

        if let Some(bytes) = local_bytes {
            self.in_flight_decodes.insert(key.clone(), streaming);
            let queue = Rc::clone(&self.completed_decodes);
            // Copy into an unshared JS ArrayBuffer and immediately drop the WASM
            // Vec<u8> before spawning async `decodeAudioData`. If a browser rejects
            // native Ogg decoding, the software decode path re-reads from `key.path`.
            let array_buffer = unshared_array_buffer(&bytes);
            drop(bytes);
            spawn_local(async move {
                let result = decode_ogg_off_main_thread(
                    &ctx,
                    array_buffer,
                    SoftwareDecodeSource::LocalPath(key.path.clone()),
                    streaming,
                )
                .await;
                queue.borrow_mut().push_back(CompletedAudioDecode {
                    key,
                    was_preload,
                    result,
                });
            });
            return Ok(());
        }

        self.in_flight_decodes.insert(key.clone(), streaming);
        let Some((url, expected_sha1)) = indexed_object_url_and_sha1(&key.path) else {
            self.completed_decodes
                .borrow_mut()
                .push_back(CompletedAudioDecode {
                    key,
                    was_preload,
                    result: Err(AsyncDecodeError::Fetch(
                        "sound path is neither in virtual store nor an indexed object".into(),
                    )),
                });
            return Ok(());
        };
        let queue = Rc::clone(&self.completed_decodes);
        spawn_local(async move {
            let result =
                fetch_and_decode_indexed_ogg(&ctx, &url, expected_sha1.as_deref(), streaming).await;
            queue.borrow_mut().push_back(CompletedAudioDecode {
                key,
                was_preload,
                result,
            });
        });
        Ok(())
    }

    fn drain_completed_decodes(&mut self) -> bool {
        let completed: Vec<CompletedAudioDecode> =
            self.completed_decodes.borrow_mut().drain(..).collect();
        if completed.is_empty() {
            return false;
        }
        let mut preload_changed = false;
        for item in completed {
            let allow_streaming = self.in_flight_decodes.remove(&item.key).unwrap_or(false);
            if item.key.generation != self.resource_generation {
                if item.was_preload {
                    self.preload_obs.stale_results =
                        self.preload_obs.stale_results.saturating_add(1);
                    preload_changed = true;
                }
                continue;
            }
            let tracked_preload = self.pending_preload_keys.remove(&item.key);
            if tracked_preload {
                self.preload_obs.pending = self.pending_preload_keys.len();
                preload_changed = true;
            }
            let admitted = match item.result {
                Ok(buffer) => {
                    self.admit_and_cache_buffer(item.key.clone(), buffer, allow_streaming)
                }
                Err(err) => {
                    if matches!(err, AsyncDecodeError::Decode(_)) {
                        self.residency_obs.audio_admission_rejections = self
                            .residency_obs
                            .audio_admission_rejections
                            .saturating_add(1);
                    }
                    Err(anyhow!("{}", err.message()))
                }
            };
            match admitted {
                Ok(buffer) => {
                    if tracked_preload {
                        self.preload_obs.completed = self.preload_obs.completed.saturating_add(1);
                    }
                    self.activate_pending_sounds(&item.key, &buffer);
                }
                Err(err) => {
                    if tracked_preload {
                        self.preload_obs.failures = self.preload_obs.failures.saturating_add(1);
                        self.preload_obs.last_error = Some(err.to_string());
                    }
                    cancel_pending_sounds(&mut self.scheduler.playing, &item.key);
                }
            }
        }
        if !self.queued_preload_keys.is_empty() {
            self.pump_preload_queue();
        }
        preload_changed
    }

    fn activate_pending_sounds(&mut self, ready_key: &AudioBufferKey, buffer: &AudioBuffer) {
        let music_gain = self.scheduler.music_gain;
        let directional_audio = self.directional_audio;
        let Some(ctx) = self.ctx.clone() else {
            return;
        };
        let suspended = ctx.state() != AudioContextState::Running;
        for sound in &mut self.scheduler.playing {
            let WebSoundHandle::Pending {
                key,
                looping,
                streaming,
                spatial,
            } = &sound.handle
            else {
                continue;
            };
            if key != ready_key {
                continue;
            }
            if suspended {
                sound.handle = if *looping {
                    WebSoundHandle::DeferredLooping {
                        key: key.clone(),
                        streaming: *streaming,
                        spatial: *spatial,
                    }
                } else {
                    WebSoundHandle::Stopped
                };
                continue;
            }
            let effective_gain =
                effective_channel_gain(&sound.category, sound.base_channel_gain, music_gain);
            sound.handle = build_active_nodes_on_ctx(
                &ctx,
                directional_audio,
                buffer,
                effective_gain,
                sound.playback_rate,
                sound.position,
                *looping,
                *streaming,
                *spatial,
            )
            .unwrap_or(WebSoundHandle::Stopped);
        }
    }

    fn activate_deferred_looping_sounds(&mut self) {
        let Some(ctx) = self.ctx.clone() else {
            return;
        };
        if ctx.state() != AudioContextState::Running {
            return;
        }
        if !self
            .scheduler
            .playing
            .iter()
            .any(|sound| matches!(sound.handle, WebSoundHandle::DeferredLooping { .. }))
        {
            return;
        }
        let music_gain = self.scheduler.music_gain;
        let directional_audio = self.directional_audio;
        let count = self.scheduler.playing.len();
        for index in 0..count {
            let (key, streaming, spatial) = match &self.scheduler.playing[index].handle {
                WebSoundHandle::DeferredLooping {
                    key,
                    streaming,
                    spatial,
                } => (key.clone(), *streaming, *spatial),
                _ => continue,
            };
            if key.generation != self.resource_generation {
                self.scheduler.playing[index].handle = WebSoundHandle::Stopped;
                continue;
            }
            let buffer = if streaming {
                self.lookup_stream_buffer(&key)
            } else {
                self.lookup_cached_buffer(&key)
            };
            if let Some(buffer) = buffer {
                let sound = &mut self.scheduler.playing[index];
                let effective_gain =
                    effective_channel_gain(&sound.category, sound.base_channel_gain, music_gain);
                sound.handle = build_active_nodes_on_ctx(
                    &ctx,
                    directional_audio,
                    &buffer,
                    effective_gain,
                    sound.playback_rate,
                    sound.position,
                    true,
                    streaming,
                    spatial,
                )
                .unwrap_or(WebSoundHandle::Stopped);
            } else if self.queue_ogg_decode(key.clone(), streaming, false).is_ok() {
                self.scheduler.playing[index].handle = WebSoundHandle::Pending {
                    key,
                    looping: true,
                    streaming,
                    spatial,
                };
            } else {
                self.scheduler.playing[index].handle = WebSoundHandle::Stopped;
            }
        }
    }

    fn admit_and_cache_buffer(
        &mut self,
        key: AudioBufferKey,
        buffer: AudioBuffer,
        allow_streaming: bool,
    ) -> Result<AudioBuffer> {
        if allow_streaming {
            if let Some(existing) = self.lookup_stream_buffer(&key) {
                return Ok(existing);
            }
        } else if let Some(existing) = self.lookup_cached_buffer(&key) {
            return Ok(existing);
        }
        let decoded_bytes = match admit_decoded_audio_buffer_bytes(
            buffer.length(),
            buffer.number_of_channels(),
            buffer.sample_rate(),
            allow_streaming,
        ) {
            Ok(bytes) => bytes,
            Err(err) => {
                self.residency_obs.audio_admission_rejections = self
                    .residency_obs
                    .audio_admission_rejections
                    .saturating_add(1);
                return Err(err);
            }
        };

        // Keep large `stream: true` music/jukebox tracks out of the 128 MiB short-SFX
        // cache so playing multi-minute music never evicts cached sound effects.
        if allow_streaming {
            self.cache_stream_buffer(key, buffer.clone());
            return Ok(buffer);
        }

        if decoded_bytes <= MAX_WEB_AUDIO_CACHE_BYTES {
            self.evict_until_fits(decoded_bytes);
            self.use_sequence = self.use_sequence.wrapping_add(1);
            self.buffer_cache.insert(
                key,
                CachedAudioBuffer {
                    buffer: buffer.clone(),
                    decoded_bytes,
                    last_used: self.use_sequence,
                },
            );
            self.residency_obs.static_buffers = self.buffer_cache.len();
            self.residency_obs.static_decoded_bytes = self
                .residency_obs
                .static_decoded_bytes
                .saturating_add(decoded_bytes);
            self.residency_obs.peak_static_decoded_bytes = self
                .residency_obs
                .peak_static_decoded_bytes
                .max(self.residency_obs.static_decoded_bytes);
        }
        Ok(buffer)
    }

    fn evict_until_fits(&mut self, incoming_bytes: u64) {
        while !self.buffer_cache.is_empty()
            && (self.buffer_cache.len() >= MAX_WEB_AUDIO_CACHE_BUFFERS
                || self
                    .residency_obs
                    .static_decoded_bytes
                    .saturating_add(incoming_bytes)
                    > MAX_WEB_AUDIO_CACHE_BYTES)
        {
            let Some(oldest_key) = self
                .buffer_cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(k, _)| k.clone())
            else {
                break;
            };
            if let Some(removed) = self.buffer_cache.remove(&oldest_key) {
                self.residency_obs.static_decoded_bytes = self
                    .residency_obs
                    .static_decoded_bytes
                    .saturating_sub(removed.decoded_bytes);
                self.residency_obs.static_cache_evictions =
                    self.residency_obs.static_cache_evictions.saturating_add(1);
            }
        }
        self.residency_obs.static_buffers = self.buffer_cache.len();
    }

    #[allow(clippy::too_many_arguments)]
    fn create_active_nodes(
        &self,
        buffer: &AudioBuffer,
        effective_gain: f32,
        playback_rate: f32,
        position: [f32; 3],
        looping: bool,
        streaming: bool,
        spatial: Option<SpatialConfig>,
    ) -> Result<WebSoundHandle> {
        let Some(ctx) = self.ctx.as_ref() else {
            return Ok(WebSoundHandle::Stopped);
        };
        build_active_nodes_on_ctx(
            ctx,
            self.directional_audio,
            buffer,
            effective_gain,
            playback_rate,
            position,
            looping,
            streaming,
            spatial,
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

    fn sync_ambient_loops(&mut self, desired: &[AmbientLoopSoundCommand]) -> Result<()> {
        let starts = self.scheduler.plan_and_update_ambient_loops(desired);
        for idx in starts {
            let target = &desired[idx];
            self.play_local_sound_with_role(
                &target.sound,
                AudioCategory::Ambient,
                target.channel_gain,
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
            if let Some(ctx) = self.ctx.as_ref() {
                let web_listener = ctx.listener();
                let pos = crate::scheduler::audio_position(listener.position);
                let (forward, up) = listener_vectors(listener);
                let _ = web_listener.set_position(
                    f64::from(pos[0]),
                    f64::from(pos[1]),
                    f64::from(pos[2]),
                );
                let _ = web_listener.set_orientation(
                    f64::from(forward[0]),
                    f64::from(forward[1]),
                    f64::from(forward[2]),
                    f64::from(up[0]),
                    f64::from(up[1]),
                    f64::from(up[2]),
                );
            }
        }
        let ready = self
            .scheduler
            .step_entity_and_delayed_voices(command, |_| Ok(()))?;
        for cmd in ready {
            self.play_positioned_sound_now(&cmd)?;
        }
        Ok(())
    }
}

fn disconnect_nodes(
    source: &AudioBufferSourceNode,
    panner_node: Option<&PannerNode>,
    gain_node: &GainNode,
) {
    let _ = source.disconnect();
    if let Some(panner) = panner_node {
        let _ = panner.disconnect();
    }
    let _ = gain_node.disconnect();
}

#[allow(clippy::too_many_arguments)]
fn build_active_nodes_on_ctx(
    ctx: &AudioContext,
    directional_audio: bool,
    buffer: &AudioBuffer,
    effective_gain: f32,
    playback_rate: f32,
    position: [f32; 3],
    looping: bool,
    streaming: bool,
    spatial: Option<SpatialConfig>,
) -> Result<WebSoundHandle> {
    let source = ctx
        .create_buffer_source()
        .map_err(|err| anyhow!("createBufferSource failed: {}", js_err(&err)))?;
    source.set_buffer(Some(buffer));
    source.set_loop(looping);
    source
        .playback_rate()
        .set_value(channel_playback_rate(playback_rate));

    let gain_node = ctx
        .create_gain()
        .map_err(|err| anyhow!("createGain failed: {}", js_err(&err)))?;
    gain_node.gain().set_value(normalized_gain(effective_gain));

    let position = sanitize_finite_vec3(position);
    let destination = ctx.destination();
    let panner_node = if let Some(spec) = spatial.filter(|s| s.linear_attenuation) {
        let panner = ctx
            .create_panner()
            .map_err(|err| anyhow!("createPanner failed: {}", js_err(&err)))?;
        panner.set_panning_model(if directional_audio {
            PanningModelType::Hrtf
        } else {
            PanningModelType::Equalpower
        });
        panner.set_distance_model(DistanceModelType::Linear);
        panner.set_ref_distance(0.0);
        panner.set_max_distance(sanitize_max_distance(spec.max_distance));
        panner.set_rolloff_factor(1.0);
        panner.position_x().set_value(position[0]);
        panner.position_y().set_value(position[1]);
        panner.position_z().set_value(position[2]);

        source
            .connect_with_audio_node(&panner)
            .map_err(|err| anyhow!("connect source->panner failed: {}", js_err(&err)))?;
        panner
            .connect_with_audio_node(&gain_node)
            .map_err(|err| anyhow!("connect panner->gain failed: {}", js_err(&err)))?;
        gain_node
            .connect_with_audio_node(&destination)
            .map_err(|err| anyhow!("connect gain->destination failed: {}", js_err(&err)))?;
        Some(panner)
    } else {
        source
            .connect_with_audio_node(&gain_node)
            .map_err(|err| anyhow!("connect source->gain failed: {}", js_err(&err)))?;
        gain_node
            .connect_with_audio_node(&destination)
            .map_err(|err| anyhow!("connect gain->destination failed: {}", js_err(&err)))?;
        None
    };

    let ended = Rc::new(Cell::new(false));
    let ended_flag = Rc::clone(&ended);
    let source_end = source.clone();
    let gain_end = gain_node.clone();
    let panner_end = panner_node.clone();
    let on_ended: Closure<dyn FnMut()> = Closure::once(move || {
        ended_flag.set(true);
        disconnect_nodes(&source_end, panner_end.as_ref(), &gain_end);
    });
    let scheduled: &AudioScheduledSourceNode = source.as_ref();
    scheduled.set_onended(Some(on_ended.as_ref().unchecked_ref()));

    if let Err(err) = source.start() {
        scheduled.set_onended(None);
        disconnect_nodes(&source, panner_node.as_ref(), &gain_node);
        drop(on_ended);
        return Err(anyhow!(
            "AudioBufferSourceNode.start failed: {}",
            js_err(&err)
        ));
    }

    Ok(WebSoundHandle::Active {
        source,
        gain_node,
        panner_node,
        streaming,
        ended,
        on_ended: Some(on_ended),
    })
}

impl Drop for WebSoundHandle {
    fn drop(&mut self) {
        if let Self::Active {
            source,
            gain_node,
            panner_node,
            ended,
            on_ended,
            ..
        } = self
        {
            let scheduled: &AudioScheduledSourceNode = source.as_ref();
            scheduled.set_onended(None);
            let _ = on_ended.take();
            if !ended.get() {
                let _ = scheduled.stop();
                disconnect_nodes(source, panner_node.as_ref(), gain_node);
                ended.set(true);
            }
        }
    }
}

impl VoiceHandle for WebSoundHandle {
    fn stop(&mut self) {
        if let Self::Active {
            source,
            gain_node,
            panner_node,
            ended,
            on_ended,
            ..
        } = self
        {
            let scheduled: &AudioScheduledSourceNode = source.as_ref();
            scheduled.set_onended(None);
            let _ = on_ended.take();
            let _ = scheduled.stop();
            disconnect_nodes(source, panner_node.as_ref(), gain_node);
            ended.set(true);
        }
        *self = Self::Stopped;
    }

    fn is_active(&self) -> bool {
        match self {
            Self::Pending { .. } | Self::DeferredLooping { .. } => true,
            Self::Active { ended, .. } => !ended.get(),
            Self::Stopped => false,
        }
    }

    fn holds_channel(&self) -> bool {
        self.is_active()
    }

    fn is_streaming(&self) -> bool {
        match self {
            Self::Pending { streaming, .. }
            | Self::DeferredLooping { streaming, .. }
            | Self::Active { streaming, .. } => *streaming,
            Self::Stopped => false,
        }
    }

    fn set_volume(&mut self, gain: f32) {
        if let Self::Active { gain_node, .. } = self {
            gain_node.gain().set_value(normalized_gain(gain));
        }
    }

    fn set_playback_rate(&mut self, rate: f32) {
        if let Self::Active { source, .. } = self {
            source
                .playback_rate()
                .set_value(channel_playback_rate(rate));
        }
    }

    fn set_position(&mut self, position: [f32; 3]) {
        if let Self::Active {
            panner_node: Some(panner),
            ..
        } = self
        {
            let position = sanitize_finite_vec3(position);
            panner.position_x().set_value(position[0]);
            panner.position_y().set_value(position[1]);
            panner.position_z().set_value(position[2]);
        }
    }
}

fn read_local_ogg_bytes(path: &Path) -> Result<Option<Vec<u8>>> {
    if let Ok((mut reader, len)) = open_resource_file(path) {
        if len > MAX_AUDIO_COMPRESSED_BYTES {
            return Err(anyhow!(
                "audio resource {} exceeds {MAX_AUDIO_COMPRESSED_BYTES} compressed bytes",
                path.display()
            ));
        }
        let initial = usize::try_from(len)
            .unwrap_or(INITIAL_AUDIO_FETCH_RESERVE_BYTES)
            .min(INITIAL_AUDIO_FETCH_RESERVE_BYTES);
        let mut bytes = Vec::with_capacity(initial);
        reader
            .by_ref()
            .take(MAX_AUDIO_COMPRESSED_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_AUDIO_COMPRESSED_BYTES {
            return Err(anyhow!(
                "audio resource {} exceeds {MAX_AUDIO_COMPRESSED_BYTES} compressed bytes",
                path.display()
            ));
        }
        Ok(Some(bytes))
    } else if let Some(lazy_res) =
        bbb_client_assets::SoundEntry::read_lazy_virtual_ogg(path, MAX_AUDIO_COMPRESSED_BYTES)
    {
        lazy_res.map(Some)
    } else {
        Ok(None)
    }
}

enum SoftwareDecodeSource {
    LocalPath(PathBuf),
    Bytes(Vec<u8>),
}

fn cancel_pending_sounds(
    playing: &mut [ScheduledVoice<WebSoundHandle>],
    failed_key: &AudioBufferKey,
) {
    for sound in playing {
        if matches!(&sound.handle, WebSoundHandle::Pending { key, .. } if key == failed_key) {
            sound.handle = WebSoundHandle::Stopped;
        }
    }
}

fn admit_decoded_audio_buffer_bytes(
    frames: u32,
    channels: u32,
    sample_rate: f32,
    allow_streaming: bool,
) -> Result<u64> {
    if channels == 0
        || !sample_rate.is_finite()
        || sample_rate <= 0.0
        || sample_rate > MAX_AUDIO_SAMPLE_RATE as f32
    {
        return Err(anyhow!(
            "decoded AudioBuffer metadata out of bounds (channels={channels}, sample_rate={sample_rate})"
        ));
    }
    let max_frames = (sample_rate.ceil() as u64).saturating_mul(MAX_AUDIO_DURATION_SECONDS);
    if u64::from(frames) > max_frames {
        return Err(anyhow!(
            "audio frame count {frames} exceeds limit {max_frames}"
        ));
    }
    let decoded_bytes = u64::from(frames)
        .saturating_mul(u64::from(channels))
        .saturating_mul(4);
    if !allow_streaming && decoded_bytes > MAX_STATIC_SOUND_DECODED_BYTES {
        return Err(anyhow!(
            "decoded audio bytes {decoded_bytes} exceed {MAX_STATIC_SOUND_DECODED_BYTES}"
        ));
    }
    Ok(decoded_bytes)
}

fn unshared_array_buffer(bytes: &[u8]) -> ArrayBuffer {
    // WebAssembly +atomics backs linear memory with SharedArrayBuffer, which
    // AudioContext.decodeAudioData rejects because it detaches its input.
    let copy = Uint8Array::new_with_length(bytes.len() as u32);
    copy.copy_from(bytes);
    copy.buffer()
}

async fn yield_to_browser_event_loop() {
    let promise = Promise::new(&mut |resolve, _reject| {
        let scheduled = web_sys::window().is_some_and(|window| {
            window
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 0)
                .is_ok()
        });
        if !scheduled {
            let _ = resolve.call0(&JsValue::UNDEFINED);
        }
    });
    let _ = JsFuture::from(promise).await;
}

async fn decode_ogg_off_main_thread(
    ctx: &AudioContext,
    array_buffer: ArrayBuffer,
    software_source: SoftwareDecodeSource,
    allow_streaming: bool,
) -> Result<AudioBuffer, AsyncDecodeError> {
    if let Ok(promise) = ctx.decode_audio_data(&array_buffer) {
        if let Ok(val) = JsFuture::from(promise).await {
            if let Ok(buffer) = val.dyn_into::<AudioBuffer>() {
                drop(software_source);
                return Ok(buffer);
            }
        }
    }
    let bytes = match software_source {
        SoftwareDecodeSource::LocalPath(path) => read_local_ogg_bytes(&path)
            .map_err(|err| AsyncDecodeError::Decode(err.to_string()))?
            .ok_or_else(|| {
                AsyncDecodeError::Decode(format!(
                    "local Ogg resource {} unavailable for software decode",
                    path.display()
                ))
            })?,
        SoftwareDecodeSource::Bytes(bytes) => bytes,
    };
    create_buffer_via_software_decode_yielding(ctx, bytes, allow_streaming)
        .await
        .map_err(|err| AsyncDecodeError::Decode(err.to_string()))
}

async fn create_buffer_via_software_decode_yielding(
    ctx: &AudioContext,
    bytes: Vec<u8>,
    allow_streaming: bool,
) -> Result<AudioBuffer> {
    let mut decoder = IncrementalOggDecoder::new(bytes, allow_streaming)?;
    while decoder.decode_slice(SOFTWARE_DECODE_SLICE_FRAMES)? == DecodeSliceStatus::More {
        yield_to_browser_event_loop().await;
    }
    let decoded = decoder.finish()?;
    let audio_buffer = ctx
        .create_buffer(
            u32::from(decoded.channels),
            decoded.frames as u32,
            decoded.sample_rate as f32,
        )
        .map_err(|err| anyhow!("AudioContext.createBuffer failed: {}", js_err(&err)))?;

    let left_js = Float32Array::new_with_length(decoded.left.len() as u32);
    left_js.copy_from(&decoded.left);
    audio_buffer
        .copy_to_channel_with_f32_array(&left_js, 0)
        .map_err(|err| anyhow!("AudioBuffer.copyToChannel(0) failed: {}", js_err(&err)))?;
    if let Some(right) = &decoded.right {
        let right_js = Float32Array::new_with_length(right.len() as u32);
        right_js.copy_from(right);
        audio_buffer
            .copy_to_channel_with_f32_array(&right_js, 1)
            .map_err(|err| anyhow!("AudioBuffer.copyToChannel(1) failed: {}", js_err(&err)))?;
    }
    Ok(audio_buffer)
}

fn indexed_object_url_and_sha1(path: &Path) -> Option<(String, Option<String>)> {
    let text = path.to_str()?.replace('\\', "/");
    let suffix = text
        .strip_prefix("/__bbb_mem__/objects/")
        .or_else(|| text.split("/objects/").nth(1))?;
    let (prefix, hash) = suffix.split_once('/')?;
    if prefix.len() != 2
        || hash.len() != 40
        || !prefix.bytes().all(|b| b.is_ascii_hexdigit())
        || !hash.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    Some((
        format!("/assets/objects/{prefix}/{hash}"),
        Some(hash.to_ascii_lowercase()),
    ))
}

async fn fetch_and_decode_indexed_ogg(
    ctx: &AudioContext,
    url: &str,
    expected_sha1: Option<&str>,
    allow_streaming: bool,
) -> Result<AudioBuffer, AsyncDecodeError> {
    let bytes = fetch_indexed_ogg(url, expected_sha1).await?;
    let array_buffer = unshared_array_buffer(&bytes);
    decode_ogg_off_main_thread(
        ctx,
        array_buffer,
        SoftwareDecodeSource::Bytes(bytes),
        allow_streaming,
    )
    .await
}

fn response_from_unshared_bytes(bytes: &[u8]) -> Option<Response> {
    // Construct Response from an unshared JS ArrayBuffer so wasm32 +atomics
    // SharedArrayBuffer views are never handed directly to the Fetch API.
    let buffer = unshared_array_buffer(bytes);
    let ctor = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("Response"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    let args = js_sys::Array::of1(&buffer.into());
    js_sys::Reflect::construct(&ctor, &args)
        .ok()?
        .dyn_into::<Response>()
        .ok()
}

async fn fetch_indexed_ogg(
    url: &str,
    expected_sha1: Option<&str>,
) -> Result<Vec<u8>, AsyncDecodeError> {
    let window = web_sys::window()
        .ok_or_else(|| AsyncDecodeError::Fetch("missing browser window".to_string()))?;
    let cache = open_audio_cache(&window).await;

    if let Some(cache) = &cache {
        if let Ok(matched) = JsFuture::from(cache.match_with_str(url)).await {
            if !matched.is_undefined() && !matched.is_null() {
                if let Ok(resp) = matched.dyn_into::<Response>() {
                    // Entries in `bbb-audio-v1` were already SHA-1 verified before
                    // `cache.put`; read once with the same bounded stream cap.
                    if let Ok(bytes) = response_bounded_bytes(&resp).await {
                        return Ok(bytes);
                    }
                }
            }
        }
    }

    let resp_val = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| AsyncDecodeError::Fetch(format!("fetch {url} failed: {}", js_err(&e))))?;
    let resp: Response = resp_val
        .dyn_into()
        .map_err(|_| AsyncDecodeError::Fetch(format!("invalid Response for {url}")))?;
    if !resp.ok() {
        return Err(AsyncDecodeError::Fetch(format!(
            "HTTP {} fetching {url}",
            resp.status()
        )));
    }
    let bytes = response_bounded_bytes(&resp).await?;
    if let Some(expected) = expected_sha1 {
        verify_sha1_digest(Sha1::digest(&bytes).into(), expected)
            .map_err(AsyncDecodeError::Fetch)?;
    }

    if let Some(cache) = cache {
        if let Some(cache_resp) = response_from_unshared_bytes(&bytes) {
            let url_owned = url.to_string();
            spawn_local(async move {
                let _ = JsFuture::from(cache.put_with_str(&url_owned, &cache_resp)).await;
            });
        }
    }
    Ok(bytes)
}

async fn open_audio_cache(window: &web_sys::Window) -> Option<web_sys::Cache> {
    if let Some(cached) = CACHED_AUDIO_STORAGE.with(|slot| slot.borrow().clone()) {
        return Some(cached);
    }
    let caches = window.caches().ok()?;
    let val = JsFuture::from(caches.open(AUDIO_CACHE_STORAGE_NAME))
        .await
        .ok()?;
    let cache = val.dyn_into::<web_sys::Cache>().ok()?;
    CACHED_AUDIO_STORAGE.with(|slot| *slot.borrow_mut() = Some(cache.clone()));
    Some(cache)
}

async fn response_bounded_bytes(resp: &Response) -> Result<Vec<u8>, AsyncDecodeError> {
    let content_length = resp
        .headers()
        .get("content-length")
        .ok()
        .flatten()
        .and_then(|val| val.parse::<u64>().ok());
    if let Some(declared) = content_length {
        if declared > MAX_AUDIO_COMPRESSED_BYTES {
            return Err(AsyncDecodeError::Decode(format!(
                "audio Content-Length {declared} exceeds {MAX_AUDIO_COMPRESSED_BYTES}"
            )));
        }
    }

    let max_usize = usize::try_from(MAX_AUDIO_COMPRESSED_BYTES).unwrap_or(usize::MAX);
    if let Ok(body_stream) = js_sys::Reflect::get(resp, &JsValue::from_str("body")) {
        if !body_stream.is_undefined() && !body_stream.is_null() {
            if let Ok(get_reader) =
                js_sys::Reflect::get(&body_stream, &JsValue::from_str("getReader"))
            {
                if let Ok(get_reader_fn) = get_reader.dyn_into::<js_sys::Function>() {
                    if let Ok(reader) = get_reader_fn.call0(&body_stream) {
                        return read_audio_stream_bounded(&reader, content_length, max_usize).await;
                    }
                }
            }
        }
    }

    let buf_promise = resp.array_buffer().map_err(|e| {
        AsyncDecodeError::Fetch(format!("Response.arrayBuffer failed: {}", js_err(&e)))
    })?;
    let buf = JsFuture::from(buf_promise).await.map_err(|e| {
        AsyncDecodeError::Fetch(format!("await Response.arrayBuffer failed: {}", js_err(&e)))
    })?;
    let array_buffer: ArrayBuffer = buf
        .dyn_into()
        .map_err(|_| AsyncDecodeError::Fetch("Response.arrayBuffer was not ArrayBuffer".into()))?;
    if u64::from(array_buffer.byte_length()) > MAX_AUDIO_COMPRESSED_BYTES {
        return Err(AsyncDecodeError::Decode(format!(
            "audio response length {} exceeds {MAX_AUDIO_COMPRESSED_BYTES}",
            array_buffer.byte_length()
        )));
    }
    Ok(Uint8Array::new(&array_buffer).to_vec())
}

async fn read_audio_stream_bounded(
    reader: &JsValue,
    content_length: Option<u64>,
    max_usize: usize,
) -> Result<Vec<u8>, AsyncDecodeError> {
    let read_fn = js_sys::Reflect::get(reader, &JsValue::from_str("read"))
        .map_err(|e| AsyncDecodeError::Fetch(format!("reader.read missing: {}", js_err(&e))))?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| AsyncDecodeError::Fetch("reader.read is not a Function".into()))?;
    let cancel_fn = js_sys::Reflect::get(reader, &JsValue::from_str("cancel"))
        .ok()
        .and_then(|val| val.dyn_into::<js_sys::Function>().ok());

    let initial_capacity = content_length
        .and_then(|len| usize::try_from(len).ok())
        .unwrap_or(0)
        .min(INITIAL_AUDIO_FETCH_RESERVE_BYTES)
        .min(max_usize);
    let mut out = Vec::with_capacity(initial_capacity);

    loop {
        let promise_val = read_fn
            .call0(reader)
            .map_err(|e| AsyncDecodeError::Fetch(format!("reader.read failed: {}", js_err(&e))))?;
        let promise: Promise = promise_val
            .dyn_into()
            .map_err(|_| AsyncDecodeError::Fetch("reader.read did not return a Promise".into()))?;
        let chunk_obj = JsFuture::from(promise).await.map_err(|e| {
            AsyncDecodeError::Fetch(format!("await reader.read failed: {}", js_err(&e)))
        })?;
        let done = js_sys::Reflect::get(&chunk_obj, &JsValue::from_str("done"))
            .ok()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if done {
            break;
        }
        let value = js_sys::Reflect::get(&chunk_obj, &JsValue::from_str("value")).map_err(|e| {
            AsyncDecodeError::Fetch(format!("read chunk value failed: {}", js_err(&e)))
        })?;
        if value.is_undefined() || value.is_null() {
            continue;
        }
        let chunk = Uint8Array::new(&value);
        let chunk_len = chunk.length() as usize;
        if out.len().saturating_add(chunk_len) > max_usize {
            if let Some(cancel) = cancel_fn.as_ref() {
                let _ = cancel.call0(reader);
            }
            return Err(AsyncDecodeError::Decode(format!(
                "audio stream exceeded {MAX_AUDIO_COMPRESSED_BYTES} bytes"
            )));
        }
        let start = out.len();
        out.try_reserve(chunk_len)
            .map_err(|e| AsyncDecodeError::Decode(format!("audio buffer reserve failed: {e}")))?;
        out.resize(start + chunk_len, 0);
        chunk.copy_to(&mut out[start..]);
    }

    Ok(out)
}

fn js_err(value: &JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EntitySoundPosition, TickableEntitySoundUpdate};

    fn test_web_runtime(resource_generation: u64) -> WebAudioRuntime {
        WebAudioRuntime {
            ctx: None,
            directional_audio: false,
            scheduler: VoiceScheduler::new(AudioChannelLimits::from_mono_sources(
                WEB_LOGICAL_MONO_SOURCES,
            )),
            resource_generation,
            buffer_cache: HashMap::new(),
            stream_cache: VecDeque::with_capacity(MAX_WEB_AUDIO_STREAM_CACHE_BUFFERS),
            use_sequence: 0,
            residency_obs: AudioResidencyObservation::default(),
            preload_obs: AudioPreloadObservation {
                resource_generation,
                ..AudioPreloadObservation::default()
            },
            in_flight_decodes: HashMap::new(),
            pending_preload_keys: HashSet::new(),
            queued_preload_keys: VecDeque::new(),
            completed_decodes: Rc::new(RefCell::new(VecDeque::new())),
        }
    }

    #[test]
    fn decoded_audio_buffer_byte_accounting_matches_frames_times_channels_times_f32() {
        let bytes = admit_decoded_audio_buffer_bytes(48_000 * 180, 2, 48_000.0, true).unwrap();
        assert_eq!(bytes, 48_000 * 180 * 2 * 4);

        // Large multi-minute streaming tracks above the static-buffer limit are
        // admitted when `stream: true`, while `stream: false` rejects them.
        let ten_min_stereo_frames = 48_000 * 600;
        assert!(
            admit_decoded_audio_buffer_bytes(ten_min_stereo_frames, 2, 48_000.0, false).is_err()
        );
        assert_eq!(
            admit_decoded_audio_buffer_bytes(ten_min_stereo_frames, 2, 48_000.0, true).unwrap(),
            u64::from(ten_min_stereo_frames) * 2 * 4
        );
        assert!(admit_decoded_audio_buffer_bytes(100, 0, 48_000.0, true).is_err());
        assert!(admit_decoded_audio_buffer_bytes(100, 2, 384_000.0, true).is_err());
    }

    #[test]
    fn symphonia_software_decode_handles_checked_in_ogg_fixture() {
        use base64::{engine::general_purpose::STANDARD, Engine as _};

        let raw = STANDARD
            .decode(include_str!("test-silence.ogg.b64").trim())
            .unwrap();
        let decoded = decode_ogg_planar(raw, false).unwrap();
        assert_eq!(decoded.channels, 1);
        assert_eq!(decoded.frames, 80);
        assert_eq!(decoded.left.len(), 80);
        assert!(decoded.right.is_none());
    }

    #[test]
    fn pending_streaming_handles_respect_stop_gain_and_tickable_mutations() {
        let mut runtime = test_web_runtime(2);
        let music_key = AudioBufferKey {
            generation: 2,
            path: PathBuf::from("/__bbb_mem__/music.ogg"),
        };
        runtime.scheduler.playing.push(ScheduledVoice {
            event_id: "minecraft:music.game".into(),
            sound_name: "minecraft:music/game/calm1".into(),
            category: AudioCategory::Music,
            entity_id: None,
            position: [0.0; 3],
            jukebox_pos: None,
            handle: WebSoundHandle::Pending {
                key: music_key.clone(),
                looping: false,
                streaming: true,
                spatial: None,
            },
            tickable: false,
            ambient_loop: false,
            base_channel_gain: 0.8,
            playback_rate: 1.0,
            minimum_delete_tick: 20,
        });
        let entity_key = AudioBufferKey {
            generation: 2,
            path: PathBuf::from("/__bbb_mem__/minecart.ogg"),
        };
        runtime.scheduler.playing.push(ScheduledVoice {
            event_id: "minecraft:entity.minecart.riding".into(),
            sound_name: "minecraft:minecart/base".into(),
            category: AudioCategory::Neutral,
            entity_id: Some(7),
            position: [1.0, 2.0, 3.0],
            jukebox_pos: None,
            handle: WebSoundHandle::Pending {
                key: entity_key,
                looping: true,
                streaming: false,
                spatial: Some(SpatialConfig {
                    linear_attenuation: true,
                    max_distance: 16.0,
                }),
            },
            tickable: true,
            ambient_loop: false,
            base_channel_gain: 0.5,
            playback_rate: 1.0,
            minimum_delete_tick: 20,
        });

        runtime.set_music_gain(0.25);
        assert_eq!(
            effective_channel_gain(
                &runtime.scheduler.playing[0].category,
                runtime.scheduler.playing[0].base_channel_gain,
                runtime.scheduler.music_gain,
            ),
            0.2
        );

        runtime
            .tick_entity_sound_positions(&TickEntitySoundPositionsCommand {
                listener: None,
                entities: vec![EntitySoundPosition {
                    entity_id: 7,
                    position: [4.0, 5.0, 6.0],
                }],
                tickable_sounds: vec![TickableEntitySoundUpdate {
                    event_id: "minecraft:entity.minecart.riding".into(),
                    entity_id: 7,
                    position: [4.0, 5.0, 6.0],
                    channel_gain: 0.9,
                    playback_rate: 1.5,
                }],
            })
            .unwrap();
        assert_eq!(runtime.scheduler.playing[1].position, [4.0, 5.0, 6.0]);
        assert_eq!(runtime.scheduler.playing[1].base_channel_gain, 0.9);
        assert_eq!(runtime.scheduler.playing[1].playback_rate, 1.5);

        runtime.stop_sounds(&StopSoundCommand {
            category: Some(AudioCategory::Music),
            name: None,
        });
        assert!(!runtime.scheduler.playing[0].handle.is_active());
        assert!(!runtime.scheduler.playing[0].handle.holds_channel());
        assert!(runtime.scheduler.playing[1].handle.is_active());

        // Entity despawn cancels the remaining pending handle before decode finishes.
        runtime
            .tick_entity_sound_positions(&TickEntitySoundPositionsCommand {
                listener: None,
                entities: Vec::new(),
                tickable_sounds: Vec::new(),
            })
            .unwrap();
        assert!(!runtime.scheduler.playing[1].handle.is_active());
        cancel_pending_sounds(&mut runtime.scheduler.playing, &music_key);
    }

    #[test]
    fn scoped_stop_sound_filters_delayed_positioned_sounds() {
        let mut runtime = test_web_runtime(1);
        let delayed_command =
            |event_id: &str, category: AudioCategory| PlayPositionedSoundCommand {
                sound: ResolvedSound {
                    event_id: event_id.to_string(),
                    sound_name: event_id.to_string(),
                    ogg_path: PathBuf::from("/__bbb_mem__/delayed.ogg"),
                    resource_generation: 1,
                    stream: false,
                    preload: false,
                    attenuation_distance: 16,
                    entry_volume: 1.0,
                    entry_pitch: 1.0,
                },
                category,
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
        runtime
            .play_positioned_sound(&delayed_command(
                "minecraft:block.cobweb.place",
                AudioCategory::Blocks,
            ))
            .unwrap();
        runtime
            .play_positioned_sound(&delayed_command(
                "minecraft:entity.ghast.warn",
                AudioCategory::Hostile,
            ))
            .unwrap();
        assert_eq!(runtime.scheduler.delayed_positioned.len(), 2);

        runtime.stop_sounds(&StopSoundCommand {
            category: Some(AudioCategory::Blocks),
            name: None,
        });
        assert_eq!(runtime.scheduler.delayed_positioned.len(), 1);
        assert_eq!(
            runtime.scheduler.delayed_positioned[0]
                .command
                .sound
                .event_id,
            "minecraft:entity.ghast.warn"
        );
    }
}
