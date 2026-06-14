use anyhow::{Context, Result};
use bbb_audio::{
    AudioCommand, AudioCommandResolver, AudioResolveError, KiraAudioRuntime, SoundEventRegistry,
    TickEntitySoundPositionsCommand,
};
use bbb_pack::{PackRoots, SoundCatalog};
use bbb_world::{SoundEntityEventState, SoundEventState, StopSoundEventState};

pub(crate) trait AudioEventSink {
    fn play_positioned_sound(&mut self, state: &SoundEventState);
    fn play_entity_sound(&mut self, state: &SoundEntityEventState, position: Option<[f64; 3]>);
    fn stop_sound(&mut self, state: &StopSoundEventState);
    fn tick_entity_sound_positions(&mut self, command: TickEntitySoundPositionsCommand);
}

pub(crate) struct NativeAudioRuntime {
    catalog: SoundCatalog,
    registry: SoundEventRegistry,
    playback: KiraAudioRuntime,
}

impl NativeAudioRuntime {
    pub(crate) fn load(roots: &PackRoots) -> Result<Self> {
        Ok(Self {
            catalog: roots.load_sound_catalog().context("load sound catalog")?,
            registry: SoundEventRegistry::vanilla_26_1(),
            playback: KiraAudioRuntime::new().context("initialize Kira audio runtime")?,
        })
    }

    fn handle_resolved_command(
        &mut self,
        command: std::result::Result<AudioCommand, AudioResolveError>,
    ) {
        match command {
            Ok(command) => self.submit_command(command),
            Err(err) => {
                tracing::warn!(?err, "failed to resolve audio command");
            }
        }
    }

    fn submit_command(&mut self, command: AudioCommand) {
        if let Err(err) = self.playback.handle_command(&command) {
            tracing::warn!(?err, "failed to submit audio command");
        }
    }
}

impl AudioEventSink for NativeAudioRuntime {
    fn play_positioned_sound(&mut self, state: &SoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::new(&self.catalog, &self.registry);
            resolver.play_positioned_sound(state)
        };
        self.handle_resolved_command(command);
    }

    fn play_entity_sound(&mut self, state: &SoundEntityEventState, position: Option<[f64; 3]>) {
        let command = {
            let resolver = AudioCommandResolver::new(&self.catalog, &self.registry);
            resolver.play_entity_sound_at(state, position)
        };
        self.handle_resolved_command(command);
    }

    fn stop_sound(&mut self, state: &StopSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::new(&self.catalog, &self.registry);
            resolver.stop_sound(state)
        };
        self.submit_command(command);
    }

    fn tick_entity_sound_positions(&mut self, command: TickEntitySoundPositionsCommand) {
        self.submit_command(AudioCommand::TickEntitySoundPositions(command));
    }
}
