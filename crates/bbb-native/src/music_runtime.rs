use bbb_control::AudioCounters;
use bbb_world::{
    ClientBackgroundMusic, ClientEnvironmentAttributes, ClientMusic, ClientSoundEventHolder,
    EnvironmentBiomeSampling, LocalSoundEventState, SoundHolderState, WorldStore,
    DIMENSION_TYPE_REGISTRY,
};

use crate::{audio_runtime::AudioEventSink, camera_pose::camera_pose_from_world};

const END_DIMENSION: &str = "minecraft:the_end";
const END_MUSIC_EVENT: &str = "minecraft:music.end";
const DRAGON_MUSIC_EVENT: &str = "minecraft:music.dragon";

/// Non-serializable owner of Vanilla `MusicManager`'s End boss-music edge.
///
/// The general delayed background-music scheduler remains out of scope. This
/// runtime only selects the zero-delay `Musics.END_BOSS`, waits for render-owned
/// backend telemetry before retrying, and applies the ordinary selected
/// background entry's replacement policy when the boss flag clears.
#[derive(Debug, Default)]
pub(crate) struct BossMusicRuntime {
    connection_generation: Option<u64>,
    owns_dragon_music: bool,
    pending_start: Option<AudioCommandObservation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AudioCommandObservation {
    commands_submitted: u64,
    resolve_failures: u64,
    submit_failures: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectedBackgroundMusic {
    event_id: String,
    replace_current_music: bool,
}

impl BossMusicRuntime {
    pub(crate) fn advance_for_ticks(
        &mut self,
        world: &mut WorldStore,
        connection_generation: u64,
        raw_client_ticks: u32,
        audio: &mut dyn AudioEventSink,
    ) {
        if self.connection_generation != Some(connection_generation) {
            self.connection_generation = Some(connection_generation);
            self.owns_dragon_music = false;
            self.pending_start = None;
        }
        for _ in 0..raw_client_ticks {
            self.tick(world, audio);
        }
    }

    fn tick(&mut self, world: &mut WorldStore, audio: &mut dyn AudioEventSink) {
        let counters = audio.counters();
        let observation = AudioCommandObservation::from(&counters);
        let active_music = counters.active_music_event_id.as_deref();
        let wants_dragon = world
            .level_info()
            .is_some_and(|level| level.dimension == END_DIMENSION)
            && world.boss_overlay_should_play_music();

        if wants_dragon {
            if active_music == Some(DRAGON_MUSIC_EVENT) {
                self.owns_dragon_music = true;
                self.pending_start = None;
                return;
            }
            if self.pending_start == Some(observation) {
                return;
            }

            if active_music.is_some() {
                audio.stop_music();
            }
            audio.play_local_sound(&dragon_music_sound(world));
            self.owns_dragon_music = true;
            self.pending_start = Some(observation);
            return;
        }

        let ordinary_music = selected_background_music(world);
        let replaces_dragon = ordinary_music.is_some_and(|music| {
            music.replace_current_music && music.event_id != DRAGON_MUSIC_EVENT
        });
        if self.owns_dragon_music && replaces_dragon {
            // The previous start may still be waiting in an older render
            // effect batch. Appending stop now preserves FIFO semantics: if
            // start has not reached Kira yet, it is still stopped immediately
            // after it does, without waiting for a telemetry round trip.
            audio.stop_music();
            self.owns_dragon_music = false;
            self.pending_start = None;
            return;
        }

        if let Some(pending) = self.pending_start {
            if pending == observation {
                return;
            }
            self.pending_start = None;
            self.owns_dragon_music = active_music == Some(DRAGON_MUSIC_EVENT);
        } else if active_music != Some(DRAGON_MUSIC_EVENT) {
            self.owns_dragon_music = false;
        }
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

fn dragon_music_sound(world: &mut WorldStore) -> LocalSoundEventState {
    let volume = camera_environment_attributes(world)
        .and_then(|attributes| attributes.music_volume)
        .filter(|volume| volume.is_finite())
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    LocalSoundEventState {
        sound: SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(DRAGON_MUSIC_EVENT.to_string()),
            fixed_range: None,
        },
        source: "music".to_string(),
        volume,
        pitch: 1.0,
        seed: world.next_sound_seed(),
    }
}

fn selected_background_music(world: &WorldStore) -> Option<SelectedBackgroundMusic> {
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
    .then(|| SelectedBackgroundMusic {
        event_id: END_MUSIC_EVENT.to_string(),
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
) -> Option<SelectedBackgroundMusic> {
    let music = if underwater {
        background.underwater.as_ref()
    } else {
        None
    }
    .or_else(|| creative.then_some(background.creative.as_ref()).flatten())
    .or(background.default.as_ref())?;
    Some(selected_music(music))
}

fn selected_music(music: &ClientMusic) -> SelectedBackgroundMusic {
    let event_id = match &music.sound {
        ClientSoundEventHolder::Reference { resource_id } => resource_id.clone(),
        ClientSoundEventHolder::Direct { sound_id, .. } => sound_id.clone(),
    };
    SelectedBackgroundMusic {
        event_id,
        replace_current_music: music.replace_current_music,
    }
}

#[cfg(test)]
mod tests {
    use bbb_audio::TickEntitySoundPositionsCommand;
    use bbb_pack::{JukeboxSongRegistry, SoundEventRegistry};
    use bbb_protocol::packets::{
        BossBarColor, BossBarOverlay, BossEvent, BossEventFlags, BossEventOperation,
        CommonPlayerSpawnInfo, PlayLogin,
    };
    use bbb_world::{
        JukeboxLevelEventState, SoundEntityEventState, SoundEventState, StopSoundEventState,
        TickableEntitySoundState,
    };

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    enum RecordedAudioEffect {
        StopMusic,
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
        let mut runtime = BossMusicRuntime::default();
        let mut audio = RecordingAudio::default();

        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);

        assert!(audio.effects.is_empty());
    }

    #[test]
    fn end_boss_music_stops_other_music_then_starts_once_with_world_seed() {
        let mut world = world_in_dimension(END_DIMENSION, 2);
        add_music_boss_bar(&mut world);
        let mut expected_world = world.clone();
        let expected_seed = expected_world.next_sound_seed();
        let mut runtime = BossMusicRuntime::default();
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
        let mut runtime = BossMusicRuntime::default();
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
    fn clearing_boss_flag_uses_selected_background_replacement_policy() {
        let mut world = world_in_dimension(END_DIMENSION, 2);
        add_music_boss_bar(&mut world);
        let mut runtime = BossMusicRuntime::default();
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
            Some(SelectedBackgroundMusic {
                event_id: "example:ambient".to_string(),
                replace_current_music: false,
            })
        );
    }

    #[test]
    fn clearing_boss_flag_orders_stop_after_an_unobserved_pending_start() {
        let mut world = world_in_dimension(END_DIMENSION, 2);
        add_music_boss_bar(&mut world);
        let mut runtime = BossMusicRuntime::default();
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
    fn boss_music_effects_enter_fifo_before_entity_sound_position_tick() {
        let runtime = include_str!("runtime.rs");
        let advance = runtime
            .split_once("pub(crate) fn advance_simulation(")
            .expect("advance_simulation")
            .1
            .split_once("pub(crate) fn update_renderer_from_publication(")
            .expect("advance_simulation end")
            .0;
        let boss_music = advance
            .find("boss_music.advance_for_ticks(")
            .expect("boss music tick");
        let entity_positions = advance
            .find("audio_events.tick_entity_sound_positions(")
            .expect("entity sound position tick");

        assert!(boss_music < entity_positions);
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
