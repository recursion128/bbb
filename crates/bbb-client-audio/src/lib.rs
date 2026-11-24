mod command;
#[cfg(any(
    all(not(target_arch = "wasm32"), any(feature = "device", test)),
    all(feature = "web", target_arch = "wasm32")
))]
mod scheduler;

pub const RECORDING_SAMPLE_RATE: u32 = 48_000;
pub const RECORDING_CHANNELS: u16 = 2;

#[cfg(all(not(target_arch = "wasm32"), any(feature = "device", test)))]
mod decode_worker;
#[cfg(all(not(target_arch = "wasm32"), any(feature = "device", test)))]
mod preload;

#[cfg(feature = "device")]
mod device;
#[cfg(feature = "device")]
mod openal;

#[cfg(all(not(target_arch = "wasm32"), any(feature = "device", test)))]
mod residency;
#[cfg(all(not(target_arch = "wasm32"), any(feature = "device", test)))]
mod runtime;
#[cfg(any(all(feature = "web", target_arch = "wasm32"), test))]
mod software_ogg;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
mod webaudio;

pub use command::{
    is_sound_device_default, AmbientLoopSoundCommand, AudioCategory, AudioChannelObservation,
    AudioCommand, AudioDeviceObservation, AudioDevicePoll, AudioDeviceReloadReason,
    AudioDeviceSnapshot, AudioFileProbe, AudioListenerState, AudioPlaybackEvent,
    AudioPreloadObservation, AudioPreloadRequest, AudioResidencyObservation, AudioVolumeSettings,
    EntitySoundPosition, PlayEntitySoundCommand, PlayJukeboxSongCommand, PlayLocalSoundCommand,
    PlayPositionedSoundCommand, ResolvedSound, SetMusicGainCommand, StopJukeboxSongCommand,
    StopSoundCommand, SyncAmbientLoopsCommand, TickEntitySoundPositionsCommand,
    TickableEntitySoundUpdate,
};

#[cfg(feature = "device")]
pub use device::audio_device_reload_reason;
#[cfg(feature = "device")]
pub use openal::{OpenAlContextObservation, OpenAlDeviceContext};
#[cfg(all(not(target_arch = "wasm32"), any(feature = "device", test)))]
pub use residency::probe_bounded_audio_file;
#[cfg(all(not(target_arch = "wasm32"), any(feature = "device", test)))]
pub use runtime::NativeAudioRuntime;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use webaudio::WebAudioRuntime;
