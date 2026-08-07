use std::{
    collections::HashMap,
    ffi::{c_void, CStr, CString},
    path::{Path, PathBuf},
    ptr,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use al_sys::{ALCcontext, ALCdevice, AlApi};
use anyhow::{anyhow, bail, Context, Result};

#[cfg(test)]
use std::sync::Mutex;

use crate::residency::{
    OpenAlStaticSound, OpenAlStreamDecoder, MAX_STATIC_SOUND_CACHE_BUFFERS,
    MAX_STATIC_SOUND_CACHE_BYTES,
};
use crate::AudioDeviceSnapshot;

const ALC_ALL_ATTRIBUTES_SIZE: i32 = 4098;
const ALC_ALL_ATTRIBUTES: i32 = 4099;
const ALC_DEFAULT_ALL_DEVICES_SPECIFIER: i32 = 4114;
const ALC_ALL_DEVICES_SPECIFIER: i32 = 4115;
const ALC_MONO_SOURCES: i32 = 4112;
const ALC_HRTF_SOFT: i32 = 6546;
const ALC_HRTF_STATUS_SOFT: i32 = 6547;
const ALC_NUM_HRTF_SPECIFIERS_SOFT: i32 = 6548;
const ALC_HRTF_ID_SOFT: i32 = 6550;
const ALC_OUTPUT_LIMITER_SOFT: i32 = 6554;
const AL_SOURCE_DISTANCE_MODEL_CAPABILITY: i32 = 512;
const AL_SOURCE_RELATIVE: i32 = 514;
const AL_PITCH: i32 = 4099;
const AL_POSITION: i32 = 4100;
const AL_LOOPING: i32 = 4103;
const AL_BUFFER: i32 = 4105;
const AL_GAIN: i32 = 4106;
const AL_ORIENTATION: i32 = 4111;
const AL_SOURCE_STATE: i32 = 4112;
const AL_STOPPED: i32 = 4116;
const AL_BUFFERS_QUEUED: i32 = 4117;
const AL_BUFFERS_PROCESSED: i32 = 4118;
const AL_REFERENCE_DISTANCE: i32 = 4128;
const AL_ROLLOFF_FACTOR: i32 = 4129;
const AL_MAX_DISTANCE: i32 = 4131;
const AL_NONE: i32 = 0;
const AL_FORMAT_MONO16: i32 = 4353;
const AL_FORMAT_STEREO16: i32 = 4355;
const AL_LINEAR_DISTANCE: i32 = 53251;
const AL_SOURCE_DISTANCE_MODEL: i32 = 53248;
const DEFAULT_MONO_SOURCES: usize = 30;
const MAX_DEVICE_ATTRIBUTES: usize = 4096;
const OPENAL_CAPACITY_ORIGIN: &str = "openal-alc-mono-sources";
const MAX_DEVICE_COUNT: usize = 256;
const MAX_DEVICE_NAME_BYTES: usize = 4096;

const ALC_ENUMERATE_ALL_EXT: &CStr = c"ALC_ENUMERATE_ALL_EXT";
const ALC_SOFT_HRTF: &CStr = c"ALC_SOFT_HRTF";
const ALC_EXT_DISCONNECT: &CStr = c"ALC_EXT_disconnect";
const ALC_SOFT_SYSTEM_EVENTS: &CStr = c"ALC_SOFT_system_events";
const AL_EXT_SOURCE_DISTANCE_MODEL: &CStr = c"AL_EXT_source_distance_model";
const AL_EXT_LINEAR_DISTANCE: &CStr = c"AL_EXT_LINEAR_DISTANCE";
const ALC_EVENT_TYPE_DEFAULT_DEVICE_CHANGED_SOFT: i32 = 6614;
const ALC_EVENT_TYPE_DEVICE_ADDED_SOFT: i32 = 6615;
const ALC_EVENT_TYPE_DEVICE_REMOVED_SOFT: i32 = 6616;
const ALC_PLAYBACK_DEVICE_SOFT: i32 = 6612;
const ALC_EVENT_SUPPORTED_SOFT: i32 = 6617;
const SYSTEM_EVENT_TYPES: [i32; 3] = [
    ALC_EVENT_TYPE_DEFAULT_DEVICE_CHANGED_SOFT,
    ALC_EVENT_TYPE_DEVICE_ADDED_SOFT,
    ALC_EVENT_TYPE_DEVICE_REMOVED_SOFT,
];

const ALC_EVENT_IS_SUPPORTED_SOFT: &CStr = c"alcEventIsSupportedSOFT";
const ALC_EVENT_CONTROL_SOFT: &CStr = c"alcEventControlSOFT";
const ALC_EVENT_CALLBACK_SOFT: &CStr = c"alcEventCallbackSOFT";

#[cfg(test)]
pub(crate) static OPENAL_RECEIPT_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn query_device_snapshot(library_path: &Path) -> Result<AudioDeviceSnapshot> {
    OpenAlDeviceEnumerator::open(library_path)?.snapshot()
}

/// Keeps the verified OpenAL library loaded across background polls. Vanilla's
/// `PollingDeviceTracker` repeats the device query, not native-library loading.
pub(crate) struct OpenAlDeviceEnumerator {
    api: Rc<AlApi>,
}

type AlcEventProcSoft = unsafe extern "C" fn(
    event_type: i32,
    device_type: i32,
    device: *mut ALCdevice,
    message_length: i32,
    message: *const i8,
    user_param: *mut c_void,
);
type AlcEventIsSupportedSoft = unsafe extern "C" fn(event_type: i32, device_type: i32) -> i32;
type AlcEventControlSoft =
    unsafe extern "C" fn(count: i32, event_types: *const i32, enable: al_sys::ALCboolean);
type AlcEventCallbackSoft =
    unsafe extern "C" fn(callback: Option<AlcEventProcSoft>, user_param: *mut c_void);

pub(crate) struct OpenAlSystemEventTracker {
    _api: Rc<AlApi>,
    update_requested: Arc<AtomicBool>,
    event_control: AlcEventControlSoft,
    event_callback: AlcEventCallbackSoft,
}

impl OpenAlDeviceEnumerator {
    pub(crate) fn open(library_path: &Path) -> Result<Self> {
        let api = AlApi::load(library_path).with_context(|| {
            format!(
                "load OpenAL device query library {}",
                library_path.display()
            )
        })?;
        Ok(Self { api: Rc::new(api) })
    }

    pub(crate) fn snapshot(&self) -> Result<AudioDeviceSnapshot> {
        if !alc_extension_present(&self.api, ptr::null_mut(), ALC_ENUMERATE_ALL_EXT) {
            return Ok(AudioDeviceSnapshot::default());
        }
        Ok(AudioDeviceSnapshot {
            default_device: query_alc_string(
                &self.api,
                ptr::null_mut(),
                ALC_DEFAULT_ALL_DEVICES_SPECIFIER,
            ),
            all_devices: query_alc_string_list(
                &self.api,
                ptr::null_mut(),
                ALC_ALL_DEVICES_SPECIFIER,
            )?,
        })
    }

    /// Mirrors `Library.createDeviceTracker`: select SOFT system-event
    /// callbacks only when all three playback event types report supported;
    /// otherwise the caller keeps Vanilla's one-second polling fallback.
    pub(crate) fn install_system_event_tracker(&self) -> Option<OpenAlSystemEventTracker> {
        if !alc_extension_present(&self.api, ptr::null_mut(), ALC_SOFT_SYSTEM_EVENTS) {
            return None;
        }
        // SAFETY: all addresses are queried from the retained verified OpenAL
        // library and converted to their published ALC_SOFT_system_events ABI.
        let event_is_supported: AlcEventIsSupportedSoft =
            unsafe { load_alc_proc(&self.api, ALC_EVENT_IS_SUPPORTED_SOFT)? };
        let event_control: AlcEventControlSoft =
            unsafe { load_alc_proc(&self.api, ALC_EVENT_CONTROL_SOFT)? };
        let event_callback: AlcEventCallbackSoft =
            unsafe { load_alc_proc(&self.api, ALC_EVENT_CALLBACK_SOFT)? };
        // SAFETY: the loaded extension function accepts these exact enum
        // values and has no pointer arguments.
        if !SYSTEM_EVENT_TYPES.iter().all(|event_type| unsafe {
            event_is_supported(*event_type, ALC_PLAYBACK_DEVICE_SOFT) == ALC_EVENT_SUPPORTED_SOFT
        }) {
            return None;
        }

        let update_requested = Arc::new(AtomicBool::new(false));
        let user_param = Arc::as_ptr(&update_requested).cast_mut().cast::<c_void>();
        // SAFETY: the static event array and Arc allocation remain valid;
        // Drop unregisters the callback before releasing the Arc.
        unsafe {
            event_control(
                SYSTEM_EVENT_TYPES.len() as i32,
                SYSTEM_EVENT_TYPES.as_ptr(),
                al_sys::ALC_TRUE,
            );
            event_callback(Some(openal_system_event_callback), user_param);
        }
        Some(OpenAlSystemEventTracker {
            _api: Rc::clone(&self.api),
            update_requested,
            event_control,
            event_callback,
        })
    }
}

impl OpenAlSystemEventTracker {
    pub(crate) fn take_update_request(&self) -> bool {
        self.update_requested.swap(false, Ordering::AcqRel)
    }
}

impl Drop for OpenAlSystemEventTracker {
    fn drop(&mut self) {
        // SAFETY: these are the same retained extension entry points used for
        // installation. Clear the callback before the Arc user pointer dies.
        unsafe {
            (self.event_callback)(None, ptr::null_mut());
            (self.event_control)(
                SYSTEM_EVENT_TYPES.len() as i32,
                SYSTEM_EVENT_TYPES.as_ptr(),
                al_sys::ALC_FALSE,
            );
        }
    }
}

unsafe extern "C" fn openal_system_event_callback(
    event_type: i32,
    device_type: i32,
    _device: *mut ALCdevice,
    _message_length: i32,
    _message: *const i8,
    user_param: *mut c_void,
) {
    if user_param.is_null()
        || device_type != ALC_PLAYBACK_DEVICE_SOFT
        || !SYSTEM_EVENT_TYPES.contains(&event_type)
    {
        return;
    }
    // SAFETY: installation passes Arc::as_ptr for an AtomicBool and Drop
    // unregisters this callback before releasing that allocation.
    unsafe { &*user_param.cast::<AtomicBool>() }.store(true, Ordering::Release);
}

unsafe fn load_alc_proc<T>(api: &AlApi, name: &CStr) -> Option<T>
where
    T: Copy,
{
    // SAFETY: the caller supplies an ALC extension function type matching
    // `name`; the retained library owns the address for the tracker's life.
    let address = unsafe { api.alcGetProcAddress(ptr::null_mut(), name.as_ptr()) };
    if address.is_null() || std::mem::size_of::<T>() != std::mem::size_of_val(&address) {
        return None;
    }
    // SAFETY: function and data pointers have the same representation on the
    // supported desktop ABIs, checked above for this concrete type.
    Some(unsafe { std::mem::transmute_copy(&address) })
}

fn query_alc_string_list(
    api: &AlApi,
    device: *mut ALCdevice,
    parameter: i32,
) -> Result<Vec<String>> {
    // SAFETY: OpenAL owns the returned double-NUL-terminated list.
    let mut cursor = unsafe { api.alcGetString(device, parameter) };
    if cursor.is_null() {
        return Ok(Vec::new());
    }
    let mut values = Vec::new();
    while values.len() < MAX_DEVICE_COUNT {
        // SAFETY: the OpenAL list contract guarantees a readable terminator.
        if unsafe { *cursor } == 0 {
            return Ok(values);
        }
        // SAFETY: each list member is a NUL-terminated OpenAL-owned string.
        let value = unsafe { CStr::from_ptr(cursor) };
        if value.to_bytes().len() > MAX_DEVICE_NAME_BYTES {
            bail!("OpenAL device name exceeds {MAX_DEVICE_NAME_BYTES} bytes");
        }
        values.push(value.to_string_lossy().into_owned());
        // SAFETY: advance past this member to the next member or final NUL.
        cursor = unsafe { cursor.add(value.to_bytes_with_nul().len()) };
    }
    bail!("OpenAL device list exceeds {MAX_DEVICE_COUNT} entries")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAlContextObservation {
    pub current_device: String,
    pub requested_hrtf: bool,
    pub hrtf_extension: bool,
    pub hrtf_specifiers: i32,
    pub hrtf_status: i32,
    pub output_limiter: bool,
    pub mono_sources: usize,
    pub capacity_origin: &'static str,
    pub supports_disconnection: bool,
}

/// Owns one OpenAL device and context initialized with the same attribute and
/// extension contract as Vanilla 26.1 `Library.init`.
pub struct OpenAlDeviceContext {
    api: Rc<AlApi>,
    device: *mut ALCdevice,
    context: *mut ALCcontext,
    observation: OpenAlContextObservation,
}

impl OpenAlDeviceContext {
    pub fn open(
        library_path: impl AsRef<Path>,
        preferred_device: Option<&str>,
        use_hrtf: bool,
    ) -> Result<Self> {
        let api = Rc::new(AlApi::load(library_path.as_ref()).with_context(|| {
            format!(
                "load verified OpenAL implementation {}",
                library_path.as_ref().display()
            )
        })?);
        let system_default = query_system_default_device(&api);
        let (device, current_device) = open_device_or_fallback(
            &api,
            device_open_plan(preferred_device, system_default.as_deref()),
        )?;

        let initialized = (|| {
            require_openal_1_1(&api, device)?;
            let hrtf_extension = alc_extension_present(&api, device, ALC_SOFT_HRTF);
            let hrtf_specifiers = alc_integer(&api, device, ALC_NUM_HRTF_SPECIFIERS_SOFT);
            check_alc_error(&api, device, "query HRTF specifiers")?;
            let attributes = context_attributes(hrtf_specifiers, hrtf_extension && use_hrtf);
            // SAFETY: `device` is an open device owned by this function and the
            // attribute list is terminated by ALC_NONE for the duration of the call.
            let context = unsafe { api.alcCreateContext(device, attributes.as_ptr()) };
            if context.is_null() {
                check_alc_error(&api, device, "create context")?;
                bail!("OpenAL returned a null context without an ALC error");
            }
            // SAFETY: `context` was just returned by this API/device pair and
            // stays owned until the matching destroy call.
            if unsafe { api.alcMakeContextCurrent(context) } == al_sys::ALC_FALSE {
                // SAFETY: creation succeeded and the context has not been destroyed.
                unsafe { api.alcDestroyContext(context) };
                bail!("make OpenAL context current");
            }

            match finish_context_initialization(
                &api,
                device,
                context,
                current_device,
                use_hrtf,
                hrtf_extension,
                hrtf_specifiers,
            ) {
                Ok(value) => Ok(value),
                Err(error) => {
                    // SAFETY: the context is current and exclusively owned here.
                    unsafe {
                        api.alcMakeContextCurrent(ptr::null_mut());
                        api.alcDestroyContext(context);
                    }
                    Err(error)
                }
            }
        })();

        match initialized {
            Ok((context, observation)) => Ok(Self {
                api,
                device,
                context,
                observation,
            }),
            Err(error) => {
                // SAFETY: `device` is still open and no surviving context owns it.
                unsafe { api.alcCloseDevice(device) };
                Err(error)
            }
        }
    }

    pub fn observation(&self) -> &OpenAlContextObservation {
        &self.observation
    }

    pub fn is_disconnected(&self) -> bool {
        if !self.observation.supports_disconnection {
            return false;
        }
        alc_integer(&self.api, self.device, 787) == 0
    }

    fn make_current(&self) -> Result<()> {
        // SAFETY: this owner keeps the context alive through the call.
        if unsafe { self.api.alcMakeContextCurrent(self.context) } == al_sys::ALC_FALSE {
            bail!("make owned OpenAL context current");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OpenAlSourceSettings {
    pub(crate) position: [f32; 3],
    pub(crate) pitch: f32,
    pub(crate) gain: f32,
    pub(crate) looping: bool,
    pub(crate) relative: bool,
    pub(crate) max_distance: Option<f32>,
}

/// Production playback owner for Vanilla's `Library`/`Channel` boundary.
/// Static native buffers are cached per immutable resource generation while
/// streaming sources own and rotate exactly four one-second OpenAL buffers.
pub(crate) struct OpenAlBackend {
    context: OpenAlDeviceContext,
    static_buffers: HashMap<(u64, PathBuf), OpenAlStaticBufferEntry>,
    static_buffer_bytes: u64,
    static_buffer_use_sequence: u64,
    static_buffer_evictions: u64,
}

pub(crate) struct OpenAlSoundSource {
    api: Rc<AlApi>,
    context: *mut ALCcontext,
    source: u32,
    static_buffer: Option<Rc<OpenAlStaticBuffer>>,
    stream: Option<OpenAlStreamState>,
}

struct OpenAlStaticBufferEntry {
    buffer: Rc<OpenAlStaticBuffer>,
    decoded_bytes: u64,
    last_used: u64,
}

struct OpenAlStaticBuffer {
    api: Rc<AlApi>,
    context: *mut ALCcontext,
    buffer: u32,
}

struct OpenAlStreamState {
    decoder: OpenAlStreamDecoder,
    buffers: Vec<u32>,
}

impl OpenAlBackend {
    pub(crate) fn open(
        library_path: impl AsRef<Path>,
        preferred_device: Option<&str>,
        use_hrtf: bool,
    ) -> Result<Self> {
        Ok(Self {
            context: OpenAlDeviceContext::open(library_path, preferred_device, use_hrtf)?,
            static_buffers: HashMap::new(),
            static_buffer_bytes: 0,
            static_buffer_use_sequence: 0,
            static_buffer_evictions: 0,
        })
    }

    pub(crate) fn observation(&self) -> &OpenAlContextObservation {
        self.context.observation()
    }

    pub(crate) fn is_disconnected(&self) -> bool {
        self.context.is_disconnected()
    }

    pub(crate) fn static_cache_observation(&self) -> (usize, u64, u64) {
        (
            self.static_buffers.len(),
            self.static_buffer_bytes,
            self.static_buffer_evictions,
        )
    }

    pub(crate) fn clear_static_buffers(&mut self) {
        self.static_buffers.clear();
        self.static_buffer_bytes = 0;
    }

    pub(crate) fn set_listener(
        &self,
        position: [f32; 3],
        forward: [f32; 3],
        up: [f32; 3],
    ) -> Result<()> {
        self.context.make_current()?;
        let orientation = [forward[0], forward[1], forward[2], up[0], up[1], up[2]];
        // SAFETY: the backend owns the current context and both fixed arrays
        // remain alive for the duration of their OpenAL calls.
        unsafe {
            self.context
                .api
                .alListener3f(AL_POSITION, position[0], position[1], position[2]);
            self.context
                .api
                .alListenerfv(AL_ORIENTATION, orientation.as_ptr());
        }
        check_al_error(&self.context.api, "update listener transform")
    }

    pub(crate) fn play_static<F>(
        &mut self,
        generation: u64,
        path: &Path,
        load_pcm: F,
        settings: OpenAlSourceSettings,
    ) -> Result<OpenAlSoundSource>
    where
        F: FnOnce() -> Result<OpenAlStaticSound>,
    {
        self.context.make_current()?;
        let key = (generation, path.to_path_buf());
        self.static_buffer_use_sequence = self.static_buffer_use_sequence.wrapping_add(1);
        let use_sequence = self.static_buffer_use_sequence;
        let buffer = if let Some(entry) = self.static_buffers.get_mut(&key) {
            entry.last_used = use_sequence;
            Rc::clone(&entry.buffer)
        } else {
            let pcm = load_pcm()?;
            let decoded_bytes = u64::try_from(pcm.samples.len())
                .ok()
                .and_then(|samples| samples.checked_mul(std::mem::size_of::<i16>() as u64))
                .context("OpenAL static buffer size overflow")?;
            self.ensure_static_buffer_capacity(decoded_bytes)?;
            let buffer_id = upload_pcm_buffer(
                &self.context.api,
                pcm.sample_rate,
                pcm.channels,
                &pcm.samples,
            )?;
            let buffer = Rc::new(OpenAlStaticBuffer {
                api: Rc::clone(&self.context.api),
                context: self.context.context,
                buffer: buffer_id,
            });
            self.static_buffers.insert(
                key,
                OpenAlStaticBufferEntry {
                    buffer: Rc::clone(&buffer),
                    decoded_bytes,
                    last_used: use_sequence,
                },
            );
            self.static_buffer_bytes = self
                .static_buffer_bytes
                .checked_add(decoded_bytes)
                .context("OpenAL static cache byte count overflow")?;
            buffer
        };
        let mut source = OpenAlSoundSource::create(
            Rc::clone(&self.context.api),
            self.context.context,
            settings,
        )?;
        // SAFETY: both identifiers belong to this current context and the
        // cached static buffer outlives the returned source.
        unsafe {
            self.context
                .api
                .alSourcei(source.source, AL_BUFFER, buffer.buffer as i32)
        };
        check_al_error(&self.context.api, "attach static buffer")?;
        source.static_buffer = Some(buffer);
        source.play()?;
        Ok(source)
    }

    fn ensure_static_buffer_capacity(&mut self, required: u64) -> Result<()> {
        if required > MAX_STATIC_SOUND_CACHE_BYTES {
            bail!(
                "OpenAL static buffer requires {required} bytes, exceeding {MAX_STATIC_SOUND_CACHE_BYTES}"
            );
        }
        while self.static_buffers.len() >= MAX_STATIC_SOUND_CACHE_BUFFERS
            || self
                .static_buffer_bytes
                .checked_add(required)
                .is_none_or(|bytes| bytes > MAX_STATIC_SOUND_CACHE_BYTES)
        {
            let Some(key) = self
                .static_buffers
                .iter()
                .filter(|(_, entry)| Rc::strong_count(&entry.buffer) == 1)
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            else {
                bail!(
                    "OpenAL static cache cannot admit {required} bytes because every bounded LRU owner is attached to a live source"
                );
            };
            let evicted = self
                .static_buffers
                .remove(&key)
                .expect("selected OpenAL static buffer exists");
            self.static_buffer_bytes = self
                .static_buffer_bytes
                .checked_sub(evicted.decoded_bytes)
                .expect("OpenAL static cache accounting cannot underflow");
            self.static_buffer_evictions = self.static_buffer_evictions.saturating_add(1);
        }
        Ok(())
    }

    pub(crate) fn play_stream(
        &mut self,
        decoder: OpenAlStreamDecoder,
        settings: OpenAlSourceSettings,
    ) -> Result<OpenAlSoundSource> {
        self.context.make_current()?;
        let mut source = OpenAlSoundSource::create(
            Rc::clone(&self.context.api),
            self.context.context,
            settings,
        )?;
        source.stream = Some(OpenAlStreamState {
            decoder,
            buffers: Vec::with_capacity(4),
        });
        source.pump_stream_buffers(4)?;
        source.play()?;
        Ok(source)
    }
}

impl Drop for OpenAlBackend {
    fn drop(&mut self) {
        self.clear_static_buffers();
    }
}

impl OpenAlSoundSource {
    fn create(
        api: Rc<AlApi>,
        context: *mut ALCcontext,
        settings: OpenAlSourceSettings,
    ) -> Result<Self> {
        make_context_current(&api, context)?;
        let mut source = 0_u32;
        // SAFETY: `source` supplies one writable identifier in the current context.
        unsafe { api.alGenSources(1, &mut source) };
        check_al_error(&api, "allocate source")?;
        let created = Self {
            api,
            context,
            source,
            static_buffer: None,
            stream: None,
        };
        created.apply_settings(settings)?;
        Ok(created)
    }

    fn apply_settings(&self, settings: OpenAlSourceSettings) -> Result<()> {
        make_context_current(&self.api, self.context)?;
        // SAFETY: the source belongs to the current context and every enum is
        // the exact OpenAL owner used by Vanilla 26.1 `Channel`.
        unsafe {
            self.api.alSource3f(
                self.source,
                AL_POSITION,
                settings.position[0],
                settings.position[1],
                settings.position[2],
            );
            self.api.alSourcef(self.source, AL_PITCH, settings.pitch);
            self.api.alSourcef(self.source, AL_GAIN, settings.gain);
            self.api
                .alSourcei(self.source, AL_LOOPING, i32::from(settings.looping));
            self.api.alSourcei(
                self.source,
                AL_SOURCE_RELATIVE,
                i32::from(settings.relative),
            );
            match settings.max_distance {
                Some(max_distance) => {
                    self.api
                        .alSourcei(self.source, AL_SOURCE_DISTANCE_MODEL, AL_LINEAR_DISTANCE);
                    self.api
                        .alSourcef(self.source, AL_MAX_DISTANCE, max_distance);
                    self.api.alSourcef(self.source, AL_ROLLOFF_FACTOR, 1.0);
                    self.api.alSourcef(self.source, AL_REFERENCE_DISTANCE, 0.0);
                }
                None => self
                    .api
                    .alSourcei(self.source, AL_SOURCE_DISTANCE_MODEL, AL_NONE),
            }
        }
        check_al_error(&self.api, "configure source")
    }

    fn play(&self) -> Result<()> {
        make_context_current(&self.api, self.context)?;
        // SAFETY: source is live in the current context.
        unsafe { self.api.alSourcePlay(self.source) };
        check_al_error(&self.api, "play source")
    }

    pub(crate) fn stop(&mut self) {
        if make_context_current(&self.api, self.context).is_err() {
            return;
        }
        // SAFETY: source is live until this owner is dropped.
        unsafe { self.api.alSourceStop(self.source) };
        let _ = check_al_error(&self.api, "stop source");
    }

    pub(crate) fn is_active(&self) -> bool {
        if make_context_current(&self.api, self.context).is_err() {
            return false;
        }
        let mut state = AL_STOPPED;
        // SAFETY: source is live and `state` supplies one writable integer.
        unsafe {
            self.api
                .alGetSourcei(self.source, AL_SOURCE_STATE, &mut state)
        };
        state != AL_STOPPED
    }

    pub(crate) fn set_volume(&mut self, gain: f32) {
        if make_context_current(&self.api, self.context).is_err() {
            return;
        }
        // SAFETY: source is live until this owner is dropped.
        unsafe { self.api.alSourcef(self.source, AL_GAIN, gain) };
    }

    pub(crate) fn set_playback_rate(&mut self, pitch: f32) {
        if make_context_current(&self.api, self.context).is_err() {
            return;
        }
        // SAFETY: source is live until this owner is dropped.
        unsafe { self.api.alSourcef(self.source, AL_PITCH, pitch) };
    }

    pub(crate) fn set_position(&mut self, position: [f32; 3]) {
        if make_context_current(&self.api, self.context).is_err() {
            return;
        }
        // SAFETY: source is live until this owner is dropped.
        unsafe {
            self.api.alSource3f(
                self.source,
                AL_POSITION,
                position[0],
                position[1],
                position[2],
            )
        };
    }

    pub(crate) fn update_stream(&mut self) -> Result<()> {
        if self.stream.is_none() {
            return Ok(());
        }
        make_context_current(&self.api, self.context)?;
        let mut processed = 0_i32;
        // SAFETY: source is live and `processed` supplies one writable integer.
        unsafe {
            self.api
                .alGetSourcei(self.source, AL_BUFFERS_PROCESSED, &mut processed)
        };
        check_al_error(&self.api, "query processed stream buffers")?;
        if processed <= 0 {
            return Ok(());
        }
        let count = usize::try_from(processed).context("negative processed buffer count")?;
        let mut removed = vec![0_u32; count];
        // SAFETY: `removed` contains exactly `processed` writable identifiers.
        unsafe {
            self.api
                .alSourceUnqueueBuffers(self.source, processed, removed.as_mut_ptr());
            self.api.alDeleteBuffers(processed, removed.as_ptr());
        }
        if let Some(stream) = self.stream.as_mut() {
            stream.buffers.retain(|buffer| !removed.contains(buffer));
        }
        check_al_error(&self.api, "replace processed stream buffers")?;
        self.pump_stream_buffers(count)
    }

    fn pump_stream_buffers(&mut self, count: usize) -> Result<()> {
        let Some(stream) = self.stream.as_mut() else {
            return Ok(());
        };
        for _ in 0..count {
            let chunk = stream.decoder.read_one_second()?;
            if chunk.samples.is_empty() {
                break;
            }
            let buffer =
                upload_pcm_buffer(&self.api, chunk.sample_rate, chunk.channels, &chunk.samples)?;
            // SAFETY: both identifiers belong to the current context.
            unsafe { self.api.alSourceQueueBuffers(self.source, 1, &buffer) };
            if let Err(error) = check_al_error(&self.api, "queue stream buffer") {
                // SAFETY: the upload succeeded but the queue did not claim it.
                unsafe { self.api.alDeleteBuffers(1, &buffer) };
                return Err(error);
            }
            stream.buffers.push(buffer);
        }
        Ok(())
    }
}

impl Drop for OpenAlSoundSource {
    fn drop(&mut self) {
        if make_context_current(&self.api, self.context).is_err() {
            return;
        }
        // SAFETY: this owner deletes the source exactly once. Stopping a
        // streaming source makes its queued buffers processed so they can be
        // detached and deleted before the source.
        unsafe { self.api.alSourceStop(self.source) };
        if self.stream.is_some() {
            let mut queued = 0_i32;
            // SAFETY: source remains live throughout cleanup.
            unsafe {
                self.api
                    .alGetSourcei(self.source, AL_BUFFERS_QUEUED, &mut queued)
            };
            if queued > 0 {
                let mut buffers = vec![0_u32; queued as usize];
                // SAFETY: stopping makes all queued buffers available and the
                // allocation contains exactly `queued` writable identifiers.
                unsafe {
                    self.api
                        .alSourceUnqueueBuffers(self.source, queued, buffers.as_mut_ptr());
                    self.api.alDeleteBuffers(queued, buffers.as_ptr());
                }
            }
        } else {
            // SAFETY: detach the backend-owned static buffer before deletion.
            unsafe { self.api.alSourcei(self.source, AL_BUFFER, 0) };
        }
        // SAFETY: source is still live and exclusively owned.
        unsafe { self.api.alDeleteSources(1, &self.source) };
        self.static_buffer.take();
    }
}

impl Drop for OpenAlStaticBuffer {
    fn drop(&mut self) {
        if make_context_current(&self.api, self.context).is_err() {
            return;
        }
        // SAFETY: the cache/source Rc graph releases this identifier exactly
        // once, after no source remains attached to it.
        unsafe { self.api.alDeleteBuffers(1, &self.buffer) };
        let _ = check_al_error(&self.api, "delete static buffer");
    }
}

fn make_context_current(api: &AlApi, context: *mut ALCcontext) -> Result<()> {
    // SAFETY: every caller retains the context owner while using the pointer.
    if unsafe { api.alcMakeContextCurrent(context) } == al_sys::ALC_FALSE {
        bail!("make source OpenAL context current");
    }
    Ok(())
}

fn upload_pcm_buffer(api: &AlApi, sample_rate: u32, channels: u16, samples: &[i16]) -> Result<u32> {
    let format = match channels {
        1 => AL_FORMAT_MONO16,
        2 => AL_FORMAT_STEREO16,
        other => bail!("OpenAL PCM channel count {other} is not mono/stereo"),
    };
    let byte_count = samples
        .len()
        .checked_mul(std::mem::size_of::<i16>())
        .and_then(|size| i32::try_from(size).ok())
        .context("OpenAL PCM buffer size overflow")?;
    let sample_rate = i32::try_from(sample_rate).context("OpenAL sample rate overflow")?;
    let mut buffer = 0_u32;
    // SAFETY: `buffer` is one writable identifier and `samples` stays alive
    // across the synchronous data upload.
    unsafe {
        api.alGenBuffers(1, &mut buffer);
        api.alBufferData(
            buffer,
            format,
            samples.as_ptr().cast(),
            byte_count,
            sample_rate,
        );
    }
    if let Err(error) = check_al_error(api, "upload PCM buffer") {
        // SAFETY: generation succeeded before the failed upload.
        unsafe { api.alDeleteBuffers(1, &buffer) };
        return Err(error);
    }
    Ok(buffer)
}

impl Drop for OpenAlDeviceContext {
    fn drop(&mut self) {
        // SAFETY: this object exclusively owns the context and device, and
        // fields are destroyed only after the matching OpenAL teardown calls.
        unsafe {
            if self.api.alcGetCurrentContext() == self.context {
                self.api.alcMakeContextCurrent(ptr::null_mut());
            }
            self.api.alcDestroyContext(self.context);
            self.api.alcCloseDevice(self.device);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn finish_context_initialization(
    api: &AlApi,
    device: *mut ALCdevice,
    context: *mut ALCcontext,
    current_device: String,
    requested_hrtf: bool,
    hrtf_extension: bool,
    hrtf_specifiers: i32,
) -> Result<(*mut ALCcontext, OpenAlContextObservation)> {
    let mono_sources = query_mono_sources(api, device)?;
    require_al_extension(api, AL_EXT_SOURCE_DISTANCE_MODEL)?;
    // SAFETY: a current context exists and `AL_SOURCE_DISTANCE_MODEL` is
    // supported by the extension checked immediately above.
    unsafe { api.alEnable(AL_SOURCE_DISTANCE_MODEL_CAPABILITY) };
    require_al_extension(api, AL_EXT_LINEAR_DISTANCE)?;
    check_al_error(api, "enable per-source distance models")?;

    let hrtf_status = alc_integer(api, device, ALC_HRTF_STATUS_SOFT);
    check_alc_error(api, device, "query HRTF status")?;
    let output_limiter = alc_integer(api, device, ALC_OUTPUT_LIMITER_SOFT) != 0;
    check_alc_error(api, device, "query output limiter")?;
    Ok((
        context,
        OpenAlContextObservation {
            current_device,
            requested_hrtf,
            hrtf_extension,
            hrtf_specifiers,
            hrtf_status,
            output_limiter,
            mono_sources,
            capacity_origin: OPENAL_CAPACITY_ORIGIN,
            supports_disconnection: alc_extension_present(api, device, ALC_EXT_DISCONNECT),
        },
    ))
}

fn context_attributes(hrtf_specifiers: i32, enable_hrtf: bool) -> Vec<i32> {
    let mut attributes = Vec::with_capacity(7);
    if hrtf_specifiers > 0 {
        attributes.extend([ALC_HRTF_SOFT, i32::from(enable_hrtf)]);
        attributes.extend([ALC_HRTF_ID_SOFT, 0]);
    }
    attributes.extend([ALC_OUTPUT_LIMITER_SOFT, 1, 0]);
    attributes
}

fn device_open_plan(preferred: Option<&str>, system_default: Option<&str>) -> Vec<Option<String>> {
    let mut candidates = Vec::with_capacity(3);
    if let Some(preferred) = preferred.filter(|name| !name.is_empty()) {
        candidates.push(Some(preferred.to_owned()));
    }
    if let Some(system_default) = system_default {
        candidates.push(Some(system_default.to_owned()));
    }
    candidates.push(None);
    candidates
}

fn open_device_or_fallback(
    api: &AlApi,
    candidates: Vec<Option<String>>,
) -> Result<(*mut ALCdevice, String)> {
    for candidate in candidates {
        let candidate = candidate
            .map(CString::new)
            .transpose()
            .context("OpenAL device name contains a NUL byte")?;
        // SAFETY: the optional CString stays alive across `alcOpenDevice` and
        // OpenAL copies the device specifier before returning.
        let device = unsafe {
            api.alcOpenDevice(candidate.as_ref().map_or(ptr::null(), |name| name.as_ptr()))
        };
        if device.is_null() {
            continue;
        }
        if check_alc_error(api, device, "open device").is_ok() {
            return Ok((device, query_device_name(api, device)));
        }
        // SAFETY: the candidate device opened but reported an ALC error and no
        // context was created from it.
        unsafe { api.alcCloseDevice(device) };
    }
    bail!("failed to open preferred, system-default, or default OpenAL device")
}

fn query_system_default_device(api: &AlApi) -> Option<String> {
    if !alc_extension_present(api, ptr::null_mut(), ALC_ENUMERATE_ALL_EXT) {
        return None;
    }
    query_alc_string(api, ptr::null_mut(), ALC_DEFAULT_ALL_DEVICES_SPECIFIER)
}

fn query_device_name(api: &AlApi, device: *mut ALCdevice) -> String {
    query_alc_string(api, device, ALC_ALL_DEVICES_SPECIFIER)
        .or_else(|| query_alc_string(api, device, al_sys::ALC_DEVICE_SPECIFIER))
        .unwrap_or_else(|| format!("Unknown ({device:p})"))
}

fn query_alc_string(api: &AlApi, device: *mut ALCdevice, parameter: i32) -> Option<String> {
    // SAFETY: OpenAL owns the returned NUL-terminated string for the lifetime
    // of the queried device (or process for a null-device query).
    let value = unsafe { api.alcGetString(device, parameter) };
    (!value.is_null()).then(|| {
        // SAFETY: the non-null OpenAL string is NUL terminated by contract.
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    })
}

fn require_openal_1_1(api: &AlApi, device: *mut ALCdevice) -> Result<()> {
    let major = alc_integer(api, device, al_sys::ALC_MAJOR_VERSION);
    let minor = alc_integer(api, device, al_sys::ALC_MINOR_VERSION);
    check_alc_error(api, device, "query OpenAL version")?;
    if major != 1 || minor < 1 {
        bail!("OpenAL 1.1 is required, device reports {major}.{minor}");
    }
    Ok(())
}

fn query_mono_sources(api: &AlApi, device: *mut ALCdevice) -> Result<usize> {
    let size = alc_integer(api, device, ALC_ALL_ATTRIBUTES_SIZE);
    check_alc_error(api, device, "get attributes size")?;
    let size = usize::try_from(size).context("negative OpenAL attribute count")?;
    if size > MAX_DEVICE_ATTRIBUTES {
        bail!("OpenAL attribute count {size} exceeds {MAX_DEVICE_ATTRIBUTES}");
    }
    let mut attributes = vec![0_i32; size];
    if size > 0 {
        // SAFETY: the allocation contains exactly `size` writable ALCint slots.
        unsafe {
            api.alcGetIntegerv(
                device,
                ALC_ALL_ATTRIBUTES,
                i32::try_from(size).context("OpenAL attribute count overflow")?,
                attributes.as_mut_ptr(),
            )
        };
        check_alc_error(api, device, "get attributes")?;
    }
    Ok(mono_sources_from_attributes(&attributes).unwrap_or(DEFAULT_MONO_SOURCES))
}

fn mono_sources_from_attributes(attributes: &[i32]) -> Option<usize> {
    let mut pairs = attributes.chunks_exact(2);
    for pair in &mut pairs {
        if pair[0] == 0 {
            return None;
        }
        if pair[0] == ALC_MONO_SOURCES {
            return usize::try_from(pair[1]).ok();
        }
    }
    None
}

fn alc_integer(api: &AlApi, device: *mut ALCdevice, parameter: i32) -> i32 {
    let mut value = 0;
    // SAFETY: `value` provides one writable ALCint and the device pointer is
    // either null for a permitted global query or owned by the caller.
    unsafe { api.alcGetIntegerv(device, parameter, 1, &mut value) };
    value
}

fn alc_extension_present(api: &AlApi, device: *mut ALCdevice, extension: &CStr) -> bool {
    // SAFETY: `extension` is NUL terminated and `device` is either null for a
    // global extension or a live OpenAL device.
    unsafe { api.alcIsExtensionPresent(device, extension.as_ptr()) != al_sys::ALC_FALSE }
}

fn require_al_extension(api: &AlApi, extension: &CStr) -> Result<()> {
    // SAFETY: a context is current while this helper is called and the name is
    // a NUL-terminated extension identifier.
    if unsafe { api.alIsExtensionPresent(extension.as_ptr()) } == al_sys::AL_FALSE {
        bail!(
            "required OpenAL extension {} is unavailable",
            extension.to_string_lossy()
        );
    }
    Ok(())
}

fn check_alc_error(api: &AlApi, device: *mut ALCdevice, operation: &str) -> Result<()> {
    // SAFETY: querying and clearing the error flag is valid for a live device;
    // the initialization path never passes a dangling pointer.
    let error = unsafe { api.alcGetError(device) };
    if error == al_sys::ALC_NO_ERROR {
        Ok(())
    } else {
        Err(anyhow!("OpenAL ALC error 0x{error:04x} during {operation}"))
    }
}

fn check_al_error(api: &AlApi, operation: &str) -> Result<()> {
    // SAFETY: an OpenAL context is current while initialization calls this.
    let error = unsafe { api.alGetError() };
    if error == al_sys::AL_NO_ERROR {
        Ok(())
    } else {
        Err(anyhow!("OpenAL AL error 0x{error:04x} during {operation}"))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        context_attributes, device_open_plan, mono_sources_from_attributes, query_device_snapshot,
        OpenAlBackend, OpenAlDeviceContext, OpenAlDeviceEnumerator, OpenAlSourceSettings,
        OpenAlStaticSound, ALC_EVENT_TYPE_DEVICE_ADDED_SOFT, ALC_PLAYBACK_DEVICE_SOFT,
        AL_BUFFERS_QUEUED, AL_GAIN, AL_LINEAR_DISTANCE, AL_LOOPING, AL_MAX_DISTANCE,
        AL_ORIENTATION, AL_PITCH, AL_POSITION, AL_REFERENCE_DISTANCE, AL_ROLLOFF_FACTOR,
        AL_SOURCE_DISTANCE_MODEL, AL_SOURCE_RELATIVE, OPENAL_RECEIPT_LOCK,
    };
    use crate::{residency::AudioResidency, ResolvedSound};

    // OpenAL Soft exposes process-global driver state while contexts are
    // created and destroyed. Keep executable receipts deterministic even
    // when Rust's test harness runs ignored tests on separate threads.
    #[test]
    fn context_attributes_match_vanilla_supported_hrtf_fallback() {
        assert_eq!(context_attributes(0, true), [6554, 1, 0]);
        assert_eq!(context_attributes(1, false), [6546, 0, 6550, 0, 6554, 1, 0]);
        assert_eq!(context_attributes(2, true), [6546, 1, 6550, 0, 6554, 1, 0]);
    }

    #[test]
    fn device_open_plan_preserves_preferred_default_null_order_and_retries() {
        assert_eq!(
            device_open_plan(Some("headset"), Some("speakers")),
            [
                Some("headset".to_owned()),
                Some("speakers".to_owned()),
                None,
            ]
        );
        assert_eq!(
            device_open_plan(Some("same"), Some("same")),
            [Some("same".to_owned()), Some("same".to_owned()), None]
        );
        assert_eq!(device_open_plan(Some(""), None), [None]);
    }

    #[test]
    fn mono_source_query_uses_exact_attribute_and_vanilla_default() {
        assert_eq!(
            mono_sources_from_attributes(&[4103, 48_000, 4112, 256, 0]),
            Some(256)
        );
        assert_eq!(mono_sources_from_attributes(&[4103, 48_000, 0]), None);
        assert_eq!(mono_sources_from_attributes(&[4112, -1, 0]), None);
    }

    #[test]
    fn system_event_callback_requests_only_playback_updates() {
        let requested = std::sync::atomic::AtomicBool::new(false);
        let user_param = (&requested as *const std::sync::atomic::AtomicBool)
            .cast_mut()
            .cast::<std::ffi::c_void>();
        // SAFETY: the callback receives the exact live AtomicBool pointer it expects.
        unsafe {
            super::openal_system_event_callback(
                ALC_EVENT_TYPE_DEVICE_ADDED_SOFT,
                6613,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                user_param,
            );
        }
        assert!(!requested.load(std::sync::atomic::Ordering::Acquire));
        // SAFETY: same live AtomicBool pointer, now with Vanilla's playback type.
        unsafe {
            super::openal_system_event_callback(
                ALC_EVENT_TYPE_DEVICE_ADDED_SOFT,
                ALC_PLAYBACK_DEVICE_SOFT,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                user_param,
            );
        }
        assert!(requested.load(std::sync::atomic::Ordering::Acquire));
    }

    #[test]
    #[ignore = "requires BBB_TEST_OPENAL_LIBRARY and an explicitly selected OpenAL driver"]
    fn verified_openal_library_executes_both_hrtf_context_branches() {
        let _receipt = OPENAL_RECEIPT_LOCK.lock().unwrap();
        let library = std::env::var_os("BBB_TEST_OPENAL_LIBRARY")
            .expect("BBB_TEST_OPENAL_LIBRARY must name the verified OpenAL library");
        let devices = query_device_snapshot(std::path::Path::new(&library)).unwrap();
        assert!(devices.all_devices.iter().all(|name| !name.is_empty()));
        let disabled = OpenAlDeviceContext::open(&library, None, false).unwrap();
        assert!(!disabled.observation().requested_hrtf);
        assert!(disabled.observation().mono_sources > 0);
        assert_eq!(
            disabled.observation().capacity_origin,
            "openal-alc-mono-sources"
        );
        drop(disabled);

        let enabled = OpenAlDeviceContext::open(&library, None, true).unwrap();
        let observation = enabled.observation();
        assert!(observation.requested_hrtf);
        assert!(observation.output_limiter);
        assert_eq!(
            observation.hrtf_status,
            i32::from(observation.hrtf_extension && observation.hrtf_specifiers > 0)
        );
    }

    #[test]
    #[ignore = "requires BBB_TEST_OPENAL_LIBRARY and an explicitly selected OpenAL driver"]
    fn verified_openal_device_tracker_executes_callback_or_polling_selection() {
        let _receipt = OPENAL_RECEIPT_LOCK.lock().unwrap();
        let library = std::env::var_os("BBB_TEST_OPENAL_LIBRARY")
            .expect("BBB_TEST_OPENAL_LIBRARY must name the verified OpenAL library");
        let enumerator = OpenAlDeviceEnumerator::open(std::path::Path::new(&library)).unwrap();
        let _snapshot = enumerator.snapshot().unwrap();
        let system_events = enumerator.install_system_event_tracker();
        if let Some(events) = system_events.as_ref() {
            assert!(!events.take_update_request());
        }
        eprintln!(
            "openal-device-tracker-mode={}",
            if system_events.is_some() {
                "callback"
            } else {
                "polling"
            }
        );
    }

    #[test]
    #[ignore = "requires BBB_TEST_OPENAL_LIBRARY, BBB_TEST_OPENAL_OGG, and an explicit driver"]
    fn verified_openal_channel_executes_static_and_four_buffer_stream_owners() {
        let _receipt = OPENAL_RECEIPT_LOCK.lock().unwrap();
        let library = std::env::var_os("BBB_TEST_OPENAL_LIBRARY")
            .expect("BBB_TEST_OPENAL_LIBRARY must name the verified OpenAL library");
        let ogg = std::env::var_os("BBB_TEST_OPENAL_OGG")
            .expect("BBB_TEST_OPENAL_OGG must name a verified Vanilla sound");
        let mut backend = OpenAlBackend::open(&library, None, true).unwrap();
        backend
            .set_listener([0.0; 3], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0])
            .unwrap();
        assert_eq!(listener_values(&backend, AL_POSITION, 3), [0.0; 3]);
        assert_eq!(
            listener_values(&backend, AL_ORIENTATION, 6),
            [0.0, 0.0, -1.0, 0.0, 1.0, 0.0]
        );
        backend
            .set_listener([1.0, 2.0, 3.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0])
            .unwrap();
        assert_eq!(listener_values(&backend, AL_POSITION, 3), [1.0, 2.0, 3.0]);
        let settings = OpenAlSourceSettings {
            position: [4.0, 5.0, 6.0],
            pitch: 1.25,
            gain: 0.75,
            looping: true,
            relative: false,
            max_distance: Some(32.0),
        };
        let static_source = backend
            .play_static(
                7,
                std::path::Path::new("receipt.ogg"),
                || {
                    Ok(OpenAlStaticSound {
                        sample_rate: 48_000,
                        channels: 1,
                        samples: vec![0; 48_000],
                    })
                },
                settings,
            )
            .unwrap();
        assert_eq!(source_i(&static_source, AL_LOOPING), 1);
        assert_eq!(source_i(&static_source, AL_SOURCE_RELATIVE), 0);
        assert_eq!(
            source_i(&static_source, AL_SOURCE_DISTANCE_MODEL),
            AL_LINEAR_DISTANCE
        );
        assert_eq!(source_f(&static_source, AL_PITCH), 1.25);
        assert_eq!(source_f(&static_source, AL_GAIN), 0.75);
        assert_eq!(source_f(&static_source, AL_MAX_DISTANCE), 32.0);
        assert_eq!(source_f(&static_source, AL_REFERENCE_DISTANCE), 0.0);
        assert_eq!(source_f(&static_source, AL_ROLLOFF_FACTOR), 1.0);
        assert_eq!(backend.static_buffers.len(), 1);
        assert_eq!(backend.static_buffer_bytes, 96_000);
        drop(static_source);
        let mut cached_source = backend
            .play_static(
                7,
                std::path::Path::new("receipt.ogg"),
                || panic!("native static-buffer hit must not decode PCM again"),
                settings,
            )
            .unwrap();
        cached_source.stop();
        assert!(!cached_source.is_active());
        drop(cached_source);
        assert_eq!(backend.static_buffers.len(), 1);
        assert_eq!(backend.static_buffer_bytes, 96_000);
        backend.clear_static_buffers();
        assert!(backend.static_buffers.is_empty());
        assert_eq!(backend.static_buffer_bytes, 0);

        let sound = ResolvedSound {
            event_id: "minecraft:receipt".into(),
            sound_name: "minecraft:receipt".into(),
            ogg_path: ogg.into(),
            resource_generation: 7,
            stream: true,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        };
        let mut residency = AudioResidency::new(7);
        let decoder = residency.openal_streaming_sound(&sound, true).unwrap();
        let stream_source = backend
            .play_stream(
                decoder,
                OpenAlSourceSettings {
                    looping: false,
                    ..settings
                },
            )
            .unwrap();
        assert_eq!(source_i(&stream_source, AL_LOOPING), 0);
        assert_eq!(source_i(&stream_source, AL_BUFFERS_QUEUED), 4);
    }

    #[test]
    #[ignore = "requires BBB_TEST_OPENAL_LIBRARY and an explicitly selected OpenAL driver"]
    fn verified_openal_sources_restore_their_context_across_device_reload() {
        let _receipt = OPENAL_RECEIPT_LOCK.lock().unwrap();
        let library = std::env::var_os("BBB_TEST_OPENAL_LIBRARY")
            .expect("BBB_TEST_OPENAL_LIBRARY must name the verified OpenAL library");
        let mut old_backend = OpenAlBackend::open(&library, None, false).unwrap();
        let settings = OpenAlSourceSettings {
            position: [0.0; 3],
            pitch: 1.0,
            gain: 1.0,
            looping: true,
            relative: true,
            max_distance: None,
        };
        let mut old_source = old_backend
            .play_static(
                1,
                std::path::Path::new("reload.ogg"),
                || {
                    Ok(OpenAlStaticSound {
                        sample_rate: 48_000,
                        channels: 1,
                        samples: vec![0; 48_000],
                    })
                },
                settings,
            )
            .unwrap();
        let new_backend = OpenAlBackend::open(&library, None, true).unwrap();
        new_backend
            .set_listener([1.0; 3], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0])
            .unwrap();
        old_source.set_volume(0.5);
        assert_eq!(source_f(&old_source, AL_GAIN), 0.5);
        drop(old_source);
        drop(old_backend);
        new_backend
            .set_listener([2.0; 3], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0])
            .unwrap();
    }

    fn source_i(source: &super::OpenAlSoundSource, parameter: i32) -> i32 {
        let mut value = 0;
        // SAFETY: the receipt source is live and `value` is writable.
        unsafe {
            source
                .api
                .alGetSourcei(source.source, parameter, &mut value)
        };
        value
    }

    fn source_f(source: &super::OpenAlSoundSource, parameter: i32) -> f32 {
        let mut value = 0.0;
        // SAFETY: the receipt source is live and `value` is writable.
        unsafe {
            source
                .api
                .alGetSourcef(source.source, parameter, &mut value)
        };
        value
    }

    fn listener_values(backend: &OpenAlBackend, parameter: i32, count: usize) -> Vec<f32> {
        let mut values = vec![0.0; count];
        // SAFETY: the backend context is current and the output has the exact
        // OpenAL listener parameter width requested by the test.
        unsafe {
            backend
                .context
                .api
                .alGetListenerfv(parameter, values.as_mut_ptr())
        };
        values
    }
}
