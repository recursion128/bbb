use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use kira::{
    listener::ListenerHandle,
    sound::{static_sound::StaticSoundHandle, streaming::StreamingSoundHandle, PlaybackState},
    track::{SpatialTrackBuilder, SpatialTrackHandle},
    AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween,
};

use crate::{
    residency::{AudioDecodeError, AudioResidency},
    AudioCategory, AudioCommand, AudioListenerState, AudioResidencyObservation,
    EntitySoundPosition, PlayPositionedSoundCommand, ResolvedSound, StopJukeboxSongCommand,
    StopSoundCommand, TickEntitySoundPositionsCommand, TickableEntitySoundUpdate,
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
    music_gain: f32,
    residency: AudioResidency,
    channel_capacity: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioChannelObservation {
    pub static_used: usize,
    pub streaming_used: usize,
    /// Kira's main-track sound slots plus the spatial-track slots used by
    /// bbb. Static and streaming handles share this admission upper bound.
    pub shared_capacity: usize,
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
    tickable: bool,
    ambient_loop: bool,
    base_channel_gain: f32,
}

enum KiraSoundHandle {
    Static(StaticSoundHandle),
    Streaming(StreamingSoundHandle<AudioDecodeError>),
}

impl KiraAudioRuntime {
    pub fn new() -> Result<Self> {
        Self::new_with_resource_generation(0)
    }

    pub fn new_with_resource_generation(resource_generation: u64) -> Result<Self> {
        let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .map_err(|_| anyhow!("initialize Kira audio manager"))?;
        let channel_capacity = manager
            .main_track()
            .sound_capacity()
            .saturating_add(manager.sub_track_capacity());
        let listener = manager
            .add_listener([0.0, 0.0, 0.0], identity_quaternion())
            .map_err(|_| anyhow!("initialize Kira audio listener"))?;
        Ok(Self {
            manager,
            listener,
            listener_position: [0.0, 0.0, 0.0],
            delayed_positioned: Vec::new(),
            playing: Vec::new(),
            music_gain: 1.0,
            residency: AudioResidency::new(resource_generation),
            channel_capacity,
        })
    }

    pub fn residency_observation(&self) -> AudioResidencyObservation {
        self.residency.observation()
    }

    pub fn channel_observation(&self) -> AudioChannelObservation {
        let (static_used, streaming_used) = active_channel_counts(
            self.playing
                .iter()
                .map(|sound| (sound.handle.is_streaming(), sound.is_active())),
        );
        AudioChannelObservation {
            static_used,
            streaming_used,
            shared_capacity: self.channel_capacity,
        }
    }

    /// Returns the most recently started music-category event that the audio
    /// backend still considers active. Stopped handles remain observable in
    /// `playing` until the next mutating command, so this query filters their
    /// live Kira state without requiring simulation to touch backend owners.
    pub fn active_music_event_id(&self) -> Option<&str> {
        latest_active_music_event_id(self.playing.iter().map(|sound| {
            (
                sound.event_id.as_str(),
                &sound.category,
                sound.handle.state() != PlaybackState::Stopped,
            )
        }))
    }

    pub fn reset_resource_generation(&mut self, resource_generation: u64) {
        self.delayed_positioned.clear();
        for sound in &mut self.playing {
            sound.handle.stop();
        }
        self.playing.clear();
        self.residency.reset_generation(resource_generation);
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
                command.looping,
                command.linear_attenuation,
                command.tickable,
            ),
            AudioCommand::PlayJukeboxSong(command) => self.play_jukebox_song(
                &command.sound,
                command.category.clone(),
                command.position,
                command.jukebox_pos,
                command.gain,
                command.channel_gain,
                command.playback_rate,
                command.fixed_range,
            ),
            AudioCommand::StopJukeboxSong(command) => {
                self.stop_jukebox_song(command);
                Ok(())
            }
            AudioCommand::StopSound(command) => {
                self.stop_sounds(command);
                Ok(())
            }
            AudioCommand::SetMusicGain(command) => {
                self.set_music_gain(command.gain);
                Ok(())
            }
            AudioCommand::SyncAmbientLoops(command) => self.sync_ambient_loops(&command.loops),
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
        self.play_local_sound_with_role(sound, category, channel_gain, playback_rate, false, false)
    }

    fn play_local_sound_with_role(
        &mut self,
        sound: &ResolvedSound,
        category: AudioCategory,
        channel_gain: f32,
        playback_rate: f32,
        looping: bool,
        ambient_loop: bool,
    ) -> Result<()> {
        self.retain_active_sounds();
        if !sound_should_start(channel_gain, &category) {
            return Ok(());
        }
        let effective_gain = effective_channel_gain(&category, channel_gain, self.music_gain);
        let volume = channel_decibels_from_gain(effective_gain);
        let playback_rate = channel_playback_rate(playback_rate);
        let handle = if sound.stream {
            let data = self
                .residency
                .streaming_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Streaming(self.manager.play(data)?)
        } else {
            let data = self
                .residency
                .static_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Static(self.manager.play(data)?)
        };
        self.playing.push(KiraPlayingSound {
            event_id: sound.event_id.clone(),
            category,
            entity_id: None,
            jukebox_pos: None,
            handle,
            track: None,
            tickable: false,
            ambient_loop,
            base_channel_gain: channel_gain,
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
            false,
            true,
            false,
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
        looping: bool,
        linear_attenuation: bool,
        tickable: bool,
    ) -> Result<()> {
        self.retain_active_sounds();
        if !tickable && !sound_should_start(channel_gain, &category) {
            return Ok(());
        }
        let mut track = if linear_attenuation {
            Some(
                self.manager.add_spatial_sub_track(
                    &self.listener,
                    audio_position(position),
                    SpatialTrackBuilder::new()
                        .persist_until_sounds_finish(true)
                        .distances((
                            MIN_SPATIAL_DISTANCE,
                            spatial_max_distance(sound, gain, fixed_range),
                        )),
                )?,
            )
        } else {
            None
        };
        let volume = channel_decibels_from_gain(channel_gain);
        let playback_rate = channel_playback_rate(playback_rate);
        let handle = if sound.stream {
            let data = self
                .residency
                .streaming_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Streaming(if let Some(track) = track.as_mut() {
                track.play(data)?
            } else {
                self.manager.play(data)?
            })
        } else {
            let data = self
                .residency
                .static_sound(sound)?
                .volume(volume)
                .playback_rate(playback_rate as f64);
            let data = if looping { data.loop_region(..) } else { data };
            KiraSoundHandle::Static(if let Some(track) = track.as_mut() {
                track.play(data)?
            } else {
                self.manager.play(data)?
            })
        };
        self.playing.push(KiraPlayingSound {
            event_id: sound.event_id.clone(),
            category,
            entity_id,
            jukebox_pos,
            handle,
            track,
            tickable,
            ambient_loop: false,
            base_channel_gain: channel_gain,
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
        fixed_range: Option<f32>,
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
            fixed_range,
            Some(jukebox_pos),
            false,
            true,
            false,
        )
    }

    fn stop_sounds(&mut self, command: &StopSoundCommand) {
        stop_delayed_positioned_sounds(&mut self.delayed_positioned, command);
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

    fn set_music_gain(&mut self, gain: f32) {
        self.music_gain = normalized_gain(gain);
        for sound in &mut self.playing {
            if sound.category == AudioCategory::Music {
                sound.handle.set_volume(effective_channel_gain(
                    &sound.category,
                    sound.base_channel_gain,
                    self.music_gain,
                ));
            }
        }
    }

    fn sync_ambient_loops(&mut self, desired: &[crate::AmbientLoopSoundCommand]) -> Result<()> {
        self.retain_active_sounds();
        let plan = ambient_loop_sync_plan(
            self.playing
                .iter()
                .filter(|sound| sound.ambient_loop)
                .map(|sound| sound.event_id.as_str()),
            desired,
        );
        for sound in &mut self.playing {
            if !sound.ambient_loop {
                continue;
            }
            let Some(desired_index) = plan
                .updates
                .iter()
                .find_map(|(event_id, index)| (event_id == &sound.event_id).then_some(*index))
            else {
                sound.handle.stop();
                continue;
            };
            let desired = &desired[desired_index];
            sound.base_channel_gain = desired.channel_gain;
            sound.handle.set_volume(desired.channel_gain);
        }
        self.retain_active_sounds();
        for desired_index in plan.starts {
            let desired = &desired[desired_index];
            self.play_local_sound_with_role(
                &desired.sound,
                AudioCategory::Ambient,
                desired.channel_gain,
                1.0,
                true,
                true,
            )?;
        }
        Ok(())
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
        let active_tickable_sounds = active_tickable_entity_sounds(&command.tickable_sounds);
        for sound in &mut self.playing {
            if entity_bound_sound_is_missing(sound.entity_id, &active_entity_ids) {
                sound.handle.stop();
            }
            if tickable_entity_sound_is_missing(sound, &active_tickable_sounds) {
                sound.handle.stop();
            }
        }
        for entity in &command.entities {
            self.update_entity_sound_position(*entity);
        }
        for sound in &command.tickable_sounds {
            self.update_tickable_entity_sound(sound);
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

    fn update_tickable_entity_sound(&mut self, update: &TickableEntitySoundUpdate) {
        for sound in &mut self.playing {
            if sound.tickable
                && sound.entity_id == Some(update.entity_id)
                && sound.event_id == update.event_id
            {
                if let Some(track) = &mut sound.track {
                    track.set_position(audio_position(update.position), Tween::default());
                }
                sound.handle.set_volume(update.channel_gain);
                sound.base_channel_gain = update.channel_gain;
                sound.handle.set_playback_rate(update.playback_rate);
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

    fn is_streaming(&self) -> bool {
        matches!(self, Self::Streaming(_))
    }

    fn set_volume(&mut self, gain: f32) {
        let volume = channel_decibels_from_gain(gain);
        match self {
            Self::Static(handle) => handle.set_volume(volume, Tween::default()),
            Self::Streaming(handle) => handle.set_volume(volume, Tween::default()),
        }
    }

    fn set_playback_rate(&mut self, playback_rate: f32) {
        let playback_rate = f64::from(channel_playback_rate(playback_rate));
        match self {
            Self::Static(handle) => handle.set_playback_rate(playback_rate, Tween::default()),
            Self::Streaming(handle) => handle.set_playback_rate(playback_rate, Tween::default()),
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

fn latest_active_music_event_id<'a>(
    sounds: impl DoubleEndedIterator<Item = (&'a str, &'a AudioCategory, bool)>,
) -> Option<&'a str> {
    sounds.rev().find_map(|(event_id, category, active)| {
        (active && category == &AudioCategory::Music).then_some(event_id)
    })
}

fn active_channel_counts(sounds: impl IntoIterator<Item = (bool, bool)>) -> (usize, usize) {
    sounds.into_iter().filter(|(_, active)| *active).fold(
        (0usize, 0usize),
        |(static_used, streaming_used), (streaming, _)| {
            if streaming {
                (static_used, streaming_used.saturating_add(1))
            } else {
                (static_used.saturating_add(1), streaming_used)
            }
        },
    )
}

fn stop_delayed_positioned_sounds(
    delayed: &mut Vec<KiraDelayedPositionedSound>,
    command: &StopSoundCommand,
) {
    if command.category.is_none() && command.name.is_none() {
        delayed.clear();
    }
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

fn active_tickable_entity_sounds(sounds: &[TickableEntitySoundUpdate]) -> HashSet<(i32, &str)> {
    sounds
        .iter()
        .map(|sound| (sound.entity_id, sound.event_id.as_str()))
        .collect()
}

fn tickable_entity_sound_is_missing(
    sound: &KiraPlayingSound,
    active_sounds: &HashSet<(i32, &str)>,
) -> bool {
    sound.tickable
        && sound
            .entity_id
            .is_some_and(|entity_id| !active_sounds.contains(&(entity_id, sound.event_id.as_str())))
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

fn normalized_gain(gain: f32) -> f32 {
    if gain.is_finite() {
        gain.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn effective_channel_gain(
    category: &AudioCategory,
    base_channel_gain: f32,
    music_gain: f32,
) -> f32 {
    if category == &AudioCategory::Music {
        base_channel_gain * normalized_gain(music_gain)
    } else {
        base_channel_gain
    }
}

#[derive(Debug, PartialEq, Eq)]
struct AmbientLoopSyncPlan {
    updates: Vec<(String, usize)>,
    starts: Vec<usize>,
}

fn ambient_loop_sync_plan<'a>(
    active_event_ids: impl IntoIterator<Item = &'a str>,
    desired: &[crate::AmbientLoopSoundCommand],
) -> AmbientLoopSyncPlan {
    let mut remaining = desired
        .iter()
        .enumerate()
        .map(|(index, sound)| (sound.sound.event_id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut updates = Vec::new();
    for active in active_event_ids {
        if let Some(index) = remaining.remove(active) {
            updates.push((active.to_string(), index));
        }
    }
    let mut starts = remaining.into_values().collect::<Vec<_>>();
    starts.sort_unstable();
    AmbientLoopSyncPlan { updates, starts }
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
            resource_generation: 0,
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
    fn channel_diagnostics_count_only_active_handles_by_decode_mode() {
        assert_eq!(
            active_channel_counts([
                (false, true),
                (false, false),
                (true, true),
                (true, true),
                (true, false),
            ]),
            (1, 2)
        );
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
            resource_generation: 0,
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
            resource_generation: 0,
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
    fn stop_sound_only_stop_all_clears_delayed_positioned_sounds() {
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

        stop_delayed_positioned_sounds(&mut delayed, &stop_blocks);

        assert_eq!(delayed.len(), 2);

        stop_delayed_positioned_sounds(
            &mut delayed,
            &StopSoundCommand {
                category: None,
                name: None,
            },
        );
        assert!(delayed.is_empty());
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

    #[test]
    fn backend_music_identity_ignores_stopped_and_naturally_completed_handles() {
        let music = AudioCategory::Music;
        let blocks = AudioCategory::Blocks;
        let active = [
            ("minecraft:music.game", &music, true),
            ("minecraft:block.note_block.harp", &blocks, true),
            ("minecraft:music.dragon", &music, true),
        ];
        assert_eq!(
            latest_active_music_event_id(active.into_iter()),
            Some("minecraft:music.dragon")
        );

        // Explicit stop and natural completion both surface as Kira's
        // `PlaybackState::Stopped`; the previous live music becomes visible.
        let dragon_stopped = [
            ("minecraft:music.game", &music, true),
            ("minecraft:music.dragon", &music, false),
        ];
        assert_eq!(
            latest_active_music_event_id(dragon_stopped.into_iter()),
            Some("minecraft:music.game")
        );
        let all_completed = [("minecraft:music.dragon", &music, false)];
        assert_eq!(
            latest_active_music_event_id(all_completed.into_iter()),
            None
        );
    }

    #[test]
    fn music_gain_clamps_and_only_scales_music_category_handles() {
        assert_eq!(normalized_gain(-1.0), 0.0);
        assert_eq!(normalized_gain(0.25), 0.25);
        assert_eq!(normalized_gain(2.0), 1.0);
        assert_eq!(normalized_gain(f32::NAN), 0.0);
        assert_eq!(
            effective_channel_gain(&AudioCategory::Music, 0.8, 0.25),
            0.2
        );
        assert_eq!(
            effective_channel_gain(&AudioCategory::Ambient, 0.8, 0.25),
            0.8
        );
    }

    #[test]
    fn ambient_loop_sync_updates_retained_stops_omitted_and_starts_new() {
        let desired = [
            crate::AmbientLoopSoundCommand {
                sound: test_resolved_sound("minecraft:ambient.keep"),
                channel_gain: 0.75,
            },
            crate::AmbientLoopSoundCommand {
                sound: test_resolved_sound("minecraft:ambient.new"),
                channel_gain: 0.25,
            },
        ];

        let plan = ambient_loop_sync_plan(
            ["minecraft:ambient.old", "minecraft:ambient.keep"],
            &desired,
        );

        assert_eq!(
            plan,
            AmbientLoopSyncPlan {
                updates: vec![("minecraft:ambient.keep".to_string(), 0)],
                starts: vec![1],
            }
        );
    }

    fn test_resolved_sound(event_id: &str) -> ResolvedSound {
        ResolvedSound {
            event_id: event_id.to_string(),
            sound_name: event_id
                .strip_prefix("minecraft:")
                .unwrap_or(event_id)
                .replace('.', "/"),
            ogg_path: format!("sounds/{event_id}.ogg").into(),
            resource_generation: 0,
            stream: false,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        }
    }
}
