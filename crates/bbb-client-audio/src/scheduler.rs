//! Backend-agnostic voice scheduling, category gain scaling, 20-tick source
//! retention, channel pooling, and distance-delay queuing shared by desktop and
//! WebAudio runtimes.

use std::collections::{HashMap, VecDeque};

#[cfg(any(feature = "device", all(feature = "web", target_arch = "wasm32")))]
use crate::AudioListenerState;
#[cfg(test)]
use crate::EntitySoundPosition;
use crate::{
    AmbientLoopSoundCommand, AudioCategory, AudioPlaybackEvent, PlayPositionedSoundCommand,
    ResolvedSound, StopJukeboxSongCommand, StopSoundCommand, TickEntitySoundPositionsCommand,
};

pub(crate) const MIN_SPATIAL_DISTANCE: f32 = 1.0;
pub(crate) const DISTANCE_DELAY_THRESHOLD_SQR: f64 = 100.0;
pub(crate) const DISTANCE_DELAY_TICKS_PER_BLOCK: f64 = 0.5;
pub(crate) const MAX_PLAYBACK_EVENTS: usize = 1024;
pub(crate) const MINIMUM_SOURCE_LIFETIME_TICKS: u64 = 20;

pub(crate) trait VoiceHandle {
    fn stop(&mut self);
    fn is_active(&self) -> bool;
    fn holds_channel(&self) -> bool;
    fn is_streaming(&self) -> bool;
    fn set_volume(&mut self, gain: f32);
    fn set_playback_rate(&mut self, playback_rate: f32);
    fn set_position(&mut self, position: [f32; 3]);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AudioChannelLimits {
    pub(crate) static_capacity: usize,
    pub(crate) streaming_capacity: usize,
}

impl AudioChannelLimits {
    /// Vanilla 26.1 `Library.init`: streaming=floor(sqrt(total)).clamp(2, 8),
    /// static=(total-streaming).clamp(8, 255).
    pub(crate) fn from_mono_sources(mono_sources: usize) -> Self {
        let streaming_capacity = ((mono_sources as f32).sqrt() as usize).clamp(2, 8);
        let static_capacity = mono_sources
            .saturating_sub(streaming_capacity)
            .clamp(8, 255);
        Self {
            static_capacity,
            streaming_capacity,
        }
    }

    pub(crate) fn admits(self, streaming: bool, static_used: usize, streaming_used: usize) -> bool {
        if streaming {
            streaming_used < self.streaming_capacity
        } else {
            static_used < self.static_capacity
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DelayedPositionedSound {
    pub(crate) command: PlayPositionedSoundCommand,
    pub(crate) remaining_ticks: u32,
}

pub(crate) struct ScheduledVoice<H> {
    pub(crate) event_id: String,
    pub(crate) sound_name: String,
    pub(crate) category: AudioCategory,
    pub(crate) entity_id: Option<i32>,
    pub(crate) position: [f32; 3],
    pub(crate) jukebox_pos: Option<[i32; 3]>,
    pub(crate) handle: H,
    pub(crate) tickable: bool,
    pub(crate) ambient_loop: bool,
    pub(crate) base_channel_gain: f32,
    pub(crate) playback_rate: f32,
    pub(crate) minimum_delete_tick: u64,
}

impl<H: VoiceHandle> ScheduledVoice<H> {
    pub(crate) fn set_position(&mut self, position: [f64; 3]) {
        let position = audio_position(position);
        if self.position == position {
            return;
        }
        self.handle.set_position(position);
        self.position = position;
    }

    pub(crate) fn is_retained_at(&self, tick_count: u64) -> bool {
        self.handle.is_active() || stopped_sound_is_retained(tick_count, self.minimum_delete_tick)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PreparedVoiceLaunch {
    pub(crate) effective_gain: f32,
    pub(crate) playback_rate: f32,
    pub(crate) position: [f32; 3],
    pub(crate) max_distance: f32,
}

pub(crate) struct VoiceScheduler<H> {
    pub(crate) listener_position: [f64; 3],
    pub(crate) delayed_positioned: Vec<DelayedPositionedSound>,
    pub(crate) playing: Vec<ScheduledVoice<H>>,
    pub(crate) music_gain: f32,
    pub(crate) channel_limits: AudioChannelLimits,
    pub(crate) channel_admission_rejections: u64,
    pub(crate) tick_count: u64,
    pub(crate) playback_events: VecDeque<AudioPlaybackEvent>,
    scratch_entity_positions: HashMap<i32, [f64; 3]>,
}

impl<H: VoiceHandle> VoiceScheduler<H> {
    pub(crate) fn new(channel_limits: AudioChannelLimits) -> Self {
        Self {
            listener_position: [0.0; 3],
            delayed_positioned: Vec::new(),
            playing: Vec::new(),
            music_gain: 1.0,
            channel_limits,
            channel_admission_rejections: 0,
            tick_count: 0,
            playback_events: VecDeque::new(),
            scratch_entity_positions: HashMap::new(),
        }
    }

    pub(crate) fn reset_generation(&mut self) {
        self.delayed_positioned.clear();
        self.playback_events.clear();
        self.music_gain = 1.0;
        self.stop_and_clear_playing();
    }

    pub(crate) fn stop_and_clear_playing(&mut self) {
        for sound in &mut self.playing {
            sound.handle.stop();
        }
        self.playing.clear();
    }

    pub(crate) fn active_music_event_id(&self) -> Option<&str> {
        self.latest_retained_music()
            .map(|sound| sound.event_id.as_str())
    }

    pub(crate) fn active_music_sound_name(&self) -> Option<&str> {
        self.latest_retained_music()
            .map(|sound| sound.sound_name.as_str())
    }

    fn latest_retained_music(&self) -> Option<&ScheduledVoice<H>> {
        self.playing
            .iter()
            .rev()
            .find(|sound| sound.category == AudioCategory::Music)
    }

    pub(crate) fn drain_playback_events(
        &mut self,
    ) -> impl Iterator<Item = AudioPlaybackEvent> + '_ {
        self.playback_events.drain(..)
    }

    pub(crate) fn entity_sound_ids(&self) -> impl Iterator<Item = i32> + '_ {
        self.playing.iter().filter_map(|sound| sound.entity_id)
    }

    pub(crate) fn retain_active_sounds(&mut self) {
        let tick_count = self.tick_count;
        self.playing
            .retain(|sound| sound.is_retained_at(tick_count));
    }

    pub(crate) fn channel_counts(&self) -> (usize, usize) {
        active_channel_counts(
            self.playing
                .iter()
                .map(|sound| (sound.handle.is_streaming(), sound.handle.holds_channel())),
        )
    }

    pub(crate) fn try_admit_channel(&mut self, streaming: bool) -> bool {
        let (static_used, streaming_used) = self.channel_counts();
        if self
            .channel_limits
            .admits(streaming, static_used, streaming_used)
        {
            true
        } else {
            self.channel_admission_rejections = self.channel_admission_rejections.saturating_add(1);
            false
        }
    }

    pub(crate) fn prepare_local_launch(
        &mut self,
        sound: &ResolvedSound,
        category: &AudioCategory,
        channel_gain: f32,
        playback_rate: f32,
    ) -> Option<PreparedVoiceLaunch> {
        self.retain_active_sounds();
        push_bounded_playback_event(&mut self.playback_events, local_playback_event(sound));
        if !sound_should_start(channel_gain, category) {
            return None;
        }
        if !self.try_admit_channel(sound.stream) {
            return None;
        }
        Some(PreparedVoiceLaunch {
            effective_gain: effective_channel_gain(category, channel_gain, self.music_gain),
            playback_rate: channel_playback_rate(playback_rate),
            position: [0.0; 3],
            max_distance: MIN_SPATIAL_DISTANCE + f32::EPSILON,
        })
    }

    pub(crate) fn enqueue_if_delayed(&mut self, command: &PlayPositionedSoundCommand) -> bool {
        self.retain_active_sounds();
        if let Some(delayed) = delayed_positioned_sound_for_command(command, self.listener_position)
        {
            self.delayed_positioned.push(delayed);
            return true;
        }
        false
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_spatial_launch(
        &mut self,
        sound: &ResolvedSound,
        category: &AudioCategory,
        position: [f64; 3],
        gain: f32,
        channel_gain: f32,
        playback_rate: f32,
        fixed_range: Option<f32>,
        linear_attenuation: bool,
        tickable: bool,
    ) -> Option<PreparedVoiceLaunch> {
        self.retain_active_sounds();
        push_bounded_playback_event(
            &mut self.playback_events,
            spatial_playback_event(sound, position, gain, fixed_range, linear_attenuation),
        );
        if !tickable && !sound_should_start(channel_gain, category) {
            return None;
        }
        if !self.try_admit_channel(sound.stream) {
            return None;
        }
        Some(PreparedVoiceLaunch {
            effective_gain: effective_channel_gain(category, channel_gain, self.music_gain),
            playback_rate: channel_playback_rate(playback_rate),
            position: audio_position(position),
            max_distance: spatial_max_distance(sound, gain, fixed_range),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn push_voice(
        &mut self,
        sound: &ResolvedSound,
        category: AudioCategory,
        entity_id: Option<i32>,
        position: [f32; 3],
        jukebox_pos: Option<[i32; 3]>,
        handle: H,
        tickable: bool,
        ambient_loop: bool,
        base_channel_gain: f32,
        playback_rate: f32,
    ) {
        self.playing.push(ScheduledVoice {
            event_id: sound.event_id.clone(),
            sound_name: sound.sound_name.clone(),
            category,
            entity_id,
            position,
            jukebox_pos,
            handle,
            tickable,
            ambient_loop,
            base_channel_gain,
            playback_rate,
            minimum_delete_tick: self
                .tick_count
                .saturating_add(MINIMUM_SOURCE_LIFETIME_TICKS),
        });
    }

    pub(crate) fn stop_sounds(&mut self, command: &StopSoundCommand) {
        if stop_command_is_stop_all(command) {
            self.delayed_positioned.clear();
            self.music_gain = 1.0;
            self.stop_and_clear_playing();
            return;
        }
        stop_delayed_positioned_sounds(&mut self.delayed_positioned, command);
        for sound in &mut self.playing {
            if stop_filter_matches(&sound.event_id, &sound.category, command) {
                sound.handle.stop();
            }
        }
    }

    pub(crate) fn stop_jukebox_song(&mut self, command: &StopJukeboxSongCommand) {
        for sound in &mut self.playing {
            if sound.jukebox_pos == Some(command.jukebox_pos) {
                sound.handle.stop();
            }
        }
    }

    pub(crate) fn set_music_gain(&mut self, gain: f32) {
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

    pub(crate) fn plan_and_update_ambient_loops(
        &mut self,
        desired: &[AmbientLoopSoundCommand],
    ) -> Vec<usize> {
        self.retain_active_sounds();
        let plan = ambient_loop_sync_plan(
            self.playing
                .iter()
                .filter(|sound| sound.ambient_loop && sound.handle.is_active())
                .map(|sound| sound.event_id.as_str()),
            desired,
        );
        for sound in &mut self.playing {
            if !sound.ambient_loop || !sound.handle.is_active() {
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
            let target = &desired[desired_index];
            sound.base_channel_gain = target.channel_gain;
            sound.handle.set_volume(effective_channel_gain(
                &sound.category,
                target.channel_gain,
                self.music_gain,
            ));
        }
        self.retain_active_sounds();
        plan.starts
    }

    /// Updates entity-bound / tickable voices, retains active/20-tick shells,
    /// and returns delayed positioned sounds ready to start on this tick.
    pub(crate) fn step_entity_and_delayed_voices(
        &mut self,
        command: &TickEntitySoundPositionsCommand,
        mut pre_tick_voice: impl FnMut(&mut H) -> anyhow::Result<()>,
    ) -> anyhow::Result<Vec<PlayPositionedSoundCommand>> {
        self.tick_count = self.tick_count.saturating_add(1);
        let has_entity_voices = self.playing.iter().any(|sound| sound.entity_id.is_some());
        if has_entity_voices {
            self.scratch_entity_positions.clear();
            self.scratch_entity_positions.extend(
                command
                    .entities
                    .iter()
                    .map(|entity| (entity.entity_id, entity.position)),
            );
        }

        for sound in &mut self.playing {
            pre_tick_voice(&mut sound.handle)?;
            if !has_entity_voices {
                continue;
            }
            let Some(entity_id) = sound.entity_id else {
                continue;
            };
            if sound.tickable {
                let Some(update) = command.tickable_sounds.iter().find(|update| {
                    update.entity_id == entity_id && update.event_id == sound.event_id
                }) else {
                    sound.handle.stop();
                    continue;
                };
                sound.set_position(update.position);
                if sound.base_channel_gain != update.channel_gain {
                    sound.base_channel_gain = update.channel_gain;
                    sound.handle.set_volume(effective_channel_gain(
                        &sound.category,
                        update.channel_gain,
                        self.music_gain,
                    ));
                }
                let rate = channel_playback_rate(update.playback_rate);
                sound.playback_rate = rate;
                sound.handle.set_playback_rate(rate);
            } else if let Some(&position) = self.scratch_entity_positions.get(&entity_id) {
                sound.set_position(position);
            } else {
                sound.handle.stop();
            }
        }
        self.retain_active_sounds();
        Ok(advance_delayed_positioned_sounds(
            &mut self.delayed_positioned,
        ))
    }
}

pub(crate) fn active_channel_counts(
    sounds: impl IntoIterator<Item = (bool, bool)>,
) -> (usize, usize) {
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

pub(crate) fn stop_delayed_positioned_sounds(
    delayed: &mut Vec<DelayedPositionedSound>,
    command: &StopSoundCommand,
) {
    if stop_command_is_stop_all(command) {
        delayed.clear();
    }
}

pub(crate) fn delayed_positioned_sound_for_command(
    command: &PlayPositionedSoundCommand,
    listener_position: [f64; 3],
) -> Option<DelayedPositionedSound> {
    vanilla_distance_delay_ticks(command.distance_delay, listener_position, command.position).map(
        |remaining_ticks| DelayedPositionedSound {
            command: command.clone(),
            remaining_ticks,
        },
    )
}

pub(crate) fn advance_delayed_positioned_sounds(
    delayed: &mut Vec<DelayedPositionedSound>,
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

#[cfg(test)]
pub(crate) fn active_entity_sound_positions(
    entities: &[EntitySoundPosition],
) -> HashMap<i32, [f64; 3]> {
    entities
        .iter()
        .map(|entity| (entity.entity_id, entity.position))
        .collect()
}

#[cfg(test)]
pub(crate) fn entity_bound_sound_is_missing(
    entity_id: Option<i32>,
    positions: &HashMap<i32, [f64; 3]>,
) -> bool {
    entity_id.is_some_and(|entity_id| !positions.contains_key(&entity_id))
}

pub(crate) fn stopped_sound_is_retained(tick_count: u64, minimum_delete_tick: u64) -> bool {
    tick_count < minimum_delete_tick
}

pub(crate) fn stop_command_is_stop_all(command: &StopSoundCommand) -> bool {
    command.category.is_none() && command.name.is_none()
}

pub(crate) fn stop_filter_matches(
    event_id: &str,
    category: &AudioCategory,
    command: &StopSoundCommand,
) -> bool {
    let category_matches = command
        .category
        .as_ref()
        .is_none_or(|stop_category| stop_category == category);
    let name_matches = command.name.as_deref().is_none_or(|name| name == event_id);
    category_matches && name_matches
}

pub(crate) fn normalized_gain(gain: f32) -> f32 {
    if gain.is_finite() {
        gain.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub(crate) fn effective_channel_gain(
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
pub(crate) struct AmbientLoopSyncPlan {
    pub(crate) updates: Vec<(String, usize)>,
    pub(crate) starts: Vec<usize>,
}

pub(crate) fn ambient_loop_sync_plan<'a>(
    active_event_ids: impl IntoIterator<Item = &'a str>,
    desired: &[AmbientLoopSoundCommand],
) -> AmbientLoopSyncPlan {
    // If multiple desired commands share an event id, retain the loudest entry
    // so rapid underwater re-entry cannot overwrite a loud loop with a quiet one.
    let mut remaining: HashMap<&str, usize> = HashMap::new();
    for (index, sound) in desired.iter().enumerate() {
        remaining
            .entry(sound.sound.event_id.as_str())
            .and_modify(|existing| {
                if normalized_gain(sound.channel_gain)
                    > normalized_gain(desired[*existing].channel_gain)
                {
                    *existing = index;
                }
            })
            .or_insert(index);
    }
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

pub(crate) fn channel_playback_rate(playback_rate: f32) -> f32 {
    if playback_rate.is_finite() {
        playback_rate.clamp(0.5, 2.0)
    } else {
        1.0
    }
}

pub(crate) fn sound_should_start(gain: f32, category: &AudioCategory) -> bool {
    category == &AudioCategory::Music || java_clamped_volume(gain) != 0.0
}

fn java_clamped_volume(gain: f32) -> f32 {
    gain.clamp(0.0, 1.0)
}

pub(crate) fn spatial_max_distance(
    sound: &ResolvedSound,
    gain: f32,
    fixed_range: Option<f32>,
) -> f32 {
    if let Some(range) = fixed_range {
        if range.is_finite() {
            return range.max(MIN_SPATIAL_DISTANCE + f32::EPSILON);
        }
    }
    let gain = if gain.is_finite() { gain } else { 0.0 };
    (sound.attenuation_distance as f32 * gain.max(1.0)).max(MIN_SPATIAL_DISTANCE + f32::EPSILON)
}

pub(crate) fn local_playback_event(sound: &ResolvedSound) -> AudioPlaybackEvent {
    AudioPlaybackEvent {
        event_id: sound.event_id.clone(),
        position: [0.0, 0.0, 0.0],
        range: f64::INFINITY,
    }
}

pub(crate) fn spatial_playback_event(
    sound: &ResolvedSound,
    position: [f64; 3],
    gain: f32,
    fixed_range: Option<f32>,
    linear_attenuation: bool,
) -> AudioPlaybackEvent {
    let range = if linear_attenuation {
        if let Some(fixed) = fixed_range.filter(|r| r.is_finite()) {
            f64::from(fixed)
        } else {
            let subtitle_gain = if gain.is_nan() {
                f64::NAN
            } else {
                f64::from(gain.max(1.0))
            };
            f64::from(sound.attenuation_distance) * subtitle_gain
        }
    } else {
        f64::INFINITY
    };
    AudioPlaybackEvent {
        event_id: sound.event_id.clone(),
        position,
        range,
    }
}

pub(crate) fn push_bounded_playback_event(
    events: &mut VecDeque<AudioPlaybackEvent>,
    event: AudioPlaybackEvent,
) {
    if events.len() == MAX_PLAYBACK_EVENTS {
        events.pop_front();
    }
    events.push_back(event);
}

pub(crate) fn vanilla_distance_delay_ticks(
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

pub(crate) fn distance_squared(lhs: [f64; 3], rhs: [f64; 3]) -> f64 {
    let dx = lhs[0] - rhs[0];
    let dy = lhs[1] - rhs[1];
    let dz = lhs[2] - rhs[2];
    dx * dx + dy * dy + dz * dz
}

pub(crate) fn audio_position(position: [f64; 3]) -> [f32; 3] {
    position.map(|coord| {
        let value = coord as f32;
        if value.is_finite() {
            value
        } else {
            0.0
        }
    })
}

#[cfg(any(feature = "device", all(feature = "web", target_arch = "wasm32")))]
pub(crate) fn listener_vectors(listener: AudioListenerState) -> ([f32; 3], [f32; 3]) {
    let y_rot = if listener.y_rot.is_finite() {
        listener.y_rot
    } else {
        0.0
    };
    let x_rot = if listener.x_rot.is_finite() {
        listener.x_rot
    } else {
        0.0
    };
    let yaw = y_rot.to_radians();
    let pitch = x_rot.to_radians();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    (
        [-sin_yaw * cos_pitch, -sin_pitch, cos_yaw * cos_pitch],
        [-sin_yaw * sin_pitch, cos_pitch, cos_yaw * sin_pitch],
    )
}

#[cfg(test)]
pub(crate) fn latest_retained_music_event_id<'a>(
    retained: impl IntoIterator<Item = (&'a str, &'a AudioCategory)>,
) -> Option<&'a str> {
    let mut latest = None;
    for (event_id, category) in retained {
        if category == &AudioCategory::Music {
            latest = Some(event_id);
        }
    }
    latest
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockVoiceHandle {
        active: bool,
        streaming: bool,
        volume: f32,
        rate: f32,
        position: [f32; 3],
    }

    impl VoiceHandle for MockVoiceHandle {
        fn stop(&mut self) {
            self.active = false;
        }
        fn is_active(&self) -> bool {
            self.active
        }
        fn holds_channel(&self) -> bool {
            self.active
        }
        fn is_streaming(&self) -> bool {
            self.streaming
        }
        fn set_volume(&mut self, gain: f32) {
            self.volume = normalized_gain(gain);
        }
        fn set_playback_rate(&mut self, playback_rate: f32) {
            self.rate = playback_rate;
        }
        fn set_position(&mut self, position: [f32; 3]) {
            self.position = position;
        }
    }

    fn test_resolved_sound(event_id: &str) -> ResolvedSound {
        ResolvedSound {
            event_id: event_id.to_string(),
            sound_name: event_id.to_string(),
            ogg_path: format!("{event_id}.ogg").into(),
            resource_generation: 0,
            stream: false,
            preload: false,
            attenuation_distance: 16,
            entry_volume: 1.0,
            entry_pitch: 1.0,
        }
    }

    #[test]
    fn channel_limits_split_and_saturate_like_vanilla_library() {
        let low = AudioChannelLimits::from_mono_sources(16);
        assert_eq!(low.streaming_capacity, 4);
        assert_eq!(low.static_capacity, 12);

        let limits = AudioChannelLimits::from_mono_sources(256);
        assert_eq!(limits.static_capacity, 248);
        assert_eq!(limits.streaming_capacity, 8);
        assert!(!limits.admits(false, 248, 0));
        assert!(limits.admits(true, 248, 0));
        assert!(!limits.admits(true, 0, 8));
        assert!(limits.admits(false, 0, 8));
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
    fn stopped_sound_lifetime_matches_vanilla_twenty_tick_boundary() {
        assert!(stopped_sound_is_retained(0, MINIMUM_SOURCE_LIFETIME_TICKS));
        assert!(stopped_sound_is_retained(19, MINIMUM_SOURCE_LIFETIME_TICKS));
        assert!(!stopped_sound_is_retained(
            20,
            MINIMUM_SOURCE_LIFETIME_TICKS
        ));
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
    fn spatial_max_distance_and_fixed_range_follow_vanilla_rules() {
        let sound = test_resolved_sound("minecraft:entity.cat.ambient");
        assert_eq!(spatial_max_distance(&sound, 0.5, None), 16.0);
        assert_eq!(spatial_max_distance(&sound, 2.0, None), 32.0);
        assert_eq!(spatial_max_distance(&sound, f32::NAN, None), 16.0);
        assert_eq!(spatial_max_distance(&sound, 2.0, Some(24.0)), 24.0);
        assert_eq!(spatial_max_distance(&sound, 2.0, Some(f32::NAN)), 32.0);
        assert_eq!(
            spatial_max_distance(&sound, 1.0, Some(0.0)),
            MIN_SPATIAL_DISTANCE + f32::EPSILON
        );
    }

    #[test]
    fn distance_delay_ticks_match_vanilla_threshold_and_flooring() {
        assert_eq!(
            vanilla_distance_delay_ticks(false, [0.0; 3], [40.0, 0.0, 0.0]),
            None
        );
        assert_eq!(
            vanilla_distance_delay_ticks(true, [0.0; 3], [10.0, 0.0, 0.0]),
            None
        );
        assert_eq!(
            vanilla_distance_delay_ticks(true, [0.0; 3], [10.1, 0.0, 0.0]),
            Some(5)
        );
        assert_eq!(
            vanilla_distance_delay_ticks(true, [0.0; 3], [40.0, 0.0, 0.0]),
            Some(20)
        );
    }

    #[test]
    fn delayed_positioned_sounds_release_after_remaining_ticks() {
        let command = PlayPositionedSoundCommand {
            sound: test_resolved_sound("minecraft:entity.generic.explode"),
            category: AudioCategory::Blocks,
            position: [12.0, 0.0, 0.0],
            packet_volume: 1.0,
            packet_pitch: 1.0,
            gain: 1.0,
            channel_gain: 1.0,
            playback_rate: 1.0,
            seed: 7,
            fixed_range: None,
            distance_delay: true,
        };
        let mut delayed = vec![DelayedPositionedSound {
            command: command.clone(),
            remaining_ticks: 2,
        }];
        assert!(advance_delayed_positioned_sounds(&mut delayed).is_empty());
        assert_eq!(delayed.len(), 1);
        assert_eq!(
            advance_delayed_positioned_sounds(&mut delayed),
            vec![command]
        );
        assert!(delayed.is_empty());
    }

    #[test]
    fn stop_sounds_leaves_delayed_positioned_intact_until_stop_all_and_resets_music_gain() {
        let cobweb = PlayPositionedSoundCommand {
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
        let ghast = PlayPositionedSoundCommand {
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
        };

        let mut scheduler =
            VoiceScheduler::<MockVoiceHandle>::new(AudioChannelLimits::from_mono_sources(256));
        assert!(scheduler.enqueue_if_delayed(&cobweb));
        assert!(scheduler.enqueue_if_delayed(&ghast));
        scheduler.set_music_gain(0.0001);

        // Vanilla `SoundEngine.stop` leaves delayed queued sounds intact on scoped stop,
        // clearing them (and resetting music gain) only on stop-all or generation reset.
        scheduler.stop_sounds(&StopSoundCommand {
            category: Some(AudioCategory::Blocks),
            name: None,
        });
        assert_eq!(scheduler.delayed_positioned.len(), 2);
        assert_eq!(scheduler.music_gain, 0.0001);

        scheduler.stop_sounds(&StopSoundCommand {
            category: None,
            name: None,
        });
        assert!(scheduler.delayed_positioned.is_empty());
        assert_eq!(scheduler.music_gain, 1.0);

        scheduler.set_music_gain(0.25);
        scheduler.reset_generation();
        assert_eq!(scheduler.music_gain, 1.0);
    }

    #[test]
    fn music_gain_and_ambient_loop_sync_share_uniform_scheduler_logic() {
        assert_eq!(
            effective_channel_gain(&AudioCategory::Music, 0.8, 0.25),
            0.2
        );
        assert_eq!(
            effective_channel_gain(&AudioCategory::Ambient, 0.8, 0.25),
            0.8
        );

        let mut scheduler =
            VoiceScheduler::<MockVoiceHandle>::new(AudioChannelLimits::from_mono_sources(256));
        scheduler.push_voice(
            &test_resolved_sound("minecraft:ambient.old"),
            AudioCategory::Ambient,
            None,
            [0.0; 3],
            None,
            MockVoiceHandle {
                active: true,
                streaming: false,
                volume: 0.5,
                rate: 1.0,
                position: [0.0; 3],
            },
            false,
            true,
            0.5,
            1.0,
        );
        scheduler.push_voice(
            &test_resolved_sound("minecraft:ambient.keep"),
            AudioCategory::Ambient,
            None,
            [0.0; 3],
            None,
            MockVoiceHandle {
                active: true,
                streaming: false,
                volume: 0.5,
                rate: 1.0,
                position: [0.0; 3],
            },
            false,
            true,
            0.5,
            1.0,
        );
        let desired = [
            AmbientLoopSoundCommand {
                sound: test_resolved_sound("minecraft:ambient.keep"),
                channel_gain: 0.75,
            },
            AmbientLoopSoundCommand {
                sound: test_resolved_sound("minecraft:ambient.new"),
                channel_gain: 0.25,
            },
        ];
        let starts = scheduler.plan_and_update_ambient_loops(&desired);
        assert_eq!(starts, vec![1]);
        assert!(!scheduler.playing[0].handle.is_active());
        assert_eq!(scheduler.playing[1].handle.volume, 0.75);
    }

    #[test]
    fn retained_music_and_playback_event_fifo_match_contract() {
        let music = AudioCategory::Music;
        let blocks = AudioCategory::Blocks;
        let retained = [
            ("minecraft:music.game", &music),
            ("minecraft:block.note_block.harp", &blocks),
            ("minecraft:music.dragon", &music),
        ];
        assert_eq!(
            latest_retained_music_event_id(retained),
            Some("minecraft:music.dragon")
        );

        let mut events = VecDeque::new();
        for index in 0..=MAX_PLAYBACK_EVENTS {
            push_bounded_playback_event(
                &mut events,
                AudioPlaybackEvent {
                    event_id: format!("minecraft:test.{index}"),
                    position: [f64::from(index as u32), 0.0, 0.0],
                    range: 16.0,
                },
            );
        }
        assert_eq!(events.len(), MAX_PLAYBACK_EVENTS);
        assert_eq!(events.front().unwrap().event_id, "minecraft:test.1");
        assert_eq!(events.back().unwrap().event_id, "minecraft:test.1024");

        let sound = test_resolved_sound("minecraft:block.note_block.harp");
        assert_eq!(local_playback_event(&sound).range, f64::INFINITY);
        assert_eq!(
            spatial_playback_event(&sound, [1.0, 2.0, 3.0], 0.25, None, true).range,
            16.0
        );
        assert_eq!(
            spatial_playback_event(&sound, [1.0, 2.0, 3.0], 2.5, None, true).range,
            40.0
        );
        assert_eq!(
            spatial_playback_event(&sound, [1.0, 2.0, 3.0], 2.5, Some(24.0), true).range,
            24.0
        );
        assert_eq!(
            spatial_playback_event(&sound, [1.0, 2.0, 3.0], 1.0, Some(24.0), false).range,
            f64::INFINITY
        );
        assert_eq!(
            audio_position([f64::NAN, f64::MAX, -12.5]),
            [0.0, 0.0, -12.5]
        );

        let active = active_entity_sound_positions(&[EntitySoundPosition {
            entity_id: 123,
            position: [1.0, 2.0, 3.0],
        }]);
        assert!(!entity_bound_sound_is_missing(None, &active));
        assert!(!entity_bound_sound_is_missing(Some(123), &active));
        assert!(entity_bound_sound_is_missing(Some(404), &active));
    }
}
