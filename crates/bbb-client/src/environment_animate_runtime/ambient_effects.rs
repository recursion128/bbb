use super::*;

pub(super) fn append_sculk_sensor_animate_effect(
    pos: BlockPos,
    properties: &BTreeMap<String, String>,
    random: &mut LevelEventSoundRandomState,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    if properties.get("sculk_sensor_phase").map(String::as_str) != Some("active") {
        return;
    }
    let direction = REDSTONE_EFFECT_DIRECTION_STEPS[random.next_int_bound(6) as usize];
    if direction[1] != 0 {
        return;
    }
    let x_offset = if direction[0] == 0 {
        0.5 - random.next_double()
    } else {
        f64::from(direction[0]) * 0.6
    };
    let z_offset = if direction[2] == 0 {
        0.5 - random.next_double()
    } else {
        f64::from(direction[2]) * 0.6
    };
    spawns.push(EnvironmentParticleSpawn {
        particle: ClientParticle::dust_transition(
            SCULK_PARTICLE_COLOR,
            REDSTONE_PARTICLE_COLOR,
            1.0,
        ),
        position: [
            f64::from(pos.x) + 0.5 + x_offset,
            f64::from(pos.y) + 0.25,
            f64::from(pos.z) + 0.5 + z_offset,
        ],
        velocity: [0.0, f64::from(random.next_float()) * 0.04, 0.0],
        always_show: false,
        color_source: None,
    });
}

pub(super) fn append_ambient_particle_spawns(
    random: &mut LevelEventSoundRandomState,
    pos: BlockPos,
    particles: &[bbb_protocol::environment_particles::AmbientParticle],
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    for particle in particles {
        if random.next_float() <= f32::from_bits(particle.probability_bits) {
            let position = [
                f64::from(pos.x) + random.next_double(),
                f64::from(pos.y) + random.next_double(),
                f64::from(pos.z) + random.next_double(),
            ];
            if let Some(particle) =
                ClientParticle::from_environment_with_registry(&particle.particle, content_registry)
            {
                spawns.push(EnvironmentParticleSpawn {
                    particle,
                    position,
                    velocity: [0.0; 3],
                    always_show: false,
                    color_source: None,
                });
            }
        }
    }
}

pub(super) fn append_firefly_spawn(
    random: &mut LevelEventSoundRandomState,
    pos: BlockPos,
    brightness: Option<u8>,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    let Some(brightness) = brightness else {
        return;
    };
    if brightness <= FIREFLY_SPAWN_MAX_BRIGHTNESS && random.next_double() <= FIREFLY_SPAWN_CHANCE {
        spawns.push(EnvironmentParticleSpawn {
            particle: ClientParticle::simple(ParticleKind::FIREFLY),
            position: [
                f64::from(pos.x) + random.next_double() * FIREFLY_HORIZONTAL_RANGE
                    - FIREFLY_HORIZONTAL_RANGE / 2.0,
                f64::from(pos.y) + random.next_double() * FIREFLY_VERTICAL_RANGE,
                f64::from(pos.z) + random.next_double() * FIREFLY_HORIZONTAL_RANGE
                    - FIREFLY_HORIZONTAL_RANGE / 2.0,
            ],
            velocity: [0.0; 3],
            always_show: false,
            color_source: None,
        });
    }
}

pub(super) fn sample_animate_position(
    random: &mut LevelEventSoundRandomState,
    center: BlockPos,
    radius: i32,
) -> BlockPos {
    BlockPos {
        x: center.x + random.next_int_bound(radius) - random.next_int_bound(radius),
        y: center.y + random.next_int_bound(radius) - random.next_int_bound(radius),
        z: center.z + random.next_int_bound(radius) - random.next_int_bound(radius),
    }
}

pub(super) fn direct_environment_bool(
    world: &WorldStore,
    pos: BlockPos,
    value: impl FnOnce(bbb_client_world::ClientEnvironmentAttributes) -> Option<bool>,
) -> bool {
    world
        .sampled_client_environment_attributes_at(
            block_position(pos),
            EnvironmentBiomeSampling::Direct,
        )
        .ok()
        .and_then(value)
        .unwrap_or(false)
}

pub(super) fn max_local_raw_brightness(
    world: &WorldStore,
    pos: BlockPos,
    sky_darken: u8,
) -> Option<u8> {
    if pos.x < -30_000_000 || pos.z < -30_000_000 || pos.x >= 30_000_000 || pos.z >= 30_000_000 {
        return Some(15);
    }
    world
        .sample_block_light(pos)
        .map(|light| light.block.max(light.sky.saturating_sub(sky_darken)))
}

pub(super) fn firefly_sound_height_allows(height: Option<i32>, block_y: i32) -> bool {
    height.is_some_and(|height| height <= block_y)
}

pub(super) fn creaking_heart_is_surrounded_by_logs(world: &WorldStore, pos: BlockPos) -> bool {
    [
        (0, -1, 0),
        (0, 1, 0),
        (0, 0, -1),
        (0, 0, 1),
        (-1, 0, 0),
        (1, 0, 0),
    ]
    .into_iter()
    .all(|(dx, dy, dz)| {
        world
            .block_view(BlockPos {
                x: pos.x + dx,
                y: pos.y + dy,
                z: pos.z + dz,
            })
            .and_then(|block| block.block_name)
            .is_some_and(block_name_is_pale_oak_log)
    })
}

pub(super) fn local_block_sound(
    world: &WorldStore,
    resource_id: &str,
    source: SoundSource,
    position: [f64; 3],
    volume: f32,
    pitch: f32,
    seed: i64,
) -> SoundEventState {
    SoundEventState {
        sound: sound_holder_state(
            world,
            &ClientSoundEventHolder::Reference {
                resource_id: resource_id.to_string(),
            },
        ),
        source: source.as_str().to_string(),
        position: ProtocolVec3d {
            x: position[0],
            y: position[1],
            z: position[2],
        },
        volume,
        pitch,
        seed,
        distance_delay: false,
    }
}

pub(super) fn block_position(pos: BlockPos) -> [f64; 3] {
    [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)]
}

pub(super) fn containing_block_coordinate(value: f64) -> i32 {
    value.floor() as i32
}
