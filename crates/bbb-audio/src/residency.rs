use std::{
    collections::HashMap,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
    mem::size_of,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Result;
use kira::{
    sound::{
        static_sound::{StaticSoundData, StaticSoundSettings},
        streaming::{Decoder as KiraStreamingDecoder, StreamingSoundData},
    },
    Frame,
};
use symphonia::core::{
    audio::{AudioBuffer, AudioBufferRef, Signal},
    codecs::Decoder as SymphoniaDecoder,
    conv::{FromSample, IntoSample},
    formats::{FormatReader, SeekMode, SeekTo},
    io::{MediaSource, MediaSourceStream},
    probe::Hint,
    sample::Sample,
};
use thiserror::Error;

use crate::ResolvedSound;

pub(crate) const MAX_AUDIO_COMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_AUDIO_SAMPLE_RATE: u32 = 192_000;
pub(crate) const MAX_AUDIO_DURATION_SECONDS: u64 = 30 * 60;
pub(crate) const MAX_AUDIO_DECODE_PACKET_FRAMES: usize = 65_536;
pub(crate) const MAX_STATIC_SOUND_DECODED_BYTES: u64 = 256 * 1024 * 1024;
pub(crate) const MAX_STATIC_SOUND_CACHE_BYTES: u64 = 512 * 1024 * 1024;
pub(crate) const MAX_STATIC_SOUND_CACHE_BUFFERS: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioFileProbe {
    pub sample_rate: u32,
    pub frames: usize,
}

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioResidencyObservation {
    pub static_buffers: usize,
    pub static_decoded_bytes: u64,
    pub peak_static_decoded_bytes: u64,
    pub static_cache_hits: u64,
    pub static_cache_misses: u64,
    pub static_cache_evictions: u64,
    pub audio_admission_rejections: u64,
    pub streaming_starts: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AudioResourceKey {
    generation: u64,
    path: PathBuf,
}

struct StaticCacheEntry {
    data: StaticSoundData,
    decoded_bytes: u64,
    last_used: u64,
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

    pub(crate) fn reset_generation(&mut self, generation: u64) {
        self.active_generation = generation;
        self.static_cache.clear();
        self.observation.static_buffers = 0;
        self.observation.static_decoded_bytes = 0;
    }

    pub(crate) fn static_sound(&mut self, sound: &ResolvedSound) -> Result<StaticSoundData> {
        let key = self.key_for_sound(sound)?;
        self.use_sequence = self.use_sequence.wrapping_add(1);
        if let Some(entry) = self.static_cache.get_mut(&key) {
            entry.last_used = self.use_sequence;
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
        let decoded_bytes = frame_bytes(data.frames.len())?;
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
                data: data.clone(),
                decoded_bytes,
                last_used: self.use_sequence,
            },
        );
        self.observation.static_buffers = self.static_cache.len();
        Ok(data)
    }

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
                .filter(|(_, entry)| Arc::strong_count(&entry.data.frames) == 1)
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

    fn reject<T>(&mut self, error: AudioDecodeError) -> Result<T> {
        self.observation.audio_admission_rejections = self
            .observation
            .audio_admission_rejections
            .saturating_add(1);
        Err(error.into())
    }
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
        let format_reader = symphonia::default::get_probe()
            .format(
                &hint,
                media_source,
                &Default::default(),
                &Default::default(),
            )?
            .format;
        let track = format_reader
            .default_track()
            .ok_or_else(|| AudioDecodeError::Limit("audio has no default track".to_string()))?;
        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or_else(|| AudioDecodeError::Limit("audio sample rate is unknown".to_string()))?;
        let channels = track.codec_params.channels.ok_or_else(|| {
            AudioDecodeError::Limit("audio channel configuration is unknown".to_string())
        })?;
        let admitted_frames_u64 = admit_audio_metadata(
            sample_rate,
            channels.count(),
            track.codec_params.n_frames,
            static_sound,
        )?;
        let admitted_frames = usize::try_from(admitted_frames_u64)
            .map_err(|_| AudioDecodeError::Limit("audio frame count does not fit usize".into()))?;
        let decoder =
            symphonia::default::get_codecs().make(&track.codec_params, &Default::default())?;
        let track_id = track.id;
        Ok(Self {
            format_reader,
            decoder,
            sample_rate,
            admitted_frames,
            emitted_frames: 0,
            track_id,
        })
    }

    fn admitted_static_bytes(&self) -> u64 {
        frame_bytes(self.admitted_frames).expect("admitted audio frame byte count was validated")
    }

    fn decode_static(&mut self) -> Result<StaticSoundData, AudioDecodeError> {
        let mut frames = Vec::new();
        frames
            .try_reserve_exact(self.admitted_frames)
            .map_err(|error| {
                AudioDecodeError::Limit(format!(
                    "cannot reserve admitted static audio frame owner: {error}"
                ))
            })?;
        loop {
            let packet = match self.next_track_packet() {
                Ok(packet) => packet,
                Err(AudioDecodeError::Symphonia(symphonia::core::errors::Error::IoError(
                    error,
                ))) if error.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(error) => return Err(error),
            };
            let buffer = self.decoder.decode(&packet)?;
            let remaining = self
                .admitted_frames
                .checked_sub(self.emitted_frames)
                .ok_or_else(|| AudioDecodeError::Limit("decoded audio frame overflow".into()))?;
            let mut decoded = frames_from_buffer(&buffer, remaining)?;
            self.emitted_frames = self
                .emitted_frames
                .checked_add(decoded.len())
                .ok_or_else(|| AudioDecodeError::Limit("decoded audio frame overflow".into()))?;
            frames.append(&mut decoded);
        }
        Ok(StaticSoundData {
            sample_rate: self.sample_rate,
            frames: frames.into(),
            settings: StaticSoundSettings::default(),
            slice: None,
        })
    }

    fn next_track_packet(&mut self) -> Result<symphonia::core::formats::Packet, AudioDecodeError> {
        loop {
            let packet = self.format_reader.next_packet()?;
            if packet.track_id() == self.track_id {
                return Ok(packet);
            }
        }
    }
}

impl KiraStreamingDecoder for BoundedAudioDecoder {
    type Error = AudioDecodeError;

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn num_frames(&self) -> usize {
        self.admitted_frames
    }

    fn decode(&mut self) -> Result<Vec<Frame>, Self::Error> {
        let packet = self.next_track_packet()?;
        let buffer = self.decoder.decode(&packet)?;
        let remaining = self
            .admitted_frames
            .checked_sub(self.emitted_frames)
            .ok_or_else(|| AudioDecodeError::Limit("streamed audio frame overflow".into()))?;
        let frames = frames_from_buffer(&buffer, remaining)?;
        self.emitted_frames = self
            .emitted_frames
            .checked_add(frames.len())
            .ok_or_else(|| AudioDecodeError::Limit("streamed audio frame overflow".into()))?;
        Ok(frames)
    }

    fn seek(&mut self, index: usize) -> Result<usize, Self::Error> {
        if index > self.admitted_frames {
            return Err(AudioDecodeError::Limit(format!(
                "audio seek frame {index} exceeds admitted duration {}",
                self.admitted_frames
            )));
        }
        let seeked = self.format_reader.seek(
            SeekMode::Accurate,
            SeekTo::TimeStamp {
                ts: u64::try_from(index)
                    .map_err(|_| AudioDecodeError::Limit("audio seek overflow".into()))?,
                track_id: self.track_id,
            },
        )?;
        let actual = usize::try_from(seeked.actual_ts)
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
    let duration_frames = u64::from(sample_rate)
        .checked_mul(MAX_AUDIO_DURATION_SECONDS)
        .ok_or_else(|| AudioDecodeError::Limit("audio duration limit overflow".into()))?;
    if frames > duration_frames {
        return Err(AudioDecodeError::Limit(format!(
            "audio declares {frames} frames, exceeding the {MAX_AUDIO_DURATION_SECONDS} second duration limit at {sample_rate} Hz"
        )));
    }
    if static_sound {
        let decoded_bytes = frames
            .checked_mul(u64::try_from(size_of::<Frame>()).expect("frame size fits u64"))
            .ok_or_else(|| AudioDecodeError::Limit("static decoded audio byte overflow".into()))?;
        if decoded_bytes > MAX_STATIC_SOUND_DECODED_BYTES {
            return Err(AudioDecodeError::Limit(format!(
                "static audio declares {decoded_bytes} decoded bytes, exceeding the {MAX_STATIC_SOUND_DECODED_BYTES} byte per-sound limit"
            )));
        }
    }
    Ok(frames)
}

fn frames_from_buffer(
    buffer: &AudioBufferRef<'_>,
    remaining: usize,
) -> Result<Vec<Frame>, AudioDecodeError> {
    match buffer {
        AudioBufferRef::U8(buffer) => frames_from_typed_buffer(buffer, remaining),
        AudioBufferRef::U16(buffer) => frames_from_typed_buffer(buffer, remaining),
        AudioBufferRef::U24(buffer) => frames_from_typed_buffer(buffer, remaining),
        AudioBufferRef::U32(buffer) => frames_from_typed_buffer(buffer, remaining),
        AudioBufferRef::S8(buffer) => frames_from_typed_buffer(buffer, remaining),
        AudioBufferRef::S16(buffer) => frames_from_typed_buffer(buffer, remaining),
        AudioBufferRef::S24(buffer) => frames_from_typed_buffer(buffer, remaining),
        AudioBufferRef::S32(buffer) => frames_from_typed_buffer(buffer, remaining),
        AudioBufferRef::F32(buffer) => frames_from_typed_buffer(buffer, remaining),
        AudioBufferRef::F64(buffer) => frames_from_typed_buffer(buffer, remaining),
    }
}

fn frames_from_typed_buffer<S: Sample>(
    buffer: &AudioBuffer<S>,
    remaining: usize,
) -> Result<Vec<Frame>, AudioDecodeError>
where
    f32: FromSample<S>,
{
    admit_decoded_chunk(buffer.frames(), remaining)?;
    match buffer.spec().channels.count() {
        1 => Ok(buffer
            .chan(0)
            .iter()
            .map(|sample| Frame::from_mono((*sample).into_sample()))
            .collect()),
        2 => Ok(buffer
            .chan(0)
            .iter()
            .zip(buffer.chan(1).iter())
            .map(|(left, right)| Frame::new((*left).into_sample(), (*right).into_sample()))
            .collect()),
        channels => Err(AudioDecodeError::Limit(format!(
            "decoded audio channel count {channels} is outside the mono/stereo limit"
        ))),
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

fn frame_bytes(frames: usize) -> Result<u64, AudioDecodeError> {
    u64::try_from(frames)
        .ok()
        .and_then(|frames| {
            frames.checked_mul(u64::try_from(size_of::<Frame>()).expect("frame size fits u64"))
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
        admit_audio_metadata(
            MAX_AUDIO_SAMPLE_RATE,
            2,
            Some(MAX_STATIC_SOUND_DECODED_BYTES / size_of::<Frame>() as u64),
            true,
        )
        .unwrap();
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
            Some(MAX_STATIC_SOUND_DECODED_BYTES / size_of::<Frame>() as u64 + 1),
            true
        )
        .is_err());
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

        assert!(residency.static_sound(&sound).is_err());
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

        assert!(residency.static_sound(&sound).is_err());
        assert_eq!(residency.observation().static_buffers, 0);
        assert_eq!(residency.observation().static_decoded_bytes, 0);

        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn production_runtime_has_no_unbounded_kira_file_decode_bypass() {
        let runtime = include_str!("runtime.rs");

        assert!(!runtime.contains("StaticSoundData::from_file"));
        assert!(!runtime.contains("StreamingSoundData::from_file"));
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

        let error = residency.static_sound(&sound).unwrap_err().to_string();
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
        let expected_frames = Arc::clone(&data.frames);
        let mut residency = AudioResidency::new(9);
        residency.static_cache.insert(
            AudioResourceKey {
                generation: 9,
                path: path.clone(),
            },
            StaticCacheEntry {
                data,
                decoded_bytes: frame_bytes(4).unwrap(),
                last_used: 0,
            },
        );
        residency.observation.static_buffers = 1;
        residency.observation.static_decoded_bytes = frame_bytes(4).unwrap();
        let sound = test_sound(path, 9);

        let first = residency.static_sound(&sound).unwrap();
        let second = residency.static_sound(&sound).unwrap();

        assert!(Arc::ptr_eq(&first.frames, &expected_frames));
        assert!(Arc::ptr_eq(&second.frames, &expected_frames));
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
        let active_pin = Arc::clone(&active.frames);
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
        let reset_pin = Arc::clone(&reset_data.frames);
        residency.static_cache.insert(
            AudioResourceKey {
                generation: 3,
                path: PathBuf::from("reset-pinned.ogg"),
            },
            StaticCacheEntry {
                data: reset_data,
                decoded_bytes: frame_bytes(1).unwrap(),
                last_used: 3,
            },
        );
        residency.observation.static_buffers = 1;
        residency.observation.static_decoded_bytes = frame_bytes(1).unwrap();
        residency.reset_generation(4);
        assert_eq!(residency.observation().static_buffers, 0);
        assert_eq!(residency.observation().static_decoded_bytes, 0);
        assert_eq!(Arc::strong_count(&reset_pin), 1);
    }

    fn test_static_sound(frames: usize) -> StaticSoundData {
        StaticSoundData {
            sample_rate: 48_000,
            frames: vec![Frame::ZERO; frames].into(),
            settings: StaticSoundSettings::default(),
            slice: None,
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
