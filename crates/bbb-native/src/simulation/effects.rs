use std::{cell::RefCell, error::Error, fmt, path::PathBuf};

use bbb_audio::TickEntitySoundPositionsCommand;
use bbb_control::AudioCounters;
use bbb_pack::{JukeboxSongRegistry, SoundEventRegistry};
use bbb_renderer::{ParticleSpawnBatch, Renderer};
use bbb_world::{
    AmbientLoopSoundState, JukeboxLevelEventState, LocalSoundEventState, SoundEntityEventState,
    SoundEventState, StopSoundEventState, TickableEntitySoundState,
};

use crate::{
    audio_runtime::{AudioEffect, AudioEventSink},
    resource_pack_runtime::ServerResourcePackReload,
};

/// Owned particle batches accepted by the future render-effect transport.
pub(crate) trait ParticleBatchSink {
    fn submit_particle_batch(&mut self, batch: ParticleSpawnBatch);
}

/// Render/winit-owned work requested by the simulation input path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlatformEffect {
    SetCursorCapture(bool),
    ClipboardWrite(String),
    OpenUri(String),
    Screenshot(PathBuf),
    ReloadTerrain,
    DumpDynamicTextures {
        requests: u32,
    },
    ProfilingToggled {
        requests: usize,
        starts: usize,
        stops: usize,
    },
    NavigateProfiler {
        digits: Vec<u8>,
    },
    StatsScreenInput(ScreenNarrationScheduleInput),
    RecipeBookInput(ScreenNarrationScheduleInput),
    SocialInteractionsInput(ScreenNarrationScheduleInput),
    GameEventScreenInput(ScreenNarrationScheduleInput),
    DeathScreenInput(ScreenNarrationScheduleInput),
    ChatNarration {
        kind: bbb_world::ChatNarrationKind,
        text: String,
    },
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScreenNarrationScheduleInput {
    MouseMove,
    MouseAction,
    KeyboardAction,
}

pub(crate) trait PlatformEffectSink {
    fn apply_platform_effect(&mut self, effect: PlatformEffect);
}

pub(crate) trait RenderEffectSink: ParticleBatchSink + PlatformEffectSink {}

impl<T> RenderEffectSink for T where T: ParticleBatchSink + PlatformEffectSink {}

impl ParticleBatchSink for Renderer {
    fn submit_particle_batch(&mut self, batch: ParticleSpawnBatch) {
        self.submit_particle_spawns(batch);
    }
}

enum RenderEffect {
    Audio(AudioEffect),
    Particles(ParticleSpawnBatch),
    Platform(PlatformEffect),
}

/// Terminal resource reload request sealed after a simulation step's ordinary
/// audio and particle effects.
pub(crate) struct ResourceReloadEffect {
    pub(crate) request: ServerResourcePackReload,
    pub(crate) sound_events: SoundEventRegistry,
    pub(crate) jukebox_songs: JukeboxSongRegistry,
}

/// Sequence-numbered, owned output of one simulation step.
///
/// The render owner consumes every non-empty batch at most once. Steps without
/// effects do not consume sequence space or enter the lossless FIFO.
pub(crate) struct RenderEffectBatch {
    sequence: u64,
    effects: Vec<RenderEffect>,
    resource_reload: Option<ResourceReloadEffect>,
}

/// Interior-mutable builder lets the existing world dispatcher receive its
/// audio and particle trait adapters simultaneously without borrowing a
/// backend or splitting cross-backend FIFO order.
pub(crate) struct RenderEffectCollector {
    sequence: u64,
    audio_counters: AudioCounters,
    effects: RefCell<Vec<RenderEffect>>,
}

impl RenderEffectCollector {
    pub(crate) fn new(sequence: u64, audio_counters: AudioCounters) -> Self {
        Self {
            sequence,
            audio_counters,
            effects: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn audio(&self) -> AudioEffectCollector<'_> {
        AudioEffectCollector { collector: self }
    }

    pub(crate) fn particles(&self) -> ParticleEffectCollector<'_> {
        ParticleEffectCollector { collector: self }
    }

    pub(crate) fn platform(&self) -> PlatformEffectCollector<'_> {
        PlatformEffectCollector { collector: self }
    }

    pub(crate) fn finish(
        self,
        resource_reload: Option<ResourceReloadEffect>,
    ) -> Option<RenderEffectBatch> {
        let effects = self.effects.into_inner();
        if effects.is_empty() && resource_reload.is_none() {
            return None;
        }
        Some(RenderEffectBatch {
            sequence: self.sequence,
            effects,
            resource_reload,
        })
    }

    fn push(&self, effect: RenderEffect) {
        self.effects.borrow_mut().push(effect);
    }
}

pub(crate) struct AudioEffectCollector<'a> {
    collector: &'a RenderEffectCollector,
}

impl AudioEventSink for AudioEffectCollector<'_> {
    fn counters(&self) -> AudioCounters {
        self.collector.audio_counters.clone()
    }

    fn set_sound_event_registry(&mut self, registry: SoundEventRegistry) {
        self.collector
            .push(RenderEffect::Audio(AudioEffect::SetSoundEventRegistry(
                registry,
            )));
    }

    fn set_jukebox_song_registry(&mut self, registry: JukeboxSongRegistry) {
        self.collector
            .push(RenderEffect::Audio(AudioEffect::SetJukeboxSongRegistry(
                registry,
            )));
    }

    fn play_local_sound(&mut self, state: &LocalSoundEventState) {
        self.collector
            .push(RenderEffect::Audio(AudioEffect::PlayLocal(state.clone())));
    }

    fn play_positioned_sound(&mut self, state: &SoundEventState) {
        self.collector
            .push(RenderEffect::Audio(AudioEffect::PlayPositioned(
                state.clone(),
            )));
    }

    fn play_entity_sound(&mut self, state: &SoundEntityEventState, position: Option<[f64; 3]>) {
        self.collector
            .push(RenderEffect::Audio(AudioEffect::PlayEntity {
                state: state.clone(),
                position,
            }));
    }

    fn play_jukebox_song(&mut self, state: &JukeboxLevelEventState) {
        self.collector
            .push(RenderEffect::Audio(AudioEffect::PlayJukebox(state.clone())));
    }

    fn stop_jukebox_song(&mut self, state: &JukeboxLevelEventState) {
        self.collector
            .push(RenderEffect::Audio(AudioEffect::StopJukebox(state.clone())));
    }

    fn stop_sound(&mut self, state: &StopSoundEventState) {
        self.collector
            .push(RenderEffect::Audio(AudioEffect::Stop(state.clone())));
    }

    fn set_music_gain(&mut self, gain: f32) {
        self.collector
            .push(RenderEffect::Audio(AudioEffect::SetMusicGain(gain)));
    }

    fn sync_ambient_loops(&mut self, loops: &[AmbientLoopSoundState]) {
        self.collector
            .push(RenderEffect::Audio(AudioEffect::SyncAmbientLoops(
                loops.to_vec(),
            )));
    }

    fn tick_entity_sound_positions(
        &mut self,
        command: TickEntitySoundPositionsCommand,
        tickable_sounds: &[TickableEntitySoundState],
    ) {
        self.collector
            .push(RenderEffect::Audio(AudioEffect::TickEntityPositions {
                command,
                tickable_sounds: tickable_sounds.to_vec(),
            }));
    }
}

pub(crate) struct ParticleEffectCollector<'a> {
    collector: &'a RenderEffectCollector,
}

impl ParticleBatchSink for ParticleEffectCollector<'_> {
    fn submit_particle_batch(&mut self, batch: ParticleSpawnBatch) {
        self.collector.push(RenderEffect::Particles(batch));
    }
}

pub(crate) struct PlatformEffectCollector<'a> {
    collector: &'a RenderEffectCollector,
}

impl PlatformEffectCollector<'_> {
    pub(crate) fn submit(&mut self, effect: PlatformEffect) {
        self.collector.push(RenderEffect::Platform(effect));
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RenderEffectSequenceError {
    pub(crate) expected: u64,
    pub(crate) actual: u64,
}

impl fmt::Display for RenderEffectSequenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "render effect sequence gap: expected {}, received {}",
            self.expected, self.actual
        )
    }
}

impl Error for RenderEffectSequenceError {}

#[derive(Default)]
pub(crate) struct RenderEffectConsumer {
    last_sequence: u64,
}

impl RenderEffectConsumer {
    pub(crate) fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub(crate) fn consume(
        &mut self,
        batch: RenderEffectBatch,
        mut audio: Option<&mut dyn AudioEventSink>,
        render: &mut dyn RenderEffectSink,
    ) -> Result<Option<ResourceReloadEffect>, RenderEffectSequenceError> {
        if batch.sequence <= self.last_sequence {
            return Ok(None);
        }
        let expected = self.last_sequence.saturating_add(1);
        if batch.sequence != expected {
            return Err(RenderEffectSequenceError {
                expected,
                actual: batch.sequence,
            });
        }
        for effect in batch.effects {
            match effect {
                RenderEffect::Audio(effect) => {
                    if let Some(audio) = audio.as_deref_mut() {
                        effect.replay(audio);
                    }
                }
                RenderEffect::Particles(batch) => render.submit_particle_batch(batch),
                RenderEffect::Platform(effect) => render.apply_platform_effect(effect),
            }
        }
        self.last_sequence = batch.sequence;
        Ok(batch.resource_reload)
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use bbb_protocol::packets::SoundSource;
    use bbb_world::{ClientSoundEventHolder, SoundHolderState};

    use super::*;

    struct RecordingAudio {
        log: Rc<RefCell<Vec<&'static str>>>,
    }

    impl AudioEventSink for RecordingAudio {
        fn counters(&self) -> AudioCounters {
            AudioCounters::default()
        }

        fn set_sound_event_registry(&mut self, _registry: SoundEventRegistry) {}

        fn set_jukebox_song_registry(&mut self, _registry: JukeboxSongRegistry) {}

        fn play_local_sound(&mut self, _state: &LocalSoundEventState) {
            self.log.borrow_mut().push("audio");
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

        fn stop_sound(&mut self, _state: &StopSoundEventState) {}

        fn set_music_gain(&mut self, _gain: f32) {
            self.log.borrow_mut().push("music_gain");
        }

        fn sync_ambient_loops(&mut self, _loops: &[AmbientLoopSoundState]) {
            self.log.borrow_mut().push("ambient_loops");
        }

        fn tick_entity_sound_positions(
            &mut self,
            _command: TickEntitySoundPositionsCommand,
            _tickable_sounds: &[TickableEntitySoundState],
        ) {
        }
    }

    struct RecordingParticles {
        log: Rc<RefCell<Vec<&'static str>>>,
    }

    impl PlatformEffectSink for RecordingParticles {
        fn apply_platform_effect(&mut self, _effect: PlatformEffect) {
            self.log.borrow_mut().push("platform");
        }
    }

    impl ParticleBatchSink for RecordingParticles {
        fn submit_particle_batch(&mut self, _batch: ParticleSpawnBatch) {
            self.log.borrow_mut().push("particles");
        }
    }

    fn local_sound() -> LocalSoundEventState {
        LocalSoundEventState {
            sound: SoundHolderState {
                kind: "direct".to_string(),
                registry_id: None,
                location: Some("minecraft:test".to_string()),
                fixed_range: None,
            },
            source: SoundSource::Master.as_str().to_string(),
            volume: 1.0,
            pitch: 1.0,
            seed: 1,
        }
    }

    fn ambient_loop() -> AmbientLoopSoundState {
        AmbientLoopSoundState {
            sound: ClientSoundEventHolder::Reference {
                resource_id: "minecraft:ambient.test".to_string(),
            },
            gain: 0.5,
            seed: 2,
        }
    }

    fn ordered_batch(sequence: u64) -> RenderEffectBatch {
        let collector = RenderEffectCollector::new(sequence, AudioCounters::default());
        let mut audio = collector.audio();
        let mut particles = collector.particles();
        audio.set_music_gain(0.5);
        audio.play_local_sound(&local_sound());
        particles.submit_particle_batch(ParticleSpawnBatch::default());
        collector
            .platform()
            .submit(PlatformEffect::SetCursorCapture(false));
        audio.sync_ambient_loops(&[ambient_loop()]);
        audio.play_local_sound(&local_sound());
        drop((audio, particles));
        collector.finish(None).expect("ordered effects")
    }

    fn reload_batch(sequence: u64, generation: u64) -> RenderEffectBatch {
        RenderEffectCollector::new(sequence, AudioCounters::default())
            .finish(Some(ResourceReloadEffect {
                request: ServerResourcePackReload {
                    generation,
                    server_packs: Vec::new(),
                },
                sound_events: SoundEventRegistry::default(),
                jukebox_songs: JukeboxSongRegistry::default(),
            }))
            .expect("resource reload effect")
    }

    #[test]
    fn empty_step_does_not_create_an_effect_batch() {
        assert!(RenderEffectCollector::new(1, AudioCounters::default())
            .finish(None)
            .is_none());
    }

    #[test]
    fn batch_is_send_and_consumer_preserves_fifo_at_most_once() {
        fn assert_send<T: Send>() {}
        assert_send::<RenderEffectBatch>();

        let log = Rc::new(RefCell::new(Vec::new()));
        let mut audio = RecordingAudio { log: log.clone() };
        let mut particles = RecordingParticles { log: log.clone() };
        let mut consumer = RenderEffectConsumer::default();

        assert!(consumer
            .consume(ordered_batch(1), Some(&mut audio), &mut particles,)
            .unwrap()
            .is_none());
        assert!(consumer
            .consume(ordered_batch(1), Some(&mut audio), &mut particles,)
            .unwrap()
            .is_none());
        assert_eq!(
            *log.borrow(),
            [
                "music_gain",
                "audio",
                "particles",
                "platform",
                "ambient_loops",
                "audio"
            ]
        );
    }

    #[test]
    fn consumer_rejects_a_lost_batch_sequence() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut audio = RecordingAudio { log: log.clone() };
        let mut particles = RecordingParticles { log };
        let mut consumer = RenderEffectConsumer::default();

        let error = consumer
            .consume(ordered_batch(2), Some(&mut audio), &mut particles)
            .err()
            .unwrap();
        assert_eq!(
            error,
            RenderEffectSequenceError {
                expected: 1,
                actual: 2,
            }
        );
    }

    #[test]
    fn duplicate_batch_does_not_repeat_terminal_resource_reload() {
        let log = Rc::new(RefCell::new(Vec::new()));
        let mut audio = RecordingAudio { log: log.clone() };
        let mut particles = RecordingParticles { log };
        let mut consumer = RenderEffectConsumer::default();

        let reload = consumer
            .consume(reload_batch(1, 7), Some(&mut audio), &mut particles)
            .unwrap()
            .unwrap();
        assert_eq!(reload.request.generation, 7);
        assert!(consumer
            .consume(reload_batch(1, 7), Some(&mut audio), &mut particles,)
            .unwrap()
            .is_none());
    }
}
