use super::*;
use crate::{
    codec::{Decoder, Encoder},
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};

#[test]
fn decodes_sound_packet_wire_order_and_fixed_position() {
    let mut payload = Encoder::new();
    payload.write_var_i32(42);
    payload.write_var_i32(4);
    payload.write_i32(20);
    payload.write_i32(-8);
    payload.write_i32(0);
    payload.write_f32(0.75);
    payload.write_f32(1.25);
    payload.write_i64(123456789);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_SOUND, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::Sound(SoundEvent {
            sound: SoundEventHolder::Reference { registry_id: 41 },
            source: SoundSource::Blocks,
            position: Vec3d {
                x: 2.5,
                y: -1.0,
                z: 0.0,
            },
            volume: 0.75,
            pitch: 1.25,
            seed: 123456789,
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_var_i32().unwrap(), 4);
    assert_eq!(decoder.read_i32().unwrap(), 20);
    assert_eq!(decoder.read_i32().unwrap(), -8);
    assert_eq!(decoder.read_i32().unwrap(), 0);
    assert_eq!(decoder.read_f32().unwrap(), 0.75);
    assert_eq!(decoder.read_f32().unwrap(), 1.25);
    assert_eq!(decoder.read_i64().unwrap(), 123456789);
    assert!(decoder.is_empty());
}

#[test]
fn decodes_sound_entity_packet_with_direct_sound_holder() {
    let mut payload = Encoder::new();
    payload.write_var_i32(0);
    payload.write_string("minecraft:entity.cat.ambient");
    payload.write_bool(true);
    payload.write_f32(32.0);
    payload.write_var_i32(6);
    payload.write_var_i32(123);
    payload.write_f32(1.0);
    payload.write_f32(0.5);
    payload.write_i64(-9);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_SOUND_ENTITY, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::SoundEntity(SoundEntityEvent {
            sound: SoundEventHolder::Direct {
                location: "minecraft:entity.cat.ambient".to_string(),
                fixed_range: Some(32.0),
            },
            source: SoundSource::Neutral,
            entity_id: 123,
            volume: 1.0,
            pitch: 0.5,
            seed: -9,
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 0);
    assert_eq!(
        decoder.read_string(32767).unwrap(),
        "minecraft:entity.cat.ambient"
    );
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_f32().unwrap(), 32.0);
    assert_eq!(decoder.read_var_i32().unwrap(), 6);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert_eq!(decoder.read_f32().unwrap(), 1.0);
    assert_eq!(decoder.read_f32().unwrap(), 0.5);
    assert_eq!(decoder.read_i64().unwrap(), -9);
    assert!(decoder.is_empty());
}

#[test]
fn decodes_stop_sound_flags() {
    let mut payload = Encoder::new();
    payload.write_u8(3);
    payload.write_var_i32(1);
    payload.write_string("minecraft:music.menu");
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_STOP_SOUND, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::StopSound(StopSound {
            source: Some(SoundSource::Music),
            name: Some("minecraft:music.menu".to_string()),
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_u8().unwrap(), 3);
    assert_eq!(decoder.read_var_i32().unwrap(), 1);
    assert_eq!(decoder.read_string(32767).unwrap(), "minecraft:music.menu");
    assert!(decoder.is_empty());
}

#[test]
fn decodes_stop_all_sounds_packet() {
    let mut payload = Encoder::new();
    payload.write_u8(0);

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_STOP_SOUND, &payload.into_inner()).unwrap(),
        PlayClientbound::StopSound(StopSound {
            source: None,
            name: None,
        })
    );
}

#[test]
fn rejects_invalid_sound_source_ordinal() {
    let mut payload = Encoder::new();
    payload.write_var_i32(42);
    payload.write_var_i32(11);
    payload.write_i32(0);
    payload.write_i32(0);
    payload.write_i32(0);
    payload.write_f32(1.0);
    payload.write_f32(1.0);
    payload.write_i64(0);

    let err =
        decode_play_clientbound(ids::play::CLIENTBOUND_SOUND, &payload.into_inner()).unwrap_err();
    assert!(err.to_string().contains("invalid sound source ordinal 11"));
}

#[test]
fn rejects_negative_sound_event_holder_id() {
    let mut payload = Encoder::new();
    payload.write_var_i32(-1);
    payload.write_var_i32(0);
    payload.write_i32(0);
    payload.write_i32(0);
    payload.write_i32(0);
    payload.write_f32(1.0);
    payload.write_f32(1.0);
    payload.write_i64(0);

    let err =
        decode_play_clientbound(ids::play::CLIENTBOUND_SOUND, &payload.into_inner()).unwrap_err();
    assert!(err.to_string().contains("invalid sound event holder id -1"));
}
