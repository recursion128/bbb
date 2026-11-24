use super::*;
use crate::{RECORDING_CHANNELS, RECORDING_SAMPLE_RATE};

const AL_SAMPLE_OFFSET: i32 = 0x1025;
const ALC_FREQUENCY: i32 = 0x1007;
const ALC_FORMAT_CHANNELS_SOFT: i32 = 0x1990;
const ALC_FORMAT_TYPE_SOFT: i32 = 0x1991;
const ALC_STEREO_SOFT: i32 = 0x1501;
const ALC_SHORT_SOFT: i32 = 0x1402;

type AlcLoopbackOpenDevice = unsafe extern "C" fn(*const c_char) -> *mut ALCdevice;
pub(super) type AlcRenderSamples = unsafe extern "C" fn(*mut ALCdevice, *mut c_void, i32);

impl OpenAlBackend {
    pub(crate) fn recording_backend(&self) -> Result<Self> {
        let api = Rc::clone(&self.context.api);
        if !alc_extension_present(&api, ptr::null_mut(), c"ALC_SOFT_loopback") {
            bail!("verified OpenAL implementation lacks ALC_SOFT_loopback");
        }
        // SAFETY: extension addresses and handles come from this retained library.
        let open: AlcLoopbackOpenDevice =
            unsafe { load_alc_proc(&api, c"alcLoopbackOpenDeviceSOFT") }
                .context("load OpenAL loopback device entry point")?;
        let render_samples: AlcRenderSamples =
            unsafe { load_alc_proc(&api, c"alcRenderSamplesSOFT") }
                .context("load OpenAL loopback renderer")?;
        let device = unsafe { open(ptr::null()) };
        if device.is_null() {
            bail!("open OpenAL recording loopback device");
        }
        let soft = match probe_soft_device_extensions(&api, device) {
            Ok(soft) => soft,
            Err(error) => {
                // SAFETY: `device` was opened above and no context owns it yet.
                unsafe { api.alcCloseDevice(device) };
                return Err(error);
            }
        };
        let mut attributes = context_attributes(soft, self.context.observation.requested_hrtf);
        attributes.pop();
        attributes.extend([
            ALC_FREQUENCY,
            RECORDING_SAMPLE_RATE as i32,
            ALC_FORMAT_CHANNELS_SOFT,
            ALC_STEREO_SOFT,
            ALC_FORMAT_TYPE_SOFT,
            ALC_SHORT_SOFT,
            0,
        ]);
        // SAFETY: the open device and terminated attributes outlive this call.
        let context = unsafe { api.alcCreateContext(device, attributes.as_ptr()) };
        if context.is_null() {
            unsafe { api.alcCloseDevice(device) };
            bail!("create OpenAL recording context");
        }
        let initialized = (|| {
            activate_new_context(&api, context)?;
            finish_context_initialization(
                &api,
                device,
                context,
                "Recording loopback".to_owned(),
                self.context.observation.requested_hrtf,
                soft,
            )
        })();
        let observation = match initialized {
            Ok((_, observation)) => observation,
            Err(error) => {
                clear_current_context_if(&api, context);
                unsafe {
                    api.alcDestroyContext(context);
                    api.alcCloseDevice(device);
                }
                self.context.make_current()?;
                return Err(error);
            }
        };
        let backend = Self {
            context: OpenAlDeviceContext {
                api,
                device,
                context,
                observation,
            },
            static_buffers: StaticBufferCache::new(),
            render_samples: Some(render_samples),
        };
        self.context.make_current()?;
        let mut position = [0.0; 3];
        let mut orientation = [0.0; 6];
        unsafe {
            self.context
                .api
                .alGetListenerfv(AL_POSITION, position.as_mut_ptr());
            self.context
                .api
                .alGetListenerfv(AL_ORIENTATION, orientation.as_mut_ptr());
        }
        backend.set_listener(
            position,
            [orientation[0], orientation[1], orientation[2]],
            [orientation[3], orientation[4], orientation[5]],
        )?;
        self.context.make_current()?;
        Ok(backend)
    }

    pub(crate) fn render_recording(&self, samples: &mut [i16]) -> Result<()> {
        let render = self
            .render_samples
            .context("OpenAL backend is not a recording loopback device")?;
        self.context.make_current()?;
        // SAFETY: stereo PCM16 and the exact writable frame count match this device;
        // `render` was resolved from the retained library when opening the loopback device.
        unsafe {
            render(
                self.context.device,
                samples.as_mut_ptr().cast(),
                (samples.len() / usize::from(RECORDING_CHANNELS)) as i32,
            );
        }
        check_alc_error(
            &self.context.api,
            self.context.device,
            "render recording audio",
        )
    }

    pub(crate) fn restore_current(&self) -> Result<()> {
        self.context.make_current()
    }
}

impl OpenAlSoundSource {
    #[cfg(test)]
    pub(crate) fn attach_recording(&mut self, backend: &mut OpenAlBackend) -> Result<()> {
        self.attach_recording_with_worker(backend, None)
    }

    pub(crate) fn attach_recording_with_worker(
        &mut self,
        backend: &mut OpenAlBackend,
        decode_worker: Option<&AudioDecodeWorker>,
    ) -> Result<()> {
        make_context_current(&self.api, self.context)?;
        if !self.is_active() {
            return Ok(());
        }
        let mut position = [0.0; 3];
        let mut pitch = 0.0;
        let mut gain = 0.0;
        let mut relative = 0;
        let mut looping = 0;
        let mut distance_model = 0;
        let mut max_distance = 0.0;
        let mut offset = 0;
        // SAFETY: the source belongs to the current context; outputs have the declared sizes.
        unsafe {
            self.api
                .alGetSourcefv(self.source, AL_POSITION, position.as_mut_ptr());
            self.api.alGetSourcef(self.source, AL_PITCH, &mut pitch);
            self.api.alGetSourcef(self.source, AL_GAIN, &mut gain);
            self.api
                .alGetSourcei(self.source, AL_SOURCE_RELATIVE, &mut relative);
            self.api.alGetSourcei(self.source, AL_LOOPING, &mut looping);
            self.api
                .alGetSourcei(self.source, AL_SOURCE_DISTANCE_MODEL, &mut distance_model);
            self.api
                .alGetSourcef(self.source, AL_MAX_DISTANCE, &mut max_distance);
            self.api
                .alGetSourcei(self.source, AL_SAMPLE_OFFSET, &mut offset);
        }
        check_al_error(&self.api, "snapshot recording source")?;
        let settings = OpenAlSourceSettings {
            position,
            pitch,
            gain,
            looping: looping != 0,
            relative: relative != 0,
            max_distance: (distance_model == AL_LINEAR_DISTANCE).then_some(max_distance),
        };
        let sample_offset = offset.max(0);
        let recording = if let Some(stream) = &self.stream {
            let start_frame = stream.buffers.removed_frames + sample_offset as u64;
            if let Some(worker) = decode_worker {
                let async_stream =
                    worker.open_stream_at(stream.path.clone(), stream.looping, start_frame);
                backend.play_stream_async(async_stream, settings)?
            } else {
                let decoder =
                    OpenAlStreamDecoder::open_at(&stream.path, stream.looping, start_frame)?;
                backend.play_stream(decoder, settings)?
            }
        } else {
            let buffer = self.static_buffer.as_ref().unwrap();
            let recording = backend.play_static_pcm(
                buffer.generation,
                &buffer.path,
                buffer.pcm.clone(),
                settings,
            )?;
            unsafe {
                recording
                    .api
                    .alSourcei(recording.source, AL_SAMPLE_OFFSET, sample_offset)
            };
            check_al_error(&recording.api, "resume recording static source")?;
            recording
        };
        self.recording = Some(Box::new(recording));
        make_context_current(&self.api, self.context)
    }

    pub(crate) fn detach_recording(&mut self) {
        self.recording.take();
    }

    pub(crate) fn pump_recording_stream(&mut self) -> Result<()> {
        if let Some(recording) = self.recording.as_mut() {
            recording.update_stream()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::residency::AudioResidency;
    use crate::ResolvedSound;

    fn render(backend: &OpenAlBackend, source: &mut OpenAlSoundSource, frames: usize) -> Vec<i16> {
        let mut result = Vec::with_capacity(frames * 2);
        for count in (0..frames)
            .step_by(512)
            .map(|start| (frames - start).min(512))
        {
            source.update_stream().unwrap();
            let mut samples = vec![0; count * 2];
            backend.render_recording(&mut samples).unwrap();
            result.extend(samples);
        }
        result
    }

    fn energy(samples: &[i16], channel: usize) -> f64 {
        samples
            .iter()
            .skip(channel)
            .step_by(2)
            .map(|sample| f64::from(*sample).powi(2))
            .sum()
    }

    fn assert_same_waveform(physical: &[i16], recorded: &[i16]) {
        let physical = &physical[2048..];
        let recorded = &recorded[2048..];
        let correlation: f64 = physical
            .iter()
            .zip(recorded)
            .map(|(a, b)| f64::from(*a) * f64::from(*b))
            .sum();
        let physical_energy: f64 = physical.iter().map(|v| f64::from(*v).powi(2)).sum();
        let recorded_energy: f64 = recorded.iter().map(|v| f64::from(*v).powi(2)).sum();
        let normalized = correlation / (physical_energy * recorded_energy).sqrt();
        let amplitude = (recorded_energy / physical_energy).sqrt();
        assert!(
            (0.9..1.1).contains(&amplitude),
            "recording changed source gain: ratio={amplitude}"
        );
        assert!(
            normalized > 0.98,
            "recording changed resumed waveform phase: correlation={normalized}"
        );
    }

    #[test]
    #[ignore = "requires verified BBB_TEST_OPENAL_LIBRARY and BBB_TEST_OPENAL_OGG"]
    fn recording_loopback_resumes_static_and_stream_and_tracks_spatial_gain_stop() {
        let _receipt = OPENAL_RECEIPT_LOCK.lock().unwrap();
        let library = std::env::var_os("BBB_TEST_OPENAL_LIBRARY").unwrap();
        let ogg = PathBuf::from(std::env::var_os("BBB_TEST_OPENAL_OGG").unwrap());
        let device = OpenAlBackend::open(&library, None, false).unwrap();
        let mut physical = device.recording_backend().unwrap();
        let mut recording = physical.recording_backend().unwrap();
        let settings = OpenAlSourceSettings {
            position: [-4.0, 0.0, -1.0],
            pitch: 1.0,
            gain: 1.0,
            looping: true,
            relative: false,
            max_distance: None,
        };
        let mut source = physical
            .play_static(7, &ogg, || Ok(OpenAlStaticSound::read(&ogg)?), settings)
            .unwrap();
        render(&physical, &mut source, 6200);
        source.attach_recording(&mut recording).unwrap();
        let expected = render(&physical, &mut source, 8192);
        let actual = render(&recording, source.recording.as_mut().unwrap(), 8192);
        assert_same_waveform(&expected, &actual);
        assert!(
            energy(&actual, 0) > energy(&actual, 1) * 2.0,
            "left source lost stereo position"
        );
        source.set_position([4.0, 0.0, -1.0]);
        source.set_volume(0.25);
        let expected = render(&physical, &mut source, 8192);
        let actual = render(&recording, source.recording.as_mut().unwrap(), 8192);
        assert_same_waveform(&expected, &actual);
        assert!(
            energy(&actual, 1) > energy(&actual, 0) * 2.0,
            "moving source did not pan right"
        );
        source.stop();
        render(&recording, source.recording.as_mut().unwrap(), 4096);
        let silent = render(&recording, source.recording.as_mut().unwrap(), 2048);
        assert!(
            silent.iter().all(|value| value.abs() <= 1),
            "stopped source remains audible"
        );
        drop(source);
        let sound = ResolvedSound {
            event_id: "minecraft:receipt".into(),
            sound_name: "minecraft:receipt".into(),
            ogg_path: ogg,
            resource_generation: 7,
            stream: true,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        };
        let mut residency = AudioResidency::new(7);
        let decoder = residency.openal_streaming_sound(&sound, true).unwrap();
        let mut stream = physical
            .play_stream(
                decoder,
                OpenAlSourceSettings {
                    looping: false,
                    relative: true,
                    position: [0.0; 3],
                    ..settings
                },
            )
            .unwrap();
        render(&physical, &mut stream, 210_123);
        assert!(stream.stream.as_ref().unwrap().buffers.removed_frames > 0);
        stream.attach_recording(&mut recording).unwrap();
        let expected = render(&physical, &mut stream, 8192);
        let actual = render(&recording, stream.recording.as_mut().unwrap(), 8192);
        assert_same_waveform(&expected, &actual);
        stream.detach_recording();
        drop(stream);
        drop(recording);
        drop(physical);
        drop(device);
    }
}
