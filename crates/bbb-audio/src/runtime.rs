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
    AudioCategory, AudioCommand, AudioListenerState, EntitySoundPosition,
    PlayPositionedSoundCommand, ResolvedSound, StopJukeboxSongCommand, StopSoundCommand,
    TickEntitySoundPositionsCommand,
};

const MIN_SPATIAL_DISTANCE: f32 = 1.0;
const DISTANCE_DELAY_THRESHOLD_SQR: f64 = 100.0;
const DISTANCE_DELAY_TICKS_PER_BLOCK: f64 = 0.5;

pub struct KiraAudioRuntime {
    manager: AudioManager<DefaultBackend>,
    listener: ListenerHandle,
    listener_position: [f64; 3],
    delayed_positioned: Vec<KiraDelayedPositionedSound>,
    playing: Vec<KiraPlayingSound>,
}

#[derive(Debug, Clone)]
struct KiraDelayedPositionedSound {
    command: PlayPositionedSoundCommand,
    remaining_ticks: u32,
}

struct KiraPlayingSound {
    event_id: String,
    category: AudioCategory,
    entity_id: Option<i32>,
    jukebox_pos: Option<[i32; 3]>,
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
            listener_position: [0.0, 0.0, 0.0],
            delayed_positioned: Vec::new(),
            playing: Vec::new(),
        })
    }

    pub fn handle_command(&mut self, command: &AudioCommand) -> Result<()> {
        match command {
            AudioCommand::PlayLocalSound(command) => self.play_local_sound(
                &command.sound,
                command.category.clone(),
                command.channel_gain,
                command.playback_rate,
            ),
            AudioCommand::PlayPositionedSound(command) => self.play_positioned_sound(command),
            AudioCommand::PlayEntitySound(command) => self.play_spatial_sound(
                &command.sound,
                command.category.clone(),
                Some(command.entity_id),
                command.position.unwrap_or([0.0, 0.0, 0.0]),
                command.gain,
                command.channel_gain,
                command.playback_rate,
                command.fixed_range,
                None,
            ),
            AudioCommand::PlayJukeboxSong(command) => self.play_jukebox_song(
                &command.sound,
                command.category.clone(),
                command.position,
                command.jukebox_pos,
                command.gain,
                command.channel_gain,
                command.playback_rate,
            ),
            AudioCommand::StopJukeboxSong(command) => {
                self.stop_jukebox_song(command);
                Ok(())
            }
            AudioCommand::StopSound(command) => {
                self.stop_sounds(command);
                Ok(())
            }
            AudioCommand::TickEntitySoundPositions(command) => {
                self.tick_entity_sound_positions(command)
            }
        }
    }

    fn play_local_sound(
        &mut self,
        sound: &ResolvedSound,
        category: AudioCategory,
        channel_gain: f32,
        playback_rate: f32,
    ) -> Result<()> {
        self.retain_active_sounds();
        if !sound_should_start(channel_gain, &category) {
            return Ok(());
        }
        let volume = channel_decibels_from_gain(channel_gain);
        let playback_rate = channel_playback_rate(playback_rate);
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
            entity_id: None,
            jukebox_pos: None,
            handle,
            track: None,
        });
        Ok(())
    }

    fn play_positioned_sound(&mut self, command: &PlayPositionedSoundCommand) -> Result<()> {
        self.retain_active_sounds();
        if !sound_should_start(command.channel_gain, &command.category) {
            return Ok(());
        }
        if let Some(delayed) = delayed_positioned_sound_for_command(command, self.listener_position)
        {
            self.delayed_positioned.push(delayed);
            return Ok(());
        }
        self.play_positioned_sound_now(command)
    }

    fn play_positioned_sound_now(&mut self, command: &PlayPositionedSoundCommand) -> Result<()> {
        self.play_spatial_sound(
            &command.sound,
            command.category.clone(),
            None,
            command.position,
            command.gain,
            command.channel_gain,
            command.playback_rate,
            command.fixed_range,
            None,
        )
    }

    fn play_spatial_sound(
        &mut self,
        sound: &ResolvedSound,
        category: AudioCategory,
        entity_id: Option<i32>,
        position: [f64; 3],
        gain: f32,
        channel_gain: f32,
        playback_rate: f32,
        fixed_range: Option<f32>,
        jukebox_pos: Option<[i32; 3]>,
    ) -> Result<()> {
        self.retain_active_sounds();
        if !sound_should_start(channel_gain, &category) {
            return Ok(());
        }
        let mut track = self.manager.add_spatial_sub_track(
            &self.listener,
            audio_position(position),
            SpatialTrackBuilder::new()
                .persist_until_sounds_finish(true)
                .distances((
                    MIN_SPATIAL_DISTANCE,
                    spatial_max_distance(sound, gain, fixed_range),
                )),
        )?;
        let volume = channel_decibels_from_gain(channel_gain);
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
            jukebox_pos,
            handle,
            track: Some(track),
        });
        Ok(())
    }

    fn play_jukebox_song(
        &mut self,
        sound: &ResolvedSound,
        category: AudioCategory,
        position: [f64; 3],
        jukebox_pos: [i32; 3],
        gain: f32,
        channel_gain: f32,
        playback_rate: f32,
    ) -> Result<()> {
        self.stop_jukebox_song(&StopJukeboxSongCommand { jukebox_pos });
        self.play_spatial_sound(
            sound,
            category,
            None,
            position,
            gain,
            channel_gain,
            playback_rate,
            None,
            Some(jukebox_pos),
        )
    }

    fn stop_sounds(&mut self, command: &StopSoundCommand) {
        self.delayed_positioned
            .retain(|sound| !delayed_positioned_sound_matches_stop(sound, command));
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

    fn stop_jukebox_song(&mut self, command: &StopJukeboxSongCommand) {
        let mut retained = Vec::with_capacity(self.playing.len());
        for mut sound in self.playing.drain(..) {
            if sound.jukebox_pos == Some(command.jukebox_pos) {
                sound.handle.stop();
            } else {
                retained.push(sound);
            }
        }
        self.playing = retained;
    }

    fn tick_entity_sound_positions(
        &mut self,
        command: &TickEntitySoundPositionsCommand,
    ) -> Result<()> {
        if let Some(listener) = command.listener {
            self.listener_position = listener.position;
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
        for command in advance_delayed_positioned_sounds(&mut self.delayed_positioned) {
            self.play_positioned_sound_now(&command)?;
        }
        Ok(())
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

fn delayed_positioned_sound_matches_stop(
    sound: &KiraDelayedPositionedSound,
    command: &StopSoundCommand,
) -> bool {
    stop_filter_matches(
        &sound.command.sound.event_id,
        &sound.command.category,
        command,
    )
}

fn delayed_positioned_sound_for_command(
    command: &PlayPositionedSoundCommand,
    listener_position: [f64; 3],
) -> Option<KiraDelayedPositionedSound> {
    vanilla_distance_delay_ticks(command.distance_delay, listener_position, command.position).map(
        |remaining_ticks| KiraDelayedPositionedSound {
            command: command.clone(),
            remaining_ticks,
        },
    )
}

fn advance_delayed_positioned_sounds(
    delayed: &mut Vec<KiraDelayedPositionedSound>,
) -> Vec<PlayPositionedSoundCommand> {
    let mut ready = Vec::new();
    let mut pending = Vec::with_capacity(delayed.len());
    for mut sound in delayed.drain(..) {
        if sound.remaining_ticks <= 1 {
            ready.push(sound.command);
        } else {
            sound.remaining_ticks -= 1;
            pending.push(sound);
        }
    }
    *delayed = pending;
    ready
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

fn spatial_max_distance(sound: &ResolvedSound, gain: f32, fixed_range: Option<f32>) -> f32 {
    if let Some(range) = fixed_range {
        if range.is_finite() {
            return range.max(MIN_SPATIAL_DISTANCE + f32::EPSILON);
        }
    }
    let gain = if gain.is_finite() { gain } else { 0.0 };
    (sound.attenuation_distance as f32 * gain.max(1.0)).max(MIN_SPATIAL_DISTANCE + f32::EPSILON)
}

fn vanilla_distance_delay_ticks(
    distance_delay: bool,
    listener_position: [f64; 3],
    sound_position: [f64; 3],
) -> Option<u32> {
    if !distance_delay {
        return None;
    }
    let distance_sqr = distance_squared(listener_position, sound_position);
    if distance_sqr <= DISTANCE_DELAY_THRESHOLD_SQR {
        return None;
    }
    Some((distance_sqr.sqrt() * DISTANCE_DELAY_TICKS_PER_BLOCK) as u32)
}

fn distance_squared(lhs: [f64; 3], rhs: [f64; 3]) -> f64 {
    let dx = lhs[0] - rhs[0];
    let dy = lhs[1] - rhs[1];
    let dz = lhs[2] - rhs[2];
    dx * dx + dy * dy + dz * dz
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
        assert_eq!(spatial_max_distance(&sound, 2.0, None), 32.0);
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

        assert_eq!(spatial_max_distance(&sound, 0.5, None), 16.0);
        assert_eq!(spatial_max_distance(&sound, 1.0, None), 16.0);
        assert_eq!(spatial_max_distance(&sound, 2.0, None), 32.0);
        assert_eq!(spatial_max_distance(&sound, f32::NAN, None), 16.0);
    }

    #[test]
    fn fixed_range_overrides_sound_attenuation_and_gain_scaling() {
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

        assert_eq!(spatial_max_distance(&sound, 0.5, Some(24.0)), 24.0);
        assert_eq!(spatial_max_distance(&sound, 2.0, Some(24.0)), 24.0);
        assert_eq!(
            spatial_max_distance(&sound, 1.0, Some(0.0)),
            MIN_SPATIAL_DISTANCE + f32::EPSILON
        );
        assert_eq!(spatial_max_distance(&sound, 2.0, Some(f32::NAN)), 32.0);
    }

    #[test]
    fn distance_delay_ticks_match_vanilla_threshold_and_flooring() {
        assert_eq!(
            vanilla_distance_delay_ticks(false, [0.0, 0.0, 0.0], [40.0, 0.0, 0.0]),
            None
        );
        assert_eq!(
            vanilla_distance_delay_ticks(true, [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]),
            None
        );
        assert_eq!(
            vanilla_distance_delay_ticks(true, [0.0, 0.0, 0.0], [10.1, 0.0, 0.0]),
            Some(5)
        );
        assert_eq!(
            vanilla_distance_delay_ticks(true, [0.0, 0.0, 0.0], [40.0, 0.0, 0.0]),
            Some(20)
        );
    }

    #[test]
    fn delayed_positioned_sounds_release_after_remaining_ticks() {
        let command = PlayPositionedSoundCommand {
            sound: test_resolved_sound("minecraft:block.cobweb.place"),
            category: AudioCategory::Blocks,
            position: [40.0, 0.0, 0.0],
            packet_volume: 1.0,
            packet_pitch: 1.0,
            gain: 1.0,
            channel_gain: 1.0,
            playback_rate: 1.0,
            seed: 0,
            fixed_range: None,
            distance_delay: true,
        };
        let mut delayed = vec![KiraDelayedPositionedSound {
            command: command.clone(),
            remaining_ticks: 2,
        }];

        assert!(advance_delayed_positioned_sounds(&mut delayed).is_empty());
        assert_eq!(delayed.len(), 1);
        assert_eq!(delayed[0].remaining_ticks, 1);
        assert_eq!(
            advance_delayed_positioned_sounds(&mut delayed),
            vec![command]
        );
        assert!(delayed.is_empty());
    }

    #[test]
    fn delayed_positioned_sound_for_command_preserves_playback_command() {
        let command = PlayPositionedSoundCommand {
            sound: test_resolved_sound("minecraft:block.cobweb.place"),
            category: AudioCategory::Blocks,
            position: [40.0, 0.0, 0.0],
            packet_volume: 1.0,
            packet_pitch: 1.0,
            gain: 1.0,
            channel_gain: 1.0,
            playback_rate: 1.0,
            seed: 99,
            fixed_range: Some(24.0),
            distance_delay: true,
        };

        let delayed = delayed_positioned_sound_for_command(&command, [0.0, 0.0, 0.0]).unwrap();

        assert_eq!(delayed.remaining_ticks, 20);
        assert_eq!(delayed.command, command);
        assert!(delayed_positioned_sound_for_command(&delayed.command, [35.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn stop_sound_filters_delayed_positioned_sounds() {
        let mut delayed = vec![
            KiraDelayedPositionedSound {
                command: PlayPositionedSoundCommand {
                    sound: test_resolved_sound("minecraft:block.cobweb.place"),
                    category: AudioCategory::Blocks,
                    position: [40.0, 0.0, 0.0],
                    packet_volume: 1.0,
                    packet_pitch: 1.0,
                    gain: 1.0,
                    channel_gain: 1.0,
                    playback_rate: 1.0,
                    seed: 0,
                    fixed_range: None,
                    distance_delay: true,
                },
                remaining_ticks: 20,
            },
            KiraDelayedPositionedSound {
                command: PlayPositionedSoundCommand {
                    sound: test_resolved_sound("minecraft:entity.ghast.warn"),
                    category: AudioCategory::Hostile,
                    position: [40.0, 0.0, 0.0],
                    packet_volume: 1.0,
                    packet_pitch: 1.0,
                    gain: 1.0,
                    channel_gain: 1.0,
                    playback_rate: 1.0,
                    seed: 0,
                    fixed_range: None,
                    distance_delay: true,
                },
                remaining_ticks: 20,
            },
        ];
        let stop_blocks = StopSoundCommand {
            category: Some(AudioCategory::Blocks),
            name: None,
        };

        delayed.retain(|sound| !delayed_positioned_sound_matches_stop(sound, &stop_blocks));

        assert_eq!(delayed.len(), 1);
        assert_eq!(
            delayed[0].command.sound.event_id,
            "minecraft:entity.ghast.warn"
        );
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

    fn test_resolved_sound(event_id: &str) -> ResolvedSound {
        ResolvedSound {
            event_id: event_id.to_string(),
            sound_name: event_id
                .strip_prefix("minecraft:")
                .unwrap_or(event_id)
                .replace('.', "/"),
            ogg_path: format!("sounds/{event_id}.ogg").into(),
            stream: false,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        }
    }
}
