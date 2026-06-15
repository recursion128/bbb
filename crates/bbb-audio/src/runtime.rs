use std::collections::HashSet;

use anyhow::{anyhow, Result};
use kira::{
    listener::ListenerHandle,
    sound::{
        static_sound::{StaticSoundData, StaticSoundHandle},
        streaming::{StreamingSoundData, StreamingSoundHandle},
        FromFileError, PlaybackState,
    },
    track::{SpatialTrackBuilder, SpatialTrackHandle},
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween,
};

use crate::{
    AudioCategory, AudioCommand, AudioListenerState, EntitySoundPosition, ResolvedSound,
    StopSoundCommand, TickEntitySoundPositionsCommand,
};

const MIN_SPATIAL_DISTANCE: f32 = 1.0;

pub struct KiraAudioRuntime {
    manager: AudioManager<DefaultBackend>,
    listener: ListenerHandle,
    playing: Vec<KiraPlayingSound>,
}

struct KiraPlayingSound {
    event_id: String,
    category: AudioCategory,
    entity_id: Option<i32>,
    handle: KiraSoundHandle,
    track: Option<SpatialTrackHandle>,
}

enum KiraSoundHandle {
    Static(StaticSoundHandle),
    Streaming(StreamingSoundHandle<FromFileError>),
}

impl KiraAudioRuntime {
    pub fn new() -> Result<Self> {
        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|_| anyhow!("initialize Kira audio manager"))?;
        let listener = manager
            .add_listener([0.0, 0.0, 0.0], identity_quaternion())
            .map_err(|_| anyhow!("initialize Kira audio listener"))?;
        Ok(Self {
            manager,
            listener,
            playing: Vec::new(),
        })
    }

    pub fn handle_command(&mut self, command: &AudioCommand) -> Result<()> {
        match command {
            AudioCommand::PlayPositionedSound(command) => self.play_spatial_sound(
                &command.sound,
                command.category.clone(),
                None,
                command.position,
                command.gain,
                command.playback_rate,
            ),
            AudioCommand::PlayEntitySound(command) => self.play_spatial_sound(
                &command.sound,
                command.category.clone(),
                Some(command.entity_id),
                command.position.unwrap_or([0.0, 0.0, 0.0]),
                command.gain,
                command.playback_rate,
            ),
            AudioCommand::StopSound(command) => {
                self.stop_sounds(command);
                Ok(())
            }
            AudioCommand::TickEntitySoundPositions(command) => {
                self.tick_entity_sound_positions(command);
                Ok(())
            }
        }
    }

    fn play_spatial_sound(
        &mut self,
        sound: &ResolvedSound,
        category: AudioCategory,
        entity_id: Option<i32>,
        position: [f64; 3],
        gain: f32,
        playback_rate: f32,
    ) -> Result<()> {
        self.retain_active_sounds();
        if !sound_should_start(gain, &category) {
            return Ok(());
        }
        let mut track = self.manager.add_spatial_sub_track(
            &self.listener,
            audio_position(position),
            SpatialTrackBuilder::new()
                .persist_until_sounds_finish(true)
                .distances((MIN_SPATIAL_DISTANCE, spatial_max_distance(sound, gain))),
        )?;
        let volume = channel_decibels_from_gain(gain);
        let playback_rate = channel_playback_rate(playback_rate);
        let handle = if sound.stream {
            let data = StreamingSoundData::from_file(&sound.ogg_path)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            KiraSoundHandle::Streaming(track.play(data)?)
        } else {
            let data = StaticSoundData::from_file(&sound.ogg_path)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            KiraSoundHandle::Static(track.play(data)?)
        };
        self.playing.push(KiraPlayingSound {
            event_id: sound.event_id.clone(),
            category,
            entity_id,
            handle,
            track: Some(track),
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

    fn tick_entity_sound_positions(&mut self, command: &TickEntitySoundPositionsCommand) {
        if let Some(listener) = command.listener {
            self.listener
                .set_position(audio_position(listener.position), Tween::default());
            self.listener
                .set_orientation(listener_orientation(listener), Tween::default());
        }
        let active_entity_ids = active_entity_sound_ids(&command.entities);
        for sound in &mut self.playing {
            if entity_bound_sound_is_missing(sound.entity_id, &active_entity_ids) {
                sound.handle.stop();
            }
        }
        for entity in &command.entities {
            self.update_entity_sound_position(*entity);
        }
        self.retain_active_sounds();
    }

    fn update_entity_sound_position(&mut self, entity: EntitySoundPosition) {
        for sound in &mut self.playing {
            if sound.entity_id == Some(entity.entity_id) {
                if let Some(track) = &mut sound.track {
                    track.set_position(audio_position(entity.position), Tween::default());
                }
            }
        }
    }

    fn retain_active_sounds(&mut self) {
        self.playing.retain(KiraPlayingSound::is_active);
    }
}

impl KiraSoundHandle {
    fn stop(&mut self) {
        match self {
            Self::Static(handle) => handle.stop(Tween::default()),
            Self::Streaming(handle) => handle.stop(Tween::default()),
        }
    }

    fn state(&self) -> PlaybackState {
        match self {
            Self::Static(handle) => handle.state(),
            Self::Streaming(handle) => handle.state(),
        }
    }
}

impl KiraPlayingSound {
    fn is_active(&self) -> bool {
        self.handle.state() != PlaybackState::Stopped
    }
}

fn sound_matches_stop(sound: &KiraPlayingSound, command: &StopSoundCommand) -> bool {
    stop_filter_matches(&sound.event_id, &sound.category, command)
}

fn active_entity_sound_ids(entities: &[EntitySoundPosition]) -> HashSet<i32> {
    entities.iter().map(|entity| entity.entity_id).collect()
}

fn entity_bound_sound_is_missing(entity_id: Option<i32>, active_entity_ids: &HashSet<i32>) -> bool {
    entity_id.is_some_and(|entity_id| !active_entity_ids.contains(&entity_id))
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

fn channel_decibels_from_gain(gain: f32) -> Decibels {
    if !gain.is_finite() || gain <= 0.0 {
        Decibels::SILENCE
    } else {
        let gain = gain.min(1.0);
        Decibels((20.0 * gain.log10()).max(Decibels::SILENCE.0))
    }
}

fn channel_playback_rate(playback_rate: f32) -> f32 {
    if playback_rate.is_finite() {
        playback_rate.clamp(0.5, 2.0)
    } else {
        1.0
    }
}

fn sound_should_start(gain: f32, category: &AudioCategory) -> bool {
    category == &AudioCategory::Music || java_clamped_volume(gain) != 0.0
}

fn java_clamped_volume(gain: f32) -> f32 {
    if gain < 0.0 {
        0.0
    } else if gain > 1.0 {
        1.0
    } else {
        gain
    }
}

fn spatial_max_distance(sound: &ResolvedSound, gain: f32) -> f32 {
    let gain = if gain.is_finite() { gain } else { 0.0 };
    (sound.attenuation_distance as f32 * gain.max(1.0)).max(MIN_SPATIAL_DISTANCE + f32::EPSILON)
}

fn audio_position(position: [f64; 3]) -> [f32; 3] {
    [position[0] as f32, position[1] as f32, position[2] as f32]
}

fn identity_quaternion() -> [f32; 4] {
    [0.0, 0.0, 0.0, 1.0]
}

fn listener_orientation(listener: AudioListenerState) -> [f32; 4] {
    let yaw = (180.0 - listener.y_rot).to_radians();
    let pitch = listener.x_rot.to_radians();
    quat_mul(quat_from_yaw(yaw), quat_from_pitch(pitch))
}

fn quat_from_yaw(yaw: f32) -> [f32; 4] {
    let half = yaw * 0.5;
    [0.0, half.sin(), 0.0, half.cos()]
}

fn quat_from_pitch(pitch: f32) -> [f32; 4] {
    let half = pitch * 0.5;
    [half.sin(), 0.0, 0.0, half.cos()]
}

fn quat_mul(lhs: [f32; 4], rhs: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = lhs;
    let [bx, by, bz, bw] = rhs;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_volume_clamps_above_identity_without_changing_attenuation() {
        let sound = ResolvedSound {
            event_id: "minecraft:entity.cat.ambient".to_string(),
            sound_name: "minecraft:mob/cat/meow1".to_string(),
            ogg_path: "sounds/mob/cat/meow1.ogg".into(),
            stream: false,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        };

        assert_eq!(channel_decibels_from_gain(1.0), Decibels::IDENTITY);
        assert_eq!(channel_decibels_from_gain(2.0), Decibels::IDENTITY);
        assert_eq!(channel_decibels_from_gain(0.0), Decibels::SILENCE);
        assert_eq!(channel_decibels_from_gain(f32::NAN), Decibels::SILENCE);
        assert_eq!(spatial_max_distance(&sound, 2.0), 32.0);
    }

    #[test]
    fn playback_rate_clamps_to_vanilla_pitch_range() {
        assert_eq!(channel_playback_rate(0.25), 0.5);
        assert_eq!(channel_playback_rate(1.25), 1.25);
        assert_eq!(channel_playback_rate(2.88), 2.0);
        assert_eq!(channel_playback_rate(f32::NAN), 1.0);
    }

    #[test]
    fn non_music_zero_volume_sounds_do_not_start() {
        assert!(!sound_should_start(0.0, &AudioCategory::Blocks));
        assert!(!sound_should_start(-1.0, &AudioCategory::Blocks));
        assert!(sound_should_start(0.01, &AudioCategory::Blocks));
        assert!(sound_should_start(0.0, &AudioCategory::Music));
        assert!(sound_should_start(f32::NAN, &AudioCategory::Blocks));
    }

    #[test]
    fn spatial_max_distance_matches_vanilla_gain_scaling() {
        let sound = ResolvedSound {
            event_id: "minecraft:entity.cat.ambient".to_string(),
            sound_name: "minecraft:mob/cat/meow1".to_string(),
            ogg_path: "sounds/mob/cat/meow1.ogg".into(),
            stream: false,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        };

        assert_eq!(spatial_max_distance(&sound, 0.5), 16.0);
        assert_eq!(spatial_max_distance(&sound, 1.0), 16.0);
        assert_eq!(spatial_max_distance(&sound, 2.0), 32.0);
        assert_eq!(spatial_max_distance(&sound, f32::NAN), 16.0);
    }

    #[test]
    fn entity_bound_sound_is_missing_when_entity_position_is_absent() {
        let active = active_entity_sound_ids(&[
            EntitySoundPosition {
                entity_id: 123,
                position: [1.0, 2.0, 3.0],
            },
            EntitySoundPosition {
                entity_id: 456,
                position: [4.0, 5.0, 6.0],
            },
        ]);

        assert!(!entity_bound_sound_is_missing(None, &active));
        assert!(!entity_bound_sound_is_missing(Some(123), &active));
        assert!(entity_bound_sound_is_missing(Some(404), &active));
    }

    #[test]
    fn listener_orientation_matches_kira_forward_at_minecraft_north() {
        assert_eq!(
            listener_orientation(AudioListenerState {
                position: [0.0, 0.0, 0.0],
                y_rot: 180.0,
                x_rot: 0.0,
            }),
            identity_quaternion()
        );

        let yaw_quarter = listener_orientation(AudioListenerState {
            position: [0.0, 0.0, 0.0],
            y_rot: 90.0,
            x_rot: 0.0,
        });
        let expected = std::f32::consts::FRAC_1_SQRT_2;
        assert!((yaw_quarter[1] - expected).abs() < 0.0001);
        assert!((yaw_quarter[3] - expected).abs() < 0.0001);
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
