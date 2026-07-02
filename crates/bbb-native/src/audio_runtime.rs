mod random;
pub(crate) mod resolver;

use anyhow::{Context, Result};
use bbb_audio::{
    AudioCommand, JukeboxSongRegistry, KiraAudioRuntime, SoundEventRegistry,
    TickEntitySoundPositionsCommand,
};
use bbb_control::AudioCounters;
use bbb_pack::{PackRoots, SoundCatalog};
use bbb_world::{
    JukeboxLevelEventState, LocalSoundEventState, SoundEntityEventState, SoundEventState,
    StopSoundEventState,
};

use resolver::{AudioCommandResolver, AudioResolveError};

pub(crate) trait AudioEventSink {
    fn counters(&self) -> AudioCounters;
    fn set_sound_event_registry(&mut self, registry: SoundEventRegistry);
    fn set_jukebox_song_registry(&mut self, registry: JukeboxSongRegistry);
    fn play_local_sound(&mut self, state: &LocalSoundEventState);
    fn play_positioned_sound(&mut self, state: &SoundEventState);
    fn play_entity_sound(&mut self, state: &SoundEntityEventState, position: Option<[f64; 3]>);
    fn play_jukebox_song(&mut self, state: &JukeboxLevelEventState);
    fn stop_jukebox_song(&mut self, state: &JukeboxLevelEventState);
    fn stop_sound(&mut self, state: &StopSoundEventState);
    fn tick_entity_sound_positions(&mut self, command: TickEntitySoundPositionsCommand);
}

pub(crate) struct NativeAudioRuntime {
    catalog: SoundCatalog,
    registry: SoundEventRegistry,
    jukebox_registry: JukeboxSongRegistry,
    playback: KiraAudioRuntime,
    counters: AudioCounters,
}

impl NativeAudioRuntime {
    pub(crate) fn load(roots: &PackRoots) -> Result<Self> {
        let catalog = load_required_native_sound_catalog(roots)?;
        let registry = SoundEventRegistry::vanilla_26_1();
        let jukebox_registry = JukeboxSongRegistry::vanilla_26_1();
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
            jukebox_registry,
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

fn load_required_native_sound_catalog(roots: &PackRoots) -> Result<SoundCatalog> {
    roots
        .load_required_sound_catalog()
        .context("load native sound catalog")
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

    fn set_jukebox_song_registry(&mut self, registry: JukeboxSongRegistry) {
        self.jukebox_registry = registry;
    }

    fn play_local_sound(&mut self, state: &LocalSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_local_sound(state)
        };
        self.handle_resolved_command(command);
    }

    fn play_positioned_sound(&mut self, state: &SoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_positioned_sound(state)
        };
        self.handle_resolved_command(command);
    }

    fn play_entity_sound(&mut self, state: &SoundEntityEventState, position: Option<[f64; 3]>) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_entity_sound_at(state, position)
        };
        self.handle_resolved_command(command);
    }

    fn play_jukebox_song(&mut self, state: &JukeboxLevelEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.play_jukebox_song(state)
        };
        self.handle_resolved_command(command);
    }

    fn stop_jukebox_song(&mut self, state: &JukeboxLevelEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.stop_jukebox_song(state)
        };
        self.submit_command(command);
    }

    fn stop_sound(&mut self, state: &StopSoundEventState) {
        let command = {
            let resolver = AudioCommandResolver::with_jukebox_registry(
                &self.catalog,
                &self.registry,
                &self.jukebox_registry,
                bbb_audio::AudioVolumeSettings::default(),
            );
            resolver.stop_sound(state)
        };
        self.submit_command(command);
    }

    fn tick_entity_sound_positions(&mut self, command: TickEntitySoundPositionsCommand) {
        self.submit_command(AudioCommand::TickEntitySoundPositions(command));
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    #[test]
    fn native_audio_catalog_loader_errors_before_kira_when_catalog_is_empty() {
        let root = unique_temp_dir("native-audio-empty-catalog");
        std::fs::create_dir_all(root.join("sources").join("26.1"))
            .expect("test source root should exist");
        let roots = PackRoots::from_root(&root).unwrap();

        let err = load_required_native_sound_catalog(&roots).unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("load native sound catalog"));
        assert!(message.contains("required sound catalog is empty"));

        std::fs::remove_dir_all(root).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-native-{label}-{nanos}"))
    }
}
