//! Recording owns the audio/video timeline and bounded native encoder handoffs.

use bbb_client_clock::{Elapsed as _, Instant};
#[cfg(any(not(target_arch = "wasm32"), test))]
use std::sync::mpsc::TrySendError;
use std::{path::Path, sync::mpsc::SyncSender, thread::JoinHandle};

use anyhow::{bail, Context, Result};
use bbb_client_audio::RECORDING_SAMPLE_RATE;
use bbb_client_render::ScreenshotPixels;

use crate::audio_runtime::NativeAudioRuntime;
use crate::control::{RecordingControlRequest, RecordingState, RecordingStatus};

#[cfg(target_os = "windows")]
mod windows;

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
enum EncoderMessage {
    Frame { pixels: ScreenshotPixels, time: i64 },
    Finish { time: i64 },
}

struct Encoder {
    sender: SyncSender<EncoderMessage>,
    thread: JoinHandle<Result<u64>>,
}

struct ActiveRecording {
    encoder: Encoder,
    started: Instant,
    #[cfg(any(not(target_arch = "wasm32"), test))]
    next_frame: u64,
}

#[derive(Default)]
pub(crate) struct RecordingRuntime {
    active: Option<ActiveRecording>,
    status: RecordingStatus,
}

impl RecordingRuntime {
    pub(crate) fn status(&self) -> RecordingStatus {
        let mut status = self.status.clone();
        if let Some(active) = &self.active {
            status.duration_seconds = active.started.clock_elapsed().as_secs_f64();
        }
        status
    }

    pub(crate) fn apply(
        &mut self,
        request: RecordingControlRequest,
        size: [u32; 2],
        audio: Option<&mut NativeAudioRuntime>,
    ) -> Result<RecordingStatus> {
        let result = match request {
            RecordingControlRequest::Start { path, fps, bitrate } => {
                self.start(Path::new(&path), size, fps, bitrate, audio)
            }
            RecordingControlRequest::Stop => self.stop(audio),
        };
        if let Err(error) = &result {
            self.status.last_error = Some(format!("{error:#}"));
            if self.active.is_none() {
                self.status.state = RecordingState::Failed;
            }
        }
        result.map(|()| self.status())
    }

    fn start(
        &mut self,
        path: &Path,
        size: [u32; 2],
        fps: u32,
        bitrate: u32,
        audio: Option<&mut NativeAudioRuntime>,
    ) -> Result<()> {
        if self.active.is_some() {
            bail!("a video recording is already active");
        }
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mp4"))
        {
            bail!("native video output must have an .mp4 extension");
        }
        let path = std::path::absolute(path)?;
        if path.exists() {
            bail!("video output already exists: {}", path.display());
        }
        let [width, height] = size.map(|dimension| dimension & !1);
        if width < 2 || height < 2 {
            bail!("video recording needs a drawable of at least 2x2 pixels");
        }
        let audio = audio.context("video recording requires initialized game audio")?;
        let parent = path
            .parent()
            .context("video output path has no parent directory")?;
        std::fs::create_dir_all(parent)?;
        let (encoder, samples) = spawn_encoder(&path, width, height, fps, bitrate)?;
        let started = match audio.begin_recording(samples) {
            Ok(started) => started,
            Err(error) => {
                drop(encoder.sender);
                let _ = join_encoder(encoder.thread);
                return Err(error.context("start game audio recording"));
            }
        };
        self.status = RecordingStatus {
            state: RecordingState::Recording,
            path: Some(path.to_string_lossy().into_owned()),
            width,
            height,
            fps,
            ..RecordingStatus::default()
        };
        self.active = Some(ActiveRecording {
            encoder,
            started,
            #[cfg(any(not(target_arch = "wasm32"), test))]
            next_frame: 0,
        });
        tracing::info!(path = %path.display(), width, height, fps, "video recording started");
        Ok(())
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) fn is_active(&self) -> bool {
        self.active.is_some()
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) fn poll(&mut self, mut audio: Option<&mut NativeAudioRuntime>) {
        let Some(active) = &self.active else {
            return;
        };
        if active.encoder.thread.is_finished() {
            let result = self.stop(audio);
            self.status.state = RecordingState::Failed;
            self.status.last_error = Some(match result {
                Ok(()) => "video encoder stopped before recording was finished".into(),
                Err(error) => format!("{error:#}"),
            });
            tracing::error!(error = ?self.status.last_error, "video recording failed");
            return;
        }
        let result = audio
            .as_deref_mut()
            .context("game audio is unavailable while recording")
            .and_then(NativeAudioRuntime::poll_recording);
        if let Err(error) = result {
            let _ = self.stop(audio);
            self.status.state = RecordingState::Failed;
            self.status.last_error = Some(format!("record game audio: {error:#}"));
            tracing::error!(error = ?self.status.last_error, "video recording failed");
        }
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) fn capture_due(&mut self) -> bool {
        let Some(active) = &mut self.active else {
            return false;
        };
        let index = (active.started.clock_elapsed().as_nanos() * u128::from(self.status.fps)
            / 1_000_000_000) as u64;
        if index < active.next_frame {
            return false;
        }
        active.next_frame = index + 1;
        true
    }

    #[cfg(any(not(target_arch = "wasm32"), test))]
    pub(crate) fn submit(
        &mut self,
        pixels: Result<ScreenshotPixels>,
        audio: Option<&mut NativeAudioRuntime>,
    ) {
        match pixels {
            Ok(pixels) => {
                let Some(active) = self.active.as_ref() else {
                    return;
                };
                let time = (active.started.clock_elapsed().as_nanos() / 100) as i64;
                match active
                    .encoder
                    .sender
                    .try_send(EncoderMessage::Frame { pixels, time })
                {
                    Ok(()) => self.status.frames += 1,
                    Err(TrySendError::Full(_)) => self.status.dropped_frames += 1,
                    // The next event-loop poll joins and reports encoder failure.
                    Err(TrySendError::Disconnected(_)) => {}
                }
            }
            Err(error) => {
                let _ = self.stop(audio);
                self.status.state = RecordingState::Failed;
                self.status.last_error = Some(format!("read recording frame: {error:#}"));
            }
        }
    }

    pub(crate) fn stop(&mut self, audio: Option<&mut NativeAudioRuntime>) -> Result<()> {
        let active = self.active.take().context("no video recording is active")?;
        let elapsed = (active.started.clock_elapsed().as_nanos() / 100) as i64;
        let audio_end = match audio {
            Some(audio) => audio
                .end_recording()
                .map(|frames| (frames * 10_000_000 / u64::from(RECORDING_SAMPLE_RATE)) as i64),
            None => Ok(elapsed),
        };
        let time = match &audio_end {
            Ok(time) => *time,
            Err(_) => elapsed,
        };
        self.status.duration_seconds = time as f64 / 10_000_000.0;
        let _ = active.encoder.sender.send(EncoderMessage::Finish { time });
        drop(active.encoder.sender);
        let encoded = join_encoder(active.encoder.thread);
        audio_end?;
        self.status.encoded_frames = encoded?;
        self.status.state = RecordingState::Saved;
        tracing::info!(path = ?self.status.path, frames = self.status.frames, "video recording saved");
        Ok(())
    }
}

impl Drop for RecordingRuntime {
    fn drop(&mut self) {
        if self.active.is_some() {
            if let Err(error) = self.stop(None) {
                tracing::error!(%error, "could not finalize video recording at shutdown");
            }
        }
    }
}

fn join_encoder(thread: JoinHandle<Result<u64>>) -> Result<u64> {
    thread
        .join()
        .map_err(|_| anyhow::anyhow!("video encoder thread panicked"))?
}

#[cfg(target_os = "windows")]
fn spawn_encoder(
    path: &Path,
    width: u32,
    height: u32,
    fps: u32,
    bitrate: u32,
) -> Result<(Encoder, SyncSender<Vec<i16>>)> {
    let path = path.to_owned();
    let (sender, receiver) = std::sync::mpsc::sync_channel(2);
    let (audio_sender, audio_receiver) = std::sync::mpsc::sync_channel(128);
    let (ready, initialized) = std::sync::mpsc::sync_channel(1);
    let thread = std::thread::Builder::new()
        .name("bbb-video".into())
        .spawn(move || {
            windows::encode(
                &path,
                windows::VideoEncodeSpec {
                    width,
                    height,
                    fps,
                    bitrate,
                },
                receiver,
                audio_receiver,
                ready,
            )
        })?;
    match initialized
        .recv()
        .context("video encoder exited during startup")?
    {
        Ok(()) => Ok((Encoder { sender, thread }, audio_sender)),
        Err(error) => {
            drop(sender);
            let _ = thread.join();
            bail!("{error}");
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn spawn_encoder(
    _path: &Path,
    _width: u32,
    _height: u32,
    _fps: u32,
    _bitrate: u32,
) -> Result<(Encoder, SyncSender<Vec<i16>>)> {
    bail!("native MP4 recording currently requires Windows");
}
