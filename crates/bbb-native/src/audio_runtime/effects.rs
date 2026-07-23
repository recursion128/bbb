use bbb_audio::TickEntitySoundPositionsCommand;
use bbb_pack::{JukeboxSongRegistry, SoundEventRegistry};
use bbb_world::{
    AmbientLoopSoundState, JukeboxLevelEventState, LocalSoundEventState, SoundEntityEventState,
    SoundEventState, StopSoundEventState, TickableEntitySoundState,
};

use super::AudioEventSink;

/// One owned audio command inside a simulation render-effect batch.
pub(crate) enum AudioEffect {
    SetSoundEventRegistry(SoundEventRegistry),
    SetJukeboxSongRegistry(JukeboxSongRegistry),
    PlayLocal(LocalSoundEventState),
    PlayPositioned(SoundEventState),
    PlayEntity {
        state: SoundEntityEventState,
        position: Option<[f64; 3]>,
    },
    PlayJukebox(JukeboxLevelEventState),
    StopJukebox(JukeboxLevelEventState),
    Stop(StopSoundEventState),
    SetMusicGain(f32),
    SyncAmbientLoops(Vec<AmbientLoopSoundState>),
    TickEntityPositions {
        command: TickEntitySoundPositionsCommand,
        tickable_sounds: Vec<TickableEntitySoundState>,
    },
}

impl AudioEffect {
    pub(crate) fn replay(self, sink: &mut dyn AudioEventSink) {
        match self {
            Self::SetSoundEventRegistry(registry) => sink.set_sound_event_registry(registry),
            Self::SetJukeboxSongRegistry(registry) => sink.set_jukebox_song_registry(registry),
            Self::PlayLocal(state) => sink.play_local_sound(&state),
            Self::PlayPositioned(state) => sink.play_positioned_sound(&state),
            Self::PlayEntity { state, position } => sink.play_entity_sound(&state, position),
            Self::PlayJukebox(state) => sink.play_jukebox_song(&state),
            Self::StopJukebox(state) => sink.stop_jukebox_song(&state),
            Self::Stop(state) => sink.stop_sound(&state),
            Self::SetMusicGain(gain) => sink.set_music_gain(gain),
            Self::SyncAmbientLoops(loops) => sink.sync_ambient_loops(&loops),
            Self::TickEntityPositions {
                command,
                tickable_sounds,
            } => sink.tick_entity_sound_positions(command, &tickable_sounds),
        }
    }
}
