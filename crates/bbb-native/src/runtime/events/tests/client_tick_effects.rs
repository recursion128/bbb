use super::*;
use crate::simulation::effects::ParticleBatchSink;
use bbb_world::{
    vanilla_entity_metadata_slime_size_accessor_for_entity_type, EntityClientMovementParticleState,
    SlimeLandingParticleKind,
};

#[derive(Default)]
struct RecordingParticleBatches {
    batches: Vec<bbb_renderer::ParticleSpawnBatch>,
}

impl ParticleBatchSink for RecordingParticleBatches {
    fn submit_particle_batch(&mut self, batch: bbb_renderer::ParticleSpawnBatch) {
        self.batches.push(batch);
    }
}

#[test]
fn slime_client_tick_effects_reach_native_particle_and_audio_sinks_in_order() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_type(
        160,
        VANILLA_ENTITY_TYPE_SLIME_ID,
    ));
    let (size_data_id, _) =
        vanilla_entity_metadata_slime_size_accessor_for_entity_type(VANILLA_ENTITY_TYPE_SLIME_ID)
            .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 160,
        values: vec![EntityDataValue {
            data_id: size_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(1),
        }],
    }));
    assert!(world.apply_entity_move(EntityMove {
        id: 160,
        has_position: false,
        delta_x: 0,
        delta_y: 0,
        delta_z: 0,
        y_rot: None,
        x_rot: None,
        on_ground: true,
    }));
    world.advance_entity_client_animations(1);

    let mut batches = RecordingParticleBatches::default();
    let mut particles = RecordingParticleSink::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particle_events: Option<&mut dyn ParticleEventSink> = Some(&mut particles);
    let mut audio_events: Option<&mut dyn crate::audio_runtime::AudioEventSink> = Some(&mut audio);
    crate::runtime::submit_entity_client_tick_effects(
        &mut batches,
        &mut world,
        &mut particle_events,
        &mut audio_events,
    );

    let [EntityClientMovementParticleState::SlimeLanding(landing)] =
        particles.entity_client_movement_states.as_slice()
    else {
        panic!(
            "expected one native slime landing particle state, got {:?}",
            particles.entity_client_movement_states
        );
    };
    assert_eq!(landing.entity_id, 160);
    assert_eq!(landing.kind, SlimeLandingParticleKind::ItemSlime);
    assert_eq!(landing.positions.len(), 17);
    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    let [AudioCommand::PlayPositionedSound(sound)] = audio.commands.as_slice() else {
        panic!("expected one positioned squish sound: {:?}", audio.commands);
    };
    assert_eq!(sound.sound.event_id, "minecraft:entity.slime.squish_small");
    assert_eq!(sound.category, AudioCategory::Hostile);
    assert_eq!(sound.position, [1.0, 64.0, -2.0]);
    assert_eq!(sound.packet_volume, 0.4);
    assert!((1.0..1.5).contains(&sound.packet_pitch));
    assert_eq!(batches.batches.len(), 2);
    assert!(world
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());
}

#[test]
fn warden_heartbeat_reaches_native_audio_with_vanilla_mix() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_type(
        161,
        VANILLA_ENTITY_TYPE_WARDEN_ID,
    ));
    world.advance_entity_client_animations(40);

    let mut batches = RecordingParticleBatches::default();
    let mut particles = RecordingParticleSink::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particle_events: Option<&mut dyn ParticleEventSink> = Some(&mut particles);
    let mut audio_events: Option<&mut dyn crate::audio_runtime::AudioEventSink> = Some(&mut audio);
    crate::runtime::submit_entity_client_tick_effects(
        &mut batches,
        &mut world,
        &mut particle_events,
        &mut audio_events,
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    let [AudioCommand::PlayPositionedSound(sound)] = audio.commands.as_slice() else {
        panic!(
            "expected one positioned heartbeat sound: {:?}",
            audio.commands
        );
    };
    assert_eq!(sound.sound.event_id, "minecraft:entity.warden.heartbeat");
    assert_eq!(sound.category, AudioCategory::Hostile);
    assert_eq!(sound.position, [1.0, 64.0, -2.0]);
    assert_eq!(sound.packet_volume, 5.0);
    assert!((0.8..1.2).contains(&sound.packet_pitch));
    assert!(particles.entity_client_movement_states.is_empty());
    assert_eq!(batches.batches.len(), 1);
    assert!(batches.batches[0].commands.is_empty());
    assert!(world
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());
}

#[test]
fn ender_dragon_flap_reaches_native_audio_with_vanilla_mix() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_type(
        164,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    ));
    // The generated default HOVERING phase advances flapTime by 0.1. One full
    // ten-tick phase cycle contains exactly one
    // `cos(old) <= -0.3 && cos(current) >= -0.3` rising crossing.
    world.advance_entity_client_animations(10);

    let mut batches = RecordingParticleBatches::default();
    let mut particles = RecordingParticleSink::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particle_events: Option<&mut dyn ParticleEventSink> = Some(&mut particles);
    let mut audio_events: Option<&mut dyn crate::audio_runtime::AudioEventSink> = Some(&mut audio);
    crate::runtime::submit_entity_client_tick_effects(
        &mut batches,
        &mut world,
        &mut particle_events,
        &mut audio_events,
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    let [AudioCommand::PlayPositionedSound(sound)] = audio.commands.as_slice() else {
        panic!("expected one positioned dragon flap: {:?}", audio.commands);
    };
    assert_eq!(sound.sound.event_id, "minecraft:entity.ender_dragon.flap");
    assert_eq!(sound.category, AudioCategory::Hostile);
    assert_eq!(sound.position, [1.0, 64.0, -2.0]);
    assert_eq!(sound.packet_volume, 5.0);
    assert!((0.8..1.1).contains(&sound.packet_pitch));
    assert!(particles.entity_client_movement_states.is_empty());
    assert_eq!(batches.batches.len(), 1);
    assert!(batches.batches[0].commands.is_empty());
    assert!(world
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());
}

#[test]
fn lightning_bolt_first_tick_reaches_native_weather_audio_in_order() {
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity_with_type(
        165,
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
    ));
    world.advance_entity_client_animations(1);

    let mut batches = RecordingParticleBatches::default();
    let mut particles = RecordingParticleSink::default();
    let mut audio = RecordingAudioSink::new(test_sound_catalog(), SoundEventRegistry::default());
    let mut particle_events: Option<&mut dyn ParticleEventSink> = Some(&mut particles);
    let mut audio_events: Option<&mut dyn crate::audio_runtime::AudioEventSink> = Some(&mut audio);
    crate::runtime::submit_entity_client_tick_effects(
        &mut batches,
        &mut world,
        &mut particle_events,
        &mut audio_events,
    );

    assert!(audio.errors.is_empty(), "{:?}", audio.errors);
    let [AudioCommand::PlayPositionedSound(thunder), AudioCommand::PlayPositionedSound(impact)] =
        audio.commands.as_slice()
    else {
        panic!(
            "expected ordered positioned thunder and impact sounds: {:?}",
            audio.commands
        );
    };
    assert_eq!(
        thunder.sound.event_id,
        "minecraft:entity.lightning_bolt.thunder"
    );
    assert_eq!(thunder.category, AudioCategory::Weather);
    assert_eq!(thunder.position, [1.0, 64.0, -2.0]);
    assert_eq!(thunder.packet_volume, 10_000.0);
    assert!((0.8..1.0).contains(&thunder.packet_pitch));
    assert_eq!(
        impact.sound.event_id,
        "minecraft:entity.lightning_bolt.impact"
    );
    assert_eq!(impact.category, AudioCategory::Weather);
    assert_eq!(impact.position, [1.0, 64.0, -2.0]);
    assert_eq!(impact.packet_volume, 2.0);
    assert!((0.5..0.7).contains(&impact.packet_pitch));
    assert!(particles.entity_client_movement_states.is_empty());
    assert_eq!(batches.batches.len(), 1);
    assert!(batches.batches[0].commands.is_empty());
    assert!(world
        .drain_entity_client_movement_effect_states()
        .next()
        .is_none());
}
