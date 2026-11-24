//! Bounded, slice-resumable Symphonia planar Ogg decoder shared by the browser
//! Web Audio software decode path and native unit tests.

use std::io::{self, Cursor, Read, Seek, SeekFrom};

use anyhow::{anyhow, Result};
#[cfg(test)]
use sha1::{Digest, Sha1};
use symphonia::core::{
    audio::{
        conv::FromSample, sample::Sample, Audio, AudioBuffer as SymphoniaAudioBuffer,
        GenericAudioBufferRef,
    },
    codecs::audio::AudioDecoder,
    formats::{probe::Hint, FormatReader, TrackType},
    io::{MediaSource, MediaSourceStream},
};

pub(crate) const MAX_AUDIO_COMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
pub(crate) const MAX_AUDIO_SAMPLE_RATE: u32 = 192_000;
pub(crate) const MAX_AUDIO_DURATION_SECONDS: u64 = 30 * 60;
pub(crate) const MAX_AUDIO_DECODE_PACKET_FRAMES: usize = 65_536;
pub(crate) const MAX_STATIC_SOUND_DECODED_BYTES: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_STREAM_SOUND_DECODED_BYTES: u64 = 256 * 1024 * 1024;

/// Maximum PCM frames decoded per cooperative browser main-thread slice before
/// yielding back to the event loop (~0.34 s at 48 kHz, ~1-2 ms of WASM CPU).
pub(crate) const SOFTWARE_DECODE_SLICE_FRAMES: usize = 16_384;

/// Upper bound on total container packets inspected per track so malformed
/// zero-frame or foreign-track packet streams cannot spin indefinitely.
pub(crate) const MAX_AUDIO_DECODE_PACKETS: usize = 262_144;

pub(crate) struct DecodedPlanarAudio {
    pub(crate) sample_rate: u32,
    pub(crate) channels: u16,
    pub(crate) frames: usize,
    pub(crate) left: Vec<f32>,
    pub(crate) right: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DecodeSliceStatus {
    More,
    Done,
}

struct MemoryMediaSource {
    cursor: Cursor<Vec<u8>>,
    len: u64,
}

impl Read for MemoryMediaSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.cursor.read(buf)
    }
}

impl Seek for MemoryMediaSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.cursor.seek(pos)
    }
}

impl MediaSource for MemoryMediaSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        Some(self.len)
    }
}

pub(crate) struct IncrementalOggDecoder {
    format_reader: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
    sample_rate: u32,
    channel_count: usize,
    admitted_frames: usize,
    emitted_frames: usize,
    packets_read: usize,
    left: Vec<f32>,
    right: Option<Vec<f32>>,
    finished: bool,
}

impl IncrementalOggDecoder {
    pub(crate) fn new(bytes: Vec<u8>, allow_streaming: bool) -> Result<Self> {
        let len = bytes.len() as u64;
        if len > MAX_AUDIO_COMPRESSED_BYTES {
            return Err(anyhow!(
                "compressed audio size {len} exceeds {MAX_AUDIO_COMPRESSED_BYTES}"
            ));
        }
        let source = MemoryMediaSource {
            cursor: Cursor::new(bytes),
            len,
        };
        let media_source = MediaSourceStream::new(Box::new(source), Default::default());
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
            .ok_or_else(|| anyhow!("audio has no default track"))?;
        let codec_params = track
            .codec_params
            .as_ref()
            .and_then(|p| p.audio())
            .ok_or_else(|| anyhow!("audio track has no codec parameters"))?;
        let sample_rate = codec_params
            .sample_rate
            .ok_or_else(|| anyhow!("audio sample rate is unknown"))?;
        if sample_rate == 0 || sample_rate > MAX_AUDIO_SAMPLE_RATE {
            return Err(anyhow!("audio sample rate {sample_rate} out of bounds"));
        }
        let channel_count = codec_params
            .channels
            .as_ref()
            .ok_or_else(|| anyhow!("audio channels unknown"))?
            .count();
        if !(1..=2).contains(&channel_count) {
            return Err(anyhow!("invalid audio channel count {channel_count}"));
        }
        let admitted_frames = track
            .num_frames
            .ok_or_else(|| anyhow!("audio duration is unknown"))?;
        if admitted_frames == 0 {
            return Err(anyhow!("audio declares zero frames"));
        }
        let max_frames = u64::from(sample_rate).saturating_mul(MAX_AUDIO_DURATION_SECONDS);
        if admitted_frames > max_frames {
            return Err(anyhow!("audio frame count {admitted_frames} exceeds limit"));
        }
        let max_decoded_bytes = if allow_streaming {
            MAX_STREAM_SOUND_DECODED_BYTES
        } else {
            MAX_STATIC_SOUND_DECODED_BYTES
        };
        let decoded_bytes = admitted_frames.saturating_mul(channel_count as u64 * 4);
        if decoded_bytes > max_decoded_bytes {
            return Err(anyhow!(
                "decoded audio bytes {decoded_bytes} exceed {max_decoded_bytes}"
            ));
        }
        let admitted_frames = usize::try_from(admitted_frames)?;
        // Cap initial PCM reservation (<= 512 KiB stereo) so a tiny crafted OGG
        // with a huge granule count cannot OOM WASM linear memory up front.
        const INITIAL_RESERVE_FRAMES: usize = 65_536;
        let initial_reserve = admitted_frames.min(INITIAL_RESERVE_FRAMES);
        let track_id = track.id;
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(codec_params, &Default::default())?;

        Ok(Self {
            format_reader,
            decoder,
            track_id,
            sample_rate,
            channel_count,
            admitted_frames,
            emitted_frames: 0,
            packets_read: 0,
            left: Vec::with_capacity(initial_reserve),
            right: (channel_count == 2).then(|| Vec::with_capacity(initial_reserve)),
            finished: false,
        })
    }

    /// Decodes packets until at least `slice_frame_budget` PCM frames have been
    /// emitted (or the stream finishes), bounding both frames and total packets
    /// inspected per call so cooperative callers can yield between slices.
    pub(crate) fn decode_slice(&mut self, slice_frame_budget: usize) -> Result<DecodeSliceStatus> {
        if self.finished {
            return Ok(DecodeSliceStatus::Done);
        }
        let frame_budget = slice_frame_budget.max(1);
        let mut slice_frames = 0usize;
        let mut slice_packets = 0usize;
        const MAX_PACKETS_PER_SLICE: usize = 256;

        while slice_frames < frame_budget && slice_packets < MAX_PACKETS_PER_SLICE {
            let packet = match self.format_reader.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) => {
                    self.finished = true;
                    return Ok(DecodeSliceStatus::Done);
                }
                Err(symphonia::core::errors::Error::IoError(err))
                    if err.kind() == io::ErrorKind::UnexpectedEof =>
                {
                    self.finished = true;
                    return Ok(DecodeSliceStatus::Done);
                }
                Err(err) => return Err(err.into()),
            };
            self.packets_read = self.packets_read.saturating_add(1);
            slice_packets += 1;
            if self.packets_read > MAX_AUDIO_DECODE_PACKETS {
                return Err(anyhow!(
                    "decoded audio packet count exceeds limit {MAX_AUDIO_DECODE_PACKETS}"
                ));
            }
            if packet.track_id != self.track_id {
                continue;
            }
            let buffer = self.decoder.decode(&packet)?;
            let chunk_frames = buffer.frames();
            if chunk_frames > MAX_AUDIO_DECODE_PACKET_FRAMES
                || chunk_frames > self.admitted_frames.saturating_sub(self.emitted_frames)
            {
                return Err(anyhow!("decoded audio chunk exceeds admitted frame budget"));
            }
            self.emitted_frames += chunk_frames;
            slice_frames += chunk_frames;
            append_planar_samples(&buffer, &mut self.left, self.right.as_mut())?;
        }

        Ok(DecodeSliceStatus::More)
    }

    pub(crate) fn finish(self) -> Result<DecodedPlanarAudio> {
        if self.left.is_empty() {
            return Err(anyhow!("decoded audio contains no frames"));
        }
        Ok(DecodedPlanarAudio {
            sample_rate: self.sample_rate,
            channels: self.channel_count as u16,
            frames: self.left.len(),
            left: self.left,
            right: self.right,
        })
    }
}

#[cfg(test)]
pub(crate) fn decode_ogg_planar(
    bytes: Vec<u8>,
    allow_streaming: bool,
) -> Result<DecodedPlanarAudio> {
    let mut decoder = IncrementalOggDecoder::new(bytes, allow_streaming)?;
    while decoder.decode_slice(SOFTWARE_DECODE_SLICE_FRAMES)? == DecodeSliceStatus::More {}
    decoder.finish()
}

pub(crate) fn verify_sha1_digest(digest: [u8; 20], expected_hex: &str) -> Result<(), String> {
    let mut actual = String::with_capacity(40);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut actual, "{byte:02x}");
    }
    if actual.eq_ignore_ascii_case(expected_hex) {
        Ok(())
    } else {
        Err(format!(
            "indexed sound SHA-1 mismatch: expected {expected_hex}, got {actual}"
        ))
    }
}

#[cfg(test)]
pub(crate) fn verify_sha1(bytes: &[u8], expected_hex: Option<&str>) -> Result<(), String> {
    let Some(expected) = expected_hex else {
        return Ok(());
    };
    verify_sha1_digest(Sha1::digest(bytes).into(), expected)
}

fn append_planar_samples(
    buffer: &GenericAudioBufferRef<'_>,
    left: &mut Vec<f32>,
    right: Option<&mut Vec<f32>>,
) -> Result<()> {
    match buffer {
        GenericAudioBufferRef::U8(b) => append_typed_planar(b, left, right),
        GenericAudioBufferRef::U16(b) => append_typed_planar(b, left, right),
        GenericAudioBufferRef::U24(b) => append_typed_planar(b, left, right),
        GenericAudioBufferRef::U32(b) => append_typed_planar(b, left, right),
        GenericAudioBufferRef::S8(b) => append_typed_planar(b, left, right),
        GenericAudioBufferRef::S16(b) => append_typed_planar(b, left, right),
        GenericAudioBufferRef::S24(b) => append_typed_planar(b, left, right),
        GenericAudioBufferRef::S32(b) => append_typed_planar(b, left, right),
        GenericAudioBufferRef::F32(b) => append_typed_planar(b, left, right),
        GenericAudioBufferRef::F64(b) => append_typed_planar(b, left, right),
    }
}

fn append_typed_planar<S: Sample>(
    buffer: &SymphoniaAudioBuffer<S>,
    left: &mut Vec<f32>,
    right: Option<&mut Vec<f32>>,
) -> Result<()>
where
    f32: FromSample<S>,
{
    let plane0 = buffer
        .plane(0)
        .ok_or_else(|| anyhow!("missing audio plane 0"))?;
    left.try_reserve(plane0.len())
        .map_err(|err| anyhow!("failed to reserve left PCM buffer: {err}"))?;
    left.extend(plane0.iter().map(|&s| f32::from_sample(s)));
    if let Some(right_out) = right {
        let plane1 = buffer
            .plane(1)
            .ok_or_else(|| anyhow!("missing stereo audio plane 1"))?;
        right_out
            .try_reserve(plane1.len())
            .map_err(|err| anyhow!("failed to reserve right PCM buffer: {err}"))?;
        right_out.extend(plane1.iter().map(|&s| f32::from_sample(s)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    fn checked_in_ogg_bytes() -> Vec<u8> {
        STANDARD
            .decode(include_str!("test-silence.ogg.b64").trim())
            .unwrap()
    }

    #[test]
    fn incremental_ogg_decode_yields_across_small_slices_with_identical_pcm() {
        let raw = checked_in_ogg_bytes();
        let full = decode_ogg_planar(raw.clone(), false).unwrap();

        let mut incremental = IncrementalOggDecoder::new(raw, false).unwrap();
        let mut slices = 0usize;
        loop {
            slices += 1;
            match incremental.decode_slice(16).unwrap() {
                DecodeSliceStatus::More => {}
                DecodeSliceStatus::Done => break,
            }
        }
        let sliced = incremental.finish().unwrap();
        assert!(slices >= 2, "expected multiple cooperative decode slices");
        assert_eq!(sliced.sample_rate, full.sample_rate);
        assert_eq!(sliced.channels, full.channels);
        assert_eq!(sliced.frames, full.frames);
        assert_eq!(sliced.left, full.left);
        assert_eq!(sliced.right, full.right);
    }

    #[test]
    fn software_ogg_decode_enforces_size_and_corrupt_input_bounds() {
        let oversized = vec![0u8; (MAX_AUDIO_COMPRESSED_BYTES as usize) + 1];
        assert!(IncrementalOggDecoder::new(oversized, false).is_err());
        assert!(IncrementalOggDecoder::new(b"not-an-ogg-stream".to_vec(), false).is_err());
    }

    #[test]
    fn sha1_digest_verification_accepts_matching_hex_and_rejects_mismatch() {
        let payload = checked_in_ogg_bytes();
        let digest = Sha1::digest(&payload);
        let expected: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert!(verify_sha1(&payload, Some(&expected)).is_ok());
        assert!(verify_sha1(&payload, Some(&expected.to_ascii_uppercase())).is_ok());
        assert!(verify_sha1(&payload, Some("0000000000000000000000000000000000000000")).is_err());
        assert!(verify_sha1(&payload, None).is_ok());
    }
}
