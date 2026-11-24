use std::{
    os::windows::ffi::OsStrExt,
    path::Path,
    sync::mpsc::{Receiver, RecvTimeoutError, SyncSender},
    time::Duration,
};

use anyhow::{bail, Context, Result};
use bbb_client_audio::{RECORDING_CHANNELS, RECORDING_SAMPLE_RATE};
use bbb_client_render::ScreenshotPixels;
use windows::{
    core::PCWSTR,
    Win32::{
        Media::MediaFoundation::*,
        System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
    },
};

use super::EncoderMessage;

struct MediaRuntime;

impl MediaRuntime {
    fn new() -> Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
            if let Err(error) = MFStartup(MF_VERSION, MFSTARTUP_FULL) {
                CoUninitialize();
                return Err(error.into());
            }
        }
        Ok(Self)
    }
}

impl Drop for MediaRuntime {
    fn drop(&mut self) {
        unsafe {
            if let Err(error) = MFShutdown() {
                tracing::error!(%error, "Media Foundation shutdown failed");
            }
            CoUninitialize();
        }
    }
}

pub(super) struct VideoEncodeSpec {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) fps: u32,
    pub(super) bitrate: u32,
}

pub(super) fn encode(
    path: &Path,
    spec: VideoEncodeSpec,
    receiver: Receiver<EncoderMessage>,
    audio: Receiver<Vec<i16>>,
    ready: SyncSender<Result<(), String>>,
) -> Result<u64> {
    let VideoEncodeSpec {
        width,
        height,
        fps,
        bitrate,
    } = spec;
    let initialized = (|| {
        let runtime = MediaRuntime::new()?;
        let (writer, video, audio) = unsafe { create_writer(path, width, height, fps, bitrate)? };
        Ok::<_, anyhow::Error>((runtime, writer, video, audio))
    })();
    let (_runtime, writer, video_stream, audio_stream) = match initialized {
        Ok(writer) => {
            let _ = ready.send(Ok(()));
            writer
        }
        Err(error) => {
            let _ = ready.send(Err(format!("initialize MP4 encoder: {error:#}")));
            return Err(error);
        }
    };
    let mut video = VideoStream {
        stream: video_stream,
        fps,
        pending: None,
        next_index: 0,
    };
    let mut audio_frames = 0;
    loop {
        let message = match receiver.recv_timeout(Duration::from_millis(5)) {
            Ok(message) => Some(message),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let mut finish = None;
        let messages: Vec<_> = message.into_iter().chain(receiver.try_iter()).collect();
        for message in messages {
            match message {
                EncoderMessage::Frame { pixels, time } => unsafe {
                    let index = (time as u64 * u64::from(fps) + 5_000_000) / 10_000_000;
                    video.advance(&writer, index)?;
                    video.pending = Some(prepare_frame(pixels, width, height)?);
                },
                EncoderMessage::Finish { time } => {
                    finish = Some(time);
                    break;
                }
            }
        }
        let chunks: Vec<_> = audio.try_iter().collect();
        for samples in chunks {
            unsafe {
                audio_frames = write_audio(&writer, audio_stream, &samples, audio_frames)?;
                video.advance(
                    &writer,
                    audio_frames * u64::from(fps) / u64::from(RECORDING_SAMPLE_RATE),
                )?;
            }
        }
        if let Some(time) = finish {
            let end = (time as u64 * u64::from(fps)).div_ceil(10_000_000).max(1);
            unsafe {
                video.advance(&writer, end)?;
            }
            break;
        }
    }
    unsafe {
        writer.Finalize().context("finalize MP4 file")?;
    }
    if video.next_index == 0 {
        bail!("recording stopped before any frame was captured");
    }
    Ok(video.next_index)
}

struct VideoStream {
    stream: u32,
    fps: u32,
    pending: Option<IMFMediaBuffer>,
    next_index: u64,
}

impl VideoStream {
    unsafe fn advance(&mut self, writer: &IMFSinkWriter, end: u64) -> Result<()> {
        if let Some(buffer) = &self.pending {
            // Audio can advance the CFR image before a queued screenshot arrives.
            let end = end.max(self.next_index);
            write_frames(writer, self.stream, buffer, self.fps, self.next_index, end)?;
            self.next_index = end;
        }
        Ok(())
    }
}

unsafe fn create_writer(
    path: &Path,
    width: u32,
    height: u32,
    fps: u32,
    bitrate: u32,
) -> Result<(IMFSinkWriter, u32, u32)> {
    let path: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let writer = MFCreateSinkWriterFromURL(PCWSTR(path.as_ptr()), None, None)?;
    let output = MFCreateMediaType()?;
    output.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
    output.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
    output.SetUINT32(&MF_MT_AVG_BITRATE, bitrate)?;
    output.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
    set_geometry(&output, width, height, fps)?;
    let stream = writer.AddStream(&output)?;
    let input = MFCreateMediaType()?;
    input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
    input.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
    input.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
    set_geometry(&input, width, height, fps)?;
    input.SetUINT32(&MF_MT_DEFAULT_STRIDE, width * 4)?;
    writer.SetInputMediaType(stream, &input, None)?;
    let output = MFCreateMediaType()?;
    set_audio_format(&output, &MFAudioFormat_AAC)?;
    output.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 24_000)?;
    output.SetUINT32(&MF_MT_AAC_PAYLOAD_TYPE, 0)?;
    let audio_stream = writer.AddStream(&output)?;
    let input = MFCreateMediaType()?;
    set_audio_format(&input, &MFAudioFormat_PCM)?;
    input.SetUINT32(
        &MF_MT_AUDIO_BLOCK_ALIGNMENT,
        u32::from(RECORDING_CHANNELS) * 2,
    )?;
    input.SetUINT32(
        &MF_MT_AUDIO_AVG_BYTES_PER_SECOND,
        RECORDING_SAMPLE_RATE * u32::from(RECORDING_CHANNELS) * 2,
    )?;
    input.SetUINT32(&MF_MT_ALL_SAMPLES_INDEPENDENT, 1)?;
    writer.SetInputMediaType(audio_stream, &input, None)?;
    writer.BeginWriting()?;
    Ok((writer, stream, audio_stream))
}

unsafe fn set_audio_format(media: &IMFMediaType, subtype: &windows::core::GUID) -> Result<()> {
    media.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio)?;
    media.SetGUID(&MF_MT_SUBTYPE, subtype)?;
    media.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16)?;
    media.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, RECORDING_SAMPLE_RATE)?;
    media.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, u32::from(RECORDING_CHANNELS))?;
    Ok(())
}

unsafe fn write_audio(
    writer: &IMFSinkWriter,
    stream: u32,
    samples: &[i16],
    start: u64,
) -> Result<u64> {
    let end = start + samples.len() as u64 / u64::from(RECORDING_CHANNELS);
    let length = std::mem::size_of_val(samples);
    let buffer = MFCreateMemoryBuffer(length as u32)?;
    let mut destination = std::ptr::null_mut();
    buffer.Lock(&mut destination, None, None)?;
    std::ptr::copy_nonoverlapping(samples.as_ptr().cast::<u8>(), destination, length);
    buffer.Unlock()?;
    buffer.SetCurrentLength(length as u32)?;
    let sample = MFCreateSample()?;
    sample.AddBuffer(&buffer)?;
    let time = start * 10_000_000 / u64::from(RECORDING_SAMPLE_RATE);
    let next = end * 10_000_000 / u64::from(RECORDING_SAMPLE_RATE);
    sample.SetSampleTime(time as i64)?;
    sample.SetSampleDuration((next - time) as i64)?;
    writer
        .WriteSample(stream, &sample)
        .context("encode game audio")?;
    Ok(end)
}

unsafe fn set_geometry(media: &IMFMediaType, width: u32, height: u32, fps: u32) -> Result<()> {
    media.SetUINT64(
        &MF_MT_FRAME_SIZE,
        (u64::from(width) << 32) | u64::from(height),
    )?;
    media.SetUINT64(&MF_MT_FRAME_RATE, (u64::from(fps) << 32) | 1)?;
    media.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, (1_u64 << 32) | 1)?;
    Ok(())
}

unsafe fn prepare_frame(
    pixels: ScreenshotPixels,
    width: u32,
    height: u32,
) -> Result<IMFMediaBuffer> {
    let rgba = if pixels.width == width && pixels.height == height {
        pixels.rgba
    } else {
        let source = image::RgbaImage::from_raw(pixels.width, pixels.height, pixels.rgba).unwrap();
        let scale =
            (width as f64 / source.width() as f64).min(height as f64 / source.height() as f64);
        let resized = image::imageops::resize(
            &source,
            (source.width() as f64 * scale).round().max(1.0) as u32,
            (source.height() as f64 * scale).round().max(1.0) as u32,
            image::imageops::FilterType::Triangle,
        );
        let mut target = image::RgbaImage::from_pixel(width, height, image::Rgba([0, 0, 0, 255]));
        image::imageops::replace(
            &mut target,
            &resized,
            i64::from((width - resized.width()) / 2),
            i64::from((height - resized.height()) / 2),
        );
        target.into_raw()
    };
    let byte_len = rgba.len();
    let buffer = MFCreateMemoryBuffer(byte_len as u32)?;
    let mut destination = std::ptr::null_mut();
    buffer.Lock(&mut destination, None, None)?;
    // Fuse RGBA -> BGRA swizzle directly into the Media Foundation buffer in one pass.
    let dst = std::slice::from_raw_parts_mut(destination, byte_len);
    for (src_px, dst_px) in rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(dst.as_chunks_mut::<4>().0)
    {
        *dst_px = [src_px[2], src_px[1], src_px[0], src_px[3]];
    }
    buffer.Unlock()?;
    buffer.SetCurrentLength(byte_len as u32)?;
    Ok(buffer)
}

unsafe fn write_frames(
    writer: &IMFSinkWriter,
    stream: u32,
    buffer: &IMFMediaBuffer,
    fps: u32,
    start: u64,
    end: u64,
) -> Result<()> {
    // The H.264 transform emits CFR samples, so gaps repeat the last image.
    for index in start..end {
        let time = index * 10_000_000 / u64::from(fps);
        let next = (index + 1) * 10_000_000 / u64::from(fps);
        let sample = MFCreateSample()?;
        sample.AddBuffer(buffer)?;
        sample.SetSampleTime(time as i64)?;
        sample.SetSampleDuration((next - time) as i64)?;
        writer
            .WriteSample(stream, &sample)
            .context("encode video frame")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_mp4_decodes_with_correct_colors_orientation_timing_and_resize() -> Result<()> {
        let path = std::env::temp_dir().join(format!(
            "bbb-native-video-{}-{}.mp4",
            std::process::id(),
            bbb_client_clock::SystemTime::now()
                .duration_since(bbb_client_clock::UNIX_EPOCH)?
                .as_nanos()
        ));
        let (sender, receiver) = std::sync::mpsc::sync_channel(4);
        let (audio_sender, audio) = std::sync::mpsc::sync_channel(128);
        for (width, height, time) in [(128, 64, 500_000), (64, 64, 1_000_000), (64, 64, 4_000_000)]
        {
            let image = image::RgbaImage::from_fn(width, height, |_, y| {
                if y < height / 2 {
                    if time < 4_000_000 {
                        image::Rgba([255, 0, 0, 255])
                    } else {
                        image::Rgba([0, 255, 0, 255])
                    }
                } else {
                    image::Rgba([0, 0, 255, 255])
                }
            });
            sender.send(EncoderMessage::Frame {
                pixels: ScreenshotPixels {
                    rgba: image.into_raw(),
                    width,
                    height,
                },
                time,
            })?;
        }
        for chunk in 0..100 {
            let mut samples = Vec::with_capacity(960);
            for frame in chunk * 480..(chunk + 1) * 480 {
                let time = f64::from(frame) / f64::from(RECORDING_SAMPLE_RATE);
                let tone = |frequency: f64| {
                    (12_000.0 * (time * std::f64::consts::TAU * frequency).sin()) as i16
                };
                samples.extend([
                    if (0.4..0.6).contains(&time) {
                        tone(440.0)
                    } else {
                        0
                    },
                    if (0.75..0.9).contains(&time) {
                        tone(880.0)
                    } else {
                        0
                    },
                ]);
            }
            audio_sender.send(samples)?;
        }
        drop(audio_sender);
        sender.send(EncoderMessage::Finish { time: 10_000_000 })?;
        drop(sender);
        let (ready, initialized) = std::sync::mpsc::sync_channel(1);
        let output_path = path.clone();
        let thread = std::thread::spawn(move || {
            encode(
                &output_path,
                VideoEncodeSpec {
                    width: 128,
                    height: 64,
                    fps: 30,
                    bitrate: 1_000_000,
                },
                receiver,
                audio,
                ready,
            )
        });
        initialized.recv()?.map_err(anyhow::Error::msg)?;
        assert_eq!(super::super::join_encoder(thread)?, 30);

        let _runtime = MediaRuntime::new()?;
        unsafe {
            let url: Vec<_> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let mut attributes = None;
            MFCreateAttributes(&mut attributes, 1)?;
            let attributes = attributes.unwrap();
            attributes.SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1)?;
            let reader = MFCreateSourceReaderFromURL(PCWSTR(url.as_ptr()), &attributes)?;
            reader.SetStreamSelection(MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32, false)?;
            let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
            let native = reader.GetNativeMediaType(stream, 0)?;
            assert_eq!(native.GetGUID(&MF_MT_SUBTYPE)?, MFVideoFormat_H264);
            assert_eq!(native.GetUINT64(&MF_MT_FRAME_SIZE)?, (128_u64 << 32) | 64);
            let output = MFCreateMediaType()?;
            output.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
            output.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
            reader.SetCurrentMediaType(stream, None, &output)?;
            let media = reader.GetCurrentMediaType(stream)?;
            let stride = media.GetUINT32(&MF_MT_DEFAULT_STRIDE)? as i32;
            let mut count = 0;
            let mut end = 0;
            loop {
                let mut sample = None;
                let mut flags = 0;
                reader.ReadSample(stream, 0, None, Some(&mut flags), None, Some(&mut sample))?;
                if let Some(sample) = sample {
                    assert!((sample.GetSampleTime()? - count * 10_000_000 / 30).abs() <= 1);
                    end = sample.GetSampleTime()? + sample.GetSampleDuration()?;
                    let buffer = sample.ConvertToContiguousBuffer()?;
                    let mut bytes = std::ptr::null_mut();
                    let mut length = 0;
                    buffer.Lock(&mut bytes, None, Some(&mut length))?;
                    let data = std::slice::from_raw_parts(bytes, length as usize);
                    let pixel = |x: usize, y: usize| {
                        let row = if stride > 0 { y } else { 63 - y };
                        let offset = row * stride.unsigned_abs() as usize + x * 4;
                        [data[offset + 2], data[offset + 1], data[offset]]
                    };
                    let top = pixel(64, 8);
                    let bottom = pixel(64, 56);
                    if count < 12 {
                        assert!(top[0] > 180 && top[1] < 70, "top must be red: {top:?}");
                    } else {
                        assert!(top[1] > 180 && top[0] < 70, "top must be green: {top:?}");
                    }
                    assert!(
                        bottom[2] > 180 && bottom[0] < 70,
                        "bottom must be blue: {bottom:?}"
                    );
                    if count >= 3 {
                        assert!(
                            pixel(4, 32).iter().all(|value| *value < 30),
                            "resized frames must be letterboxed"
                        );
                    }
                    buffer.Unlock()?;
                    count += 1;
                }
                if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                    break;
                }
            }
            assert_eq!(count, 30);
            assert!((end - 10_000_000).abs() <= 1);

            let reader = MFCreateSourceReaderFromURL(PCWSTR(url.as_ptr()), None)?;
            reader.SetStreamSelection(MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32, false)?;
            let stream = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;
            let native = reader.GetNativeMediaType(stream, 0)?;
            assert_eq!(native.GetGUID(&MF_MT_SUBTYPE)?, MFAudioFormat_AAC);
            assert_eq!(native.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND)?, 48_000);
            assert_eq!(native.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS)?, 2);
            let output = MFCreateMediaType()?;
            set_audio_format(&output, &MFAudioFormat_PCM)?;
            reader.SetCurrentMediaType(stream, None, &output)?;
            let mut decoded = Vec::new();
            let mut audio_end = 0;
            loop {
                let mut sample = None;
                let mut flags = 0;
                reader.ReadSample(stream, 0, None, Some(&mut flags), None, Some(&mut sample))?;
                if let Some(sample) = sample {
                    let expected = decoded.len() as i64 * 10_000_000 / 48_000;
                    assert!((sample.GetSampleTime()? - expected).abs() <= 1);
                    audio_end = sample.GetSampleTime()? + sample.GetSampleDuration()?;
                    let buffer = sample.ConvertToContiguousBuffer()?;
                    let mut bytes = std::ptr::null_mut();
                    let mut length = 0;
                    buffer.Lock(&mut bytes, None, Some(&mut length))?;
                    let data = std::slice::from_raw_parts(bytes, length as usize);
                    decoded.extend(data.as_chunks::<4>().0.iter().map(|frame| {
                        [
                            i16::from_le_bytes([frame[0], frame[1]]),
                            i16::from_le_bytes([frame[2], frame[3]]),
                        ]
                    }));
                    buffer.Unlock()?;
                }
                if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                    break;
                }
            }
            assert!((10_000_000..=10_000_000 + 1024 * 10_000_000 / 48_000).contains(&audio_end));
            let energy = |channel: usize, start: usize, end: usize| {
                decoded[start..end]
                    .iter()
                    .map(|frame| f64::from(frame[channel]).powi(2))
                    .sum::<f64>()
                    / (end - start) as f64
            };
            for channel in 0..2 {
                assert!(energy(channel, 0, 14_400) < 100.0, "initial silence");
                assert!(energy(channel, 31_200, 33_600) < 100.0, "middle silence");
                assert!(energy(channel, 45_600, 48_000) < 100.0, "final silence");
            }
            assert!(energy(0, 22_080, 25_920) > 10_000_000.0, "left tone");
            assert!(
                energy(1, 22_080, 25_920) < 100.0,
                "right channel silent during left tone"
            );
            assert!(energy(1, 38_400, 41_280) > 10_000_000.0, "right tone");
            assert!(
                energy(0, 38_400, 41_280) < 100.0,
                "left channel silent during right tone"
            );
            let onset = decoded
                .iter()
                .position(|frame| frame[0].unsigned_abs() > 1_000)
                .unwrap();
            assert!(
                onset.abs_diff(19_200) <= 1024,
                "audio onset must match the green frame"
            );
        }
        std::fs::remove_file(path)?;
        Ok(())
    }
}
