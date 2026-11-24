use super::*;
use crate::{
    codec::{Decoder, Encoder},
    ids,
    packets::{
        chunks, decode_play_clientbound, encode_explosion, encode_level_particles,
        encode_play_clientbound_explode, encode_play_clientbound_level_particles, BlockPos,
        PlayClientbound,
    },
    particle_types::{
        VANILLA_PARTICLE_TYPES, VANILLA_PARTICLE_TYPE_CLOUD_ID,
        VANILLA_PARTICLE_TYPE_EXPLOSION_EMITTER_ID, VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
    },
};

#[test]
fn decodes_typed_explosion_packet_wire_order() {
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
    payload.write_var_i32(VANILLA_PARTICLE_TYPE_EXPLOSION_EMITTER_ID);
    payload.write_var_i32(42);
    payload.write_var_i32(2);
    payload.write_var_i32(VANILLA_PARTICLE_TYPE_EXPLOSION_ID);
    payload.write_f32(0.5);
    payload.write_f32(1.25);
    payload.write_var_i32(3);
    payload.write_var_i32(VANILLA_PARTICLE_TYPE_CLOUD_ID);
    payload.write_f32(2.0);
    payload.write_f32(0.75);
    payload.write_var_i32(5);
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
            explosion_particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_EXPLOSION_EMITTER_ID,
                raw_options: Vec::new(),
            },
            explosion_sound: SoundEventHolder::Reference { registry_id: 41 },
            block_particles: vec![
                WeightedExplosionParticle {
                    particle: ParticlePayload {
                        particle_type_id: VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
                        raw_options: Vec::new(),
                    },
                    scaling: 0.5,
                    speed: 1.25,
                    weight: 3,
                },
                WeightedExplosionParticle {
                    particle: ParticlePayload {
                        particle_type_id: VANILLA_PARTICLE_TYPE_CLOUD_ID,
                        raw_options: Vec::new(),
                    },
                    scaling: 2.0,
                    speed: 0.75,
                    weight: 5,
                },
            ],
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
        decoder.read_var_i32().unwrap(),
        VANILLA_PARTICLE_TYPE_EXPLOSION_EMITTER_ID
    );
    assert_eq!(decoder.read_var_i32().unwrap(), 42);
    assert_eq!(decoder.read_var_i32().unwrap(), 2);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_PARTICLE_TYPE_EXPLOSION_ID
    );
    assert_eq!(decoder.read_f32().unwrap(), 0.5);
    assert_eq!(decoder.read_f32().unwrap(), 1.25);
    assert_eq!(decoder.read_var_i32().unwrap(), 3);
    assert_eq!(
        decoder.read_var_i32().unwrap(),
        VANILLA_PARTICLE_TYPE_CLOUD_ID
    );
    assert_eq!(decoder.read_f32().unwrap(), 2.0);
    assert_eq!(decoder.read_f32().unwrap(), 0.75);
    assert_eq!(decoder.read_var_i32().unwrap(), 5);
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
    payload.write_var_i32(VANILLA_PARTICLE_TYPE_EXPLOSION_ID);
    payload.write_var_i32(0);
    payload.write_string("minecraft:entity.generic.explode");
    payload.write_bool(true);
    payload.write_f32(24.0);
    payload.write_var_i32(0);

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
            explosion_particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
                raw_options: Vec::new(),
            },
            explosion_sound: SoundEventHolder::Direct {
                location: "minecraft:entity.generic.explode".to_string(),
                fixed_range: Some(24.0),
            },
            block_particles: Vec::new(),
        })
    );
}

#[test]
fn rejects_invalid_explosion_weighted_list_domains() {
    fn payload_with_tail(mut write_tail: impl FnMut(&mut Encoder)) -> Vec<u8> {
        let mut payload = Encoder::new();
        payload.write_f64(0.0);
        payload.write_f64(0.0);
        payload.write_f64(0.0);
        payload.write_f32(1.0);
        payload.write_i32(1);
        payload.write_bool(false);
        payload.write_var_i32(VANILLA_PARTICLE_TYPE_EXPLOSION_ID);
        payload.write_var_i32(1);
        write_tail(&mut payload);
        payload.into_inner()
    }

    let oversized = payload_with_tail(|payload| {
        payload.write_var_i32((MAX_EXPLOSION_BLOCK_PARTICLES + 1) as i32);
    });
    assert!(matches!(
        decode_play_clientbound(ids::play::CLIENTBOUND_EXPLODE, &oversized),
        Err(ProtocolError::PacketTooLarge(actual, max))
            if actual == MAX_EXPLOSION_BLOCK_PARTICLES + 1
                && max == MAX_EXPLOSION_BLOCK_PARTICLES
    ));

    let negative_count = payload_with_tail(|payload| payload.write_var_i32(-1));
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_EXPLODE, &negative_count)
            .unwrap_err()
            .to_string()
            .contains("invalid explosion block particle count -1")
    );

    let negative_weight = payload_with_tail(|payload| {
        payload.write_var_i32(1);
        payload.write_var_i32(VANILLA_PARTICLE_TYPE_CLOUD_ID);
        payload.write_f32(1.0);
        payload.write_f32(1.0);
        payload.write_var_i32(-1);
    });
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_EXPLODE, &negative_weight)
            .unwrap_err()
            .to_string()
            .contains("invalid negative explosion block particle weight -1")
    );

    let overflowed_weight = payload_with_tail(|payload| {
        payload.write_var_i32(2);
        for weight in [i32::MAX, 1] {
            payload.write_var_i32(VANILLA_PARTICLE_TYPE_CLOUD_ID);
            payload.write_f32(1.0);
            payload.write_f32(1.0);
            payload.write_var_i32(weight);
        }
    });
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_EXPLODE, &overflowed_weight)
            .unwrap_err()
            .to_string()
            .contains("total weight exceeds i32::MAX")
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

    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_LEVEL_PARTICLES, &trailing)
            .unwrap_err()
            .to_string()
            .contains("particle packet has 1 trailing byte(s)")
    );
}

#[test]
fn encodes_level_particles_packet_wire_order() {
    let mut particle_options = Encoder::new();
    particle_options.write_i32(0x00ff00);
    particle_options.write_f32(0.5);
    let particle_options = particle_options.into_inner();

    let packet = LevelParticles {
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
            raw_options: particle_options.clone(),
        },
    };

    let mut expected = Encoder::new();
    expected.write_bool(true);
    expected.write_bool(false);
    expected.write_f64(10.0);
    expected.write_f64(64.5);
    expected.write_f64(-3.25);
    expected.write_f32(0.1);
    expected.write_f32(0.2);
    expected.write_f32(0.3);
    expected.write_f32(1.5);
    expected.write_i32(16);
    expected.write_var_i32(14);
    expected.write_bytes(&particle_options);
    let expected = expected.into_inner();

    assert_eq!(encode_level_particles(&packet), expected);

    let (packet_id, payload) = encode_play_clientbound_level_particles(packet.clone());
    assert_eq!(packet_id, ids::play::CLIENTBOUND_LEVEL_PARTICLES);
    assert_eq!(payload, expected);
    assert_eq!(
        decode_play_clientbound(packet_id, &payload).unwrap(),
        PlayClientbound::LevelParticles(packet)
    );
}

#[test]
fn encodes_explosion_packet_wire_order() {
    let packet = Explosion {
        center: Vec3d {
            x: 1.5,
            y: 64.0625,
            z: -2.5,
        },
        radius: 4.0,
        block_count: 9,
        player_knockback: Some(Vec3d {
            x: 0.25,
            y: -0.5,
            z: 1.5,
        }),
        explosion_particle: ParticlePayload {
            particle_type_id: VANILLA_PARTICLE_TYPE_EXPLOSION_EMITTER_ID,
            raw_options: Vec::new(),
        },
        explosion_sound: SoundEventHolder::Reference { registry_id: 41 },
        block_particles: vec![
            WeightedExplosionParticle {
                particle: ParticlePayload {
                    particle_type_id: VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
                    raw_options: Vec::new(),
                },
                scaling: 0.5,
                speed: 1.25,
                weight: 3,
            },
            WeightedExplosionParticle {
                particle: ParticlePayload {
                    particle_type_id: VANILLA_PARTICLE_TYPE_CLOUD_ID,
                    raw_options: Vec::new(),
                },
                scaling: 2.0,
                speed: 0.75,
                weight: 5,
            },
        ],
    };

    let mut expected = Encoder::new();
    expected.write_f64(1.5);
    expected.write_f64(64.0625);
    expected.write_f64(-2.5);
    expected.write_f32(4.0);
    expected.write_i32(9);
    expected.write_bool(true);
    expected.write_f64(0.25);
    expected.write_f64(-0.5);
    expected.write_f64(1.5);
    expected.write_var_i32(VANILLA_PARTICLE_TYPE_EXPLOSION_EMITTER_ID);
    expected.write_var_i32(42);
    expected.write_var_i32(2);
    expected.write_var_i32(VANILLA_PARTICLE_TYPE_EXPLOSION_ID);
    expected.write_f32(0.5);
    expected.write_f32(1.25);
    expected.write_var_i32(3);
    expected.write_var_i32(VANILLA_PARTICLE_TYPE_CLOUD_ID);
    expected.write_f32(2.0);
    expected.write_f32(0.75);
    expected.write_var_i32(5);
    let expected = expected.into_inner();

    assert_eq!(encode_explosion(&packet).unwrap(), expected);

    let (packet_id, payload) = encode_play_clientbound_explode(packet.clone()).unwrap();
    assert_eq!(packet_id, ids::play::CLIENTBOUND_EXPLODE);
    assert_eq!(payload, expected);
    assert_eq!(
        decode_play_clientbound(packet_id, &payload).unwrap(),
        PlayClientbound::Explosion(packet)
    );
}

#[test]
fn encodes_explosion_packet_without_knockback_or_block_particles() {
    let packet = Explosion {
        center: Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        radius: 1.0,
        block_count: 0,
        player_knockback: None,
        explosion_particle: ParticlePayload {
            particle_type_id: VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
            raw_options: Vec::new(),
        },
        explosion_sound: SoundEventHolder::Direct {
            location: "minecraft:entity.generic.explode".to_string(),
            fixed_range: Some(16.0),
        },
        block_particles: Vec::new(),
    };
    let (packet_id, payload) = encode_play_clientbound_explode(packet.clone()).unwrap();
    assert_eq!(
        decode_play_clientbound(packet_id, &payload).unwrap(),
        PlayClientbound::Explosion(packet)
    );
}

#[test]
fn rejects_unknown_level_particle_type() {
    let mut payload = Encoder::new();
    payload.write_bool(false);
    payload.write_bool(false);
    payload.write_f64(0.0);
    payload.write_f64(0.0);
    payload.write_f64(0.0);
    payload.write_f32(0.0);
    payload.write_f32(0.0);
    payload.write_f32(0.0);
    payload.write_f32(0.0);
    payload.write_i32(1);
    payload.write_var_i32(117);

    assert!(decode_play_clientbound(
        ids::play::CLIENTBOUND_LEVEL_PARTICLES,
        &payload.into_inner(),
    )
    .unwrap_err()
    .to_string()
    .contains("unknown particle type id 117"));
}

#[test]
fn every_vanilla_particle_type_has_a_wire_decoder() {
    for particle_type in VANILLA_PARTICLE_TYPES {
        let mut options = Encoder::new();
        match particle_type.id {
            1 | 2 | 29 | 105 | 111 | 115 => options.write_var_i32(0),
            8 | 38 => options.write_f32(0.0),
            14 => {
                options.write_i32(0);
                options.write_f32(1.0);
            }
            15 => {
                options.write_i32(0);
                options.write_i32(0);
                options.write_f32(1.0);
            }
            16 | 46 => {
                options.write_i32(0);
                options.write_f32(0.0);
            }
            21 | 36 | 42 => options.write_i32(0),
            47 => {
                options.write_var_i32(0);
                options.write_var_i32(1);
                options.write_var_i32(0);
                options.write_var_i32(0);
            }
            48 => {
                options.write_var_i32(0);
                options.write_i64(chunks::encode_block_pos(BlockPos { x: 0, y: 0, z: 0 }));
                options.write_var_i32(0);
            }
            49 => {
                options.write_f64(0.0);
                options.write_f64(0.0);
                options.write_f64(0.0);
                options.write_i32(0);
                options.write_var_i32(0);
            }
            _ if particle_type.codec_family == VanillaParticleCodecFamily::Simple => {}
            id => panic!("particle type {id} has no test wire payload"),
        }
        let options = options.into_inner();
        let packet = decode_level_particles_with_options(particle_type.id, &options);
        assert_eq!(packet.particle.particle_type_id, particle_type.id);
        assert_eq!(packet.particle.raw_options, options);
    }
}

#[test]
fn classifies_every_vanilla_particle_command_codec() {
    for particle_type in VANILLA_PARTICLE_TYPES {
        let options = vanilla_particle_command_options(particle_type.resource_id)
            .expect("generated particle must have command options");
        let expected = if particle_type.codec_family == VanillaParticleCodecFamily::Simple {
            VanillaParticleCommandOptions::Empty
        } else if matches!(
            particle_type.resource_id,
            "minecraft:dragon_breath" | "minecraft:effect" | "minecraft:instant_effect"
        ) {
            VanillaParticleCommandOptions::OptionalMap
        } else {
            VanillaParticleCommandOptions::RequiredMap
        };
        assert_eq!(options, expected, "{}", particle_type.resource_id);
    }
    assert_eq!(vanilla_particle_command_options("minecraft:missing"), None);
}

#[test]
fn decodes_item_particle_options_from_item_stack_template() {
    let mut particle_options = Encoder::new();
    particle_options.write_var_i32(42);
    particle_options.write_var_i32(3);
    particle_options.write_var_i32(0);
    particle_options.write_var_i32(0);
    let particle_options = particle_options.into_inner();

    let packet = decode_level_particles_with_options(47, &particle_options);

    assert_eq!(
        packet.particle,
        ParticlePayload {
            particle_type_id: 47,
            raw_options: particle_options,
        }
    );
}

#[test]
fn decodes_vibration_particle_options_with_block_position_source() {
    let mut particle_options = Encoder::new();
    particle_options.write_var_i32(0);
    particle_options.write_i64(chunks::encode_block_pos(BlockPos { x: 1, y: 64, z: -2 }));
    particle_options.write_var_i32(20);
    let particle_options = particle_options.into_inner();

    let packet = decode_level_particles_with_options(48, &particle_options);

    assert_eq!(
        packet.particle,
        ParticlePayload {
            particle_type_id: 48,
            raw_options: particle_options,
        }
    );
}

#[test]
fn decodes_vibration_particle_options_with_entity_position_source() {
    let mut particle_options = Encoder::new();
    particle_options.write_var_i32(1);
    particle_options.write_var_i32(123);
    particle_options.write_f32(1.25);
    particle_options.write_var_i32(9);
    let particle_options = particle_options.into_inner();

    let mut payload = Encoder::new();
    payload.write_var_i32(48);
    payload.write_bytes(&particle_options);
    let payload = payload.into_inner();

    let mut decoder = Decoder::new(&payload);
    assert_eq!(
        decode_particle_payload(&mut decoder).unwrap(),
        ParticlePayload {
            particle_type_id: 48,
            raw_options: particle_options,
        }
    );
    assert!(decoder.is_empty());
}

/// `ParticleTypes.STREAM_CODEC` (`ParticleTypes.java:158-159`) as the official
/// 26.1 jar wrote each option class after `/particle` decoded it, and the
/// client decoders reading the same bytes back.
#[test]
fn particle_options_write_the_official_option_bytes() {
    use crate::packets::{DataComponentPatchSummary, ItemStackTemplateSummary, Vec3d};
    use crate::particle_types::{
        VANILLA_PARTICLE_TYPE_BLOCK_ID, VANILLA_PARTICLE_TYPE_DRAGON_BREATH_ID,
        VANILLA_PARTICLE_TYPE_DUST_COLOR_TRANSITION_ID, VANILLA_PARTICLE_TYPE_DUST_ID,
        VANILLA_PARTICLE_TYPE_EFFECT_ID, VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID,
        VANILLA_PARTICLE_TYPE_FLAME_ID, VANILLA_PARTICLE_TYPE_ITEM_ID,
        VANILLA_PARTICLE_TYPE_SCULK_CHARGE_ID, VANILLA_PARTICLE_TYPE_SHRIEK_ID,
        VANILLA_PARTICLE_TYPE_TRAIL_ID, VANILLA_PARTICLE_TYPE_VIBRATION_ID,
    };

    let red = 0xffff_0000_u32 as i32;
    let stone = |component_patch| {
        ParticleOptions::Item(Box::new(ItemStackTemplateSummary {
            item_id: crate::item_types::VANILLA_ITEM_STONE_ID,
            count: 1,
            component_patch,
        }))
    };
    let cases = [
        (VANILLA_PARTICLE_TYPE_FLAME_ID, ParticleOptions::Simple, ""),
        (
            VANILLA_PARTICLE_TYPE_BLOCK_ID,
            ParticleOptions::Block { state_id: 136 },
            "8801",
        ),
        (
            VANILLA_PARTICLE_TYPE_DRAGON_BREATH_ID,
            ParticleOptions::Power { power: 2.0 },
            "40000000",
        ),
        (
            VANILLA_PARTICLE_TYPE_DUST_ID,
            ParticleOptions::Dust {
                color: red,
                scale: 1.0,
            },
            "ffff00003f800000",
        ),
        (
            VANILLA_PARTICLE_TYPE_DUST_COLOR_TRANSITION_ID,
            ParticleOptions::DustColorTransition {
                from_color: red,
                to_color: 0xff00_00ff_u32 as i32,
                scale: 1.0,
            },
            "ffff0000ff0000ff3f800000",
        ),
        (
            VANILLA_PARTICLE_TYPE_EFFECT_ID,
            ParticleOptions::Spell {
                color: -1,
                power: 1.0,
            },
            "ffffffff3f800000",
        ),
        (
            VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID,
            ParticleOptions::Color { color: red },
            "ffff0000",
        ),
        (
            VANILLA_PARTICLE_TYPE_SCULK_CHARGE_ID,
            ParticleOptions::SculkCharge { roll: 1.5 },
            "3fc00000",
        ),
        (
            VANILLA_PARTICLE_TYPE_ITEM_ID,
            stone(Default::default()),
            "01010000",
        ),
        (
            VANILLA_PARTICLE_TYPE_ITEM_ID,
            stone(
                DataComponentPatchSummary {
                    added_type_ids: vec![1],
                    max_stack_size: Some(64),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
            ),
            "010101000140",
        ),
        (
            VANILLA_PARTICLE_TYPE_VIBRATION_ID,
            ParticleOptions::Vibration {
                destination: BlockPos { x: 1, y: 2, z: 3 },
                arrival_in_ticks: 20,
            },
            "00000000400000300214",
        ),
        (
            VANILLA_PARTICLE_TYPE_VIBRATION_ID,
            ParticleOptions::VibrationEntity {
                entity_id: 7,
                y_offset: 0.5,
                arrival_in_ticks: 20,
            },
            "01073f00000014",
        ),
        (
            VANILLA_PARTICLE_TYPE_TRAIL_ID,
            ParticleOptions::Trail {
                target: Vec3d {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                color: red,
                duration: 5,
            },
            "3ff000000000000040000000000000004008000000000000ffff000005",
        ),
        (
            VANILLA_PARTICLE_TYPE_SHRIEK_ID,
            ParticleOptions::Shriek { delay: -1 },
            "ffffffff0f",
        ),
    ];
    for (particle_type_id, options, expected) in cases {
        let payload = options.payload(particle_type_id).unwrap();
        let hex: String = payload
            .raw_options
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(hex, expected, "{options:?}");
        let packet = decode_level_particles_with_options(particle_type_id, &payload.raw_options);
        assert_eq!(packet.particle, payload);
        assert_eq!(
            ParticleOptions::decode(particle_type_id, &payload.raw_options).unwrap(),
            options
        );
    }
}

/// An option value only encodes for a type registered with its class, and an
/// item template refuses what `ItemStackTemplate`'s constructor refuses
/// (`ItemStackTemplate.java:52-60`).
#[test]
fn particle_options_refuse_a_foreign_type_or_an_empty_item() {
    use crate::packets::ItemStackTemplateSummary;
    use crate::particle_types::{VANILLA_PARTICLE_TYPE_FLAME_ID, VANILLA_PARTICLE_TYPE_ITEM_ID};

    let dust = ParticleOptions::Dust {
        color: -1,
        scale: 1.0,
    };
    assert!(dust.payload(VANILLA_PARTICLE_TYPE_FLAME_ID).is_err());
    assert!(dust.payload(-1).is_err());
    let air = ParticleOptions::Item(Box::new(ItemStackTemplateSummary {
        item_id: crate::item_types::VANILLA_ITEM_AIR_ID,
        count: 1,
        component_patch: Default::default(),
    }));
    assert!(air.payload(VANILLA_PARTICLE_TYPE_ITEM_ID).is_err());
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

    let mut trailing = payload;
    trailing.push(0);
    assert!(
        decode_play_clientbound(ids::play::CLIENTBOUND_PROJECTILE_POWER, &trailing)
            .unwrap_err()
            .to_string()
            .contains("trailing bytes after projectile power packet")
    );
}

#[test]
fn encodes_projectile_power_packet() {
    let update = ProjectilePower {
        entity_id: 123,
        acceleration_power: 0.75,
    };
    let (packet_id, payload) = encode_play_clientbound_projectile_power(update);
    assert_eq!(packet_id, ids::play::CLIENTBOUND_PROJECTILE_POWER);
    assert_eq!(
        decode_play_clientbound(packet_id, &payload).unwrap(),
        PlayClientbound::ProjectilePower(update)
    );
}

fn decode_level_particles_with_options(
    particle_type_id: i32,
    particle_options: &[u8],
) -> LevelParticles {
    let mut payload = Encoder::new();
    payload.write_bool(false);
    payload.write_bool(false);
    payload.write_f64(0.0);
    payload.write_f64(0.0);
    payload.write_f64(0.0);
    payload.write_f32(0.0);
    payload.write_f32(0.0);
    payload.write_f32(0.0);
    payload.write_f32(0.0);
    payload.write_i32(1);
    payload.write_var_i32(particle_type_id);
    payload.write_bytes(particle_options);

    match decode_play_clientbound(
        ids::play::CLIENTBOUND_LEVEL_PARTICLES,
        &payload.into_inner(),
    )
    .unwrap()
    {
        PlayClientbound::LevelParticles(packet) => packet,
        other => panic!("expected level particles packet, got {other:?}"),
    }
}
