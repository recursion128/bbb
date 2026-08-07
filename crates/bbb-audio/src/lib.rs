mod command;

#[cfg(feature = "kira")]
mod preload;

#[cfg(feature = "device")]
mod device;
#[cfg(feature = "device")]
mod openal;

#[cfg(feature = "kira")]
mod residency;
#[cfg(feature = "kira")]
mod runtime;

pub use command::{
    AmbientLoopSoundCommand, AudioCategory, AudioCommand, AudioListenerState, AudioVolumeSettings,
    EntitySoundPosition, PlayEntitySoundCommand, PlayJukeboxSongCommand, PlayLocalSoundCommand,
    PlayPositionedSoundCommand, ResolvedSound, SetMusicGainCommand, StopJukeboxSongCommand,
    StopSoundCommand, SyncAmbientLoopsCommand, TickEntitySoundPositionsCommand,
    TickableEntitySoundUpdate,
};

#[cfg(feature = "device")]
pub use device::{
    audio_device_reload_reason, is_sound_device_default, AudioDeviceObservation, AudioDevicePoll,
    AudioDeviceReloadReason, AudioDeviceSnapshot,
};
#[cfg(feature = "device")]
pub use openal::{OpenAlContextObservation, OpenAlDeviceContext};
#[cfg(feature = "kira")]
pub use preload::{AudioPreloadObservation, AudioPreloadRequest};
#[cfg(feature = "kira")]
pub use residency::{probe_bounded_audio_file, AudioFileProbe, AudioResidencyObservation};
#[cfg(feature = "kira")]
pub use runtime::{AudioChannelObservation, AudioPlaybackEvent, KiraAudioRuntime};
