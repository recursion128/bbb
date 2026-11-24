use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    mem::size_of,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
#[cfg(test)]
use kira::{
    sound::{
        static_sound::{StaticSoundData, StaticSoundSettings},
        streaming::{Decoder as KiraStreamingDecoder, StreamingSoundData},
    },
    Frame,
};
use symphonia::core::{
    audio::{
        conv::{FromSample, IntoSample},
        sample::Sample,
        Audio, AudioBuffer, GenericAudioBufferRef,
    },
    codecs::audio::AudioDecoder as SymphoniaDecoder,
    formats::{probe::Hint, FormatReader, SeekMode, SeekTo, TrackType},
    io::{MediaSource, MediaSourceStream},
    packet::Packet,
    units::Timestamp,
};
use thiserror::Error;

use crate::{preload::AudioPreloadRequest, ResolvedSound};

pub(crate) const MAX_AUDIO_COMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_AUDIO_SAMPLE_RATE: u32 = 192_000;
pub(crate) const MAX_AUDIO_DURATION_SECONDS: u64 = 30 * 60;
pub(crate) const MAX_AUDIO_DECODE_PACKET_FRAMES: usize = 65_536;
pub(crate) const MAX_STATIC_SOUND_DECODED_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const MAX_STATIC_SOUND_CACHE_BYTES: u64 = 512 * 1024 * 1024;
pub(crate) const MAX_STATIC_SOUND_CACHE_BUFFERS: usize = 65_536;
const INITIAL_STATIC_RESERVE_FRAMES: usize = 65_536;

pub use crate::command::{AudioFileProbe, AudioResidencyObservation};

/// Opens and admits one compressed audio resource through the same bounded
/// metadata path used by playback, without requiring an audio device or
/// decoding the complete stream.
pub fn probe_bounded_audio_file(path: impl AsRef<Path>) -> Result<AudioFileProbe> {
    let decoder = BoundedAudioDecoder::open(path.as_ref(), false).map_err(anyhow::Error::new)?;
    Ok(AudioFileProbe {
        sample_rate: decoder.sample_rate,
        frames: decoder.admitted_frames,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AudioResourceKey {
    generation: u64,
    path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StereoFrame {
    pub(crate) left: f32,
    pub(crate) right: f32,
}

impl StereoFrame {
    #[inline]
    fn from_mono(sample: f32) -> Self {
        Self {
            left: sample,
            right: sample,
        }
    }

    #[inline]
    fn new(left: f32, right: f32) -> Self {
        Self { left, right }
    }
}

/// Compact interleaved mono/stereo PCM16 buffer (2 bytes/frame mono, 4 bytes/frame stereo).
#[derive(Debug, Clone)]
pub(crate) struct DecodedStaticAudio {
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) samples: Arc<[i16]>,
}

struct StaticCacheEntry {
    data: DecodedStaticAudio,
    decoded_bytes: u64,
    last_used: u64,
}

#[cfg(feature = "device")]
pub(crate) struct OpenAlStaticSound {
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) samples: Arc<[i16]>,
}

#[cfg(any(feature = "device", test))]
pub(crate) struct OpenAlStreamDecoder {
    decoder: BoundedAudioDecoder,
    path: PathBuf,
    looping: bool,
    seek_frame: Option<usize>,
}

#[cfg(any(feature = "device", test))]
pub(crate) struct OpenAlPcmChunk {
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) samples: Vec<i16>,
}

pub(crate) struct AudioResidency {
    active_generation: u64,
    static_cache: HashMap<AudioResourceKey, StaticCacheEntry>,
    use_sequence: u64,
    observation: AudioResidencyObservation,
}

impl AudioResidency {
    pub(crate) fn new(active_generation: u64) -> Self {
        Self {
            active_generation,
            static_cache: HashMap::new(),
            use_sequence: 0,
            observation: AudioResidencyObservation::default(),
        }
    }

    pub(crate) fn observation(&self) -> AudioResidencyObservation {
        self.observation
    }

    pub(crate) fn active_generation(&self) -> u64 {
        self.active_generation
    }

    pub(crate) fn reset_generation(&mut self, generation: u64) {
        self.active_generation = generation;
        self.static_cache.clear();
        self.observation.static_buffers = 0;
        self.observation.static_decoded_bytes = 0;
    }

    pub(crate) fn preload_static_sound(
        &mut self,
        request: &AudioPreloadRequest,
        cancelled: impl Fn() -> bool,
    ) -> Result<PreloadDecodeOutcome> {
        if request.resource_generation != self.active_generation {
            return self.reject(AudioDecodeError::Limit(format!(
                "sound preload resource generation {} does not match active generation {}",
                request.resource_generation, self.active_generation
            )));
        }
        let key = AudioResourceKey {
            generation: request.resource_generation,
            path: request.ogg_path.clone(),
        };
        if self.static_cache.contains_key(&key) {
            return Ok(PreloadDecodeOutcome::Decoded);
        }

        let mut decoder = match BoundedAudioDecoder::open(&request.ogg_path, true) {
            Ok(decoder) => decoder,
            Err(error) => return self.reject(error),
        };
        let admitted_bytes = decoder.admitted_static_bytes();
        self.ensure_preload_capacity(admitted_bytes)?;
        let Some(data) = decoder.decode_static_until(cancelled)? else {
            return Ok(PreloadDecodeOutcome::Cancelled);
        };
        let decoded_bytes = pcm16_sample_bytes(data.samples.len())?;
        self.insert_static_owner(key, data, decoded_bytes)?;
        Ok(PreloadDecodeOutcome::Decoded)
    }

    pub(crate) fn install_preloaded(
        &mut self,
        mut prepared: AudioResidency,
    ) -> Result<PreloadInstallOutcome> {
        if prepared.active_generation != self.active_generation {
            return Err(AudioDecodeError::Limit(format!(
                "prepared sound generation {} does not match active generation {}",
                prepared.active_generation, self.active_generation
            ))
            .into());
        }
        self.observation.audio_admission_rejections = self
            .observation
            .audio_admission_rejections
            .saturating_add(prepared.observation.audio_admission_rejections);

        let mut entries = prepared.static_cache.drain().collect::<Vec<_>>();
        entries.sort_by(|(left, _), (right, _)| left.path.cmp(&right.path));
        let mut outcome = PreloadInstallOutcome::default();
        for (key, entry) in entries {
            if self.static_cache.contains_key(&key) {
                outcome.already_resident = outcome.already_resident.saturating_add(1);
                continue;
            }
            if let Err(error) = self.ensure_static_capacity(entry.decoded_bytes) {
                outcome.failures = outcome.failures.saturating_add(1);
                outcome.last_error = Some(error.to_string());
                continue;
            }
            if let Err(error) = self.insert_static_owner(key, entry.data, entry.decoded_bytes) {
                outcome.failures = outcome.failures.saturating_add(1);
                outcome.last_error = Some(error.to_string());
                continue;
            }
            outcome.installed = outcome.installed.saturating_add(1);
        }
        Ok(outcome)
    }

    #[cfg(test)]
    pub(crate) fn decoded_static_sound(
        &mut self,
        sound: &ResolvedSound,
    ) -> Result<DecodedStaticAudio> {
        let key = self.key_for_sound(sound)?;
        let next_use_sequence = self.use_sequence.wrapping_add(1);
        if let Some(entry) = self.static_cache.get_mut(&key) {
            self.use_sequence = next_use_sequence;
            entry.last_used = next_use_sequence;
            self.observation.static_cache_hits =
                self.observation.static_cache_hits.saturating_add(1);
            return Ok(entry.data.clone());
        }

        self.observation.static_cache_misses =
            self.observation.static_cache_misses.saturating_add(1);
        let mut decoder = match BoundedAudioDecoder::open(&sound.ogg_path, true) {
            Ok(decoder) => decoder,
            Err(error) => return self.reject(error),
        };
        let admitted_bytes = decoder.admitted_static_bytes();
        self.ensure_static_capacity(admitted_bytes)?;
        let data = match decoder.decode_static() {
            Ok(data) => data,
            Err(error) => return self.reject(error),
        };
        let decoded_bytes = pcm16_sample_bytes(data.samples.len())?;
        self.insert_static_owner(key, data.clone(), decoded_bytes)?;
        Ok(data)
    }

    #[cfg(test)]
    pub(crate) fn static_sound(&mut self, sound: &ResolvedSound) -> Result<StaticSoundData> {
        let data = self.decoded_static_sound(sound)?;
        let frames: Arc<[Frame]> = if data.channels == 1 {
            data.samples
                .iter()
                .map(|&sample| {
                    let s = f32::from(sample) / 32768.0;
                    Frame::new(s, s)
                })
                .collect()
        } else {
            data.samples
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| Frame::new(f32::from(pair[0]) / 32768.0, f32::from(pair[1]) / 32768.0))
                .collect()
        };
        Ok(StaticSoundData {
            sample_rate: data.sample_rate,
            frames,
            settings: StaticSoundSettings::default(),
            slice: None,
        })
    }

    /// Removes a preloaded static sound from the CPU staging cache on first
    /// OpenAL upload so OpenAL's buffer cache remains the single static owner.
    #[cfg(feature = "device")]
    pub(crate) fn take_openal_static_sound_if_cached(
        &mut self,
        sound: &ResolvedSound,
    ) -> Result<Option<OpenAlStaticSound>> {
        let key = self.key_for_sound(sound)?;
        let Some(entry) = self.static_cache.remove(&key) else {
            return Ok(None);
        };
        self.observation.static_decoded_bytes = self
            .observation
            .static_decoded_bytes
            .saturating_sub(entry.decoded_bytes);
        self.observation.static_buffers = self.static_cache.len();
        self.observation.static_cache_hits = self.observation.static_cache_hits.saturating_add(1);
        Ok(Some(OpenAlStaticSound {
            sample_rate: entry.data.sample_rate,
            channels: entry.data.channels,
            samples: entry.data.samples,
        }))
    }

    #[cfg(feature = "device")]
    pub(crate) fn record_static_cache_hit(&mut self) {
        self.observation.static_cache_hits = self.observation.static_cache_hits.saturating_add(1);
    }

    #[cfg(feature = "device")]
    pub(crate) fn check_generation(&mut self, generation: u64) -> Result<()> {
        if generation != self.active_generation {
            return self.reject(AudioDecodeError::Limit(format!(
                "sound resource generation {generation} does not match active generation {}",
                self.active_generation
            )));
        }
        Ok(())
    }

    #[cfg(feature = "device")]
    pub(crate) fn record_streaming_start(&mut self, sound: &ResolvedSound) -> Result<()> {
        self.key_for_sound(sound)?;
        self.observation.streaming_starts = self.observation.streaming_starts.saturating_add(1);
        Ok(())
    }

    #[cfg(feature = "device")]
    pub(crate) fn record_static_cache_miss(&mut self) {
        self.observation.static_cache_misses =
            self.observation.static_cache_misses.saturating_add(1);
    }

    #[cfg(feature = "device")]
    pub(crate) fn record_admission_rejection(&mut self) {
        self.observation.audio_admission_rejections = self
            .observation
            .audio_admission_rejections
            .saturating_add(1);
    }

    #[cfg(test)]
    pub(crate) fn streaming_sound(
        &mut self,
        sound: &ResolvedSound,
    ) -> Result<StreamingSoundData<AudioDecodeError>> {
        self.key_for_sound(sound)?;
        let decoder = match BoundedAudioDecoder::open(&sound.ogg_path, false) {
            Ok(decoder) => decoder,
            Err(error) => return self.reject(error),
        };
        self.observation.streaming_starts = self.observation.streaming_starts.saturating_add(1);
        Ok(StreamingSoundData::from_decoder(decoder))
    }

    #[cfg(all(feature = "device", test))]
    pub(crate) fn openal_streaming_sound(
        &mut self,
        sound: &ResolvedSound,
        looping: bool,
    ) -> Result<OpenAlStreamDecoder> {
        self.key_for_sound(sound)?;
        let decoder = match OpenAlStreamDecoder::open(&sound.ogg_path, looping) {
            Ok(decoder) => decoder,
            Err(error) => return self.reject(error),
        };
        self.observation.streaming_starts = self.observation.streaming_starts.saturating_add(1);
        Ok(decoder)
    }

    fn key_for_sound(&mut self, sound: &ResolvedSound) -> Result<AudioResourceKey> {
        if sound.resource_generation != self.active_generation {
            return self.reject(AudioDecodeError::Limit(format!(
                "sound resource generation {} does not match active generation {}",
                sound.resource_generation, self.active_generation
            )));
        }
        Ok(AudioResourceKey {
            generation: sound.resource_generation,
            path: sound.ogg_path.clone(),
        })
    }

    fn ensure_static_capacity(&mut self, required: u64) -> Result<()> {
        while self.static_cache.len() >= MAX_STATIC_SOUND_CACHE_BUFFERS
            || match self.observation.static_decoded_bytes.checked_add(required) {
                Some(bytes) => bytes > MAX_STATIC_SOUND_CACHE_BYTES,
                None => true,
            }
        {
            let Some(eviction_key) = self
                .static_cache
                .iter()
                .filter(|(_, entry)| Arc::strong_count(&entry.data.samples) == 1)
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            else {
                return self.reject(AudioDecodeError::Limit(format!(
                    "static decoded audio cache cannot admit another owner ({required} decoded bytes) within the {MAX_STATIC_SOUND_CACHE_BUFFERS} owner / {MAX_STATIC_SOUND_CACHE_BYTES} byte limits because retained sounds are active"
                )));
            };
            let evicted = self
                .static_cache
                .remove(&eviction_key)
                .expect("selected audio cache entry exists");
            self.observation.static_decoded_bytes = self
                .observation
                .static_decoded_bytes
                .checked_sub(evicted.decoded_bytes)
                .expect("audio cache accounting cannot underflow");
            self.observation.static_cache_evictions =
                self.observation.static_cache_evictions.saturating_add(1);
            self.observation.static_buffers = self.static_cache.len();
        }
        Ok(())
    }

    fn ensure_preload_capacity(&mut self, required: u64) -> Result<()> {
        let exceeds_count = self.static_cache.len() >= MAX_STATIC_SOUND_CACHE_BUFFERS;
        let exceeds_bytes = self
            .observation
            .static_decoded_bytes
            .checked_add(required)
            .is_none_or(|bytes| bytes > MAX_STATIC_SOUND_CACHE_BYTES);
        if exceeds_count || exceeds_bytes {
            return self.reject(AudioDecodeError::Limit(format!(
                "preloaded static audio cannot exceed the {MAX_STATIC_SOUND_CACHE_BUFFERS} owner / {MAX_STATIC_SOUND_CACHE_BYTES} byte cache limits"
            )));
        }
        Ok(())
    }

    fn insert_static_owner(
        &mut self,
        key: AudioResourceKey,
        data: DecodedStaticAudio,
        decoded_bytes: u64,
    ) -> Result<()> {
        self.use_sequence = self.use_sequence.wrapping_add(1);
        self.observation.static_decoded_bytes = self
            .observation
            .static_decoded_bytes
            .checked_add(decoded_bytes)
            .ok_or_else(|| AudioDecodeError::Limit("static cache byte count overflow".into()))?;
        self.observation.peak_static_decoded_bytes = self
            .observation
            .peak_static_decoded_bytes
            .max(self.observation.static_decoded_bytes);
        self.static_cache.insert(
            key,
            StaticCacheEntry {
                data,
                decoded_bytes,
                last_used: self.use_sequence,
            },
        );
        self.observation.static_buffers = self.static_cache.len();
        Ok(())
    }

    fn reject<T>(&mut self, error: AudioDecodeError) -> Result<T> {
        self.observation.audio_admission_rejections = self
            .observation
            .audio_admission_rejections
            .saturating_add(1);
        Err(error.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreloadDecodeOutcome {
    Decoded,
    Cancelled,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PreloadInstallOutcome {
    pub(crate) installed: usize,
    pub(crate) already_resident: usize,
    pub(crate) failures: usize,
    pub(crate) last_error: Option<String>,
}

#[derive(Debug, Error)]
pub(crate) enum AudioDecodeError {
    #[error("{0}")]
    Limit(String),
    #[error("audio I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("audio decode failed: {0}")]
    Symphonia(#[from] symphonia::core::errors::Error),
}

struct BoundedAudioDecoder {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn SymphoniaDecoder>,
    sample_rate: u32,
    channels: u16,
    admitted_frames: usize,
    emitted_frames: usize,
    track_id: u32,
}

impl BoundedAudioDecoder {
    fn open(path: &Path, static_sound: bool) -> Result<Self, AudioDecodeError> {
        let file = BoundedAudioFile::open(path)?;
        let media_source = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        hint.with_extension("ogg");
        let format_reader = symphonia::default::get_probe().probe(
            &hint,
            media_source,
            Default::default(),
            Default::default(),
        )?;
        let track = format_reader
            .default_track(TrackType::Audio)
            .ok_or_else(|| AudioDecodeError::Limit("audio has no default track".to_string()))?;
        let codec_params = track
            .codec_params
            .as_ref()
            .and_then(|params| params.audio())
            .ok_or_else(|| AudioDecodeError::Limit("audio track has no codec parameters".into()))?;
        let sample_rate = codec_params
            .sample_rate
            .ok_or_else(|| AudioDecodeError::Limit("audio sample rate is unknown".to_string()))?;
        let channels = codec_params.channels.as_ref().ok_or_else(|| {
            AudioDecodeError::Limit("audio channel configuration is unknown".to_string())
        })?;
        let channel_count = channels.count();
        let admitted_frames_u64 =
            admit_audio_metadata(sample_rate, channel_count, track.num_frames, static_sound)?;
        let admitted_frames = usize::try_from(admitted_frames_u64)
            .map_err(|_| AudioDecodeError::Limit("audio frame count does not fit usize".into()))?;
        let track_id = track.id;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(codec_params, &Default::default())?;
        Ok(Self {
            format_reader,
            decoder,
            sample_rate,
            channels: u16::try_from(channel_count)
                .expect("admitted mono/stereo channel count fits u16"),
            admitted_frames,
            emitted_frames: 0,
            track_id,
        })
    }

    fn decode_audio_packet(
        &mut self,
        packet: &Packet,
    ) -> Result<GenericAudioBufferRef<'_>, AudioDecodeError> {
        let buffer = self.decoder.decode(packet)?;
        let remaining = self.admitted_frames - self.emitted_frames;
        admit_decoded_chunk(buffer.frames(), remaining)?;
        match buffer.spec().channels().count() {
            1 | 2 => {}
            channels => {
                return Err(AudioDecodeError::Limit(format!(
                    "decoded audio channel count {channels} is outside the mono/stereo limit"
                )));
            }
        }
        self.emitted_frames += buffer.frames();
        Ok(buffer)
    }

    fn next_audio_buffer(&mut self) -> Result<(GenericAudioBufferRef<'_>, i64), AudioDecodeError> {
        let packet = self.next_track_packet()?;
        Ok((self.decode_audio_packet(&packet)?, packet.pts.get()))
    }

    fn admitted_static_bytes(&self) -> u64 {
        pcm16_frame_bytes(self.admitted_frames, self.channels)
            .expect("admitted audio frame byte count was validated")
    }

    #[cfg(test)]
    fn decode_static(&mut self) -> Result<DecodedStaticAudio, AudioDecodeError> {
        self.decode_static_until(|| false)?.ok_or_else(|| {
            AudioDecodeError::Limit("unconditional static decode was cancelled".into())
        })
    }

    fn decode_static_until(
        &mut self,
        cancelled: impl Fn() -> bool,
    ) -> Result<Option<DecodedStaticAudio>, AudioDecodeError> {
        let channels = self.channels;
        let initial_samples = self
            .admitted_frames
            .min(INITIAL_STATIC_RESERVE_FRAMES)
            .saturating_mul(usize::from(channels));
        let mut samples = Vec::new();
        samples.try_reserve(initial_samples).map_err(|error| {
            AudioDecodeError::Limit(format!(
                "cannot reserve admitted static audio sample buffer: {error}"
            ))
        })?;
        loop {
            if cancelled() {
                return Ok(None);
            }
            let packet = match self.next_track_packet() {
                Ok(packet) => packet,
                Err(AudioDecodeError::Symphonia(symphonia::core::errors::Error::IoError(
                    error,
                ))) if error.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error),
            };
            let buffer = self.decode_audio_packet(&packet)?;
            let needed = buffer.frames().saturating_mul(usize::from(channels));
            samples.try_reserve(needed).map_err(|error| {
                AudioDecodeError::Limit(format!(
                    "cannot grow admitted static audio sample buffer: {error}"
                ))
            })?;
            visit_frames(&buffer, 0, |frame| {
                samples.push(vanilla_pcm16(frame.left));
                if channels == 2 {
                    samples.push(vanilla_pcm16(frame.right));
                }
            });
        }
        if samples.is_empty() {
            return Err(AudioDecodeError::Limit(
                "decoded static audio contains zero frames".into(),
            ));
        }
        Ok(Some(DecodedStaticAudio {
            sample_rate: self.sample_rate,
            channels,
            samples: samples.into(),
        }))
    }

    fn next_track_packet(&mut self) -> Result<Packet, AudioDecodeError> {
        loop {
            let packet = self.format_reader.next_packet()?.ok_or_else(|| {
                AudioDecodeError::Symphonia(symphonia::core::errors::Error::IoError(
                    io::Error::new(io::ErrorKind::UnexpectedEof, "audio stream ended"),
                ))
            })?;
            if packet.track_id == self.track_id {
                return Ok(packet);
            }
        }
    }

    fn seek_to_frame(&mut self, index: usize) -> Result<usize, AudioDecodeError> {
        if index > self.admitted_frames {
            return Err(AudioDecodeError::Limit(format!(
                "audio seek frame {index} exceeds admitted duration {}",
                self.admitted_frames
            )));
        }
        let seeked = self.format_reader.seek(
            SeekMode::Accurate,
            SeekTo::Timestamp {
                ts: Timestamp::new(
                    i64::try_from(index)
                        .map_err(|_| AudioDecodeError::Limit("audio seek overflow".into()))?,
                ),
                track_id: self.track_id,
            },
        )?;
        let actual = usize::try_from(seeked.actual_ts.get())
            .map_err(|_| AudioDecodeError::Limit("audio seek result overflow".into()))?;
        if actual > self.admitted_frames {
            return Err(AudioDecodeError::Limit(
                "audio seek result exceeds admitted duration".into(),
            ));
        }
        self.emitted_frames = actual;
        Ok(actual)
    }
}

#[cfg(any(feature = "device", test))]
pub(crate) fn decode_static_pack_until(
    path: &Path,
    cancelled: impl Fn() -> bool,
) -> Option<Result<DecodedStaticAudio, AudioDecodeError>> {
    let mut decoder = match BoundedAudioDecoder::open(path, true) {
        Ok(decoder) => decoder,
        Err(error) => return Some(Err(error)),
    };
    let decoded = match decoder.decode_static_until(cancelled) {
        Ok(Some(value)) => value,
        Ok(None) => return None,
        Err(error) => return Some(Err(error)),
    };
    if let Err(error) = pcm16_sample_bytes(decoded.samples.len()) {
        return Some(Err(error));
    }
    Some(Ok(decoded))
}

#[cfg(all(feature = "device", test))]
impl OpenAlStaticSound {
    pub(crate) fn read(path: &Path) -> Result<Self, AudioDecodeError> {
        let mut decoder = BoundedAudioDecoder::open(path, true)?;
        let data = decoder.decode_static()?;
        Ok(Self {
            sample_rate: data.sample_rate,
            channels: data.channels,
            samples: data.samples,
        })
    }
}

#[cfg(any(feature = "device", test))]
impl OpenAlStreamDecoder {
    pub(crate) fn open(path: &Path, looping: bool) -> Result<Self, AudioDecodeError> {
        let decoder = BoundedAudioDecoder::open(path, false)?;
        Ok(Self {
            decoder,
            path: path.to_path_buf(),
            looping,
            seek_frame: None,
        })
    }

    pub(crate) fn open_at(
        path: &Path,
        looping: bool,
        frame: u64,
    ) -> Result<Self, AudioDecodeError> {
        let mut decoder = BoundedAudioDecoder::open(path, false)?;
        if decoder.admitted_frames == 0 {
            return Err(AudioDecodeError::Limit("audio declares zero frames".into()));
        }
        let frame = if looping {
            frame % decoder.admitted_frames as u64
        } else {
            frame
        };
        decoder.seek_to_frame(frame as usize)?;
        decoder.decoder.reset();
        Ok(Self {
            decoder,
            path: path.to_path_buf(),
            looping,
            seek_frame: Some(frame as usize),
        })
    }

    #[cfg(feature = "device")]
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(feature = "device")]
    pub(crate) fn looping(&self) -> bool {
        self.looping
    }

    pub(crate) fn read_one_second(&mut self) -> Result<OpenAlPcmChunk, AudioDecodeError> {
        let expected_samples = usize::try_from(self.decoder.sample_rate)
            .ok()
            .and_then(|rate| rate.checked_mul(usize::from(self.decoder.channels)))
            .ok_or_else(|| AudioDecodeError::Limit("streaming PCM size overflow".into()))?;
        let channels = self.decoder.channels;
        let mut samples = Vec::with_capacity(expected_samples);
        let mut reopened_empty = false;
        loop {
            match self.decoder.next_audio_buffer() {
                Ok((buffer, timestamp)) => {
                    // Vorbis seek preroll can advance timestamps without producing PCM.
                    let skipped = self.seek_frame.map_or(0, |frame| {
                        ((frame as i64 - timestamp).max(0) as usize).min(buffer.frames())
                    });
                    if skipped < buffer.frames() {
                        self.seek_frame = None;
                        visit_frames(&buffer, skipped, |frame| {
                            samples.push(vanilla_pcm16(frame.left));
                            if channels == 2 {
                                samples.push(vanilla_pcm16(frame.right));
                            }
                        });
                    }
                }
                Err(AudioDecodeError::Symphonia(symphonia::core::errors::Error::IoError(
                    error,
                ))) if error.kind() == io::ErrorKind::UnexpectedEof => {
                    if samples.is_empty() && self.looping && !reopened_empty {
                        reopened_empty = true;
                        self.decoder = BoundedAudioDecoder::open(&self.path, false)?;
                        continue;
                    }
                    break;
                }
                Err(error) => return Err(error),
            }
            if samples.len() >= expected_samples {
                break;
            }
        }
        Ok(OpenAlPcmChunk {
            sample_rate: self.decoder.sample_rate,
            channels: self.decoder.channels,
            samples,
        })
    }
}

#[cfg(test)]
fn pcm16_samples(frames: &[StereoFrame], channels: u16) -> Vec<i16> {
    let mut samples = Vec::with_capacity(frames.len() * usize::from(channels));
    for frame in frames {
        samples.push(vanilla_pcm16(frame.left));
        if channels == 2 {
            samples.push(vanilla_pcm16(frame.right));
        }
    }
    samples
}

fn vanilla_pcm16(sample: f32) -> i16 {
    (sample * 32767.5 - 0.5).clamp(-32768.0, 32767.0) as i16
}

#[cfg(test)]
impl KiraStreamingDecoder for BoundedAudioDecoder {
    type Error = AudioDecodeError;

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn num_frames(&self) -> usize {
        self.admitted_frames
    }

    fn decode(&mut self) -> Result<Vec<Frame>, Self::Error> {
        let (buffer, _) = self.next_audio_buffer()?;
        let mut frames = Vec::with_capacity(buffer.frames());
        visit_frames(&buffer, 0, |frame| {
            frames.push(Frame::new(frame.left, frame.right));
        });
        Ok(frames)
    }

    fn seek(&mut self, index: usize) -> Result<usize, Self::Error> {
        self.seek_to_frame(index)
    }
}

fn admit_audio_metadata(
    sample_rate: u32,
    channels: usize,
    frames: Option<u64>,
    static_sound: bool,
) -> Result<u64, AudioDecodeError> {
    if sample_rate == 0 || sample_rate > MAX_AUDIO_SAMPLE_RATE {
        return Err(AudioDecodeError::Limit(format!(
            "audio sample rate {sample_rate} exceeds the {MAX_AUDIO_SAMPLE_RATE} Hz limit"
        )));
    }
    if !(1..=2).contains(&channels) {
        return Err(AudioDecodeError::Limit(format!(
            "audio channel count {channels} is outside the mono/stereo limit"
        )));
    }
    let frames =
        frames.ok_or_else(|| AudioDecodeError::Limit("audio duration is unknown".to_string()))?;
    if frames == 0 {
        return Err(AudioDecodeError::Limit(
            "audio declares zero frames".to_string(),
        ));
    }
    let duration_frames = u64::from(sample_rate)
        .checked_mul(MAX_AUDIO_DURATION_SECONDS)
        .ok_or_else(|| AudioDecodeError::Limit("audio duration limit overflow".into()))?;
    if frames > duration_frames {
        return Err(AudioDecodeError::Limit(format!(
            "audio declares {frames} frames, exceeding the {MAX_AUDIO_DURATION_SECONDS} second duration limit at {sample_rate} Hz"
        )));
    }
    if static_sound {
        let bytes_per_frame = u64::try_from(channels)
            .ok()
            .and_then(|ch| ch.checked_mul(u64::try_from(size_of::<i16>()).ok()?))
            .ok_or_else(|| AudioDecodeError::Limit("static decoded audio byte overflow".into()))?;
        let decoded_bytes = frames
            .checked_mul(bytes_per_frame)
            .ok_or_else(|| AudioDecodeError::Limit("static decoded audio byte overflow".into()))?;
        if decoded_bytes > MAX_STATIC_SOUND_DECODED_BYTES {
            return Err(AudioDecodeError::Limit(format!(
                "static audio declares {decoded_bytes} decoded bytes, exceeding the {MAX_STATIC_SOUND_DECODED_BYTES} byte per-sound limit"
            )));
        }
    }
    Ok(frames)
}

fn visit_frames(buffer: &GenericAudioBufferRef<'_>, skip: usize, visit: impl FnMut(StereoFrame)) {
    match buffer {
        GenericAudioBufferRef::U8(buffer) => visit_typed_frames(buffer, skip, visit),
        GenericAudioBufferRef::U16(buffer) => visit_typed_frames(buffer, skip, visit),
        GenericAudioBufferRef::U24(buffer) => visit_typed_frames(buffer, skip, visit),
        GenericAudioBufferRef::U32(buffer) => visit_typed_frames(buffer, skip, visit),
        GenericAudioBufferRef::S8(buffer) => visit_typed_frames(buffer, skip, visit),
        GenericAudioBufferRef::S16(buffer) => visit_typed_frames(buffer, skip, visit),
        GenericAudioBufferRef::S24(buffer) => visit_typed_frames(buffer, skip, visit),
        GenericAudioBufferRef::S32(buffer) => visit_typed_frames(buffer, skip, visit),
        GenericAudioBufferRef::F32(buffer) => visit_typed_frames(buffer, skip, visit),
        GenericAudioBufferRef::F64(buffer) => visit_typed_frames(buffer, skip, visit),
    }
}

fn visit_typed_frames<S: Sample>(
    buffer: &AudioBuffer<S>,
    skip: usize,
    mut visit: impl FnMut(StereoFrame),
) where
    f32: FromSample<S>,
{
    let left = &buffer
        .plane(0)
        .expect("mono and stereo buffers have a left plane")[skip..];
    if buffer.spec().channels().count() == 1 {
        for &sample in left {
            visit(StereoFrame::from_mono(sample.into_sample()));
        }
    } else {
        let right = &buffer.plane(1).expect("stereo buffers have a right plane")[skip..];
        for (&l, &r) in left.iter().zip(right) {
            visit(StereoFrame::new(l.into_sample(), r.into_sample()));
        }
    }
}

fn admit_decoded_chunk(frames: usize, remaining: usize) -> Result<(), AudioDecodeError> {
    if frames > MAX_AUDIO_DECODE_PACKET_FRAMES {
        return Err(AudioDecodeError::Limit(format!(
            "decoded audio packet has {frames} frames, exceeding the {MAX_AUDIO_DECODE_PACKET_FRAMES} frame packet limit"
        )));
    }
    if frames > remaining {
        return Err(AudioDecodeError::Limit(format!(
            "decoded audio packet has {} frames but only {remaining} admitted frames remain",
            frames
        )));
    }
    Ok(())
}

fn pcm16_sample_bytes(samples: usize) -> Result<u64, AudioDecodeError> {
    u64::try_from(samples)
        .ok()
        .and_then(|samples| {
            samples.checked_mul(u64::try_from(size_of::<i16>()).expect("sample size fits u64"))
        })
        .ok_or_else(|| AudioDecodeError::Limit("decoded audio byte count overflow".into()))
}

fn pcm16_frame_bytes(frames: usize, channels: u16) -> Result<u64, AudioDecodeError> {
    u64::try_from(frames)
        .ok()
        .and_then(|frames| frames.checked_mul(u64::from(channels)))
        .and_then(|samples| {
            samples.checked_mul(u64::try_from(size_of::<i16>()).expect("sample size fits u64"))
        })
        .ok_or_else(|| AudioDecodeError::Limit("decoded audio byte count overflow".into()))
}

struct BoundedAudioFile {
    file: File,
    length: u64,
    position: u64,
}

impl BoundedAudioFile {
    fn open(path: &Path) -> Result<Self, AudioDecodeError> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(AudioDecodeError::Limit(format!(
                "audio resource {} is not a regular file",
                path.display()
            )));
        }
        let length = metadata.len();
        if length > MAX_AUDIO_COMPRESSED_BYTES {
            return Err(AudioDecodeError::Limit(format!(
                "audio resource {} has {length} compressed bytes, exceeding the {MAX_AUDIO_COMPRESSED_BYTES} byte limit",
                path.display()
            )));
        }
        Ok(Self {
            file,
            length,
            position: 0,
        })
    }
}

impl Read for BoundedAudioFile {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let remaining = self.length.saturating_sub(self.position);
        let maximum = usize::try_from(remaining.min(buffer.len() as u64))
            .expect("bounded read length fits usize");
        let read = self.file.read(&mut buffer[..maximum])?;
        self.position = self
            .position
            .checked_add(read as u64)
            .ok_or_else(|| io::Error::other("audio file position overflow"))?;
        Ok(read)
    }
}

impl Seek for BoundedAudioFile {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let target = match position {
            SeekFrom::Start(position) => i128::from(position),
            SeekFrom::Current(offset) => i128::from(self.position) + i128::from(offset),
            SeekFrom::End(offset) => i128::from(self.length) + i128::from(offset),
        };
        if target < 0 || target > i128::from(self.length) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "audio seek exceeds admitted compressed file",
            ));
        }
        let target = u64::try_from(target)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "audio seek overflow"))?;
        self.position = self.file.seek(SeekFrom::Start(target))?;
        Ok(self.position)
    }
}

impl MediaSource for BoundedAudioFile {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn metadata_admission_accepts_exact_limits_and_rejects_each_overflow() {
        let stereo_pcm16_frame_bytes = (2 * size_of::<i16>()) as u64;
        admit_audio_metadata(
            MAX_AUDIO_SAMPLE_RATE,
            2,
            Some(MAX_STATIC_SOUND_DECODED_BYTES / stereo_pcm16_frame_bytes),
            true,
        )
        .unwrap();
        assert!(admit_audio_metadata(48_000, 1, Some(0), false).is_err());
        assert!(admit_audio_metadata(48_000, 2, Some(0), true).is_err());
        assert!(admit_audio_metadata(MAX_AUDIO_SAMPLE_RATE + 1, 2, Some(1), false).is_err());
        assert!(admit_audio_metadata(48_000, 3, Some(1), false).is_err());
        assert!(admit_audio_metadata(48_000, 2, None, false).is_err());
        assert!(admit_audio_metadata(
            48_000,
            2,
            Some(48_000 * (MAX_AUDIO_DURATION_SECONDS + 1)),
            false
        )
        .is_err());
        admit_decoded_chunk(
            MAX_AUDIO_DECODE_PACKET_FRAMES,
            MAX_AUDIO_DECODE_PACKET_FRAMES,
        )
        .unwrap();
        assert!(admit_decoded_chunk(MAX_AUDIO_DECODE_PACKET_FRAMES + 1, usize::MAX).is_err());
        assert!(admit_decoded_chunk(2, 1).is_err());
        assert!(admit_audio_metadata(
            48_000,
            2,
            Some(MAX_STATIC_SOUND_DECODED_BYTES / stereo_pcm16_frame_bytes + 1),
            true
        )
        .is_err());
    }

    #[test]
    fn pcm16_conversion_matches_vanilla_chunked_sample_buffer() {
        assert_eq!(vanilla_pcm16(-1.0), -32768);
        assert_eq!(vanilla_pcm16(0.0), 0);
        assert_eq!(vanilla_pcm16(1.0), 32767);
        assert_eq!(vanilla_pcm16(2.0), 32767);
        let frames = [StereoFrame::new(-1.0, 1.0), StereoFrame::new(0.5, -0.5)];
        assert_eq!(pcm16_samples(&frames, 1), [-32768, 16383]);
        assert_eq!(pcm16_samples(&frames, 2), [-32768, 32767, 16383, -16384]);
    }

    #[test]
    fn compressed_file_admission_rejects_before_probe() {
        let exact_path = unique_temp_path("exact-sized-audio");
        let exact_file = File::create(&exact_path).unwrap();
        exact_file.set_len(MAX_AUDIO_COMPRESSED_BYTES).unwrap();
        drop(exact_file);
        assert!(BoundedAudioFile::open(&exact_path).is_ok());

        let path = unique_temp_path("oversized-audio");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_AUDIO_COMPRESSED_BYTES + 1).unwrap();
        drop(file);

        let error = BoundedAudioDecoder::open(&path, false)
            .err()
            .expect("oversized audio should fail")
            .to_string();
        assert!(error.contains("compressed bytes"), "{error}");

        std::fs::remove_file(exact_path).unwrap();
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn malformed_audio_is_rejected_without_a_cache_owner() {
        let path = unique_temp_path("malformed-audio");
        std::fs::write(&path, b"not an ogg stream").unwrap();
        let sound = ResolvedSound {
            event_id: "minecraft:test".into(),
            sound_name: "minecraft:test".into(),
            ogg_path: path.clone(),
            resource_generation: 7,
            stream: false,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        };
        let mut residency = AudioResidency::new(7);

        assert!(residency.decoded_static_sound(&sound).is_err());
        assert_eq!(residency.observation().static_buffers, 0);
        assert_eq!(residency.observation().audio_admission_rejections, 1);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn truncated_ogg_is_rejected_without_a_cache_owner() {
        let path = unique_temp_path("truncated-audio");
        std::fs::write(&path, b"OggS\0\x02\0\0").unwrap();
        let sound = test_sound(path.clone(), 4);
        let mut residency = AudioResidency::new(4);

        assert!(residency.decoded_static_sound(&sound).is_err());
        assert_eq!(residency.observation().static_buffers, 0);
        assert_eq!(residency.observation().static_decoded_bytes, 0);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn stale_generation_is_rejected_before_open_and_reset_drops_owners() {
        let mut residency = AudioResidency::new(2);
        let sound = ResolvedSound {
            event_id: "minecraft:test".into(),
            sound_name: "minecraft:test".into(),
            ogg_path: PathBuf::from("does-not-exist.ogg"),
            resource_generation: 1,
            stream: false,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        };

        let error = residency
            .decoded_static_sound(&sound)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("does not match active generation"),
            "{error}"
        );
        residency.reset_generation(3);
        assert_eq!(residency.observation().static_buffers, 0);
        assert_eq!(residency.observation().static_decoded_bytes, 0);
    }

    #[test]
    fn exact_generation_path_hit_shares_the_frame_owner() {
        let path = PathBuf::from("shared.ogg");
        let data = test_static_sound(4);
        let expected_samples = Arc::clone(&data.samples);
        let mut residency = AudioResidency::new(9);
        let bytes = pcm16_frame_bytes(4, 2).unwrap();
        residency.static_cache.insert(
            AudioResourceKey {
                generation: 9,
                path: path.clone(),
            },
            StaticCacheEntry {
                data,
                decoded_bytes: bytes,
                last_used: 0,
            },
        );
        residency.observation.static_buffers = 1;
        residency.observation.static_decoded_bytes = bytes;
        let sound = test_sound(path, 9);

        let first = residency.decoded_static_sound(&sound).unwrap();
        let second = residency.decoded_static_sound(&sound).unwrap();

        assert!(Arc::ptr_eq(&first.samples, &expected_samples));
        assert!(Arc::ptr_eq(&second.samples, &expected_samples));
        assert_eq!(residency.observation().static_cache_hits, 2);
        assert_eq!(residency.observation().static_cache_misses, 0);
    }

    #[test]
    fn capacity_evicts_only_inactive_lru_and_rejects_when_all_are_pinned() {
        let mut residency = AudioResidency::new(3);
        for (path, last_used) in [("older.ogg", 1), ("newer.ogg", 2)] {
            residency.static_cache.insert(
                AudioResourceKey {
                    generation: 3,
                    path: PathBuf::from(path),
                },
                StaticCacheEntry {
                    data: test_static_sound(1),
                    decoded_bytes: MAX_STATIC_SOUND_CACHE_BYTES / 2,
                    last_used,
                },
            );
        }
        residency.observation.static_buffers = 2;
        residency.observation.static_decoded_bytes = MAX_STATIC_SOUND_CACHE_BYTES;
        residency.ensure_static_capacity(1).unwrap();
        assert!(!residency.static_cache.contains_key(&AudioResourceKey {
            generation: 3,
            path: PathBuf::from("older.ogg"),
        }));
        assert!(residency.static_cache.contains_key(&AudioResourceKey {
            generation: 3,
            path: PathBuf::from("newer.ogg"),
        }));
        assert_eq!(residency.observation().static_cache_evictions, 1);

        residency.static_cache.clear();

        let active = test_static_sound(1);
        let active_pin = Arc::clone(&active.samples);
        residency.static_cache.insert(
            AudioResourceKey {
                generation: 3,
                path: PathBuf::from("active.ogg"),
            },
            StaticCacheEntry {
                data: active,
                decoded_bytes: MAX_STATIC_SOUND_CACHE_BYTES,
                last_used: 2,
            },
        );
        residency.observation.static_buffers = 1;
        residency.observation.static_decoded_bytes = MAX_STATIC_SOUND_CACHE_BYTES;
        let error = residency.ensure_static_capacity(1).unwrap_err().to_string();
        assert!(error.contains("retained sounds are active"), "{error}");
        assert_eq!(Arc::strong_count(&active_pin), 2);
        assert_eq!(residency.static_cache.len(), 1);

        drop(active_pin);
        residency.ensure_static_capacity(1).unwrap();
        assert!(residency.static_cache.is_empty());

        let reset_data = test_static_sound(1);
        let reset_pin = Arc::clone(&reset_data.samples);
        let reset_bytes = pcm16_frame_bytes(1, 2).unwrap();
        residency.static_cache.insert(
            AudioResourceKey {
                generation: 3,
                path: PathBuf::from("reset-pinned.ogg"),
            },
            StaticCacheEntry {
                data: reset_data,
                decoded_bytes: reset_bytes,
                last_used: 3,
            },
        );
        residency.observation.static_buffers = 1;
        residency.observation.static_decoded_bytes = reset_bytes;
        residency.reset_generation(4);
        assert_eq!(residency.observation().static_buffers, 0);
        assert_eq!(residency.observation().static_decoded_bytes, 0);
        assert_eq!(Arc::strong_count(&reset_pin), 1);
    }

    fn test_static_sound(frames: usize) -> DecodedStaticAudio {
        DecodedStaticAudio {
            sample_rate: 48_000,
            channels: 2,
            samples: vec![0_i16; frames * 2].into(),
        }
    }

    fn test_sound(path: PathBuf, generation: u64) -> ResolvedSound {
        ResolvedSound {
            event_id: "minecraft:test".into(),
            sound_name: "minecraft:test".into(),
            ogg_path: path,
            resource_generation: generation,
            stream: false,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        }
    }

    fn unique_temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-{label}-{nanos}.ogg"))
    }
}
