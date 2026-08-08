use bbb_control::AudioCounters;
use bbb_world::{
    ClientBackgroundMusic, ClientEnvironmentAttributes, ClientGameEventScreenState, ClientMusic,
    ClientSoundEventHolder, EnvironmentBiomeSampling, LevelEventSoundRandomState,
    LocalSoundEventState, SoundHolderState, WorldStore, DIMENSION_TYPE_REGISTRY,
};

use crate::{audio_runtime::AudioEventSink, camera_pose::camera_pose_from_world};

const END_DIMENSION: &str = "minecraft:the_end";
const END_MUSIC_EVENT: &str = "minecraft:music.end";
const DRAGON_MUSIC_EVENT: &str = "minecraft:music.dragon";
const CREDITS_MUSIC_EVENT: &str = "minecraft:music.credits";
const STARTING_DELAY_TICKS: i32 = 100;
const MIN_GAIN_STEP: f32 = 0.0005;
const MAX_GAIN_STEP: f32 = 0.005;
const STOP_GAIN_THRESHOLD: f32 = 0.0001;

/// Non-serializable owner of Vanilla `MusicManager` state.
#[derive(Debug)]
pub(crate) struct MusicRuntime {
    connection_generation: Option<u64>,
    current_music: Option<String>,
    current_sound_name: Option<String>,
    current_gain: f32,
    next_song_delay: i32,
    max_frequency_ticks: i32,
    final_music_volume: f32,
    random: LevelEventSoundRandomState,
    pending_start: Option<AudioCommandObservation>,
    pending_stop: Option<PendingMusicStop>,
    active_win_generation: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AudioCommandObservation {
    commands_submitted: u64,
    resolve_failures: u64,
    submit_failures: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingMusicStop {
    observation: AudioCommandObservation,
    event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectedMusic {
    event_id: String,
    min_delay: i32,
    max_delay: i32,
    replace_current_music: bool,
}

impl Default for MusicRuntime {
    fn default() -> Self {
        Self {
            connection_generation: None,
            current_music: None,
            current_sound_name: None,
            current_gain: 1.0,
            next_song_delay: STARTING_DELAY_TICKS,
            max_frequency_ticks: crate::startup::MusicFrequencyArg::Default.max_frequency_ticks(),
            final_music_volume: 1.0,
            random: LevelEventSoundRandomState::default(),
            pending_start: None,
            pending_stop: None,
            active_win_generation: None,
        }
    }
}

impl MusicRuntime {
    pub(crate) fn configured(
        frequency: crate::startup::MusicFrequencyArg,
        final_music_volume: f32,
    ) -> Self {
        Self {
            max_frequency_ticks: frequency.max_frequency_ticks(),
            final_music_volume: final_music_volume.clamp(0.0, 1.0),
            ..Self::default()
        }
    }

    pub(crate) fn advance_for_ticks(
        &mut self,
        world: &mut WorldStore,
        connection_generation: u64,
        raw_client_ticks: u32,
        audio: &mut dyn AudioEventSink,
    ) {
        if self.connection_generation != Some(connection_generation) {
            let previous_win_generation = self.active_win_generation.take();
            self.connection_generation = Some(connection_generation);
            let counters = audio.counters();
            self.current_music = counters.active_music_event_id;
            self.current_sound_name = counters.active_music_sound_name;
            self.pending_start = None;
            self.pending_stop = None;
            if previous_win_generation.is_some()
                && self.current_music.as_deref() == Some(CREDITS_MUSIC_EVENT)
            {
                let observation = AudioCommandObservation::from(&audio.counters());
                self.stop_current(world, observation, audio, true);
            }
            if self.current_music.is_none() {
                self.next_song_delay = self.next_song_delay.max(STARTING_DELAY_TICKS);
            }
        }
        self.synchronize_win_screen(world, audio);
        for _ in 0..raw_client_ticks {
            self.tick(world, audio);
        }
    }

    fn synchronize_win_screen(&mut self, world: &mut WorldStore, audio: &mut dyn AudioEventSink) {
        let generation = match world.game_event_screen() {
            Some(ClientGameEventScreenState::Win { generation }) => Some(generation),
            Some(ClientGameEventScreenState::DemoHelp { .. }) | None => None,
        };
        if generation == self.active_win_generation {
            return;
        }
        if self.active_win_generation.is_some()
            && self.current_music.as_deref() == Some(CREDITS_MUSIC_EVENT)
        {
            let observation = AudioCommandObservation::from(&audio.counters());
            self.stop_current(world, observation, audio, true);
        }
        self.active_win_generation = generation;
        if generation.is_some() {
            self.next_song_delay = 0;
        }
    }

    fn tick(&mut self, world: &mut WorldStore, audio: &mut dyn AudioEventSink) {
        let counters = audio.counters();
        let observation = AudioCommandObservation::from(&counters);
        let active_music = counters.active_music_event_id.as_deref();
        let active_sound_name = counters.active_music_sound_name.as_deref();
        let backend_ended =
            self.observe_backend(world, active_music, active_sound_name, observation);

        let target_gain = music_volume(world);
        if self.current_music.is_some() && self.current_gain != target_gain {
            if !self.fade_playing(world, target_gain, observation, audio) {
                return;
            }
        }

        let selected = situational_music(world);
        let Some(selected) = selected else {
            self.next_song_delay = self.next_song_delay.max(STARTING_DELAY_TICKS);
            return;
        };

        if backend_ended {
            self.next_song_delay = self.next_song_delay.min(next_song_delay(
                &selected,
                self.max_frequency_ticks,
                &mut self.random,
            ));
        }

        if let Some(current_music) = self.current_music.as_deref() {
            if can_replace(&selected, current_music) {
                self.stop_current(world, observation, audio, false);
                self.next_song_delay =
                    next_int_inclusive(&mut self.random, 0, selected.min_delay / 2);
            }
        }

        self.next_song_delay = self.next_song_delay.min(next_song_delay(
            &selected,
            self.max_frequency_ticks,
            &mut self.random,
        ));
        if self.current_music.is_none() {
            let should_start = self.next_song_delay <= 0;
            self.next_song_delay = self.next_song_delay.saturating_sub(1);
            if should_start {
                self.start_playing(world, selected, observation, audio);
            }
        }
    }

    fn observe_backend(
        &mut self,
        world: &mut WorldStore,
        active_music: Option<&str>,
        active_sound_name: Option<&str>,
        observation: AudioCommandObservation,
    ) -> bool {
        if let Some(pending) = self.pending_start {
            if pending == observation {
                return false;
            }
            self.pending_start = None;
            if active_music == self.current_music.as_deref() {
                self.pending_stop = None;
                self.current_sound_name = active_sound_name.map(str::to_string);
                if let Some(sound_name) = self.current_sound_name.clone() {
                    if self.final_music_volume > 0.0 {
                        world.show_now_playing_music(sound_name);
                    } else {
                        world.clear_now_playing_music();
                    }
                }
                return false;
            }
            self.current_music = None;
            self.current_sound_name = None;
            world.clear_now_playing_music();
            return true;
        }

        if let Some(pending) = self.pending_stop.as_ref() {
            let command_is_unobserved = pending.observation == observation;
            let stopped_instance_is_retained = pending
                .event_id
                .as_deref()
                .is_some_and(|event_id| active_music == Some(event_id));
            if command_is_unobserved || stopped_instance_is_retained {
                return false;
            }
            self.pending_stop = None;
        }

        match (self.current_music.as_deref(), active_music) {
            (Some(current), Some(active)) if current == active => false,
            (Some(_), None) => {
                self.current_music = None;
                self.current_sound_name = None;
                world.clear_now_playing_music();
                true
            }
            (_, Some(active)) => {
                self.current_music = Some(active.to_string());
                self.current_sound_name = active_sound_name.map(str::to_string);
                false
            }
            (None, None) => false,
        }
    }

    fn fade_playing(
        &mut self,
        world: &mut WorldStore,
        target_gain: f32,
        observation: AudioCommandObservation,
        audio: &mut dyn AudioEventSink,
    ) -> bool {
        if self.current_gain < target_gain {
            self.current_gain += self.current_gain.clamp(MIN_GAIN_STEP, MAX_GAIN_STEP);
            self.current_gain = self.current_gain.min(target_gain);
        } else {
            self.current_gain = 0.03 * target_gain + 0.97 * self.current_gain;
            if (self.current_gain - target_gain).abs() < STOP_GAIN_THRESHOLD
                || self.current_gain < target_gain
            {
                self.current_gain = target_gain;
            }
        }
        self.current_gain = self.current_gain.clamp(0.0, 1.0);
        if self.current_gain <= STOP_GAIN_THRESHOLD {
            self.stop_current(world, observation, audio, true);
            self.next_song_delay = self.next_song_delay.saturating_add(STARTING_DELAY_TICKS);
            false
        } else {
            audio.set_music_gain(self.current_gain);
            true
        }
    }

    fn stop_current(
        &mut self,
        world: &mut WorldStore,
        observation: AudioCommandObservation,
        audio: &mut dyn AudioEventSink,
        hide_toast: bool,
    ) {
        let stopped_event_id = self.current_music.clone();
        audio.stop_music();
        self.current_music = None;
        self.current_sound_name = None;
        self.pending_start = None;
        self.pending_stop = Some(PendingMusicStop {
            observation,
            event_id: stopped_event_id,
        });
        if hide_toast {
            world.hide_now_playing_music();
        } else {
            world.clear_now_playing_music();
        }
    }

    fn start_playing(
        &mut self,
        world: &mut WorldStore,
        selected: SelectedMusic,
        observation: AudioCommandObservation,
        audio: &mut dyn AudioEventSink,
    ) {
        audio.play_local_sound(&music_sound(world, &selected.event_id));
        self.current_music = Some(selected.event_id);
        self.pending_start = Some(observation);
        self.next_song_delay = i32::MAX;
    }
}

impl From<&AudioCounters> for AudioCommandObservation {
    fn from(counters: &AudioCounters) -> Self {
        Self {
            commands_submitted: counters.commands_submitted,
            resolve_failures: counters.resolve_failures,
            submit_failures: counters.submit_failures,
        }
    }
}

fn situational_music(world: &WorldStore) -> Option<SelectedMusic> {
    if matches!(
        world.game_event_screen(),
        Some(ClientGameEventScreenState::Win { .. })
    ) {
        return Some(SelectedMusic {
            event_id: CREDITS_MUSIC_EVENT.to_string(),
            min_delay: 0,
            max_delay: 0,
            replace_current_music: true,
        });
    }
    let wants_dragon = world
        .level_info()
        .is_some_and(|level| level.dimension == END_DIMENSION)
        && world.boss_overlay_should_play_music();
    if wants_dragon {
        return Some(SelectedMusic {
            event_id: DRAGON_MUSIC_EVENT.to_string(),
            min_delay: 0,
            max_delay: 0,
            replace_current_music: true,
        });
    }
    selected_background_music(world)
}

fn can_replace(selected: &SelectedMusic, current_music: &str) -> bool {
    selected.replace_current_music && selected.event_id != current_music
}

fn music_volume(world: &WorldStore) -> f32 {
    camera_environment_attributes(world)
        .and_then(|attributes| attributes.music_volume)
        .filter(|volume| volume.is_finite())
        .unwrap_or(1.0)
        .clamp(0.0, 1.0)
}

fn music_sound(world: &mut WorldStore, event_id: &str) -> LocalSoundEventState {
    LocalSoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(event_id.to_string()),
            fixed_range: None,
        },
        source: "music".to_string(),
        volume: 1.0,
        pitch: 1.0,
        seed: world.next_sound_seed(),
    }
}

fn selected_background_music(world: &WorldStore) -> Option<SelectedMusic> {
    let attributes = camera_environment_attributes(world);
    if let Some(background) = attributes
        .as_ref()
        .and_then(|attributes| attributes.background_music.as_ref())
    {
        return selected_background_music_from(
            background,
            world
                .local_player()
                .abilities
                .is_some_and(|abilities| abilities.instabuild && abilities.can_fly),
            world.local_player_eye_in_water(),
        );
    }

    // Focused/offline fixtures without synchronized dimension contents still
    // use Vanilla's built-in End dimension contract. A synchronized custom
    // dimension type with no background attribute deliberately stays empty.
    (world
        .level_info()
        .is_some_and(|level| level.dimension == END_DIMENSION)
        && world.registry_content(DIMENSION_TYPE_REGISTRY).is_none())
    .then(|| SelectedMusic {
        event_id: END_MUSIC_EVENT.to_string(),
        min_delay: 6_000,
        max_delay: 24_000,
        replace_current_music: true,
    })
}

fn camera_environment_attributes(world: &WorldStore) -> Option<ClientEnvironmentAttributes> {
    let sampled = camera_pose_from_world(world).map_or_else(
        || world.sampled_client_environment_attributes(),
        |camera| {
            world.sampled_client_environment_attributes_at(
                [
                    f64::from(camera.position[0]),
                    f64::from(camera.position[1] + camera.eye_height),
                    f64::from(camera.position[2]),
                ],
                EnvironmentBiomeSampling::Gaussian,
            )
        },
    );
    sampled.ok().or_else(|| {
        world
            .level_info()
            .map(|level| level.environment_attributes())
    })
}

fn selected_background_music_from(
    background: &ClientBackgroundMusic,
    creative: bool,
    underwater: bool,
) -> Option<SelectedMusic> {
    let music = if underwater {
        background.underwater.as_ref()
    } else {
        None
    }
    .or_else(|| creative.then_some(background.creative.as_ref()).flatten())
    .or(background.default.as_ref())?;
    Some(selected_music(music))
}

fn selected_music(music: &ClientMusic) -> SelectedMusic {
    let event_id = match &music.sound {
        ClientSoundEventHolder::Reference { resource_id } => resource_id.clone(),
        ClientSoundEventHolder::Direct { sound_id, .. } => sound_id.clone(),
    };
    SelectedMusic {
        event_id,
        min_delay: music.min_delay,
        max_delay: music.max_delay,
        replace_current_music: music.replace_current_music,
    }
}

fn next_song_delay(
    selected: &SelectedMusic,
    max_frequency_ticks: i32,
    random: &mut LevelEventSoundRandomState,
) -> i32 {
    if max_frequency_ticks == 0 {
        return STARTING_DELAY_TICKS;
    }
    next_int_inclusive(
        random,
        selected.min_delay.min(max_frequency_ticks),
        selected.max_delay.min(max_frequency_ticks),
    )
}

fn next_int_inclusive(
    random: &mut LevelEventSoundRandomState,
    min_inclusive: i32,
    max_inclusive: i32,
) -> i32 {
    if min_inclusive >= max_inclusive {
        min_inclusive
    } else {
        random.next_int_bound(max_inclusive - min_inclusive + 1) + min_inclusive
    }
}

#[cfg(test)]
mod tests {
    use bbb_audio::TickEntitySoundPositionsCommand;
    use bbb_pack::{JukeboxSongRegistry, SoundEventRegistry};
    use bbb_protocol::packets::{
        BossBarColor, BossBarOverlay, BossEvent, BossEventFlags, BossEventOperation,
        CommonPlayerSpawnInfo, GameEvent, PlayLogin,
    };
    use bbb_world::{
        JukeboxLevelEventState, SoundEntityEventState, SoundEventState, StopSoundEventState,
        TickableEntitySoundState,
    };

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum RecordedAudioEffect {
        StopMusic,
        SetMusicGain(f32),
        PlayLocal(LocalSoundEventState),
    }

    #[derive(Default)]
    struct RecordingAudio {
        counters: AudioCounters,
        effects: Vec<RecordedAudioEffect>,
    }

    impl AudioEventSink for RecordingAudio {
        fn counters(&self) -> AudioCounters {
            self.counters.clone()
        }

        fn set_sound_event_registry(&mut self, _registry: SoundEventRegistry) {}

        fn set_jukebox_song_registry(&mut self, _registry: JukeboxSongRegistry) {}

        fn play_local_sound(&mut self, state: &LocalSoundEventState) {
            self.effects
                .push(RecordedAudioEffect::PlayLocal(state.clone()));
        }

        fn play_positioned_sound(&mut self, _state: &SoundEventState) {}

        fn play_entity_sound(
            &mut self,
            _state: &SoundEntityEventState,
            _position: Option<[f64; 3]>,
        ) {
        }

        fn play_jukebox_song(&mut self, _state: &JukeboxLevelEventState) {}

        fn stop_jukebox_song(&mut self, _state: &JukeboxLevelEventState) {}

        fn stop_sound(&mut self, state: &StopSoundEventState) {
            assert_eq!(state.source.as_deref(), Some("music"));
            assert_eq!(state.name, None);
            self.effects.push(RecordedAudioEffect::StopMusic);
        }

        fn set_music_gain(&mut self, gain: f32) {
            self.effects.push(RecordedAudioEffect::SetMusicGain(gain));
        }

        fn sync_ambient_loops(&mut self, _loops: &[bbb_world::AmbientLoopSoundState]) {}

        fn tick_entity_sound_positions(
            &mut self,
            _command: TickEntitySoundPositionsCommand,
            _tickable_sounds: &[TickableEntitySoundState],
        ) {
        }
    }

    #[test]
    fn non_end_boss_music_flag_is_a_noop() {
        let mut world = world_in_dimension("minecraft:overworld", 0);
        add_music_boss_bar(&mut world);
        let mut runtime = MusicRuntime::default();
        let mut audio = RecordingAudio::default();

        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);

        assert!(audio.effects.is_empty());
    }

    #[test]
    fn win_screen_music_starts_restarts_for_replacement_and_stops_on_removal() {
        let mut world = world_in_dimension("minecraft:overworld", 0);
        let mut runtime = MusicRuntime::default();
        let mut audio = RecordingAudio::default();

        world.apply_game_event(GameEvent {
            event_id: 4,
            param: 0.0,
        });
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        let [RecordedAudioEffect::PlayLocal(first)] = audio.effects.as_slice() else {
            panic!("opening the win screen must start credits music");
        };
        assert_eq!(first.sound.location.as_deref(), Some(CREDITS_MUSIC_EVENT));

        audio.counters.active_music_event_id = Some(CREDITS_MUSIC_EVENT.to_string());
        audio.counters.commands_submitted = 1;
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        assert_eq!(audio.effects.len(), 1);

        world.apply_game_event(GameEvent {
            event_id: 4,
            param: 0.0,
        });
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        assert!(matches!(
            &audio.effects[1..],
            [
                RecordedAudioEffect::StopMusic,
                RecordedAudioEffect::PlayLocal(_)
            ]
        ));

        world.clear_client_level();
        runtime.advance_for_ticks(&mut world, 1, 0, &mut audio);
        assert_eq!(audio.effects.last(), Some(&RecordedAudioEffect::StopMusic));
    }

    #[test]
    fn end_boss_music_stops_other_music_then_starts_once_with_world_seed() {
        let mut world = world_in_dimension(END_DIMENSION, 2);
        add_music_boss_bar(&mut world);
        let mut expected_world = world.clone();
        let expected_seed = expected_world.next_sound_seed();
        let mut runtime = MusicRuntime::default();
        let mut audio = RecordingAudio {
            counters: AudioCounters {
                active_music_event_id: Some(END_MUSIC_EVENT.to_string()),
                ..AudioCounters::default()
            },
            effects: Vec::new(),
        };

        runtime.advance_for_ticks(&mut world, 3, 1, &mut audio);

        assert_eq!(audio.effects.len(), 2);
        assert_eq!(audio.effects[0], RecordedAudioEffect::StopMusic);
        let RecordedAudioEffect::PlayLocal(sound) = &audio.effects[1] else {
            panic!("dragon music must follow the stop command");
        };
        assert_eq!(sound.sound.location.as_deref(), Some(DRAGON_MUSIC_EVENT));
        assert_eq!(sound.source, "music");
        assert_eq!(sound.volume, 1.0);
        assert_eq!(sound.pitch, 1.0);
        assert_eq!(sound.seed, expected_seed);
        assert_eq!(world.next_sound_seed(), expected_world.next_sound_seed());

        // Unchanged backend telemetry means the ordered effect batch has not
        // been observed yet; raw ticks must not enqueue duplicate starts.
        runtime.advance_for_ticks(&mut world, 3, 4, &mut audio);
        assert_eq!(audio.effects.len(), 2);

        audio.counters.active_music_event_id = Some(DRAGON_MUSIC_EVENT.to_string());
        audio.counters.commands_submitted = 2;
        runtime.advance_for_ticks(&mut world, 3, 1, &mut audio);
        assert_eq!(audio.effects.len(), 2);
    }

    #[test]
    fn end_boss_music_restarts_after_backend_reports_natural_completion() {
        let mut world = world_in_dimension(END_DIMENSION, 2);
        add_music_boss_bar(&mut world);
        let mut runtime = MusicRuntime::default();
        let mut audio = RecordingAudio::default();

        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        assert_eq!(audio.effects.len(), 1);
        audio.counters.active_music_event_id = Some(DRAGON_MUSIC_EVENT.to_string());
        audio.counters.commands_submitted = 1;
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        assert_eq!(audio.effects.len(), 1);

        audio.counters.active_music_event_id = None;
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        assert_eq!(audio.effects.len(), 2);
        assert!(matches!(
            audio.effects[1],
            RecordedAudioEffect::PlayLocal(_)
        ));
    }

    #[test]
    fn confirmed_resolved_music_identity_drives_now_playing_show_and_explicit_hide() {
        let mut world = world_in_dimension("minecraft:overworld", 0);
        let mut runtime = MusicRuntime::configured(crate::startup::MusicFrequencyArg::Default, 1.0);
        let mut audio = RecordingAudio::default();
        world.apply_game_event(GameEvent {
            event_id: 4,
            param: 0.0,
        });

        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        assert_eq!(world.now_playing_toast().mutation_sequence, 0);

        audio.counters.active_music_event_id = Some(CREDITS_MUSIC_EVENT.to_string());
        audio.counters.active_music_sound_name =
            Some("minecraft:music/game/end/credits1".to_string());
        audio.counters.commands_submitted = 1;
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        assert_eq!(
            world.now_playing_toast().sound_name.as_deref(),
            Some("minecraft:music/game/end/credits1")
        );
        assert!(world.now_playing_toast().wanted_visible);
        assert_eq!(world.now_playing_toast().mutation_sequence, 1);

        world.clear_client_level();
        runtime.advance_for_ticks(&mut world, 1, 0, &mut audio);
        assert_eq!(world.now_playing_toast().sound_name, None);
        assert!(!world.now_playing_toast().wanted_visible);
        assert_eq!(world.now_playing_toast().mutation_sequence, 2);

        audio.counters.commands_submitted = 2;
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        assert_eq!(runtime.current_music, None);
        assert_eq!(runtime.current_sound_name, None);
        assert_eq!(world.now_playing_toast().mutation_sequence, 2);

        audio.counters.active_music_event_id = None;
        audio.counters.active_music_sound_name = None;
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        assert_eq!(runtime.pending_stop, None);
    }

    #[test]
    fn silently_started_music_retains_identity_without_showing_now_playing() {
        let mut world = world_in_dimension(END_DIMENSION, 2);
        add_music_boss_bar(&mut world);
        let mut runtime = MusicRuntime::configured(crate::startup::MusicFrequencyArg::Default, 0.0);
        let mut audio = RecordingAudio::default();

        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        audio.counters.active_music_event_id = Some(DRAGON_MUSIC_EVENT.to_string());
        audio.counters.active_music_sound_name = Some("minecraft:music/game/end/boss1".to_string());
        audio.counters.commands_submitted = 1;
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);

        assert_eq!(
            runtime.current_sound_name.as_deref(),
            Some("minecraft:music/game/end/boss1")
        );
        assert_eq!(world.now_playing_toast().sound_name, None);
        assert_eq!(world.now_playing_toast().mutation_sequence, 0);
    }

    #[test]
    fn clearing_boss_flag_uses_selected_background_replacement_policy() {
        let mut world = world_in_dimension(END_DIMENSION, 2);
        add_music_boss_bar(&mut world);
        let mut runtime = MusicRuntime::default();
        let mut audio = RecordingAudio::default();
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        audio.counters.active_music_event_id = Some(DRAGON_MUSIC_EVENT.to_string());
        audio.counters.commands_submitted = 1;
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);

        clear_music_boss_flag(&mut world);
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);

        assert_eq!(audio.effects.last(), Some(&RecordedAudioEffect::StopMusic));

        let non_replacing = ClientBackgroundMusic {
            default: Some(ClientMusic {
                sound: ClientSoundEventHolder::Reference {
                    resource_id: "example:ambient".to_string(),
                },
                min_delay: 0,
                max_delay: 0,
                replace_current_music: false,
            }),
            ..ClientBackgroundMusic::default()
        };
        assert_eq!(
            selected_background_music_from(&non_replacing, false, false),
            Some(SelectedMusic {
                event_id: "example:ambient".to_string(),
                min_delay: 0,
                max_delay: 0,
                replace_current_music: false,
            })
        );
    }

    #[test]
    fn clearing_boss_flag_orders_stop_after_an_unobserved_pending_start() {
        let mut world = world_in_dimension(END_DIMENSION, 2);
        add_music_boss_bar(&mut world);
        let mut runtime = MusicRuntime::default();
        let mut audio = RecordingAudio::default();

        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        clear_music_boss_flag(&mut world);
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);

        assert!(matches!(
            audio.effects.as_slice(),
            [
                RecordedAudioEffect::PlayLocal(_),
                RecordedAudioEffect::StopMusic
            ]
        ));
    }

    #[test]
    fn background_selection_prefers_underwater_then_creative_then_default() {
        let music = |event_id: &str| ClientMusic {
            sound: ClientSoundEventHolder::Direct {
                sound_id: event_id.to_string(),
                range_bits: None,
            },
            min_delay: 0,
            max_delay: 0,
            replace_current_music: false,
        };
        let background = ClientBackgroundMusic {
            default: Some(music("example:default")),
            creative: Some(music("example:creative")),
            underwater: Some(music("example:underwater")),
        };

        assert_eq!(
            selected_background_music_from(&background, true, true)
                .unwrap()
                .event_id,
            "example:underwater"
        );
        assert_eq!(
            selected_background_music_from(&background, true, false)
                .unwrap()
                .event_id,
            "example:creative"
        );
        assert_eq!(
            selected_background_music_from(&background, false, false)
                .unwrap()
                .event_id,
            "example:default"
        );
    }

    #[test]
    fn ordinary_end_music_waits_the_initial_hundred_tick_delay() {
        let mut world = world_in_dimension(END_DIMENSION, 2);
        let mut runtime = MusicRuntime::default();
        let mut audio = RecordingAudio::default();

        runtime.advance_for_ticks(&mut world, 1, 100, &mut audio);
        assert!(audio.effects.is_empty());

        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        let [RecordedAudioEffect::PlayLocal(sound)] = audio.effects.as_slice() else {
            panic!("ordinary End music must start after the initial delay");
        };
        assert_eq!(sound.sound.location.as_deref(), Some(END_MUSIC_EVENT));
        assert_eq!(runtime.next_song_delay, i32::MAX);
    }

    #[test]
    fn backend_start_failure_reenters_the_zero_delay_boss_schedule() {
        let mut world = world_in_dimension(END_DIMENSION, 2);
        add_music_boss_bar(&mut world);
        let mut runtime = MusicRuntime::default();
        let mut audio = RecordingAudio::default();

        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        audio.counters.resolve_failures = 1;
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);

        assert_eq!(
            audio
                .effects
                .iter()
                .filter(|effect| matches!(effect, RecordedAudioEffect::PlayLocal(_)))
                .count(),
            2
        );
    }

    #[test]
    fn gain_fades_follow_vanilla_rise_fall_and_zero_stop_rules() {
        let mut runtime = MusicRuntime {
            current_music: Some("example:music".to_string()),
            ..MusicRuntime::default()
        };
        let mut audio = RecordingAudio::default();
        let mut world = WorldStore::new();
        let observation = AudioCommandObservation::from(&audio.counters);

        assert!(runtime.fade_playing(&mut world, 0.0, observation, &mut audio));
        assert!((runtime.current_gain - 0.97).abs() < 0.0001);
        assert_eq!(
            audio.effects.last(),
            Some(&RecordedAudioEffect::SetMusicGain(0.97))
        );

        runtime.current_gain = 0.001;
        assert!(runtime.fade_playing(&mut world, 1.0, observation, &mut audio));
        assert!((runtime.current_gain - 0.002).abs() < 0.0001);

        runtime.current_gain = 0.0001;
        assert!(!runtime.fade_playing(&mut world, 0.0, observation, &mut audio));
        assert_eq!(runtime.current_music, None);
        assert_eq!(audio.effects.last(), Some(&RecordedAudioEffect::StopMusic));
    }

    #[test]
    fn default_frequency_clamps_ranges_and_mth_inclusive_order() {
        let mut random = LevelEventSoundRandomState::default();
        let bounded = SelectedMusic {
            event_id: "example:bounded".to_string(),
            min_delay: 10,
            max_delay: 20,
            replace_current_music: false,
        };
        let delay = next_song_delay(
            &bounded,
            crate::startup::MusicFrequencyArg::Default.max_frequency_ticks(),
            &mut random,
        );
        assert!((10..=20).contains(&delay));

        let clamped = SelectedMusic {
            event_id: "example:clamped".to_string(),
            min_delay: 30_000,
            max_delay: 40_000,
            replace_current_music: false,
        };
        assert_eq!(
            next_song_delay(
                &clamped,
                crate::startup::MusicFrequencyArg::Default.max_frequency_ticks(),
                &mut random,
            ),
            crate::startup::MusicFrequencyArg::Default.max_frequency_ticks()
        );
        assert_eq!(next_int_inclusive(&mut random, 9, 3), 9);
        assert_eq!(
            next_song_delay(
                &clamped,
                crate::startup::MusicFrequencyArg::Frequent.max_frequency_ticks(),
                &mut random,
            ),
            12_000
        );
        assert_eq!(
            next_song_delay(
                &bounded,
                crate::startup::MusicFrequencyArg::Constant.max_frequency_ticks(),
                &mut random,
            ),
            STARTING_DELAY_TICKS
        );
    }

    #[test]
    fn connection_change_discards_pending_start_and_restarts_current_selection() {
        let mut world = world_in_dimension(END_DIMENSION, 2);
        add_music_boss_bar(&mut world);
        let mut runtime = MusicRuntime::default();
        let mut audio = RecordingAudio::default();

        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        runtime.advance_for_ticks(&mut world, 2, 1, &mut audio);

        assert_eq!(
            audio
                .effects
                .iter()
                .filter(|effect| matches!(effect, RecordedAudioEffect::PlayLocal(_)))
                .count(),
            2
        );
    }

    #[test]
    fn music_effects_enter_fifo_before_entity_sound_position_tick() {
        let runtime = include_str!("runtime.rs");
        let advance = runtime
            .split_once("pub(crate) fn advance_simulation(")
            .expect("advance_simulation")
            .1
            .split_once("pub(crate) fn update_renderer_from_publication(")
            .expect("advance_simulation end")
            .0;
        let music = advance
            .find("music.advance_for_ticks(")
            .expect("music tick");
        let entity_positions = advance
            .find("audio_events.tick_entity_sound_positions(")
            .expect("entity sound position tick");

        assert!(music < entity_positions);
    }

    fn world_in_dimension(dimension: &str, dimension_type_id: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 1,
            hardcore: false,
            levels: vec![dimension.to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 8,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id,
                dimension: dimension.to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        });
        world
    }

    fn add_music_boss_bar(world: &mut WorldStore) {
        assert!(world.apply_boss_event(BossEvent {
            id: uuid::Uuid::from_u128(1),
            operation: BossEventOperation::Add {
                name: Box::new(bbb_protocol::RichTextComponent::plain("Dragon")),
                progress: 1.0,
                color: BossBarColor::Purple,
                overlay: BossBarOverlay::Progress,
                flags: BossEventFlags {
                    darken_screen: false,
                    play_music: true,
                    create_world_fog: false,
                },
            },
        }));
    }

    fn clear_music_boss_flag(world: &mut WorldStore) {
        assert!(world.apply_boss_event(BossEvent {
            id: uuid::Uuid::from_u128(1),
            operation: BossEventOperation::UpdateProperties {
                flags: BossEventFlags {
                    darken_screen: false,
                    play_music: false,
                    create_world_fog: false,
                },
            },
        }));
    }
}
