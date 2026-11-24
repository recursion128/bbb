use super::*;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;

fn state(resource_id: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(resource_id).unwrap().id)
        .unwrap()
}

#[test]
fn body_fall_particles_power_threshold_uses_epsilon_safe_attribute_and_java_int_cast() {
    let stone = state("minecraft:stone");
    for (distance, safe, expected) in [
        (3.999_998, 3.0, None),
        (3.999_999_5, 3.0, Some(40)),
        (10.0, 9.0, Some(40)),
        (10.0, 10.0, None),
        (0.0, -1.0, Some(40)),
        (f64::NAN, 3.0, None),
        (f64::NEG_INFINITY, 3.0, None),
        (f64::INFINITY, 3.0, Some(375)),
    ] {
        assert_eq!(
            body_fall_particles(
                Vec3d::default(),
                BlockPos { x: 0, y: 0, z: 0 },
                stone,
                distance,
                safe
            )
            .map(|packet| packet.count),
            expected,
            "distance={distance}, safe={safe}",
        );
    }
}

#[test]
fn body_fall_particles_count_keeps_widened_float_scale_and_caps_at_375() {
    for (power, expected) in [
        (1, 40),
        (19, 220),
        (20, 230),
        (21, 240),
        (22, 250),
        (34, 370),
        (35, 375),
        (100, 375),
    ] {
        let packet = body_fall_particles(
            Vec3d::default(),
            BlockPos { x: 0, y: 0, z: 0 },
            state("minecraft:stone"),
            f64::from(power + 3),
            3.0,
        )
        .unwrap();
        assert_eq!(packet.count, expected, "power={power}");
    }
}

#[test]
fn body_fall_particles_air_gate_and_block_state_varint_payload() {
    for resource in ["minecraft:air", "minecraft:cave_air", "minecraft:void_air"] {
        assert!(body_fall_particles(
            Vec3d::default(),
            BlockPos { x: 0, y: 0, z: 0 },
            state(resource),
            8.0,
            3.0
        )
        .is_none());
    }
    let block_state = state("minecraft:oak_stairs");
    assert!(block_state > 127);
    let packet = body_fall_particles(
        Vec3d::default(),
        BlockPos { x: 0, y: 0, z: 0 },
        block_state,
        8.0,
        3.0,
    )
    .unwrap();
    assert_eq!(
        packet.particle.particle_type_id,
        VANILLA_PARTICLE_TYPE_BLOCK_ID
    );
    let mut decoder = bbb_protocol::codec::Decoder::new(&packet.particle.raw_options);
    assert_eq!(decoder.read_var_i32().unwrap(), block_state);
    assert!(decoder.remaining().is_empty());
    assert!(!packet.override_limiter);
    assert!(!packet.always_show);
    assert_eq!(packet.offset, Vec3d::default());
    assert_eq!(packet.max_speed.to_bits(), 0.15_f32.to_bits());
}

#[test]
fn body_fall_particles_keeps_same_column_and_projects_different_column_to_support_edge() {
    for (position, support, expected) in [
        (
            Vec3d {
                x: -0.1,
                y: 42.0,
                z: -0.9,
            },
            BlockPos {
                x: -1,
                y: -100,
                z: -1,
            },
            Vec3d {
                x: -0.1,
                y: 42.0,
                z: -0.9,
            },
        ),
        (
            Vec3d {
                x: 2.5,
                y: 42.0,
                z: 1.5,
            },
            BlockPos { x: 0, y: 41, z: 0 },
            Vec3d {
                x: 1.0,
                y: 42.0,
                z: 0.75,
            },
        ),
        (
            Vec3d {
                x: -1.5,
                y: 42.0,
                z: -1.5,
            },
            BlockPos { x: 0, y: 41, z: 0 },
            Vec3d {
                x: 0.0,
                y: 42.0,
                z: 0.0,
            },
        ),
        (
            Vec3d {
                x: 0.5,
                y: 42.0,
                z: 2.5,
            },
            BlockPos { x: 0, y: 41, z: 0 },
            Vec3d {
                x: 0.5,
                y: 42.0,
                z: 1.0,
            },
        ),
    ] {
        let packet =
            body_fall_particles(position, support, state("minecraft:stone"), 8.0, 3.0).unwrap();
        assert_eq!(packet.position, expected);
    }
}

#[test]
fn body_fall_damage_keeps_double_distance_epsilon_and_widened_float_modifier() {
    for (distance, modifier, expected) in [
        (2.5, 1.0, -1),
        (3.999_998, 1.0, 0),
        (3.999_999_5, 1.0, 1),
        (7.999_998_95, 0.2, 1),
        (10.0, 0.0, 0),
    ] {
        assert_eq!(
            calculate_body_fall_damage(distance, modifier, 3.0, 1.0, false),
            expected,
        );
    }
    assert_eq!(calculate_body_fall_damage(10.0, 1.0, 6.0, 0.5, false), 2);
    assert_eq!(
        calculate_body_fall_damage(f64::INFINITY, 1.0, 3.0, 1.0, true),
        0,
    );
}

#[test]
fn stored_fall_damage_reads_modified_attributes_and_keeps_immunity() {
    let mut living = LivingEntityState::new("minecraft:armor_stand").unwrap();
    living
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_SAFE_FALL_DISTANCE_ID)
        .unwrap()
        .set_base_value(6.0);
    living
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_FALL_DAMAGE_MULTIPLIER_ID)
        .unwrap()
        .set_base_value(0.5);
    assert_eq!(calculate_fall_damage(&living, 10.0, 1.0, false), 2);
    assert_eq!(calculate_fall_damage(&living, 10.0, 0.2, false), 0);
    assert_eq!(calculate_fall_damage(&living, 10.0, 1.0, true), 0);
}

#[test]
fn body_block_fall_sound_uses_float_foot_offset_and_preserves_source_and_position() {
    for (height, expected_y) in [(1.2, 0), (1.0 + f64::from(0.2_f32), 1)] {
        let position = Vec3d {
            x: -0.1,
            y: height,
            z: -0.9,
        };
        let once = Box::new(state("minecraft:stone"));
        let sound = body_block_fall_sound(position, SoundSource::Players, move |pos| {
            assert_eq!(
                pos,
                BlockPos {
                    x: -1,
                    y: expected_y,
                    z: -1
                }
            );
            let block_state = *once;
            drop(once);
            block_state
        })
        .unwrap();
        assert_eq!(sound.position, position);
        assert_eq!(sound.source, SoundSource::Players);
        assert_eq!(sound.resource_id, "minecraft:block.stone.fall");
        assert_eq!(sound.volume, 0.5);
        assert_eq!(sound.pitch, 0.75);
    }
}

#[test]
fn body_block_fall_sound_scales_catalog_volume_and_pitch() {
    for (resource, sound_id, volume, pitch) in [
        ("minecraft:anvil", "minecraft:block.anvil.fall", 0.15, 0.75),
        (
            "minecraft:twisting_vines",
            "minecraft:block.weeping_vines.fall",
            0.5,
            0.75,
        ),
    ] {
        let sound =
            body_block_fall_sound(Vec3d::default(), SoundSource::Neutral, |_| state(resource))
                .unwrap();
        assert_eq!(sound.resource_id, sound_id);
        assert_eq!(sound.volume, volume);
        assert_eq!(sound.pitch, pitch);
    }
}

#[test]
fn body_block_fall_sound_skips_all_air_variants() {
    for block_state in [
        state("minecraft:air"),
        state("minecraft:cave_air"),
        state("minecraft:void_air"),
    ] {
        assert!(
            body_block_fall_sound(Vec3d::default(), SoundSource::Players, |_| block_state)
                .is_none()
        );
    }
}
