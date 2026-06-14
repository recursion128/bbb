use anyhow::{anyhow, Result};
use kira::{
    sound::{
        static_sound::{StaticSoundData, StaticSoundHandle},
        streaming::{StreamingSoundData, StreamingSoundHandle},
        FromFileError,
    },
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween,
};

use crate::{AudioCategory, AudioCommand, ResolvedSound, StopSoundCommand};

pub struct KiraAudioRuntime {
    manager: AudioManager<DefaultBackend>,
    playing: Vec<KiraPlayingSound>,
}

struct KiraPlayingSound {
    event_id: String,
    category: AudioCategory,
    handle: KiraSoundHandle,
}

enum KiraSoundHandle {
    Static(StaticSoundHandle),
    Streaming(StreamingSoundHandle<FromFileError>),
}

impl KiraAudioRuntime {
    pub fn new() -> Result<Self> {
        Ok(Self {
            manager: AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
                .map_err(|_| anyhow!("initialize Kira audio manager"))?,
            playing: Vec::new(),
        })
    }

    pub fn handle_command(&mut self, command: &AudioCommand) -> Result<()> {
        match command {
            AudioCommand::PlayPositionedSound(command) => self.play_resolved_sound(
                &command.sound,
                command.category.clone(),
                command.gain,
                command.playback_rate,
            ),
            AudioCommand::PlayEntitySound(command) => self.play_resolved_sound(
                &command.sound,
                command.category.clone(),
                command.gain,
                command.playback_rate,
            ),
            AudioCommand::StopSound(command) => {
                self.stop_sounds(command);
                Ok(())
            }
            AudioCommand::TickEntitySoundPositions => Ok(()),
        }
    }

    fn play_resolved_sound(
        &mut self,
        sound: &ResolvedSound,
        category: AudioCategory,
        gain: f32,
        playback_rate: f32,
    ) -> Result<()> {
        let volume = decibels_from_gain(gain);
        let handle = if sound.stream {
            let data = StreamingSoundData::from_file(&sound.ogg_path)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            KiraSoundHandle::Streaming(self.manager.play(data)?)
        } else {
            let data = StaticSoundData::from_file(&sound.ogg_path)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            KiraSoundHandle::Static(self.manager.play(data)?)
        };
        self.playing.push(KiraPlayingSound {
            event_id: sound.event_id.clone(),
            category,
            handle,
        });
        Ok(())
    }

    fn stop_sounds(&mut self, command: &StopSoundCommand) {
        let mut retained = Vec::with_capacity(self.playing.len());
        for mut sound in self.playing.drain(..) {
            if sound_matches_stop(&sound, command) {
                sound.handle.stop();
            } else {
                retained.push(sound);
            }
        }
        self.playing = retained;
    }
}

impl KiraSoundHandle {
    fn stop(&mut self) {
        match self {
            Self::Static(handle) => handle.stop(Tween::default()),
            Self::Streaming(handle) => handle.stop(Tween::default()),
        }
    }
}

fn sound_matches_stop(sound: &KiraPlayingSound, command: &StopSoundCommand) -> bool {
    stop_filter_matches(&sound.event_id, &sound.category, command)
}

fn stop_filter_matches(
    event_id: &str,
    category: &AudioCategory,
    command: &StopSoundCommand,
) -> bool {
    let category_matches = command
        .category
        .as_ref()
        .map_or(true, |stop_category| stop_category == category);
    let name_matches = command
        .name
        .as_deref()
        .map_or(true, |name| name == event_id);
    category_matches && name_matches
}

fn decibels_from_gain(gain: f32) -> Decibels {
    if !gain.is_finite() || gain <= 0.0 {
        Decibels::SILENCE
    } else {
        Decibels((20.0 * gain.log10()).max(Decibels::SILENCE.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_to_decibels_preserves_identity_and_silence() {
        assert_eq!(decibels_from_gain(1.0), Decibels::IDENTITY);
        assert_eq!(decibels_from_gain(0.0), Decibels::SILENCE);
        assert_eq!(decibels_from_gain(f32::NAN), Decibels::SILENCE);
    }

    #[test]
    fn stop_filter_matches_source_name_and_all() {
        let event_id = "minecraft:music.menu";
        let category = AudioCategory::Music;

        assert!(stop_filter_matches(
            event_id,
            &category,
            &StopSoundCommand {
                category: Some(AudioCategory::Music),
                name: Some(event_id.to_string()),
            },
        ));
        assert!(stop_filter_matches(
            event_id,
            &category,
            &StopSoundCommand {
                category: Some(AudioCategory::Music),
                name: None,
            },
        ));
        assert!(stop_filter_matches(
            event_id,
            &category,
            &StopSoundCommand {
                category: None,
                name: None,
            },
        ));
        assert!(!stop_filter_matches(
            event_id,
            &category,
            &StopSoundCommand {
                category: Some(AudioCategory::Blocks),
                name: Some(event_id.to_string()),
            },
        ));
        assert!(!stop_filter_matches(
            event_id,
            &category,
            &StopSoundCommand {
                category: Some(AudioCategory::Music),
                name: Some("minecraft:music.game".to_string()),
            },
        ));
    }
}
