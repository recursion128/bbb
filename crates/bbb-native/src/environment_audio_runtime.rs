use bbb_control::AudioCounters;
use bbb_protocol::packets::{SoundSource, Vec3d as ProtocolVec3d};
use bbb_world::{
    AmbientLoopSoundState, BlockPos, ClientAmbientMood, ClientAmbientSounds,
    ClientSoundEventHolder, EnvironmentBiomeSampling, LevelEventSoundRandomState,
    LocalPlayerAmbientAudioTickState, LocalPlayerBubbleColumnAudioTransition, LocalPlayerPoseState,
    LocalPlayerUnderwaterAudioTransition, LocalSoundEventState, SoundEventState, SoundHolderState,
    TerrainLight, WorldStore,
};

use crate::audio_runtime::AudioEventSink;

const LOOP_SOUND_CROSS_FADE_TICKS: i32 = 40;
const SKY_MOOD_RECOVERY_RATE: f32 = 0.001;
const SOUND_EVENT_REGISTRY: &str = "minecraft:sound_event";
const UNDERWATER_ENTER_SOUND: &str = "minecraft:ambient.underwater.enter";
const UNDERWATER_EXIT_SOUND: &str = "minecraft:ambient.underwater.exit";
const UNDERWATER_LOOP_SOUND: &str = "minecraft:ambient.underwater.loop";
const UNDERWATER_ADDITION_SOUND: &str = "minecraft:ambient.underwater.loop.additions";
const UNDERWATER_RARE_ADDITION_SOUND: &str = "minecraft:ambient.underwater.loop.additions.rare";
const UNDERWATER_ULTRA_RARE_ADDITION_SOUND: &str =
    "minecraft:ambient.underwater.loop.additions.ultra_rare";
const BUBBLE_COLUMN_UPWARDS_INSIDE_SOUND: &str = "minecraft:block.bubble_column.upwards_inside";
const BUBBLE_COLUMN_WHIRLPOOL_INSIDE_SOUND: &str = "minecraft:block.bubble_column.whirlpool_inside";

/// Non-serializable owner of Vanilla LocalPlayer ambient-sound handler state.
#[derive(Debug, Default)]
pub(crate) struct EnvironmentAudioRuntime {
    local_player_identity: Option<(u64, u64, usize)>,
    previous_loop_sound: Option<ClientSoundEventHolder>,
    loop_sounds: Vec<LoopSound>,
    underwater_loops: Vec<UnderwaterLoopSound>,
    moodiness: f32,
    random: LevelEventSoundRandomState,
}

#[derive(Debug)]
struct LoopSound {
    sound: ClientSoundEventHolder,
    seed: i64,
    fade_direction: i32,
    fade: i32,
    stopped: bool,
}

#[derive(Debug)]
struct UnderwaterLoopSound {
    seed: i64,
    fade: i32,
}

impl EnvironmentAudioRuntime {
    pub(crate) fn project_diagnostics(&self, counters: &mut AudioCounters) {
        counters.sound_mood_percent = (self.moodiness * 100.0).round().clamp(0.0, 100.0) as u8;
    }

    pub(crate) fn advance_for_ticks(
        &mut self,
        world: &mut WorldStore,
        connection_generation: u64,
        local_player_ticks: u32,
        audio: &mut dyn AudioEventSink,
    ) {
        // Every respawn constructs fresh LocalPlayer ambient handlers, even
        // when Vanilla retains the same ClientLevel. `level_load.starts` is
        // the canonical respawn/login edge; pairing it with connection and
        // level identities keeps clear/reconnect values unambiguous.
        let local_player_identity = (
            connection_generation,
            world.client_level_generation(),
            world.level_load().starts,
        );
        if self.local_player_identity != Some(local_player_identity) {
            self.local_player_identity = Some(local_player_identity);
            self.previous_loop_sound = None;
            self.loop_sounds.clear();
            self.underwater_loops.clear();
            self.moodiness = 0.0;
            self.random = LevelEventSoundRandomState::default();
            audio.sync_ambient_loops(&[]);
        }

        let mut ambient_ticks = world.take_pending_local_player_ambient_audio_ticks();
        for _ in ambient_ticks.len() as u32..local_player_ticks {
            world.advance_local_player_ambient_audio_tick();
        }
        ambient_ticks.extend(world.take_pending_local_player_ambient_audio_ticks());

        for ambient_tick in ambient_ticks {
            let Some(player) = world.local_player_pose() else {
                continue;
            };
            self.tick(world, player, ambient_tick, audio);
        }
    }

    fn tick(
        &mut self,
        world: &mut WorldStore,
        player: LocalPlayerPoseState,
        ambient_tick: LocalPlayerAmbientAudioTickState,
        audio: &mut dyn AudioEventSink,
    ) {
        self.loop_sounds.retain(|sound| !sound.stopped);
        self.apply_underwater_tick(world, ambient_tick, audio);
        self.apply_bubble_column_tick(world, ambient_tick, audio);
        let ambient = world
            .sampled_client_environment_attributes_at(
                [player.position.x, player.position.y, player.position.z],
                EnvironmentBiomeSampling::Direct,
            )
            .ok()
            .and_then(|attributes| attributes.ambient_sounds)
            .unwrap_or_default();

        self.select_loop_sound(ambient.loop_sound.as_ref(), world);
        self.play_additions(&ambient, world, audio);
        if let Some(mood) = ambient.mood.as_ref() {
            let mood_sound = sound_holder_state(world, &mood.sound);
            self.advance_mood(
                mood,
                mood_sound,
                player,
                |pos| world.sample_block_light(pos),
                audio,
            );
        }
        self.advance_loop_sounds(ambient_tick.underwater, audio);
    }

    fn apply_underwater_tick(
        &mut self,
        world: &mut WorldStore,
        tick: LocalPlayerAmbientAudioTickState,
        audio: &mut dyn AudioEventSink,
    ) {
        match tick.underwater_transition {
            Some(LocalPlayerUnderwaterAudioTransition::Enter { loop_seed }) => {
                let sound_seed = self.random.next_long();
                let sound = world.record_positioned_sound(positioned_ambient_sound(
                    UNDERWATER_ENTER_SOUND,
                    tick.position,
                    sound_seed,
                ));
                audio.play_positioned_sound(&sound);
                self.underwater_loops
                    .push(UnderwaterLoopSound::new(loop_seed));
            }
            Some(LocalPlayerUnderwaterAudioTransition::Exit) => {
                let sound_seed = self.random.next_long();
                let sound = world.record_positioned_sound(positioned_ambient_sound(
                    UNDERWATER_EXIT_SOUND,
                    tick.position,
                    sound_seed,
                ));
                audio.play_positioned_sound(&sound);
            }
            None => {}
        }

        if tick.underwater {
            let sample = self.random.next_float();
            let sound = if sample < 1.0e-4 {
                Some(UNDERWATER_ULTRA_RARE_ADDITION_SOUND)
            } else if sample < 0.001 {
                Some(UNDERWATER_RARE_ADDITION_SOUND)
            } else if sample < 0.01 {
                Some(UNDERWATER_ADDITION_SOUND)
            } else {
                None
            };
            if let Some(sound) = sound {
                let seed = world.next_sound_seed();
                let state = world.record_local_sound(local_ambient_sound(sound, seed));
                audio.play_local_sound(&state);
            }
        }
    }

    fn apply_bubble_column_tick(
        &mut self,
        world: &mut WorldStore,
        tick: LocalPlayerAmbientAudioTickState,
        audio: &mut dyn AudioEventSink,
    ) {
        let sound_id = match tick.bubble_column_transition {
            Some(LocalPlayerBubbleColumnAudioTransition::Upwards) => {
                BUBBLE_COLUMN_UPWARDS_INSIDE_SOUND
            }
            Some(LocalPlayerBubbleColumnAudioTransition::Whirlpool) => {
                BUBBLE_COLUMN_WHIRLPOOL_INSIDE_SOUND
            }
            None => return,
        };
        let seed = self.random.next_long();
        let sound = world.record_positioned_sound(positioned_sound(
            sound_id,
            SoundSource::Players,
            tick.bubble_column_position,
            1.0,
            1.0,
            seed,
        ));
        audio.play_positioned_sound(&sound);
    }

    fn select_loop_sound(
        &mut self,
        selected: Option<&ClientSoundEventHolder>,
        world: &mut WorldStore,
    ) {
        if selected == self.previous_loop_sound.as_ref() {
            return;
        }

        self.previous_loop_sound = selected.cloned();
        for sound in &mut self.loop_sounds {
            sound.fade_out();
        }
        let Some(selected) = selected else {
            return;
        };
        if let Some(sound) = self
            .loop_sounds
            .iter_mut()
            .find(|sound| sound.sound == *selected)
        {
            sound.fade_in();
        } else {
            let mut sound = LoopSound::new(selected.clone(), world.next_sound_seed());
            sound.fade_in();
            self.loop_sounds.push(sound);
        }
    }

    fn play_additions(
        &mut self,
        ambient: &ClientAmbientSounds,
        world: &mut WorldStore,
        audio: &mut dyn AudioEventSink,
    ) {
        for addition in &ambient.additions {
            if self.random.next_double() < f64::from_bits(addition.tick_chance_bits) {
                audio.play_local_sound(&LocalSoundEventState {
                    sound: sound_holder_state(world, &addition.sound),
                    source: SoundSource::Ambient.as_str().to_string(),
                    volume: 1.0,
                    pitch: 1.0,
                    seed: world.next_sound_seed(),
                });
            }
        }
    }

    fn advance_mood(
        &mut self,
        mood: &ClientAmbientMood,
        mood_sound: SoundHolderState,
        player: LocalPlayerPoseState,
        sample_light: impl FnOnce(BlockPos) -> Option<TerrainLight>,
        audio: &mut dyn AudioEventSink,
    ) {
        let Some(search_span) = mood
            .block_search_extent
            .checked_mul(2)
            .and_then(|span| span.checked_add(1))
            .filter(|span| *span > 0)
        else {
            return;
        };
        if mood.tick_delay <= 0 {
            return;
        }

        let sample_pos = BlockPos {
            x: containing_block_coordinate(
                player.position.x + f64::from(self.random.next_int_bound(search_span))
                    - f64::from(mood.block_search_extent),
            ),
            y: containing_block_coordinate(
                player.position.y
                    + player.eye_height()
                    + f64::from(self.random.next_int_bound(search_span))
                    - f64::from(mood.block_search_extent),
            ),
            z: containing_block_coordinate(
                player.position.z + f64::from(self.random.next_int_bound(search_span))
                    - f64::from(mood.block_search_extent),
            ),
        };
        let Some(light) = sample_light(sample_pos) else {
            return;
        };

        if light.sky > 0 {
            self.moodiness -= f32::from(light.sky) / 15.0 * SKY_MOOD_RECOVERY_RATE;
        } else {
            self.moodiness -= (f32::from(light.block) - 1.0) / mood.tick_delay as f32;
        }

        if self.moodiness >= 1.0 {
            let sample_center = [
                f64::from(sample_pos.x) + 0.5,
                f64::from(sample_pos.y) + 0.5,
                f64::from(sample_pos.z) + 0.5,
            ];
            let player_eye = [
                player.position.x,
                player.position.y + player.eye_height(),
                player.position.z,
            ];
            let direction = [
                sample_center[0] - player_eye[0],
                sample_center[1] - player_eye[1],
                sample_center[2] - player_eye[2],
            ];
            let distance = direction
                .iter()
                .map(|component| component * component)
                .sum::<f64>()
                .sqrt();
            let offset = f64::from_bits(mood.offset_bits);
            let sound_source_distance = distance + offset;
            if distance > 0.0 && distance.is_finite() && sound_source_distance.is_finite() {
                let scale = sound_source_distance / distance;
                audio.play_positioned_sound(&SoundEventState {
                    sound: mood_sound,
                    source: SoundSource::Ambient.as_str().to_string(),
                    position: ProtocolVec3d {
                        x: player_eye[0] + direction[0] * scale,
                        y: player_eye[1] + direction[1] * scale,
                        z: player_eye[2] + direction[2] * scale,
                    },
                    volume: 1.0,
                    pitch: 1.0,
                    seed: self.random.next_long(),
                    distance_delay: false,
                });
            }
            self.moodiness = 0.0;
        } else {
            self.moodiness = self.moodiness.max(0.0);
        }
    }

    fn advance_loop_sounds(&mut self, underwater: bool, audio: &mut dyn AudioEventSink) {
        self.underwater_loops
            .retain_mut(|sound| sound.advance(underwater));
        for sound in &mut self.loop_sounds {
            sound.advance();
        }
        let desired = self
            .underwater_loops
            .iter()
            .map(UnderwaterLoopSound::state)
            .chain(
                self.loop_sounds
                    .iter()
                    .filter(|sound| !sound.stopped)
                    .map(LoopSound::state),
            )
            .collect::<Vec<_>>();
        audio.sync_ambient_loops(&desired);
    }
}

impl UnderwaterLoopSound {
    fn new(seed: i64) -> Self {
        Self { seed, fade: 0 }
    }

    fn advance(&mut self, underwater: bool) -> bool {
        if self.fade < 0 {
            return false;
        }
        self.fade = if underwater {
            self.fade.saturating_add(1)
        } else {
            self.fade.saturating_sub(2)
        }
        .min(LOOP_SOUND_CROSS_FADE_TICKS);
        true
    }

    fn state(&self) -> AmbientLoopSoundState {
        AmbientLoopSoundState {
            sound: ClientSoundEventHolder::Reference {
                resource_id: UNDERWATER_LOOP_SOUND.to_string(),
            },
            gain: (self.fade as f32 / LOOP_SOUND_CROSS_FADE_TICKS as f32).clamp(0.0, 1.0),
            seed: self.seed,
        }
    }
}

impl LoopSound {
    fn new(sound: ClientSoundEventHolder, seed: i64) -> Self {
        Self {
            sound,
            seed,
            fade_direction: 0,
            fade: 0,
            stopped: false,
        }
    }

    fn fade_out(&mut self) {
        self.fade = self.fade.min(LOOP_SOUND_CROSS_FADE_TICKS);
        self.fade_direction = -1;
    }

    fn fade_in(&mut self) {
        self.fade = self.fade.max(0);
        self.fade_direction = 1;
        self.stopped = false;
    }

    fn advance(&mut self) {
        if self.fade < 0 {
            self.stopped = true;
        }
        self.fade = self.fade.saturating_add(self.fade_direction);
    }

    fn state(&self) -> AmbientLoopSoundState {
        AmbientLoopSoundState {
            sound: self.sound.clone(),
            gain: (self.fade as f32 / LOOP_SOUND_CROSS_FADE_TICKS as f32).clamp(0.0, 1.0),
            seed: self.seed,
        }
    }
}

pub(crate) fn sound_holder_state(
    world: &WorldStore,
    sound: &ClientSoundEventHolder,
) -> SoundHolderState {
    let registry_id = match sound {
        ClientSoundEventHolder::Reference { resource_id } => world
            .resolved_registry_entry_by_resource_id(SOUND_EVENT_REGISTRY, resource_id)
            .map(|entry| entry.holder_id),
        ClientSoundEventHolder::Direct { .. } => None,
    };
    sound_holder_state_from_parts(sound, registry_id)
}

fn positioned_ambient_sound(sound_id: &str, position: ProtocolVec3d, seed: i64) -> SoundEventState {
    positioned_sound(sound_id, SoundSource::Ambient, position, 1.0, 1.0, seed)
}

fn positioned_sound(
    sound_id: &str,
    source: SoundSource,
    position: ProtocolVec3d,
    volume: f32,
    pitch: f32,
    seed: i64,
) -> SoundEventState {
    SoundEventState {
        sound: builtin_sound_holder(sound_id),
        source: source.as_str().to_string(),
        position,
        volume,
        pitch,
        seed,
        distance_delay: false,
    }
}

fn local_ambient_sound(sound_id: &str, seed: i64) -> LocalSoundEventState {
    LocalSoundEventState {
        sound: builtin_sound_holder(sound_id),
        source: SoundSource::Ambient.as_str().to_string(),
        volume: 1.0,
        pitch: 1.0,
        seed,
    }
}

fn builtin_sound_holder(sound_id: &str) -> SoundHolderState {
    SoundHolderState {
        kind: "direct".to_string(),
        registry_id: None,
        location: Some(sound_id.to_string()),
        fixed_range: None,
    }
}

fn sound_holder_state_from_parts(
    sound: &ClientSoundEventHolder,
    registry_id: Option<i32>,
) -> SoundHolderState {
    match sound {
        ClientSoundEventHolder::Reference { .. } => SoundHolderState {
            kind: "reference".to_string(),
            registry_id,
            location: None,
            fixed_range: None,
        },
        ClientSoundEventHolder::Direct {
            sound_id,
            range_bits,
        } => SoundHolderState {
            kind: "direct".to_string(),
            registry_id: None,
            location: Some(sound_id.clone()),
            fixed_range: range_bits.map(f32::from_bits),
        },
    }
}

fn containing_block_coordinate(value: f64) -> i32 {
    value.floor() as i32
}

#[cfg(test)]
mod tests {
    use bbb_audio::TickEntitySoundPositionsCommand;
    use bbb_control::AudioCounters;
    use bbb_pack::{JukeboxSongRegistry, SoundEventRegistry};
    use bbb_protocol::packets::{
        ChunkBiomeData, ChunkPos as ProtocolChunkPos, ChunksBiomes, CommonPlayerSpawnInfo,
        PlayLogin, Respawn,
    };
    use bbb_world::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, ClientAmbientAddition,
        JukeboxLevelEventState, LightData, PaletteDomain, PaletteKind, PalettedContainerData,
        RegistryPacketEntry, SoundEntityEventState, StopSoundEventState, TickableEntitySoundState,
        WorldDimension,
    };

    use super::*;

    #[derive(Default)]
    struct RecordingAudio {
        local: Vec<LocalSoundEventState>,
        positioned: Vec<SoundEventState>,
        loop_syncs: Vec<Vec<AmbientLoopSoundState>>,
    }

    impl AudioEventSink for RecordingAudio {
        fn counters(&self) -> AudioCounters {
            AudioCounters::default()
        }

        fn set_sound_event_registry(&mut self, _registry: SoundEventRegistry) {}

        fn set_jukebox_song_registry(&mut self, _registry: JukeboxSongRegistry) {}

        fn play_local_sound(&mut self, state: &LocalSoundEventState) {
            self.local.push(state.clone());
        }

        fn play_positioned_sound(&mut self, state: &SoundEventState) {
            self.positioned.push(state.clone());
        }

        fn play_entity_sound(
            &mut self,
            _state: &SoundEntityEventState,
            _position: Option<[f64; 3]>,
        ) {
        }

        fn play_jukebox_song(&mut self, _state: &JukeboxLevelEventState) {}

        fn stop_jukebox_song(&mut self, _state: &JukeboxLevelEventState) {}

        fn stop_sound(&mut self, _state: &StopSoundEventState) {}

        fn set_music_gain(&mut self, _gain: f32) {}

        fn sync_ambient_loops(&mut self, loops: &[AmbientLoopSoundState]) {
            self.loop_syncs.push(loops.to_vec());
        }

        fn tick_entity_sound_positions(
            &mut self,
            _command: TickEntitySoundPositionsCommand,
            _tickable_sounds: &[TickableEntitySoundState],
        ) {
        }
    }

    #[test]
    fn environment_audio_cross_fades_loops_over_40_ticks_and_reuses_a_fading_holder() {
        let mut runtime = EnvironmentAudioRuntime::default();
        let mut world = WorldStore::default();
        let mut audio = RecordingAudio::default();
        let first = direct_sound("example:first", None);
        let second = direct_sound("example:second", None);

        runtime.select_loop_sound(Some(&first), &mut world);
        for _ in 0..LOOP_SOUND_CROSS_FADE_TICKS {
            runtime.advance_loop_sounds(false, &mut audio);
        }
        let first_seed = audio.loop_syncs.last().unwrap()[0].seed;
        assert_eq!(audio.loop_syncs[0][0].gain, 1.0 / 40.0);
        assert_eq!(audio.loop_syncs.last().unwrap()[0].gain, 1.0);

        runtime.select_loop_sound(Some(&second), &mut world);
        runtime.advance_loop_sounds(false, &mut audio);
        let transition = audio.loop_syncs.last().unwrap();
        assert_eq!(transition.len(), 2);
        assert_eq!(transition[0].sound, first);
        assert_eq!(transition[0].gain, 39.0 / 40.0);
        assert_eq!(transition[1].sound, second);
        assert_eq!(transition[1].gain, 1.0 / 40.0);

        for _ in 0..19 {
            runtime.advance_loop_sounds(false, &mut audio);
        }
        runtime.select_loop_sound(Some(&first), &mut world);
        runtime.advance_loop_sounds(false, &mut audio);
        let reversed = audio.loop_syncs.last().unwrap();
        assert_eq!(reversed[0].sound, first);
        assert_eq!(reversed[0].gain, 21.0 / 40.0);
        assert_eq!(reversed[0].seed, first_seed);
        assert_eq!(reversed[1].gain, 19.0 / 40.0);
    }

    #[test]
    fn environment_audio_omits_a_faded_loop_when_vanilla_stops_it() {
        let mut runtime = EnvironmentAudioRuntime::default();
        let mut world = WorldStore::default();
        let mut audio = RecordingAudio::default();
        let sound = direct_sound("example:loop", None);

        runtime.select_loop_sound(Some(&sound), &mut world);
        for _ in 0..LOOP_SOUND_CROSS_FADE_TICKS {
            runtime.advance_loop_sounds(false, &mut audio);
        }
        runtime.select_loop_sound(None, &mut world);
        for _ in 0..=LOOP_SOUND_CROSS_FADE_TICKS {
            runtime.advance_loop_sounds(false, &mut audio);
        }
        assert_eq!(audio.loop_syncs.last().unwrap().len(), 1);
        assert_eq!(audio.loop_syncs.last().unwrap()[0].gain, 0.0);

        runtime.advance_loop_sounds(false, &mut audio);
        assert!(audio.loop_syncs.last().unwrap().is_empty());
    }

    #[test]
    fn underwater_audio_plays_transition_sounds_and_fades_loop_at_vanilla_rates() {
        let mut runtime = EnvironmentAudioRuntime::default();
        let mut expected_level_random = LevelEventSoundRandomState::default();
        let enter_seed = expected_level_random.next_long();
        let addition_sample = expected_level_random.next_float();
        assert!(addition_sample >= 0.01);
        let mut world = WorldStore::default();
        let mut audio = RecordingAudio::default();
        let position = ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };

        runtime.apply_underwater_tick(
            &mut world,
            LocalPlayerAmbientAudioTickState {
                position,
                bubble_column_position: position,
                underwater: true,
                underwater_transition: Some(LocalPlayerUnderwaterAudioTransition::Enter {
                    loop_seed: 12,
                }),
                bubble_column_transition: None,
            },
            &mut audio,
        );
        runtime.advance_loop_sounds(true, &mut audio);

        assert_eq!(audio.positioned.len(), 1);
        assert_eq!(
            audio.positioned[0].sound.location.as_deref(),
            Some(UNDERWATER_ENTER_SOUND)
        );
        assert_eq!(audio.positioned[0].source, "ambient");
        assert_eq!(audio.positioned[0].position, position);
        assert_eq!(audio.positioned[0].seed, enter_seed);
        assert_eq!(world.last_sound(), audio.positioned.last());
        assert_eq!(audio.loop_syncs.last().unwrap().len(), 1);
        assert_eq!(audio.loop_syncs.last().unwrap()[0].gain, 1.0 / 40.0);
        assert_eq!(audio.loop_syncs.last().unwrap()[0].seed, 12);

        for _ in 1..LOOP_SOUND_CROSS_FADE_TICKS {
            runtime.advance_loop_sounds(true, &mut audio);
        }
        assert_eq!(audio.loop_syncs.last().unwrap()[0].gain, 1.0);

        let exit_seed = expected_level_random.next_long();
        runtime.apply_underwater_tick(
            &mut world,
            LocalPlayerAmbientAudioTickState {
                position,
                bubble_column_position: position,
                underwater: false,
                underwater_transition: Some(LocalPlayerUnderwaterAudioTransition::Exit),
                bubble_column_transition: None,
            },
            &mut audio,
        );
        runtime.advance_loop_sounds(false, &mut audio);
        assert_eq!(
            audio.positioned.last().unwrap().sound.location.as_deref(),
            Some(UNDERWATER_EXIT_SOUND)
        );
        assert_eq!(audio.positioned.last().unwrap().seed, exit_seed);
        assert_eq!(audio.loop_syncs.last().unwrap()[0].gain, 38.0 / 40.0);
        assert_eq!(
            runtime.random.next_long(),
            expected_level_random.next_long(),
            "enter/exit and the underwater handler must share level.random"
        );

        for _ in 0..20 {
            runtime.advance_loop_sounds(false, &mut audio);
        }
        assert_eq!(audio.loop_syncs.last().unwrap()[0].gain, 0.0);
        runtime.advance_loop_sounds(false, &mut audio);
        assert!(audio.loop_syncs.last().unwrap().is_empty());
    }

    #[test]
    fn underwater_audio_quick_reentry_keeps_the_fading_instance_and_starts_a_new_loop() {
        let mut runtime = EnvironmentAudioRuntime::default();
        let mut world = WorldStore::default();
        let mut audio = RecordingAudio::default();
        let position = ProtocolVec3d::default();

        runtime.apply_underwater_tick(
            &mut world,
            LocalPlayerAmbientAudioTickState {
                position,
                bubble_column_position: position,
                underwater: true,
                underwater_transition: Some(LocalPlayerUnderwaterAudioTransition::Enter {
                    loop_seed: 2,
                }),
                bubble_column_transition: None,
            },
            &mut audio,
        );
        for _ in 0..LOOP_SOUND_CROSS_FADE_TICKS {
            runtime.advance_loop_sounds(true, &mut audio);
        }
        runtime.apply_underwater_tick(
            &mut world,
            LocalPlayerAmbientAudioTickState {
                position,
                bubble_column_position: position,
                underwater: false,
                underwater_transition: Some(LocalPlayerUnderwaterAudioTransition::Exit),
                bubble_column_transition: None,
            },
            &mut audio,
        );
        runtime.advance_loop_sounds(false, &mut audio);
        runtime.apply_underwater_tick(
            &mut world,
            LocalPlayerAmbientAudioTickState {
                position,
                bubble_column_position: position,
                underwater: true,
                underwater_transition: Some(LocalPlayerUnderwaterAudioTransition::Enter {
                    loop_seed: 5,
                }),
                bubble_column_transition: None,
            },
            &mut audio,
        );
        runtime.advance_loop_sounds(true, &mut audio);

        let loops = audio.loop_syncs.last().unwrap();
        assert_eq!(loops.len(), 2);
        assert_eq!(loops[0].seed, 2);
        assert_eq!(loops[0].gain, 39.0 / 40.0);
        assert_eq!(loops[1].seed, 5);
        assert_eq!(loops[1].gain, 1.0 / 40.0);
    }

    #[test]
    fn underwater_audio_additions_use_vanilla_thresholds() {
        for (seed, expected_sound) in [
            (5188, Some(UNDERWATER_ULTRA_RARE_ADDITION_SOUND)),
            (5184, Some(UNDERWATER_RARE_ADDITION_SOUND)),
            (5120, Some(UNDERWATER_ADDITION_SOUND)),
            (0, None),
        ] {
            let mut runtime = EnvironmentAudioRuntime {
                random: LevelEventSoundRandomState::with_seed(seed),
                ..EnvironmentAudioRuntime::default()
            };
            let mut world = WorldStore::default();
            let mut audio = RecordingAudio::default();

            runtime.apply_underwater_tick(
                &mut world,
                LocalPlayerAmbientAudioTickState {
                    position: ProtocolVec3d::default(),
                    bubble_column_position: ProtocolVec3d::default(),
                    underwater: true,
                    underwater_transition: None,
                    bubble_column_transition: None,
                },
                &mut audio,
            );

            assert_eq!(
                audio
                    .local
                    .last()
                    .and_then(|sound| sound.sound.location.as_deref()),
                expected_sound,
                "unexpected underwater addition for random seed {seed}"
            );
            assert!(audio.local.iter().all(|sound| {
                sound.source == "ambient" && sound.volume == 1.0 && sound.pitch == 1.0
            }));
        }
    }

    #[test]
    fn bubble_column_audio_maps_entry_kind_to_player_positioned_sound() {
        let mut runtime = EnvironmentAudioRuntime::default();
        let mut expected_random = LevelEventSoundRandomState::default();
        let mut world = WorldStore::default();
        let mut audio = RecordingAudio::default();
        let position = ProtocolVec3d {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        };

        for (transition, expected_sound) in [
            (
                LocalPlayerBubbleColumnAudioTransition::Upwards,
                BUBBLE_COLUMN_UPWARDS_INSIDE_SOUND,
            ),
            (
                LocalPlayerBubbleColumnAudioTransition::Whirlpool,
                BUBBLE_COLUMN_WHIRLPOOL_INSIDE_SOUND,
            ),
        ] {
            runtime.apply_bubble_column_tick(
                &mut world,
                LocalPlayerAmbientAudioTickState {
                    position,
                    bubble_column_position: position,
                    underwater: false,
                    underwater_transition: None,
                    bubble_column_transition: Some(transition),
                },
                &mut audio,
            );
            let sound = audio.positioned.last().unwrap();
            assert_eq!(sound.sound.location.as_deref(), Some(expected_sound));
            assert_eq!(sound.source, "player");
            assert_eq!(sound.position, position);
            assert_eq!(sound.volume, 1.0);
            assert_eq!(sound.pitch, 1.0);
            assert_eq!(sound.seed, expected_random.next_long());
            assert!(!sound.distance_delay);
        }
        assert_eq!(world.last_sound(), audio.positioned.last());
    }

    #[test]
    fn ambient_handlers_consume_level_random_in_vanilla_order() {
        let ambient = ClientAmbientSounds {
            loop_sound: None,
            mood: None,
            additions: vec![ClientAmbientAddition {
                sound: direct_sound("example:never", None),
                tick_chance_bits: (-1.0_f64).to_bits(),
            }],
        };
        let mut runtime = EnvironmentAudioRuntime {
            random: LevelEventSoundRandomState::with_seed(0),
            ..EnvironmentAudioRuntime::default()
        };
        let mut expected = LevelEventSoundRandomState::with_seed(0);
        let _ = expected.next_float();
        let bubble_seed = expected.next_long();
        let _ = expected.next_double();
        let mut world = WorldStore::default();
        let mut audio = RecordingAudio::default();

        runtime.apply_underwater_tick(
            &mut world,
            LocalPlayerAmbientAudioTickState {
                position: ProtocolVec3d::default(),
                bubble_column_position: ProtocolVec3d::default(),
                underwater: true,
                underwater_transition: None,
                bubble_column_transition: None,
            },
            &mut audio,
        );
        runtime.apply_bubble_column_tick(
            &mut world,
            LocalPlayerAmbientAudioTickState {
                position: ProtocolVec3d::default(),
                bubble_column_position: ProtocolVec3d::default(),
                underwater: true,
                underwater_transition: None,
                bubble_column_transition: Some(LocalPlayerBubbleColumnAudioTransition::Upwards),
            },
            &mut audio,
        );
        runtime.play_additions(&ambient, &mut world, &mut audio);

        assert_eq!(audio.positioned[0].seed, bubble_seed);
        assert_eq!(runtime.random.next_long(), expected.next_long());
        assert!(audio.local.is_empty());
    }

    #[test]
    fn environment_audio_tests_additions_in_order_and_preserves_random_consumption() {
        let ambient = ClientAmbientSounds {
            loop_sound: None,
            mood: None,
            additions: vec![
                ClientAmbientAddition {
                    sound: direct_sound("example:first", None),
                    tick_chance_bits: 1.0_f64.to_bits(),
                },
                ClientAmbientAddition {
                    sound: direct_sound("example:never", None),
                    tick_chance_bits: (-1.0_f64).to_bits(),
                },
                ClientAmbientAddition {
                    sound: direct_sound("example:third", None),
                    tick_chance_bits: 1.0_f64.to_bits(),
                },
            ],
        };
        let mut runtime = EnvironmentAudioRuntime::default();
        let mut expected_random = LevelEventSoundRandomState::default();
        for _ in &ambient.additions {
            let _ = expected_random.next_double();
        }
        let mut world = WorldStore::default();
        let mut audio = RecordingAudio::default();

        runtime.play_additions(&ambient, &mut world, &mut audio);

        assert_eq!(audio.local.len(), 2);
        assert_eq!(
            audio.local[0].sound.location.as_deref(),
            Some("example:first")
        );
        assert_eq!(
            audio.local[1].sound.location.as_deref(),
            Some("example:third")
        );
        assert_eq!(runtime.random.next_long(), expected_random.next_long());
    }

    #[test]
    fn environment_audio_mood_uses_eye_center_offset_and_level_random_seed() {
        let mood = ClientAmbientMood {
            sound: direct_sound("example:mood", Some(24.0)),
            tick_delay: 100,
            block_search_extent: 0,
            offset_bits: 2.0_f64.to_bits(),
        };
        let player = player_pose([10.0, 20.0, 30.0]);
        let mut expected_random = LevelEventSoundRandomState::default();
        for _ in 0..3 {
            let _ = expected_random.next_int_bound(1);
        }
        let expected_seed = expected_random.next_long();
        let mut runtime = EnvironmentAudioRuntime {
            moodiness: 0.999,
            ..EnvironmentAudioRuntime::default()
        };
        let mut audio = RecordingAudio::default();
        let mut sampled = None;

        runtime.advance_mood(
            &mood,
            sound_holder_state_from_parts(&mood.sound, None),
            player,
            |pos| {
                sampled = Some(pos);
                Some(TerrainLight::DARK)
            },
            &mut audio,
        );

        let sample_pos = BlockPos {
            x: 10,
            y: 21,
            z: 30,
        };
        assert_eq!(sampled, Some(sample_pos));
        assert_eq!(audio.positioned.len(), 1);
        let sound = &audio.positioned[0];
        assert_eq!(sound.sound.location.as_deref(), Some("example:mood"));
        assert_eq!(sound.sound.fixed_range, Some(24.0));
        assert_eq!(sound.seed, expected_seed);
        assert_eq!(sound.source, "ambient");
        assert_eq!(runtime.moodiness, 0.0);

        let eye = [10.0, 20.0 + player.eye_height(), 30.0];
        let direction = [0.5, 21.5 - eye[1], 0.5];
        let distance = direction
            .iter()
            .map(|component| component * component)
            .sum::<f64>()
            .sqrt();
        let scale = (distance + 2.0) / distance;
        assert_position_close(
            sound.position,
            [
                eye[0] + direction[0] * scale,
                eye[1] + direction[1] * scale,
                eye[2] + direction[2] * scale,
            ],
        );
    }

    #[test]
    fn environment_audio_missing_light_skips_mood_without_fabricating_darkness() {
        let mood = ClientAmbientMood {
            sound: direct_sound("example:mood", None),
            tick_delay: 1,
            block_search_extent: 0,
            offset_bits: 2.0_f64.to_bits(),
        };
        let mut runtime = EnvironmentAudioRuntime {
            moodiness: 0.75,
            ..EnvironmentAudioRuntime::default()
        };
        let mut expected_random = LevelEventSoundRandomState::default();
        for _ in 0..3 {
            let _ = expected_random.next_int_bound(1);
        }
        let mut audio = RecordingAudio::default();

        runtime.advance_mood(
            &mood,
            sound_holder_state_from_parts(&mood.sound, None),
            player_pose([0.0, 0.0, 0.0]),
            |_| None,
            &mut audio,
        );

        assert_eq!(runtime.moodiness, 0.75);
        assert!(audio.positioned.is_empty());
        assert_eq!(runtime.random.next_long(), expected_random.next_long());
    }

    #[test]
    fn environment_audio_sky_light_recovers_moodiness_at_vanilla_rate() {
        let mood = ClientAmbientMood {
            sound: direct_sound("example:mood", None),
            tick_delay: 1,
            block_search_extent: 0,
            offset_bits: 0.0_f64.to_bits(),
        };
        let mut runtime = EnvironmentAudioRuntime {
            moodiness: 0.5,
            ..EnvironmentAudioRuntime::default()
        };
        let mut audio = RecordingAudio::default();

        runtime.advance_mood(
            &mood,
            sound_holder_state_from_parts(&mood.sound, None),
            player_pose([0.0, 0.0, 0.0]),
            |_| Some(TerrainLight { sky: 15, block: 0 }),
            &mut audio,
        );

        assert!((runtime.moodiness - 0.499).abs() < f32::EPSILON);
        assert!(audio.positioned.is_empty());
    }

    #[test]
    fn environment_audio_projects_vanilla_rounded_mood_percent() {
        for (moodiness, expected) in [
            (-0.25, 0),
            (0.004, 0),
            (0.006, 1),
            (0.754, 75),
            (0.756, 76),
            (1.25, 100),
        ] {
            let runtime = EnvironmentAudioRuntime {
                moodiness,
                ..EnvironmentAudioRuntime::default()
            };
            let mut counters = AudioCounters {
                commands_submitted: 7,
                ..AudioCounters::default()
            };

            runtime.project_diagnostics(&mut counters);

            assert_eq!(counters.sound_mood_percent, expected);
            assert_eq!(counters.commands_submitted, 7);
        }
    }

    #[test]
    fn environment_audio_resolves_named_holders_against_current_registry_order() {
        let sound = ClientSoundEventHolder::Reference {
            resource_id: "example:target".to_string(),
        };
        let mut first = WorldStore::default();
        first.record_registry_entries(
            SOUND_EVENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::stub("example:other"),
                RegistryPacketEntry::stub("example:target"),
            ],
        );
        let mut reordered = WorldStore::default();
        reordered.record_registry_entries(
            SOUND_EVENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::stub("example:target"),
                RegistryPacketEntry::stub("example:other"),
            ],
        );

        assert_eq!(sound_holder_state(&first, &sound).registry_id, Some(1));
        assert_eq!(sound_holder_state(&reordered, &sound).registry_id, Some(0));
    }

    #[test]
    fn environment_audio_level_replacement_stops_old_loops_without_running_ticks() {
        let mut runtime = EnvironmentAudioRuntime::default();
        runtime.local_player_identity = Some((1, 0, 0));
        runtime.previous_loop_sound = Some(direct_sound("example:old", None));
        runtime
            .loop_sounds
            .push(LoopSound::new(direct_sound("example:old", None), 7));
        runtime.underwater_loops.push(UnderwaterLoopSound::new(8));
        let mut world = WorldStore::default();
        let mut audio = RecordingAudio::default();

        runtime.advance_for_ticks(&mut world, 2, 0, &mut audio);

        assert_eq!(audio.loop_syncs, vec![Vec::new()]);
        assert!(runtime.loop_sounds.is_empty());
        assert!(runtime.underwater_loops.is_empty());
        assert_eq!(runtime.previous_loop_sound, None);
        assert_eq!(runtime.moodiness, 0.0);
    }

    #[test]
    fn environment_audio_rebuilds_local_player_handlers_on_same_dimension_respawn() {
        let mut world = world_with_biome_ambient_loops();
        let mut runtime = EnvironmentAudioRuntime::default();
        let mut audio = RecordingAudio::default();
        runtime.advance_for_ticks(&mut world, 1, 0, &mut audio);
        runtime.previous_loop_sound = Some(direct_sound("example:old", None));
        runtime
            .loop_sounds
            .push(LoopSound::new(direct_sound("example:old", None), 7));
        let level_generation = world.client_level_generation();

        world.apply_respawn(&Respawn {
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "example:level".to_string(),
                seed: 1,
                game_type: 0,
                previous_game_type: 0,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            data_to_keep: 0,
        });
        assert_eq!(world.client_level_generation(), level_generation);

        runtime.advance_for_ticks(&mut world, 1, 0, &mut audio);
        assert!(runtime.loop_sounds.is_empty());
        assert_eq!(runtime.previous_loop_sound, None);
        assert!(audio.loop_syncs.last().is_some_and(Vec::is_empty));
    }

    #[test]
    fn environment_audio_observes_chunks_biomes_replacement_on_the_next_tick() {
        let mut world = world_with_biome_ambient_loops();
        let mut runtime = EnvironmentAudioRuntime::default();
        let mut audio = RecordingAudio::default();

        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);
        let initial = audio.loop_syncs.last().unwrap();
        assert_eq!(initial.len(), 1);
        assert_eq!(
            initial[0].sound,
            ClientSoundEventHolder::Reference {
                resource_id: "example:loop_a".to_string()
            }
        );
        assert_eq!(initial[0].gain, 1.0 / 40.0);

        world
            .apply_biome_update(ChunksBiomes {
                chunks: vec![ChunkBiomeData {
                    pos: ProtocolChunkPos { x: 0, z: 0 },
                    raw_biomes: [0, 1].repeat(world.dimension().section_count()),
                }],
            })
            .unwrap();
        runtime.advance_for_ticks(&mut world, 1, 1, &mut audio);

        let replaced = audio.loop_syncs.last().unwrap();
        assert_eq!(replaced.len(), 2);
        assert_eq!(
            replaced[0].sound,
            ClientSoundEventHolder::Reference {
                resource_id: "example:loop_a".to_string()
            }
        );
        assert_eq!(replaced[0].gain, 0.0);
        assert_eq!(
            replaced[1].sound,
            ClientSoundEventHolder::Reference {
                resource_id: "example:loop_b".to_string()
            }
        );
        assert_eq!(replaced[1].gain, 1.0 / 40.0);
    }

    #[test]
    fn environment_audio_ticks_after_player_movement_and_before_music() {
        let runtime = include_str!("runtime.rs");
        let advance = runtime
            .split_once("pub(crate) fn advance_simulation(")
            .expect("advance_simulation")
            .1
            .split_once("pub(crate) fn update_renderer_from_publication(")
            .expect("advance_simulation end")
            .0;
        let movement = advance
            .find("advance_player_input_for_client_ticks(")
            .expect("local player movement");
        let ambient = advance
            .find("environment_audio.advance_for_ticks(")
            .expect("environment audio tick");
        let music = advance
            .find("music.advance_for_ticks(")
            .expect("music tick");

        assert!(movement < ambient);
        assert!(ambient < music);
    }

    fn world_with_biome_ambient_loops() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension::default());
        world.begin_registry_configuration();
        world.record_registry_entries(
            "minecraft:worldgen/biome",
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:biome_a",
                    biome_with_ambient_loop_nbt("example:loop_a"),
                ),
                RegistryPacketEntry::with_raw_data(
                    "example:biome_b",
                    biome_with_ambient_loop_nbt("example:loop_b"),
                ),
            ],
        );
        world.record_registry_entries(
            SOUND_EVENT_REGISTRY,
            0,
            vec![
                RegistryPacketEntry::stub("example:loop_a"),
                RegistryPacketEntry::stub("example:loop_b"),
            ],
        );
        for data in crate::runtime::minimal_non_empty_synchronized_registry_data() {
            match data.registry.as_str() {
                "minecraft:worldgen/biome" | SOUND_EVENT_REGISTRY => {}
                _ => world.record_registry_data(data),
            }
        }
        world.finish_registry_configuration().unwrap();
        world.apply_login(&PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["example:level".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "example:level".to_string(),
                seed: 12345,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        world.set_local_player_pose(player_pose([1.0, 32.0, 1.0]));
        world.insert_decoded_chunk(chunk_with_biome(world.dimension(), 0));
        world
    }

    fn biome_with_ambient_loop_nbt(loop_sound: &str) -> Vec<u8> {
        let mut out = vec![10];
        write_nbt_byte(&mut out, "has_precipitation", 1);
        write_nbt_float(&mut out, "temperature", 0.8);
        write_nbt_float(&mut out, "downfall", 0.4);
        out.push(10);
        write_nbt_utf(&mut out, "effects");
        write_nbt_string(&mut out, "water_color", "#3f76e4");
        out.push(0);
        out.push(10);
        write_nbt_utf(&mut out, "attributes");
        out.push(10);
        write_nbt_utf(&mut out, "minecraft:audio/ambient_sounds");
        write_nbt_string(&mut out, "loop", loop_sound);
        out.push(0);
        out.push(0);
        out.push(0);
        out
    }

    fn chunk_with_biome(dimension: WorldDimension, biome_id: i32) -> ChunkColumn {
        ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..dimension.section_count())
                .map(|_| ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(PaletteDomain::BlockStates, 4096, 0),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, biome_id),
                })
                .collect(),
            block_entities: Vec::new(),
            light: LightData::default(),
        }
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn write_nbt_utf(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u16).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }

    fn write_nbt_byte(out: &mut Vec<u8>, name: &str, value: i8) {
        out.push(1);
        write_nbt_utf(out, name);
        out.push(value as u8);
    }

    fn write_nbt_float(out: &mut Vec<u8>, name: &str, value: f32) {
        out.push(5);
        write_nbt_utf(out, name);
        out.extend_from_slice(&value.to_bits().to_be_bytes());
    }

    fn write_nbt_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_nbt_utf(out, name);
        write_nbt_utf(out, value);
    }

    fn direct_sound(sound_id: &str, fixed_range: Option<f32>) -> ClientSoundEventHolder {
        ClientSoundEventHolder::Direct {
            sound_id: sound_id.to_string(),
            range_bits: fixed_range.map(f32::to_bits),
        }
    }

    fn player_pose(position: [f64; 3]) -> LocalPlayerPoseState {
        LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            ..LocalPlayerPoseState::default()
        }
    }

    fn assert_position_close(actual: ProtocolVec3d, expected: [f64; 3]) {
        assert!((actual.x - expected[0]).abs() < 1.0e-12);
        assert!((actual.y - expected[1]).abs() < 1.0e-12);
        assert!((actual.z - expected[2]).abs() < 1.0e-12);
    }
}
