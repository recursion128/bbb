use super::*;
use crate::{
    codec::{Decoder, Encoder},
    ids,
    packets::{decode_play_clientbound, PlayClientbound},
};

#[test]
fn decodes_explosion_packet_prefix_and_raw_effect_payload() {
    let mut payload = Encoder::new();
    payload.write_f64(1.0);
    payload.write_f64(2.0);
    payload.write_f64(3.0);
    payload.write_f32(4.5);
    payload.write_i32(7);
    payload.write_bool(true);
    payload.write_f64(0.25);
    payload.write_f64(-0.5);
    payload.write_f64(1.5);
    payload.write_bytes(&[0x2d, 0x2a, 0x01, 0x00]);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_EXPLODE, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::Explosion(Explosion {
            center: Vec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            radius: 4.5,
            block_count: 7,
            player_knockback: Some(Vec3d {
                x: 0.25,
                y: -0.5,
                z: 1.5,
            }),
            raw_effect_payload: vec![0x2d, 0x2a, 0x01, 0x00],
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_f64().unwrap(), 1.0);
    assert_eq!(decoder.read_f64().unwrap(), 2.0);
    assert_eq!(decoder.read_f64().unwrap(), 3.0);
    assert_eq!(decoder.read_f32().unwrap(), 4.5);
    assert_eq!(decoder.read_i32().unwrap(), 7);
    assert!(decoder.read_bool().unwrap());
    assert_eq!(decoder.read_f64().unwrap(), 0.25);
    assert_eq!(decoder.read_f64().unwrap(), -0.5);
    assert_eq!(decoder.read_f64().unwrap(), 1.5);
    assert_eq!(
        decoder
            .read_exact(decoder.remaining_len(), "explosion effect payload")
            .unwrap(),
        &[0x2d, 0x2a, 0x01, 0x00]
    );
    assert!(decoder.is_empty());
}

#[test]
fn decodes_explosion_without_knockback() {
    let mut payload = Encoder::new();
    payload.write_f64(0.0);
    payload.write_f64(0.0);
    payload.write_f64(0.0);
    payload.write_f32(1.0);
    payload.write_i32(0);
    payload.write_bool(false);
    payload.write_bytes(&[0x2d]);

    assert_eq!(
        decode_play_clientbound(ids::play::CLIENTBOUND_EXPLODE, &payload.into_inner()).unwrap(),
        PlayClientbound::Explosion(Explosion {
            center: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            radius: 1.0,
            block_count: 0,
            player_knockback: None,
            raw_effect_payload: vec![0x2d],
        })
    );
}

#[test]
fn decodes_level_particles_packet_wire_order() {
    let mut particle_options = Encoder::new();
    particle_options.write_i32(0x00ff00);
    particle_options.write_f32(0.5);
    let particle_options = particle_options.into_inner();

    let mut payload = Encoder::new();
    payload.write_bool(true);
    payload.write_bool(false);
    payload.write_f64(10.0);
    payload.write_f64(64.5);
    payload.write_f64(-3.25);
    payload.write_f32(0.1);
    payload.write_f32(0.2);
    payload.write_f32(0.3);
    payload.write_f32(1.5);
    payload.write_i32(16);
    payload.write_var_i32(14);
    payload.write_bytes(&particle_options);
    let payload = payload.into_inner();

    let packet = decode_play_clientbound(ids::play::CLIENTBOUND_LEVEL_PARTICLES, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::LevelParticles(LevelParticles {
            override_limiter: true,
            always_show: false,
            position: Vec3d {
                x: 10.0,
                y: 64.5,
                z: -3.25,
            },
            offset: Vec3d {
                x: f64::from(0.1_f32),
                y: f64::from(0.2_f32),
                z: f64::from(0.3_f32),
            },
            max_speed: 1.5,
            count: 16,
            particle: ParticlePayload {
                particle_type_id: 14,
                raw_options: particle_options,
            },
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert!(decoder.read_bool().unwrap());
    assert!(!decoder.read_bool().unwrap());
    assert_eq!(decoder.read_f64().unwrap(), 10.0);
    assert_eq!(decoder.read_f64().unwrap(), 64.5);
    assert_eq!(decoder.read_f64().unwrap(), -3.25);
    assert_eq!(decoder.read_f32().unwrap(), 0.1);
    assert_eq!(decoder.read_f32().unwrap(), 0.2);
    assert_eq!(decoder.read_f32().unwrap(), 0.3);
    assert_eq!(decoder.read_f32().unwrap(), 1.5);
    assert_eq!(decoder.read_i32().unwrap(), 16);
    assert_eq!(decoder.read_var_i32().unwrap(), 14);
    assert_eq!(decoder.read_i32().unwrap(), 0x00ff00);
    assert_eq!(decoder.read_f32().unwrap(), 0.5);
    assert!(decoder.is_empty());
}

#[test]
fn decodes_projectile_power_packet() {
    let mut payload = Encoder::new();
    payload.write_var_i32(123);
    payload.write_f64(0.75);
    let payload = payload.into_inner();

    let packet =
        decode_play_clientbound(ids::play::CLIENTBOUND_PROJECTILE_POWER, &payload).unwrap();
    assert_eq!(
        packet,
        PlayClientbound::ProjectilePower(ProjectilePower {
            entity_id: 123,
            acceleration_power: 0.75,
        })
    );

    let mut decoder = Decoder::new(&payload);
    assert_eq!(decoder.read_var_i32().unwrap(), 123);
    assert_eq!(decoder.read_f64().unwrap(), 0.75);
    assert!(decoder.is_empty());
}
