use super::*;
use crate::{scheduler::ScheduledVoice, RECORDING_CHANNELS, RECORDING_SAMPLE_RATE};
use std::sync::mpsc::SyncSender;

const RECORDING_CHUNK_FRAMES: usize = 1024;
const RECORDING_CHUNK_SAMPLES: usize = RECORDING_CHUNK_FRAMES * RECORDING_CHANNELS as usize;

pub(super) struct RecordingAudio {
    pub(super) backend: OpenAlBackend,
    sender: SyncSender<Vec<i16>>,
    epoch: Instant,
    frames: u64,
    pending: Vec<i16>,
    pub(super) failure: Option<String>,
}

impl RecordingAudio {
    fn flush(&mut self, playing: &mut [ScheduledVoice<PlayingSoundHandle>]) -> Result<()> {
        if let Some(error) = &self.failure {
            anyhow::bail!("{error}");
        }
        let end = (self.epoch.elapsed().as_nanos() * u128::from(RECORDING_SAMPLE_RATE)
            / 1_000_000_000) as u64;
        while self.frames < end {
            for sound in playing.iter_mut() {
                match &mut sound.handle {
                    #[cfg(test)]
                    PlayingSoundHandle::Kira { .. } => {}
                    PlayingSoundHandle::OpenAl {
                        state: OpenAlVoiceState::Active(source),
                        ..
                    } => {
                        source.pump_recording_stream()?;
                    }
                    PlayingSoundHandle::OpenAl { .. } => {}
                }
            }
            let available =
                RECORDING_CHUNK_FRAMES - self.pending.len() / usize::from(RECORDING_CHANNELS);
            let frames = (end - self.frames).min(available as u64) as usize;
            let old_len = self.pending.len();
            let sample_count = frames * usize::from(RECORDING_CHANNELS);
            self.pending.resize(old_len + sample_count, 0);
            if let Err(error) = self
                .backend
                .render_recording(&mut self.pending[old_len..old_len + sample_count])
            {
                self.pending.truncate(old_len);
                return Err(error);
            }
            if self.pending.len() == RECORDING_CHUNK_SAMPLES {
                self.send_pending()?;
            }
            self.frames += frames as u64;
        }
        Ok(())
    }

    fn send_pending(&mut self) -> Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let samples = std::mem::replace(
            &mut self.pending,
            Vec::with_capacity(RECORDING_CHUNK_SAMPLES),
        );
        self.sender.try_send(samples).map_err(|error| match error {
            std::sync::mpsc::TrySendError::Full(_) => {
                anyhow!("recording audio encoder queue is full")
            }
            std::sync::mpsc::TrySendError::Disconnected(_) => {
                anyhow!("recording audio encoder disconnected")
            }
        })
    }
}

impl NativeAudioRuntime {
    pub fn begin_recording(&mut self, sender: SyncSender<Vec<i16>>) -> Result<Instant> {
        self.flush_pending_decodes(Duration::from_secs(2));
        let Some(openal_state) = Self::openal_backend_mut(&mut self.backend) else {
            anyhow::bail!("audio recording requires native OpenAL playback");
        };
        if openal_state.recording.is_some() || openal_state.recording_error.is_some() {
            anyhow::bail!("audio recording is already active");
        }
        let mut backend = openal_state.backend.recording_backend()?;
        for sound in &mut self.scheduler.playing {
            match &mut sound.handle {
                #[cfg(test)]
                PlayingSoundHandle::Kira { .. } => {}
                PlayingSoundHandle::OpenAl {
                    state: OpenAlVoiceState::Active(source),
                    ..
                } => {
                    if let Err(error) = source.attach_recording_with_worker(
                        &mut backend,
                        Some(&openal_state.decode_worker),
                    ) {
                        detach_recording_sources(&mut self.scheduler.playing);
                        openal_state.backend.restore_current()?;
                        return Err(error);
                    }
                }
                PlayingSoundHandle::OpenAl { .. } => {}
            }
        }
        openal_state.backend.restore_current()?;
        let epoch = Instant::now();
        openal_state.recording = Some(RecordingAudio {
            backend,
            sender,
            epoch,
            frames: 0,
            pending: Vec::with_capacity(RECORDING_CHUNK_SAMPLES),
            failure: None,
        });
        Ok(epoch)
    }

    pub fn poll_recording(&mut self) -> Result<()> {
        self.drain_completed_decodes();
        let Some(openal_state) = Self::openal_backend_mut(&mut self.backend) else {
            return Ok(());
        };
        if let Some(error) = &openal_state.recording_error {
            anyhow::bail!("{error}");
        }
        let Some(mut recording) = openal_state.recording.take() else {
            return Ok(());
        };
        let result = recording.flush(&mut self.scheduler.playing);
        match result {
            Ok(()) => {
                openal_state.recording = Some(recording);
                openal_state.backend.restore_current()
            }
            Err(error) => {
                detach_recording_sources(&mut self.scheduler.playing);
                drop(recording);
                openal_state.recording_error = Some(format!("{error:#}"));
                openal_state.backend.restore_current()?;
                Err(error)
            }
        }
    }

    pub fn end_recording(&mut self) -> Result<u64> {
        let result = self.poll_recording();
        let Some(openal_state) = Self::openal_backend_mut(&mut self.backend) else {
            anyhow::bail!("no audio recording is active");
        };
        if let Some(error) = openal_state.recording_error.take() {
            anyhow::bail!("{error}");
        }
        result?;
        let mut recording = openal_state
            .recording
            .take()
            .context("no audio recording is active")?;
        detach_recording_sources(&mut self.scheduler.playing);
        openal_state.backend.restore_current()?;
        recording.send_pending()?;
        Ok(recording.frames)
    }
}

fn detach_recording_sources(playing: &mut [ScheduledVoice<PlayingSoundHandle>]) {
    for sound in playing {
        match &mut sound.handle {
            #[cfg(test)]
            PlayingSoundHandle::Kira { .. } => {}
            PlayingSoundHandle::OpenAl {
                state: OpenAlVoiceState::Active(source),
                ..
            } => {
                source.detach_recording();
            }
            PlayingSoundHandle::OpenAl { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires verified BBB_TEST_OPENAL_LIBRARY and BBB_TEST_OPENAL_OGG"]
    fn recording_runtime_delivers_bounded_pcm_and_keeps_playback_after_capture_failure() {
        fn openal_state_mut(runtime: &mut NativeAudioRuntime) -> &mut OpenAlBackendState {
            match &mut runtime.backend {
                ActiveAudioBackend::OpenAl(openal_state) => openal_state,
                ActiveAudioBackend::Kira(_) => unreachable!("expected OpenAL backend"),
            }
        }

        let _receipt = crate::openal::OPENAL_RECEIPT_LOCK.lock().unwrap();
        let library = std::env::var_os("BBB_TEST_OPENAL_LIBRARY").unwrap();
        let device = OpenAlBackend::open(&library, None, false).unwrap();
        let physical = device.recording_backend().unwrap();
        let mut runtime = NativeAudioRuntime::from_openal_backend(
            physical,
            7,
            None,
            AudioChannelLimits::from_mono_sources(30),
            "recording-test",
        )
        .unwrap();
        let sound = ResolvedSound {
            event_id: "minecraft:receipt".into(),
            sound_name: "minecraft:receipt".into(),
            ogg_path: std::env::var_os("BBB_TEST_OPENAL_OGG").unwrap().into(),
            resource_generation: 7,
            stream: false,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        };
        runtime
            .play_local_sound_with_role(&sound, AudioCategory::Music, 1.0, 1.0, true, false)
            .unwrap();
        runtime.flush_pending_decodes(Duration::from_secs(5));
        openal_state_mut(&mut runtime)
            .backend
            .render_recording(&mut vec![0; 4800])
            .unwrap();
        let (sender, receiver) = std::sync::mpsc::sync_channel(16);
        runtime.begin_recording(sender).unwrap();
        openal_state_mut(&mut runtime)
            .recording
            .as_mut()
            .unwrap()
            .epoch = Instant::now() - Duration::from_millis(100);
        runtime.poll_recording().unwrap();
        let mut chunks: Vec<_> = receiver.try_iter().collect();
        let frames = runtime.end_recording().unwrap();
        chunks.extend(receiver.try_iter());
        assert_eq!(
            chunks.iter().map(|chunk| chunk.len() / 2).sum::<usize>() as u64,
            frames
        );
        assert!(frames >= 4800);
        assert!(chunks
            .iter()
            .all(|chunk| !chunk.is_empty() && chunk.len() <= 2048 && chunk.len() % 2 == 0));
        assert!(chunks.iter().flatten().any(|sample| sample.abs() > 100));
        assert!(matches!(
            receiver.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Disconnected)
        ));
        let (sender, _receiver) = std::sync::mpsc::sync_channel(1);
        runtime.begin_recording(sender).unwrap();
        openal_state_mut(&mut runtime)
            .recording
            .as_mut()
            .unwrap()
            .epoch = Instant::now() - Duration::from_millis(100);
        runtime
            .handle_command(&AudioCommand::SetMusicGain(crate::SetMusicGainCommand {
                gain: 0.5,
            }))
            .unwrap();
        assert!(runtime
            .poll_recording()
            .unwrap_err()
            .to_string()
            .contains("queue is full"));
        assert!(runtime.end_recording().is_err());
        let mut audible = vec![0; 8192];
        openal_state_mut(&mut runtime)
            .backend
            .render_recording(&mut audible)
            .unwrap();
        assert!(
            audible.iter().any(|sample| sample.abs() > 100),
            "capture failure stopped normal playback"
        );
    }
}
