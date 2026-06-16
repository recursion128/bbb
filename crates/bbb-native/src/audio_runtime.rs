use anyhow::{Context, Result};
use bbb_audio::{
    AudioCommand, AudioCommandResolver, AudioResolveError, KiraAudioRuntime, SoundEventRegistry,
    TickEntitySoundPositionsCommand,
};
use bbb_control::AudioCounters;
use bbb_pack::{PackRoots, SoundCatalog};
use bbb_world::{SoundEntityEventState, SoundEventState, StopSoundEventState};

pub(crate) trait AudioEventSink {
    fn counters(&self) -> AudioCounters;
    fn set_sound_event_registry(&mut self, registry: SoundEventRegistry);
    fn play_positioned_sound(&mut self, state: &SoundEventState);
    fn play_entity_sound(&mut self, state: &SoundEntityEventState, position: Option<[f64; 3]>);
    fn stop_sound(&mut self, state: &StopSoundEventState);
    fn tick_entity_sound_positions(&mut self, command: TickEntitySoundPositionsCommand);
}

pub(crate) struct NativeAudioRuntime {
    catalog: SoundCatalog,
    registry: SoundEventRegistry,
    playback: KiraAudioRuntime,
    counters: AudioCounters,
}

impl NativeAudioRuntime {
    pub(crate) fn load(roots: &PackRoots) -> Result<Self> {
        let catalog = roots.load_sound_catalog().context("load sound catalog")?;
        let registry = SoundEventRegistry::vanilla_26_1();
        let playback = KiraAudioRuntime::new().context("initialize Kira audio runtime")?;
        let counters = AudioCounters {
            enabled: true,
            catalog_events: catalog.len(),
            registry_entries: registry.len(),
            ..AudioCounters::default()
        };
        Ok(Self {
            catalog,
            registry,
            playback,
            counters,
        })
    }

    pub(crate) fn counters(&self) -> AudioCounters {
        self.counters.clone()
    }

    fn handle_resolved_command(
        &mut self,
        command: std::result::Result<AudioCommand, AudioResolveError>,
    ) {
        match command {
            Ok(command) => self.submit_command(command),
            Err(err) => {
                self.counters.resolve_failures += 1;
                self.counters.last_resolve_error = Some(err.to_string());
                tracing::warn!(?err, "failed to resolve audio command");
            }
        }
    }

    fn submit_command(&mut self, command: AudioCommand) {
        match self.playback.handle_command(&command) {
            Ok(()) => {
                self.counters.commands_submitted += 1;
            }
            Err(err) => {
                self.counters.submit_failures += 1;
                self.counters.last_submit_error = Some(err.to_string());
                tracing::warn!(?err, "failed to submit audio command");
            }
        }
    }
}

impl AudioEventSink for NativeAudioRuntime {
    fn counters(&self) -> AudioCounters {
        self.counters()
    }

    fn set_sound_event_registry(&mut self, registry: SoundEventRegistry) {
        self.counters.registry_entries = registry.len();
        self.counters.registry_updates += 1;
        self.registry = registry;
    }

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
