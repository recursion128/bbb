mod recording;

use std::{
    collections::HashMap,
    ffi::{c_char, c_void, CStr, CString},
    path::{Path, PathBuf},
    ptr,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, AtomicPtr, Ordering},
        Arc,
    },
};

use anyhow::{anyhow, bail, Context, Result};
use libloading::Library;

#[cfg(test)]
use std::sync::Mutex;

use crate::decode_worker::{AsyncOpenAlStreamHandle, AudioDecodeWorker, StreamChunkPop};
use crate::residency::{
    DecodedStaticAudio, OpenAlStaticSound, OpenAlStreamDecoder, MAX_STATIC_SOUND_CACHE_BUFFERS,
    MAX_STATIC_SOUND_CACHE_BYTES,
};
use crate::scheduler::{channel_playback_rate, normalized_gain, MIN_SPATIAL_DISTANCE};
use crate::AudioDeviceSnapshot;

fn sanitize_finite_vec3(v: [f32; 3], default: [f32; 3]) -> [f32; 3] {
    [
        if v[0].is_finite() { v[0] } else { default[0] },
        if v[1].is_finite() { v[1] } else { default[1] },
        if v[2].is_finite() { v[2] } else { default[2] },
    ]
}

fn sanitize_max_distance(max_distance: f32) -> f32 {
    if max_distance.is_finite() {
        max_distance.max(MIN_SPATIAL_DISTANCE + f32::EPSILON)
    } else {
        MIN_SPATIAL_DISTANCE + f32::EPSILON
    }
}

type ALboolean = i8;
type ALCboolean = i8;

#[repr(C)]
pub(crate) struct ALCdevice {
    _private: [u8; 0],
}

#[repr(C)]
pub(crate) struct ALCcontext {
    _private: [u8; 0],
}

const AL_FALSE: ALboolean = 0;
const AL_NO_ERROR: i32 = 0;
const ALC_FALSE: ALCboolean = 0;
const ALC_TRUE: ALCboolean = 1;
const ALC_NO_ERROR: i32 = 0;
const ALC_MAJOR_VERSION: i32 = 0x1000;
const ALC_MINOR_VERSION: i32 = 0x1001;
const ALC_DEVICE_SPECIFIER: i32 = 0x1005;
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
const OPENAL_STREAM_BUFFER_COUNT: usize = 4;
const MAX_DEVICE_ATTRIBUTES: usize = 4096;
const OPENAL_CAPACITY_ORIGIN: &str = "openal-alc-mono-sources";
const MAX_DEVICE_COUNT: usize = 256;
const MAX_DEVICE_NAME_BYTES: usize = 4096;

const ALC_ENUMERATE_ALL_EXT: &CStr = c"ALC_ENUMERATE_ALL_EXT";
const ALC_SOFT_HRTF: &CStr = c"ALC_SOFT_HRTF";
const ALC_SOFT_OUTPUT_LIMITER: &CStr = c"ALC_SOFT_output_limiter";
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

macro_rules! define_openal_api {
    ($( $name:ident ( $( $arg:ident : $ty:ty ),* $(,)? ) $( -> $ret:ty )? );* $(;)?) => {
        #[allow(non_snake_case)]
        struct AlApi {
            // Keep the shared library loaded for the lifetime of every resolved
            // OpenAL function pointer held in this struct.
            _library: Library,
            $( $name: unsafe extern "C" fn($( $ty ),*) $( -> $ret )?, )*
        }

        #[allow(non_snake_case)]
        impl AlApi {
            fn load(path: &Path) -> Result<Self> {
                // SAFETY: loading the verified OpenAL 1.1 shared library from
                // disk; `_library` stays owned by `Self` so all copied symbol
                // pointers remain valid while `Self` is alive.
                let library = unsafe { Library::new(path) }
                    .with_context(|| format!("open dynamic library {}", path.display()))?;
                $(
                    let $name: unsafe extern "C" fn($( $ty ),*) $( -> $ret )? = {
                        let symbol_name = concat!(stringify!($name), "\0");
                        // SAFETY: each symbol name and signature matches the
                        // OpenAL 1.1 C ABI, and `_library` outlives the copied
                        // raw function pointer stored in `Self`.
                        let symbol = unsafe {
                            library.get::<unsafe extern "C" fn($( $ty ),*) $( -> $ret )?>(
                                symbol_name.as_bytes(),
                            )
                        }
                        .with_context(|| {
                            format!(
                                "resolve OpenAL symbol {} from {}",
                                stringify!($name),
                                path.display()
                            )
                        })?;
                        *symbol
                    };
                )*
                Ok(Self {
                    _library: library,
                    $( $name, )*
                })
            }

            $(
                #[inline]
                unsafe fn $name(&self, $( $arg: $ty ),*) $( -> $ret )? {
                    // SAFETY: caller upholds OpenAL 1.1 C ABI preconditions;
                    // `_library` guarantees the code address remains mapped.
                    unsafe { (self.$name)($( $arg ),*) }
                }
            )*
        }
    };
}

define_openal_api! {
    alcOpenDevice(devicename: *const c_char) -> *mut ALCdevice;
    alcCloseDevice(device: *mut ALCdevice) -> ALCboolean;
    alcCreateContext(device: *mut ALCdevice, attrlist: *const i32) -> *mut ALCcontext;
    alcMakeContextCurrent(context: *mut ALCcontext) -> ALCboolean;
    alcDestroyContext(context: *mut ALCcontext);
    alcGetCurrentContext() -> *mut ALCcontext;
    alcGetError(device: *mut ALCdevice) -> i32;
    alcIsExtensionPresent(device: *mut ALCdevice, extname: *const c_char) -> ALCboolean;
    alcGetProcAddress(device: *mut ALCdevice, funcname: *const c_char) -> *mut c_void;
    alcGetString(device: *mut ALCdevice, param: i32) -> *const c_char;
    alcGetIntegerv(device: *mut ALCdevice, param: i32, size: i32, values: *mut i32);
    alEnable(capability: i32);
    alGetError() -> i32;
    alIsExtensionPresent(extname: *const c_char) -> ALboolean;
    alListener3f(param: i32, value1: f32, value2: f32, value3: f32);
    alListenerfv(param: i32, values: *const f32);
    alGetListenerfv(param: i32, values: *mut f32);
    alGenSources(n: i32, sources: *mut u32);
    alDeleteSources(n: i32, sources: *const u32);
    alSourcef(source: u32, param: i32, value: f32);
    alSource3f(source: u32, param: i32, value1: f32, value2: f32, value3: f32);
    alSourcei(source: u32, param: i32, value: i32);
    alGetSourcef(source: u32, param: i32, value: *mut f32);
    alGetSourcefv(source: u32, param: i32, values: *mut f32);
    alGetSourcei(source: u32, param: i32, value: *mut i32);
    alSourcePlay(source: u32);
    alSourceStop(source: u32);
    alSourceQueueBuffers(source: u32, nb: i32, buffers: *const u32);
    alSourceUnqueueBuffers(source: u32, nb: i32, buffers: *mut u32);
    alGenBuffers(n: i32, buffers: *mut u32);
    alDeleteBuffers(n: i32, buffers: *const u32);
    alBufferData(buffer: u32, format: i32, data: *const c_void, size: i32, freq: i32);
}

#[cfg(test)]
pub(crate) static OPENAL_RECEIPT_LOCK: Mutex<()> = Mutex::new(());

/// Tracks the process-wide current OpenAL context (`alcMakeContextCurrent` is
/// process-global in OpenAL 1.1) so hot source/listener operations skip
/// redundant FFI switches when the required context is already current.
struct ContextCurrentTracker {
    current: AtomicPtr<ALCcontext>,
}

impl ContextCurrentTracker {
    const fn new() -> Self {
        Self {
            current: AtomicPtr::new(ptr::null_mut()),
        }
    }

    fn ensure_current(
        &self,
        context: *mut ALCcontext,
        activate: impl FnOnce(*mut ALCcontext) -> Result<()>,
    ) -> Result<()> {
        if !context.is_null() && self.current.load(Ordering::Acquire) == context {
            return Ok(());
        }
        activate(context)?;
        self.current.store(context, Ordering::Release);
        Ok(())
    }

    fn activate_new(
        &self,
        context: *mut ALCcontext,
        activate: impl FnOnce(*mut ALCcontext) -> Result<()>,
    ) -> Result<()> {
        activate(context)?;
        self.current.store(context, Ordering::Release);
        Ok(())
    }

    fn clear_if_current(&self, context: *mut ALCcontext) {
        let _ = self.current.compare_exchange(
            context,
            ptr::null_mut(),
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
}

static CURRENT_OPENAL_CONTEXT: ContextCurrentTracker = ContextCurrentTracker::new();

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
    message: *const c_char,
    user_param: *mut c_void,
);
type AlcEventIsSupportedSoft = unsafe extern "C" fn(event_type: i32, device_type: i32) -> i32;
type AlcEventControlSoft =
    unsafe extern "C" fn(count: i32, event_types: *const i32, enable: ALCboolean);
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
                ALC_TRUE,
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
                ALC_FALSE,
            );
        }
    }
}

unsafe extern "C" fn openal_system_event_callback(
    event_type: i32,
    device_type: i32,
    _device: *mut ALCdevice,
    _message_length: i32,
    _message: *const c_char,
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
            let soft = probe_soft_device_extensions(&api, device)?;
            let attributes = context_attributes(soft, use_hrtf);
            let attr_ptr = if attributes.len() > 1 {
                attributes.as_ptr()
            } else {
                ptr::null()
            };
            // SAFETY: `device` is an open device owned by this function and `attr_ptr`
            // is either null (standard OpenAL 1.1 default context) or a zero-terminated
            // attribute list live for the duration of the call.
            let context = unsafe { api.alcCreateContext(device, attr_ptr) };
            if context.is_null() {
                check_alc_error(&api, device, "create context")?;
                bail!("OpenAL returned a null context without an ALC error");
            }
            if let Err(error) = activate_new_context(&api, context) {
                // SAFETY: creation succeeded and the context has not been destroyed.
                unsafe { api.alcDestroyContext(context) };
                return Err(error);
            }

            match finish_context_initialization(
                &api,
                device,
                context,
                current_device,
                use_hrtf,
                soft,
            ) {
                Ok(value) => Ok(value),
                Err(error) => {
                    clear_current_context_if(&api, context);
                    // SAFETY: the context is detached and exclusively owned here.
                    unsafe { api.alcDestroyContext(context) };
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
        make_context_current(&self.api, self.context)
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

struct StaticBufferCache<T> {
    entries: HashMap<(u64, PathBuf), StaticBufferCacheEntry<T>>,
    bytes: u64,
    use_sequence: u64,
    evictions: u64,
    max_buffers: usize,
    max_bytes: u64,
}

struct StaticBufferCacheEntry<T> {
    buffer: Rc<T>,
    decoded_bytes: u64,
    last_used: u64,
}

impl<T> StaticBufferCache<T> {
    fn new() -> Self {
        Self::with_limits(MAX_STATIC_SOUND_CACHE_BUFFERS, MAX_STATIC_SOUND_CACHE_BYTES)
    }

    fn with_limits(max_buffers: usize, max_bytes: u64) -> Self {
        Self {
            entries: HashMap::new(),
            bytes: 0,
            use_sequence: 0,
            evictions: 0,
            max_buffers,
            max_bytes,
        }
    }

    fn contains(&self, key: &(u64, PathBuf)) -> bool {
        self.entries.contains_key(key)
    }

    fn get(&mut self, key: &(u64, PathBuf)) -> Option<Rc<T>> {
        let entry = self.entries.get_mut(key)?;
        self.use_sequence = self.use_sequence.wrapping_add(1);
        entry.last_used = self.use_sequence;
        Some(Rc::clone(&entry.buffer))
    }

    /// Evicts unpinned LRU entries until `required` bytes fit. Returns
    /// `Ok(true)` when the buffer can be admitted to the cache, or `Ok(false)`
    /// when every cached buffer is currently attached to a live source so the
    /// caller should fall back to an uncached source-owned buffer.
    fn ensure_capacity(&mut self, required: u64) -> Result<bool> {
        if required > self.max_bytes {
            bail!(
                "OpenAL static buffer requires {required} bytes, exceeding {}",
                self.max_bytes
            );
        }
        while self.entries.len() >= self.max_buffers
            || self
                .bytes
                .checked_add(required)
                .is_none_or(|bytes| bytes > self.max_bytes)
        {
            let Some(key) = self
                .entries
                .iter()
                .filter(|(_, entry)| Rc::strong_count(&entry.buffer) == 1)
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            else {
                return Ok(false);
            };
            let evicted = self
                .entries
                .remove(&key)
                .expect("selected OpenAL static buffer exists");
            self.bytes = self
                .bytes
                .checked_sub(evicted.decoded_bytes)
                .expect("OpenAL static cache accounting cannot underflow");
            self.evictions = self.evictions.saturating_add(1);
        }
        Ok(true)
    }

    fn insert(&mut self, key: (u64, PathBuf), buffer: Rc<T>, decoded_bytes: u64) -> Result<()> {
        self.use_sequence = self.use_sequence.wrapping_add(1);
        let use_sequence = self.use_sequence;
        self.bytes = self
            .bytes
            .checked_add(decoded_bytes)
            .context("OpenAL static cache byte count overflow")?;
        self.entries.insert(
            key,
            StaticBufferCacheEntry {
                buffer,
                decoded_bytes,
                last_used: use_sequence,
            },
        );
        Ok(())
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }

    fn observation(&self) -> (usize, u64, u64) {
        (self.entries.len(), self.bytes, self.evictions)
    }
}

/// Production playback owner for Vanilla's `Library`/`Channel` boundary.
/// Static native buffers are cached per immutable resource generation while
/// streaming sources own and rotate exactly four one-second OpenAL buffers.
pub(crate) struct OpenAlBackend {
    context: OpenAlDeviceContext,
    static_buffers: StaticBufferCache<OpenAlStaticBuffer>,
    render_samples: Option<recording::AlcRenderSamples>,
}

pub(crate) struct OpenAlSoundSource {
    api: Rc<AlApi>,
    context: *mut ALCcontext,
    source: u32,
    explicitly_stopped: bool,
    static_buffer: Option<Rc<OpenAlStaticBuffer>>,
    stream: Option<OpenAlStreamState>,
    recording: Option<Box<OpenAlSoundSource>>,
}

struct OpenAlStaticBuffer {
    api: Rc<AlApi>,
    context: *mut ALCcontext,
    buffer: u32,
    generation: u64,
    path: PathBuf,
    pcm: DecodedStaticAudio,
}

enum OpenAlStreamSource {
    Sync(OpenAlStreamDecoder),
    Async(AsyncOpenAlStreamHandle),
}

struct StreamBufferPool {
    owned_buffers: [u32; OPENAL_STREAM_BUFFER_COUNT],
    free_buffers: Vec<u32>,
    queued_buffers: Vec<(u32, u64)>,
    removed_frames: u64,
}

impl StreamBufferPool {
    fn new(owned_buffers: [u32; OPENAL_STREAM_BUFFER_COUNT]) -> Self {
        let mut free_buffers = Vec::with_capacity(OPENAL_STREAM_BUFFER_COUNT);
        free_buffers.extend(owned_buffers.iter().rev().copied());
        Self {
            owned_buffers,
            free_buffers,
            queued_buffers: Vec::with_capacity(OPENAL_STREAM_BUFFER_COUNT),
            removed_frames: 0,
        }
    }

    fn take_free(&mut self) -> Option<u32> {
        self.free_buffers.pop()
    }

    fn return_unused(&mut self, buffer: u32) {
        self.free_buffers.push(buffer);
    }

    fn mark_queued(&mut self, buffer: u32, frames: u64) {
        self.queued_buffers.push((buffer, frames));
    }

    fn recycle_processed(&mut self, removed: &[u32]) {
        let mut removed_frames = 0;
        self.queued_buffers.retain(|(buffer, frames)| {
            let keep = !removed.contains(buffer);
            if !keep {
                removed_frames += *frames;
            }
            keep
        });
        self.removed_frames += removed_frames;
        self.free_buffers.extend_from_slice(removed);
    }

    fn has_free(&self) -> bool {
        !self.free_buffers.is_empty()
    }
}

struct OpenAlStreamState {
    path: PathBuf,
    looping: bool,
    source: OpenAlStreamSource,
    buffers: StreamBufferPool,
    eof: bool,
}

impl OpenAlBackend {
    pub(crate) fn open(
        library_path: impl AsRef<Path>,
        preferred_device: Option<&str>,
        use_hrtf: bool,
    ) -> Result<Self> {
        Ok(Self {
            context: OpenAlDeviceContext::open(library_path, preferred_device, use_hrtf)?,
            static_buffers: StaticBufferCache::new(),
            render_samples: None,
        })
    }

    pub(crate) fn observation(&self) -> &OpenAlContextObservation {
        self.context.observation()
    }

    pub(crate) fn is_disconnected(&self) -> bool {
        self.context.is_disconnected()
    }

    pub(crate) fn static_cache_observation(&self) -> (usize, u64, u64) {
        self.static_buffers.observation()
    }

    pub(crate) fn has_static_buffer(&self, generation: u64, path: &Path) -> bool {
        self.static_buffers
            .contains(&(generation, path.to_path_buf()))
    }

    pub(crate) fn clear_static_buffers(&mut self) {
        self.static_buffers.clear();
    }

    pub(crate) fn set_listener(
        &self,
        position: [f32; 3],
        forward: [f32; 3],
        up: [f32; 3],
    ) -> Result<()> {
        self.context.make_current()?;
        let position = sanitize_finite_vec3(position, [0.0, 0.0, 0.0]);
        let forward = sanitize_finite_vec3(forward, [0.0, 0.0, -1.0]);
        let up = sanitize_finite_vec3(up, [0.0, 1.0, 0.0]);
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
        let buffer = if let Some(cached) = self.static_buffers.get(&key) {
            cached
        } else {
            let pcm = load_pcm()?;
            self.admit_static_pcm(
                key,
                generation,
                path,
                DecodedStaticAudio {
                    sample_rate: pcm.sample_rate,
                    channels: pcm.channels,
                    samples: pcm.samples,
                },
            )?
        };
        self.start_static_source(buffer, settings)
    }

    pub(crate) fn play_static_pcm(
        &mut self,
        generation: u64,
        path: &Path,
        pcm: DecodedStaticAudio,
        settings: OpenAlSourceSettings,
    ) -> Result<OpenAlSoundSource> {
        self.context.make_current()?;
        let key = (generation, path.to_path_buf());
        let buffer = if let Some(cached) = self.static_buffers.get(&key) {
            cached
        } else {
            self.admit_static_pcm(key, generation, path, pcm)?
        };
        self.start_static_source(buffer, settings)
    }

    fn admit_static_pcm(
        &mut self,
        key: (u64, PathBuf),
        generation: u64,
        path: &Path,
        pcm: DecodedStaticAudio,
    ) -> Result<Rc<OpenAlStaticBuffer>> {
        let decoded_bytes = u64::try_from(pcm.samples.len())
            .ok()
            .and_then(|count| count.checked_mul(std::mem::size_of::<i16>() as u64))
            .context("OpenAL static buffer size overflow")?;
        let cacheable = self.static_buffers.ensure_capacity(decoded_bytes)?;
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
            generation,
            path: path.to_owned(),
            pcm,
        });
        if cacheable {
            self.static_buffers
                .insert(key, Rc::clone(&buffer), decoded_bytes)?;
        }
        Ok(buffer)
    }

    fn start_static_source(
        &self,
        buffer: Rc<OpenAlStaticBuffer>,
        settings: OpenAlSourceSettings,
    ) -> Result<OpenAlSoundSource> {
        let mut source = OpenAlSoundSource::create(
            Rc::clone(&self.context.api),
            self.context.context,
            settings,
        )?;
        // SAFETY: both identifiers belong to this current context and the
        // buffer outlives the returned source.
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

    pub(crate) fn play_stream(
        &mut self,
        decoder: OpenAlStreamDecoder,
        settings: OpenAlSourceSettings,
    ) -> Result<OpenAlSoundSource> {
        let path = decoder.path().to_path_buf();
        let looping = decoder.looping();
        self.start_stream_source(path, looping, OpenAlStreamSource::Sync(decoder), settings)
    }

    pub(crate) fn play_stream_async(
        &mut self,
        stream: AsyncOpenAlStreamHandle,
        settings: OpenAlSourceSettings,
    ) -> Result<OpenAlSoundSource> {
        let path = stream.path().to_path_buf();
        let looping = stream.looping();
        self.start_stream_source(path, looping, OpenAlStreamSource::Async(stream), settings)
    }

    fn start_stream_source(
        &mut self,
        path: PathBuf,
        looping: bool,
        stream_source: OpenAlStreamSource,
        settings: OpenAlSourceSettings,
    ) -> Result<OpenAlSoundSource> {
        self.context.make_current()?;
        let mut source = OpenAlSoundSource::create(
            Rc::clone(&self.context.api),
            self.context.context,
            settings,
        )?;
        let mut owned_buffers = [0_u32; OPENAL_STREAM_BUFFER_COUNT];
        // SAFETY: the context is current and `owned_buffers` holds 4 writable IDs.
        unsafe {
            self.context.api.alGenBuffers(
                OPENAL_STREAM_BUFFER_COUNT as i32,
                owned_buffers.as_mut_ptr(),
            );
        }
        check_al_error(&self.context.api, "allocate stream buffers")?;
        source.stream = Some(OpenAlStreamState {
            path,
            looping,
            source: stream_source,
            buffers: StreamBufferPool::new(owned_buffers),
            eof: false,
        });
        source.pump_stream_buffers()?;
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
            explicitly_stopped: false,
            static_buffer: None,
            stream: None,
            recording: None,
        };
        created.apply_settings(settings)?;
        Ok(created)
    }

    fn apply_settings(&self, settings: OpenAlSourceSettings) -> Result<()> {
        make_context_current(&self.api, self.context)?;
        let position = sanitize_finite_vec3(settings.position, [0.0; 3]);
        let pitch = channel_playback_rate(settings.pitch);
        let gain = normalized_gain(settings.gain);
        // SAFETY: the source belongs to the current context and every enum is
        // the exact OpenAL owner used by Vanilla 26.1 `Channel`.
        unsafe {
            self.api.alSource3f(
                self.source,
                AL_POSITION,
                position[0],
                position[1],
                position[2],
            );
            self.api.alSourcef(self.source, AL_PITCH, pitch);
            self.api.alSourcef(self.source, AL_GAIN, gain);
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
                    self.api.alSourcef(
                        self.source,
                        AL_MAX_DISTANCE,
                        sanitize_max_distance(max_distance),
                    );
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
        self.explicitly_stopped = true;
        if let Some(recording) = self.recording.as_mut() {
            recording.stop();
        }
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
        if state != AL_STOPPED {
            return true;
        }
        !self.explicitly_stopped
            && self.stream.as_ref().is_some_and(|stream| {
                matches!(stream.source, OpenAlStreamSource::Async(_)) && !stream.eof
            })
    }

    pub(crate) fn set_volume(&mut self, gain: f32) {
        let gain = normalized_gain(gain);
        if let Some(recording) = self.recording.as_mut() {
            recording.set_volume(gain);
        }
        if make_context_current(&self.api, self.context).is_err() {
            return;
        }
        // SAFETY: source is live until this owner is dropped.
        unsafe { self.api.alSourcef(self.source, AL_GAIN, gain) };
        let _ = check_al_error(&self.api, "set source volume");
    }

    pub(crate) fn set_playback_rate(&mut self, pitch: f32) {
        let pitch = channel_playback_rate(pitch);
        if let Some(recording) = self.recording.as_mut() {
            recording.set_playback_rate(pitch);
        }
        if make_context_current(&self.api, self.context).is_err() {
            return;
        }
        // SAFETY: source is live until this owner is dropped.
        unsafe { self.api.alSourcef(self.source, AL_PITCH, pitch) };
        let _ = check_al_error(&self.api, "set source playback rate");
    }

    pub(crate) fn set_position(&mut self, position: [f32; 3]) {
        let position = sanitize_finite_vec3(position, [0.0; 3]);
        if let Some(recording) = self.recording.as_mut() {
            recording.set_position(position);
        }
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
        let _ = check_al_error(&self.api, "set source position");
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
        if processed > 0 {
            let count = usize::try_from(processed).context("negative processed buffer count")?;
            let mut removed = vec![0_u32; count];
            // SAFETY: `removed` contains exactly `processed` writable identifiers.
            unsafe {
                self.api
                    .alSourceUnqueueBuffers(self.source, processed, removed.as_mut_ptr());
            }
            check_al_error(&self.api, "unqueue processed stream buffers")?;
            if let Some(stream) = self.stream.as_mut() {
                stream.buffers.recycle_processed(&removed);
            }
        }
        let queued_any = self.pump_stream_buffers()?;
        if queued_any && !self.explicitly_stopped {
            let mut state = AL_STOPPED;
            // SAFETY: source is live and `state` supplies one writable integer.
            unsafe {
                self.api
                    .alGetSourcei(self.source, AL_SOURCE_STATE, &mut state)
            };
            if state == AL_STOPPED {
                self.play()?;
            }
        }
        Ok(())
    }

    fn pump_stream_buffers(&mut self) -> Result<bool> {
        let Some(stream) = self.stream.as_mut() else {
            return Ok(false);
        };
        let mut queued_any = false;
        while stream.buffers.has_free() && !stream.eof {
            let chunk = match &mut stream.source {
                OpenAlStreamSource::Sync(decoder) => {
                    let chunk = decoder.read_one_second()?;
                    if chunk.samples.is_empty() {
                        stream.eof = true;
                        break;
                    }
                    chunk
                }
                OpenAlStreamSource::Async(handle) => match handle.try_pop_chunk() {
                    StreamChunkPop::Chunk(chunk) => chunk,
                    StreamChunkPop::Empty => break,
                    StreamChunkPop::Eof => {
                        stream.eof = true;
                        break;
                    }
                    StreamChunkPop::Error(error) => {
                        stream.eof = true;
                        bail!("{error}");
                    }
                },
            };
            let buffer = stream
                .buffers
                .take_free()
                .expect("free stream buffer available");
            if let Err(error) = upload_pcm_into_buffer(
                &self.api,
                buffer,
                chunk.sample_rate,
                chunk.channels,
                &chunk.samples,
            ) {
                stream.buffers.return_unused(buffer);
                return Err(error);
            }
            // SAFETY: both identifiers belong to the current context.
            unsafe { self.api.alSourceQueueBuffers(self.source, 1, &buffer) };
            if let Err(error) = check_al_error(&self.api, "queue stream buffer") {
                stream.buffers.return_unused(buffer);
                return Err(error);
            }
            stream.buffers.mark_queued(
                buffer,
                (chunk.samples.len() / usize::from(chunk.channels)) as u64,
            );
            queued_any = true;
        }
        Ok(queued_any)
    }
}

impl Drop for OpenAlSoundSource {
    fn drop(&mut self) {
        if make_context_current(&self.api, self.context).is_err() {
            return;
        }
        // SAFETY: this owner deletes the source exactly once. Stopping a
        // streaming source makes its queued buffers processed so they can be
        // detached before deleting the source and all four owned buffers.
        unsafe { self.api.alSourceStop(self.source) };
        if let Some(stream) = self.stream.take() {
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
                }
            }
            // SAFETY: source is still live and exclusively owned.
            unsafe {
                self.api.alDeleteSources(1, &self.source);
                self.api.alDeleteBuffers(
                    OPENAL_STREAM_BUFFER_COUNT as i32,
                    stream.buffers.owned_buffers.as_ptr(),
                );
            }
        } else {
            // SAFETY: detach the static buffer before deleting the source.
            unsafe {
                self.api.alSourcei(self.source, AL_BUFFER, 0);
                self.api.alDeleteSources(1, &self.source);
            }
        }
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

fn activate_new_context(api: &AlApi, context: *mut ALCcontext) -> Result<()> {
    CURRENT_OPENAL_CONTEXT.activate_new(context, |ctx| {
        // SAFETY: `ctx` was just created on this API and stays live across the call.
        if unsafe { api.alcMakeContextCurrent(ctx) } == ALC_FALSE {
            bail!("make OpenAL context current");
        }
        Ok(())
    })
}

fn make_context_current(api: &AlApi, context: *mut ALCcontext) -> Result<()> {
    CURRENT_OPENAL_CONTEXT.ensure_current(context, |ctx| {
        // SAFETY: every caller retains the context owner while using the pointer.
        if unsafe { api.alcMakeContextCurrent(ctx) } == ALC_FALSE {
            bail!("make source OpenAL context current");
        }
        Ok(())
    })
}

fn clear_current_context_if(api: &AlApi, context: *mut ALCcontext) {
    // SAFETY: `context` has not yet been destroyed.
    unsafe {
        if api.alcGetCurrentContext() == context {
            api.alcMakeContextCurrent(ptr::null_mut());
        }
    }
    CURRENT_OPENAL_CONTEXT.clear_if_current(context);
}

fn upload_pcm_into_buffer(
    api: &AlApi,
    buffer: u32,
    sample_rate: u32,
    channels: u16,
    samples: &[i16],
) -> Result<()> {
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
    // SAFETY: `buffer` is an unqueued identifier in the current context and
    // `samples` stays alive across the synchronous data upload.
    unsafe {
        api.alBufferData(
            buffer,
            format,
            samples.as_ptr().cast(),
            byte_count,
            sample_rate,
        );
    }
    check_al_error(api, "upload PCM buffer")
}

fn upload_pcm_buffer(api: &AlApi, sample_rate: u32, channels: u16, samples: &[i16]) -> Result<u32> {
    let mut buffer = 0_u32;
    // SAFETY: `buffer` is one writable identifier in the current context.
    unsafe {
        api.alGenBuffers(1, &mut buffer);
    }
    check_al_error(api, "allocate PCM buffer")?;
    if let Err(error) = upload_pcm_into_buffer(api, buffer, sample_rate, channels, samples) {
        // SAFETY: generation succeeded before the failed upload.
        unsafe { api.alDeleteBuffers(1, &buffer) };
        return Err(error);
    }
    Ok(buffer)
}

impl Drop for OpenAlDeviceContext {
    fn drop(&mut self) {
        clear_current_context_if(&self.api, self.context);
        // SAFETY: this object exclusively owns the context and device, and
        // fields are destroyed only after the matching OpenAL teardown calls.
        unsafe {
            self.api.alcDestroyContext(self.context);
            self.api.alcCloseDevice(self.device);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SoftDeviceExtensions {
    hrtf_extension: bool,
    hrtf_specifiers: i32,
    output_limiter_extension: bool,
}

fn probe_soft_device_extensions(
    api: &AlApi,
    device: *mut ALCdevice,
) -> Result<SoftDeviceExtensions> {
    probe_soft_device_extensions_with(
        |ext| alc_extension_present(api, device, ext),
        |param, op| {
            let value = alc_integer(api, device, param);
            check_alc_error(api, device, op)?;
            Ok(value)
        },
    )
}

fn probe_soft_device_extensions_with<E, I>(
    mut extension_present: E,
    mut query_int: I,
) -> Result<SoftDeviceExtensions>
where
    E: FnMut(&CStr) -> bool,
    I: FnMut(i32, &str) -> Result<i32>,
{
    let hrtf_extension = extension_present(ALC_SOFT_HRTF);
    let output_limiter_extension = extension_present(ALC_SOFT_OUTPUT_LIMITER);
    let hrtf_specifiers = if hrtf_extension {
        query_int(ALC_NUM_HRTF_SPECIFIERS_SOFT, "query HRTF specifiers")?
    } else {
        0
    };
    Ok(SoftDeviceExtensions {
        hrtf_extension,
        hrtf_specifiers,
        output_limiter_extension,
    })
}

fn query_soft_context_status(
    api: &AlApi,
    device: *mut ALCdevice,
    soft: SoftDeviceExtensions,
) -> Result<(i32, bool)> {
    query_soft_context_status_with(soft, |param, op| {
        let value = alc_integer(api, device, param);
        check_alc_error(api, device, op)?;
        Ok(value)
    })
}

fn query_soft_context_status_with<I>(
    soft: SoftDeviceExtensions,
    mut query_int: I,
) -> Result<(i32, bool)>
where
    I: FnMut(i32, &str) -> Result<i32>,
{
    let hrtf_status = if soft.hrtf_extension {
        query_int(ALC_HRTF_STATUS_SOFT, "query HRTF status")?
    } else {
        0
    };
    let output_limiter = if soft.output_limiter_extension {
        query_int(ALC_OUTPUT_LIMITER_SOFT, "query output limiter")? != 0
    } else {
        false
    };
    Ok((hrtf_status, output_limiter))
}

fn finish_context_initialization(
    api: &AlApi,
    device: *mut ALCdevice,
    context: *mut ALCcontext,
    current_device: String,
    requested_hrtf: bool,
    soft: SoftDeviceExtensions,
) -> Result<(*mut ALCcontext, OpenAlContextObservation)> {
    let mono_sources = query_mono_sources(api, device)?;
    require_al_extension(api, AL_EXT_SOURCE_DISTANCE_MODEL)?;
    // SAFETY: a current context exists and `AL_SOURCE_DISTANCE_MODEL` is
    // supported by the extension checked immediately above.
    unsafe { api.alEnable(AL_SOURCE_DISTANCE_MODEL_CAPABILITY) };
    require_al_extension(api, AL_EXT_LINEAR_DISTANCE)?;
    check_al_error(api, "enable per-source distance models")?;

    let (hrtf_status, output_limiter) = query_soft_context_status(api, device, soft)?;
    Ok((
        context,
        OpenAlContextObservation {
            current_device,
            requested_hrtf,
            hrtf_extension: soft.hrtf_extension,
            hrtf_specifiers: soft.hrtf_specifiers,
            hrtf_status,
            output_limiter,
            mono_sources,
            capacity_origin: OPENAL_CAPACITY_ORIGIN,
            supports_disconnection: alc_extension_present(api, device, ALC_EXT_DISCONNECT),
        },
    ))
}

/// Mirrors Vanilla `Library.createAttributes` when OpenAL-Soft extensions are
/// available, and omits `ALC_SOFT_HRTF` / `ALC_SOFT_output_limiter` attributes
/// on standard OpenAL 1.1 drivers that do not advertise them.
fn context_attributes(soft: SoftDeviceExtensions, requested_hrtf: bool) -> Vec<i32> {
    let mut attributes = Vec::with_capacity(7);
    if soft.hrtf_extension && soft.hrtf_specifiers > 0 {
        attributes.extend([ALC_HRTF_SOFT, i32::from(requested_hrtf)]);
        attributes.extend([ALC_HRTF_ID_SOFT, 0]);
    }
    if soft.output_limiter_extension {
        attributes.extend([ALC_OUTPUT_LIMITER_SOFT, 1]);
    }
    attributes.push(0);
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
        .or_else(|| query_alc_string(api, device, ALC_DEVICE_SPECIFIER))
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
    let major = alc_integer(api, device, ALC_MAJOR_VERSION);
    let minor = alc_integer(api, device, ALC_MINOR_VERSION);
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
    for pair in attributes.as_chunks::<2>().0 {
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
    unsafe { api.alcIsExtensionPresent(device, extension.as_ptr()) != ALC_FALSE }
}

fn require_al_extension(api: &AlApi, extension: &CStr) -> Result<()> {
    // SAFETY: a context is current while this helper is called and the name is
    // a NUL-terminated extension identifier.
    if unsafe { api.alIsExtensionPresent(extension.as_ptr()) } == AL_FALSE {
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
    if error == ALC_NO_ERROR {
        Ok(())
    } else {
        Err(anyhow!("OpenAL ALC error 0x{error:04x} during {operation}"))
    }
}

fn check_al_error(api: &AlApi, operation: &str) -> Result<()> {
    // SAFETY: an OpenAL context is current while initialization calls this.
    let error = unsafe { api.alGetError() };
    if error == AL_NO_ERROR {
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
    fn al_api_load_rejects_missing_library() {
        let missing = std::env::temp_dir().join("bbb-nonexistent-openal-library.so");
        let err = super::AlApi::load(&missing)
            .err()
            .expect("missing library should fail to load");
        assert!(
            err.to_string().contains("open dynamic library"),
            "unexpected error: {err:#}"
        );
    }

    #[test]
    fn context_attributes_match_vanilla_supported_hrtf_fallback() {
        let soft = |hrtf_extension, hrtf_specifiers, output_limiter_extension| {
            super::SoftDeviceExtensions {
                hrtf_extension,
                hrtf_specifiers,
                output_limiter_extension,
            }
        };
        // Full OpenAL-Soft support matches Vanilla `Library.createAttributes`.
        assert_eq!(context_attributes(soft(true, 0, true), true), [6554, 1, 0]);
        assert_eq!(
            context_attributes(soft(true, 1, true), false),
            [6546, 0, 6550, 0, 6554, 1, 0]
        );
        assert_eq!(
            context_attributes(soft(true, 2, true), true),
            [6546, 1, 6550, 0, 6554, 1, 0]
        );

        // Standard OpenAL 1.1 drivers without OpenAL-Soft extensions omit those pairs.
        assert_eq!(context_attributes(soft(false, 0, false), true), [0]);
        assert_eq!(context_attributes(soft(false, 2, false), true), [0]);
        assert_eq!(
            context_attributes(soft(true, 2, false), true),
            [6546, 1, 6550, 0, 0]
        );
        assert_eq!(context_attributes(soft(false, 0, true), true), [6554, 1, 0]);
    }

    #[test]
    fn soft_extension_probe_and_status_skip_absent_enums_cleanly() {
        use std::ffi::CStr;

        // 1. Standard OpenAL 1.1 driver (e.g., Apple OpenAL): neither extension advertised,
        //    and querying any SOFT enum fails with ALC_INVALID_ENUM.
        let mut queried_none = Vec::new();
        let none_soft = super::probe_soft_device_extensions_with(
            |_| false,
            |param, _| {
                queried_none.push(param);
                anyhow::bail!("ALC_INVALID_ENUM for 0x{param:04x}");
            },
        )
        .expect("standard OpenAL 1.1 probe must not query absent SOFT enums");
        assert_eq!(
            none_soft,
            super::SoftDeviceExtensions {
                hrtf_extension: false,
                hrtf_specifiers: 0,
                output_limiter_extension: false,
            }
        );
        assert_eq!(context_attributes(none_soft, true), [0]);
        let (hrtf_status, output_limiter) =
            super::query_soft_context_status_with(none_soft, |param, _| {
                queried_none.push(param);
                anyhow::bail!("ALC_INVALID_ENUM for 0x{param:04x}");
            })
            .expect("standard OpenAL 1.1 status query must fall back without querying SOFT enums");
        assert_eq!((hrtf_status, output_limiter), (0, false));
        assert!(queried_none.is_empty());

        // 2. Partial support: only ALC_SOFT_HRTF advertised (no ALC_SOFT_output_limiter).
        let mut queried_hrtf = Vec::new();
        let hrtf_only = super::probe_soft_device_extensions_with(
            |ext: &CStr| ext == super::ALC_SOFT_HRTF,
            |param, _| {
                queried_hrtf.push(param);
                assert_eq!(param, super::ALC_NUM_HRTF_SPECIFIERS_SOFT);
                Ok(2)
            },
        )
        .unwrap();
        assert_eq!(hrtf_only.hrtf_specifiers, 2);
        assert!(!hrtf_only.output_limiter_extension);
        assert_eq!(context_attributes(hrtf_only, true), [6546, 1, 6550, 0, 0]);
        let (status_hrtf, limiter_hrtf) =
            super::query_soft_context_status_with(hrtf_only, |param, _| {
                queried_hrtf.push(param);
                assert_eq!(param, super::ALC_HRTF_STATUS_SOFT);
                Ok(1)
            })
            .unwrap();
        assert_eq!((status_hrtf, limiter_hrtf), (1, false));
        assert_eq!(
            queried_hrtf,
            [
                super::ALC_NUM_HRTF_SPECIFIERS_SOFT,
                super::ALC_HRTF_STATUS_SOFT
            ]
        );

        // 3. Partial support: only ALC_SOFT_output_limiter advertised (no ALC_SOFT_HRTF).
        let mut queried_limiter = Vec::new();
        let limiter_only = super::probe_soft_device_extensions_with(
            |ext: &CStr| ext == super::ALC_SOFT_OUTPUT_LIMITER,
            |param, _| {
                queried_limiter.push(param);
                anyhow::bail!("unexpected HRTF specifier query 0x{param:04x}");
            },
        )
        .unwrap();
        assert!(!limiter_only.hrtf_extension);
        assert_eq!(limiter_only.hrtf_specifiers, 0);
        assert!(limiter_only.output_limiter_extension);
        assert_eq!(context_attributes(limiter_only, true), [6554, 1, 0]);
        let (status_lim, limiter_lim) =
            super::query_soft_context_status_with(limiter_only, |param, _| {
                queried_limiter.push(param);
                assert_eq!(param, super::ALC_OUTPUT_LIMITER_SOFT);
                Ok(1)
            })
            .unwrap();
        assert_eq!((status_lim, limiter_lim), (0, true));
        assert_eq!(queried_limiter, [super::ALC_OUTPUT_LIMITER_SOFT]);
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
    fn context_current_tracker_skips_redundant_switches_and_clears_on_destroy() {
        use std::cell::Cell;

        let tracker = super::ContextCurrentTracker::new();
        let ctx_a = 0x1000usize as *mut super::ALCcontext;
        let ctx_b = 0x2000usize as *mut super::ALCcontext;
        let calls = Cell::new(0_usize);
        let activate = |_ctx: *mut super::ALCcontext| {
            calls.set(calls.get() + 1);
            Ok(())
        };

        tracker.activate_new(ctx_a, activate).unwrap();
        assert_eq!(calls.get(), 1);
        // Repeated ensure_current for the already-active context must skip FFI.
        tracker.ensure_current(ctx_a, activate).unwrap();
        tracker.ensure_current(ctx_a, activate).unwrap();
        assert_eq!(calls.get(), 1);

        // Switching to a different context invokes FFI once.
        tracker.ensure_current(ctx_b, activate).unwrap();
        assert_eq!(calls.get(), 2);
        tracker.ensure_current(ctx_b, activate).unwrap();
        assert_eq!(calls.get(), 2);

        // Clearing an inactive context preserves the currently active context.
        tracker.clear_if_current(ctx_a);
        tracker.ensure_current(ctx_b, activate).unwrap();
        assert_eq!(calls.get(), 2);

        // Clearing the active context forces the next call (even at the same
        // pointer address after allocator reuse) to invoke FFI again.
        tracker.clear_if_current(ctx_b);
        tracker.ensure_current(ctx_b, activate).unwrap();
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn static_buffer_cache_falls_back_to_uncached_when_all_entries_are_pinned() {
        use std::path::PathBuf;
        use std::rc::Rc;

        let mut cache = super::StaticBufferCache::<u32>::with_limits(2, 100);
        let key_a = (1, PathBuf::from("a.ogg"));
        let key_b = (1, PathBuf::from("b.ogg"));
        let key_c = (1, PathBuf::from("c.ogg"));

        assert!(cache.ensure_capacity(60).unwrap());
        let buf_a = Rc::new(10_u32);
        cache.insert(key_a.clone(), Rc::clone(&buf_a), 60).unwrap();

        // Both entries fit in byte budget (60 + 40 = 100) and count budget (2).
        assert!(cache.ensure_capacity(40).unwrap());
        let buf_b = Rc::new(20_u32);
        cache.insert(key_b.clone(), Rc::clone(&buf_b), 40).unwrap();
        assert_eq!(cache.observation(), (2, 100, 0));

        // Both `buf_a` and `buf_b` are still pinned by live sources (`strong_count == 2`):
        // admitting `key_c` must NOT fail, and must fall back cleanly to uncached (`Ok(false)`),
        // keeping cache entries and byte accounting intact.
        assert!(!cache.ensure_capacity(30).unwrap());
        assert_eq!(cache.observation(), (2, 100, 0));
        assert!(cache.contains(&key_a));
        assert!(cache.contains(&key_b));
        assert!(!cache.contains(&key_c));

        // Once `buf_a` finishes playing (drops to strong_count == 1), it can be evicted.
        drop(buf_a);
        assert!(cache.ensure_capacity(30).unwrap());
        let buf_c = Rc::new(30_u32);
        cache.insert(key_c.clone(), Rc::clone(&buf_c), 30).unwrap();
        assert_eq!(cache.observation(), (2, 70, 1));
        assert!(!cache.contains(&key_a));
        assert!(cache.contains(&key_b));
        assert!(cache.contains(&key_c));

        // Single buffer exceeding the entire cache byte limit still errors.
        assert!(cache.ensure_capacity(101).is_err());
        drop(buf_b);
        drop(buf_c);
    }

    #[test]
    fn stream_buffer_pool_recycles_exact_four_owned_buffers() {
        let owned = [11_u32, 22, 33, 44];
        let mut pool = super::StreamBufferPool::new(owned);

        for _ in 0..super::OPENAL_STREAM_BUFFER_COUNT {
            let id = pool.take_free().unwrap();
            assert!(owned.contains(&id));
            pool.mark_queued(id, 48_000);
        }
        assert!(!pool.has_free());

        for cycle in 1..=12_u64 {
            let oldest = pool.queued_buffers[0].0;
            pool.recycle_processed(&[oldest]);
            assert_eq!(pool.removed_frames, cycle * 48_000);
            let recycled = pool.take_free().unwrap();
            assert_eq!(recycled, oldest);
            pool.mark_queued(recycled, 48_000);
            assert!(!pool.has_free());
        }
        assert_eq!(pool.owned_buffers, owned);
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
                        samples: vec![0; 48_000].into(),
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
        assert_eq!(backend.static_cache_observation(), (1, 96_000, 0));
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
        assert_eq!(backend.static_cache_observation(), (1, 96_000, 0));
        backend.clear_static_buffers();
        assert_eq!(backend.static_cache_observation(), (0, 0, 0));

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
                        samples: vec![0; 48_000].into(),
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
