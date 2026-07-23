use bbb_protocol::{
    heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    packets::{SoundSource, Vec3d as ProtocolVec3d},
    NbtSummaryEntry, NbtSummaryValue,
};
use bbb_world::{
    block_name_is_pale_oak_log, BlockPos, ClientParticleOptions, ClientSoundEventHolder,
    EnvironmentBiomeSampling, LevelEventSoundRandomState, LocalPlayerPoseState, SoundEventState,
    WorldStore,
};

use crate::{
    audio_runtime::AudioEventSink,
    environment_audio_runtime::sound_holder_state,
    particle_runtime::{
        EnvironmentParticleSpawn, LevelParticleSpawnContext, ParticleBiomeSampler,
        ParticleEventSink,
    },
    simulation::effects::ParticleBatchSink,
};

const ANIMATE_TICK_ITERATIONS: usize = 667;
const NEAR_SAMPLE_RADIUS: i32 = 16;
const FAR_SAMPLE_RADIUS: i32 = 32;
const FIREFLY_SOUND_CHANCE_BOUND: i32 = 30;
const FIREFLY_SPAWN_MAX_BRIGHTNESS: u8 = 13;
const FIREFLY_SPAWN_CHANCE: f64 = 0.7;
const FIREFLY_HORIZONTAL_RANGE: f64 = 10.0;
const FIREFLY_VERTICAL_RANGE: f64 = 5.0;
const CREAKING_SOUND_CHANCE_BOUND: i32 = 16;
const SOUND_EVENT_FIREFLY_BUSH_IDLE: &str = "minecraft:block.firefly_bush.idle";
const SOUND_EVENT_CREAKING_HEART_IDLE: &str = "minecraft:block.creaking_heart.idle";
const PARTICLE_FIREFLY: &str = "minecraft:firefly";

/// Non-serializable owner of Vanilla `ClientLevel.animateTick` random cursors.
#[derive(Debug)]
pub(crate) struct EnvironmentAnimateRuntime {
    level_identity: Option<(u64, u64)>,
    level_random: LevelEventSoundRandomState,
    animate_seed_random: LevelEventSoundRandomState,
}

impl Default for EnvironmentAnimateRuntime {
    fn default() -> Self {
        Self {
            level_identity: None,
            level_random: LevelEventSoundRandomState::with_seed(0),
            animate_seed_random: LevelEventSoundRandomState::with_seed(1),
        }
    }
}

impl EnvironmentAnimateRuntime {
    pub(crate) fn advance_for_ticks(
        &mut self,
        world: &mut WorldStore,
        connection_generation: u64,
        running_level_ticks: u32,
        mut particle_events: Option<&mut dyn ParticleEventSink>,
        particle_batches: &mut dyn ParticleBatchSink,
        audio: &mut Option<&mut dyn AudioEventSink>,
    ) {
        let level_identity = (connection_generation, world.client_level_generation());
        if self.level_identity != Some(level_identity) {
            self.level_identity = Some(level_identity);
            self.level_random = LevelEventSoundRandomState::with_seed(0);
            self.animate_seed_random = LevelEventSoundRandomState::with_seed(1);
        }

        for _ in 0..running_level_ticks {
            let Some(player) = world.local_player_pose() else {
                continue;
            };
            let spawns = self.tick(world, player, audio);
            if spawns.is_empty() {
                continue;
            }
            let Some(particle_events) = particle_events.as_deref_mut() else {
                continue;
            };
            let biome_sampler = EnvironmentWorldParticleBiomeSampler { world };
            let batch = particle_events.spawn_environment_particles(
                &spawns,
                LevelParticleSpawnContext {
                    camera_position: Some([
                        player.position.x,
                        player.position.y + player.eye_height(),
                        player.position.z,
                    ]),
                    vibration_entity_position: None,
                },
                Some(&biome_sampler),
            );
            if !batch.is_empty() {
                particle_batches.submit_particle_batch(batch);
            }
        }
    }

    fn tick(
        &mut self,
        world: &mut WorldStore,
        player: LocalPlayerPoseState,
        audio: &mut Option<&mut dyn AudioEventSink>,
    ) -> Vec<EnvironmentParticleSpawn> {
        let center = BlockPos {
            x: containing_block_coordinate(player.position.x),
            y: containing_block_coordinate(player.position.y),
            z: containing_block_coordinate(player.position.z),
        };
        let sky_darken = crate::runtime::entity_shadow_sky_darken(world);
        let mut animate_random =
            LevelEventSoundRandomState::with_seed(self.animate_seed_random.next_long());
        let mut spawns = Vec::new();
        for _ in 0..ANIMATE_TICK_ITERATIONS {
            self.do_animate_tick(
                world,
                center,
                NEAR_SAMPLE_RADIUS,
                sky_darken,
                &mut animate_random,
                audio,
                &mut spawns,
            );
            self.do_animate_tick(
                world,
                center,
                FAR_SAMPLE_RADIUS,
                sky_darken,
                &mut animate_random,
                audio,
                &mut spawns,
            );
        }
        spawns
    }

    fn do_animate_tick(
        &mut self,
        world: &mut WorldStore,
        center: BlockPos,
        radius: i32,
        sky_darken: u8,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        let pos = sample_animate_position(&mut self.level_random, center, radius);
        let Some(block) = world.probe_block(pos) else {
            return;
        };
        let Some(block_name) = block.block_name.as_deref() else {
            return;
        };

        match block_name {
            "minecraft:firefly_bush" => {
                self.animate_firefly_bush(world, pos, sky_darken, animate_random, audio, spawns);
            }
            "minecraft:creaking_heart" => {
                self.animate_creaking_heart(
                    world,
                    pos,
                    block
                        .block_properties
                        .get("creaking_heart_state")
                        .map(String::as_str),
                    animate_random,
                    audio,
                );
            }
            _ => {}
        }

        if world.sample_block_collision_shape_is_full_block(pos) != Some(false) {
            return;
        }
        let particles = world
            .sampled_client_environment_attributes_at(
                block_position(pos),
                EnvironmentBiomeSampling::Direct,
            )
            .ok()
            .and_then(|attributes| attributes.ambient_particles)
            .unwrap_or_default();
        append_ambient_particle_spawns(&mut self.level_random, pos, particles, spawns);
    }

    fn animate_firefly_bush(
        &mut self,
        world: &mut WorldStore,
        pos: BlockPos,
        sky_darken: u8,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
        spawns: &mut Vec<EnvironmentParticleSpawn>,
    ) {
        if animate_random.next_int_bound(FIREFLY_SOUND_CHANCE_BOUND) == 0
            && direct_environment_bool(world, pos, |attributes| attributes.firefly_bush_sounds)
            && firefly_sound_height_allows(
                world.sample_heightmap_first_available(
                    VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
                    pos.x,
                    pos.z,
                ),
                pos.y,
            )
        {
            let seed = self.level_random.next_long();
            if let Some(audio) = audio.as_deref_mut() {
                audio.play_positioned_sound(&local_block_sound(
                    world,
                    SOUND_EVENT_FIREFLY_BUSH_IDLE,
                    SoundSource::Ambient,
                    [
                        f64::from(pos.x) + 0.5,
                        f64::from(pos.y) + 0.5,
                        f64::from(pos.z) + 0.5,
                    ],
                    seed,
                ));
            }
        }

        append_firefly_spawn(
            animate_random,
            pos,
            max_local_raw_brightness(world, pos, sky_darken),
            spawns,
        );
    }

    fn animate_creaking_heart(
        &mut self,
        world: &mut WorldStore,
        pos: BlockPos,
        state: Option<&str>,
        animate_random: &mut LevelEventSoundRandomState,
        audio: &mut Option<&mut dyn AudioEventSink>,
    ) {
        if !direct_environment_bool(world, pos, |attributes| attributes.creaking_active)
            || state.is_none_or(|state| state == "uprooted")
            || animate_random.next_int_bound(CREAKING_SOUND_CHANCE_BOUND) != 0
            || !creaking_heart_is_surrounded_by_logs(world, pos)
        {
            return;
        }
        let seed = self.level_random.next_long();
        if let Some(audio) = audio.as_deref_mut() {
            audio.play_positioned_sound(&local_block_sound(
                world,
                SOUND_EVENT_CREAKING_HEART_IDLE,
                SoundSource::Blocks,
                [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)],
                seed,
            ));
        }
    }
}

fn append_ambient_particle_spawns(
    random: &mut LevelEventSoundRandomState,
    pos: BlockPos,
    particles: Vec<bbb_world::ClientAmbientParticle>,
    spawns: &mut Vec<EnvironmentParticleSpawn>,
) {
    for particle in particles {
        if random.next_float() <= f32::from_bits(particle.probability_bits) {
            spawns.push(EnvironmentParticleSpawn {
                particle: particle.particle,
                position: [
                    f64::from(pos.x) + random.next_double(),
                    f64::from(pos.y) + random.next_double(),
                    f64::from(pos.z) + random.next_double(),
                ],
            });
        }
    }
}

fn append_firefly_spawn(
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
            particle: simple_particle_options(PARTICLE_FIREFLY),
            position: [
                f64::from(pos.x) + random.next_double() * FIREFLY_HORIZONTAL_RANGE
                    - FIREFLY_HORIZONTAL_RANGE / 2.0,
                f64::from(pos.y) + random.next_double() * FIREFLY_VERTICAL_RANGE,
                f64::from(pos.z) + random.next_double() * FIREFLY_HORIZONTAL_RANGE
                    - FIREFLY_HORIZONTAL_RANGE / 2.0,
            ],
        });
    }
}

fn sample_animate_position(
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

fn direct_environment_bool(
    world: &WorldStore,
    pos: BlockPos,
    value: impl FnOnce(bbb_world::ClientEnvironmentAttributes) -> Option<bool>,
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

fn max_local_raw_brightness(world: &WorldStore, pos: BlockPos, sky_darken: u8) -> Option<u8> {
    if pos.x < -30_000_000 || pos.z < -30_000_000 || pos.x >= 30_000_000 || pos.z >= 30_000_000 {
        return Some(15);
    }
    world
        .sample_block_light(pos)
        .map(|light| light.block.max(light.sky.saturating_sub(sky_darken)))
}

fn firefly_sound_height_allows(height: Option<i32>, block_y: i32) -> bool {
    height.is_some_and(|height| height <= block_y)
}

fn creaking_heart_is_surrounded_by_logs(world: &WorldStore, pos: BlockPos) -> bool {
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
            .probe_block(BlockPos {
                x: pos.x + dx,
                y: pos.y + dy,
                z: pos.z + dz,
            })
            .and_then(|block| block.block_name)
            .is_some_and(|name| block_name_is_pale_oak_log(&name))
    })
}

fn local_block_sound(
    world: &WorldStore,
    resource_id: &str,
    source: SoundSource,
    position: [f64; 3],
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
        volume: 1.0,
        pitch: 1.0,
        seed,
        distance_delay: false,
    }
}

fn simple_particle_options(resource_id: &str) -> ClientParticleOptions {
    ClientParticleOptions {
        resource_id: resource_id.to_string(),
        fields: vec![NbtSummaryEntry {
            name: "type".to_string(),
            value: NbtSummaryValue::String(resource_id.to_string()),
        }],
    }
}

fn block_position(pos: BlockPos) -> [f64; 3] {
    [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)]
}

fn containing_block_coordinate(value: f64) -> i32 {
    value.floor() as i32
}

struct EnvironmentWorldParticleBiomeSampler<'a> {
    world: &'a WorldStore,
}

impl ParticleBiomeSampler for EnvironmentWorldParticleBiomeSampler<'_> {
    fn biome_id_at(&self, pos: BlockPos) -> Option<i32> {
        self.world.probe_block(pos).and_then(|probe| probe.biome_id)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use bbb_protocol::packets::{
        BlockUpdate, ChunkBiomeData, ChunkPos as ProtocolChunkPos, ChunksBiomes,
        CommonPlayerSpawnInfo, PlayLogin,
    };
    use bbb_world::{
        ChunkColumn, ChunkPos, ChunkSection, ChunkState, ClientAmbientParticle, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, RegistryPacketEntry, WorldDimension,
    };

    use super::*;

    #[test]
    fn animate_position_consumes_near_then_far_coordinate_draws() {
        let center = BlockPos {
            x: 10,
            y: 20,
            z: 30,
        };
        let mut actual = LevelEventSoundRandomState::with_seed(42);
        let near = sample_animate_position(&mut actual, center, NEAR_SAMPLE_RADIUS);
        let far = sample_animate_position(&mut actual, center, FAR_SAMPLE_RADIUS);

        let mut expected = LevelEventSoundRandomState::with_seed(42);
        let expected_near = BlockPos {
            x: center.x + expected.next_int_bound(NEAR_SAMPLE_RADIUS)
                - expected.next_int_bound(NEAR_SAMPLE_RADIUS),
            y: center.y + expected.next_int_bound(NEAR_SAMPLE_RADIUS)
                - expected.next_int_bound(NEAR_SAMPLE_RADIUS),
            z: center.z + expected.next_int_bound(NEAR_SAMPLE_RADIUS)
                - expected.next_int_bound(NEAR_SAMPLE_RADIUS),
        };
        let expected_far = BlockPos {
            x: center.x + expected.next_int_bound(FAR_SAMPLE_RADIUS)
                - expected.next_int_bound(FAR_SAMPLE_RADIUS),
            y: center.y + expected.next_int_bound(FAR_SAMPLE_RADIUS)
                - expected.next_int_bound(FAR_SAMPLE_RADIUS),
            z: center.z + expected.next_int_bound(FAR_SAMPLE_RADIUS)
                - expected.next_int_bound(FAR_SAMPLE_RADIUS),
        };

        assert_eq!(near, expected_near);
        assert_eq!(far, expected_far);
        assert_eq!(actual.next_long(), expected.next_long());
    }

    #[test]
    fn simple_particle_options_preserve_the_canonical_type_field() {
        let options = simple_particle_options(PARTICLE_FIREFLY);
        assert_eq!(options.resource_id, PARTICLE_FIREFLY);
        assert_eq!(
            options.fields,
            vec![NbtSummaryEntry {
                name: "type".to_string(),
                value: NbtSummaryValue::String(PARTICLE_FIREFLY.to_string()),
            }]
        );
    }

    #[test]
    fn local_block_sounds_preserve_position_source_and_shared_level_seed() {
        let world = WorldStore::default();
        let firefly = local_block_sound(
            &world,
            SOUND_EVENT_FIREFLY_BUSH_IDLE,
            SoundSource::Ambient,
            [1.5, 2.5, 3.5],
            42,
        );
        let creaking = local_block_sound(
            &world,
            SOUND_EVENT_CREAKING_HEART_IDLE,
            SoundSource::Blocks,
            [1.0, 2.0, 3.0],
            84,
        );

        assert_eq!(
            firefly.position,
            ProtocolVec3d {
                x: 1.5,
                y: 2.5,
                z: 3.5
            }
        );
        assert_eq!(firefly.source, "ambient");
        assert_eq!(firefly.seed, 42);
        assert_eq!(
            creaking.position,
            ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0
            }
        );
        assert_eq!(creaking.source, "block");
        assert_eq!(creaking.seed, 84);
    }

    #[test]
    fn animate_tick_runs_exactly_667_near_far_pairs() {
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut world = WorldStore::default();
        let mut audio = None;
        let player = player_pose([0.0, 64.0, 0.0]);

        assert!(runtime.tick(&mut world, player, &mut audio).is_empty());

        let mut expected = LevelEventSoundRandomState::with_seed(0);
        let center = BlockPos { x: 0, y: 64, z: 0 };
        for _ in 0..ANIMATE_TICK_ITERATIONS {
            let _ = sample_animate_position(&mut expected, center, NEAR_SAMPLE_RADIUS);
            let _ = sample_animate_position(&mut expected, center, FAR_SAMPLE_RADIUS);
        }
        assert_eq!(runtime.level_random.next_long(), expected.next_long());

        let mut expected_animate_seeds = LevelEventSoundRandomState::with_seed(1);
        let _ = expected_animate_seeds.next_long();
        assert_eq!(
            runtime.animate_seed_random.next_long(),
            expected_animate_seeds.next_long()
        );
    }

    #[test]
    fn ambient_particles_test_in_order_and_draw_positions_only_on_success() {
        let pos = BlockPos {
            x: -4,
            y: 12,
            z: 30,
        };
        let particles = vec![
            ambient_particle("minecraft:ash", 1.0),
            ambient_particle("minecraft:white_ash", -1.0),
            ambient_particle("minecraft:cloud", 1.0),
        ];
        let mut actual = LevelEventSoundRandomState::with_seed(91);
        let mut spawns = Vec::new();

        append_ambient_particle_spawns(&mut actual, pos, particles, &mut spawns);

        let mut expected = LevelEventSoundRandomState::with_seed(91);
        let mut expected_positions = Vec::new();
        for succeeds in [true, false, true] {
            let _ = expected.next_float();
            if succeeds {
                expected_positions.push([
                    f64::from(pos.x) + expected.next_double(),
                    f64::from(pos.y) + expected.next_double(),
                    f64::from(pos.z) + expected.next_double(),
                ]);
            }
        }
        assert_eq!(spawns.len(), 2);
        assert_eq!(spawns[0].particle.resource_id, "minecraft:ash");
        assert_eq!(spawns[1].particle.resource_id, "minecraft:cloud");
        assert_eq!(
            spawns
                .iter()
                .map(|spawn| spawn.position)
                .collect::<Vec<_>>(),
            expected_positions
        );
        assert_eq!(actual.next_long(), expected.next_long());
    }

    #[test]
    fn firefly_light_gate_and_success_preserve_vanilla_random_draws() {
        let pos = BlockPos {
            x: 10,
            y: 20,
            z: -3,
        };
        let seed = (0..100_000)
            .find(|seed| {
                LevelEventSoundRandomState::with_seed(*seed).next_double() <= FIREFLY_SPAWN_CHANCE
            })
            .expect("successful deterministic firefly seed");
        let mut actual = LevelEventSoundRandomState::with_seed(seed);
        let mut spawns = Vec::new();

        append_firefly_spawn(&mut actual, pos, Some(13), &mut spawns);

        let mut expected = LevelEventSoundRandomState::with_seed(seed);
        assert!(expected.next_double() <= FIREFLY_SPAWN_CHANCE);
        let expected_position = [
            f64::from(pos.x) + expected.next_double() * FIREFLY_HORIZONTAL_RANGE
                - FIREFLY_HORIZONTAL_RANGE / 2.0,
            f64::from(pos.y) + expected.next_double() * FIREFLY_VERTICAL_RANGE,
            f64::from(pos.z) + expected.next_double() * FIREFLY_HORIZONTAL_RANGE
                - FIREFLY_HORIZONTAL_RANGE / 2.0,
        ];
        assert_eq!(spawns.len(), 1);
        assert_eq!(spawns[0].position, expected_position);
        assert_eq!(actual.next_long(), expected.next_long());

        for brightness in [None, Some(14)] {
            let mut actual = LevelEventSoundRandomState::with_seed(seed);
            let mut untouched = LevelEventSoundRandomState::with_seed(seed);
            let mut spawns = Vec::new();
            append_firefly_spawn(&mut actual, pos, brightness, &mut spawns);
            assert!(spawns.is_empty());
            assert_eq!(actual.next_long(), untouched.next_long());
        }
    }

    #[test]
    fn collision_and_six_side_pale_log_queries_use_loaded_block_shapes_and_tag_membership() {
        let mut world = world_with_ambient_particles();
        let center = BlockPos { x: 8, y: 32, z: 8 };
        assert_eq!(
            world.sample_block_collision_shape_is_full_block(center),
            Some(false)
        );
        set_block(&mut world, center, block_state_id("minecraft:stone", &[]));
        assert_eq!(
            world.sample_block_collision_shape_is_full_block(center),
            Some(true)
        );
        assert!(!creaking_heart_is_surrounded_by_logs(&world, center));

        let pale_log = block_state_id("minecraft:pale_oak_log", &[("axis", "y")]);
        for (dx, dy, dz) in [
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
            (-1, 0, 0),
            (1, 0, 0),
        ] {
            set_block(
                &mut world,
                BlockPos {
                    x: center.x + dx,
                    y: center.y + dy,
                    z: center.z + dz,
                },
                pale_log,
            );
        }
        assert!(creaking_heart_is_surrounded_by_logs(&world, center));

        set_block(
            &mut world,
            BlockPos {
                x: center.x + 1,
                ..center
            },
            block_state_id("minecraft:oak_log", &[("axis", "y")]),
        );
        assert!(!creaking_heart_is_surrounded_by_logs(&world, center));
    }

    #[test]
    fn ambient_particles_observe_chunks_biomes_replacement_on_the_next_tick() {
        let mut world = world_with_ambient_particles();
        let player = player_pose([1.0, 32.0, 1.0]);
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut audio = None;

        let initial = runtime.tick(&mut world, player, &mut audio);
        assert!(!initial.is_empty());
        assert!(initial
            .iter()
            .all(|spawn| spawn.particle.resource_id == "minecraft:ash"));

        world
            .apply_biome_update(ChunksBiomes {
                chunks: vec![ChunkBiomeData {
                    pos: ProtocolChunkPos { x: 0, z: 0 },
                    raw_biomes: [0, 1].repeat(world.dimension().section_count()),
                }],
            })
            .unwrap();
        let replaced = runtime.tick(&mut world, player, &mut audio);

        assert!(!replaced.is_empty());
        assert!(replaced
            .iter()
            .all(|spawn| spawn.particle.resource_id == "minecraft:white_ash"));
    }

    #[test]
    fn full_block_collision_suppresses_ambient_particles() {
        let stone = block_state_id("minecraft:stone", &[]);
        let mut world = world_with_ambient_particles_and_block_state(stone);
        let mut runtime = EnvironmentAnimateRuntime::default();
        let mut audio = None;

        let spawns = runtime.tick(&mut world, player_pose([1.0, 32.0, 1.0]), &mut audio);

        assert!(spawns.is_empty());
    }

    #[test]
    fn environment_animate_ticks_after_movement_and_before_music_and_particle_intake() {
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
        let animate = advance
            .find("environment_animate.advance_for_ticks(")
            .expect("environment animate tick");
        let music = advance
            .find("music.advance_for_ticks(")
            .expect("music tick");
        let particle_intake = advance
            .find("submit_entity_client_tick_effects(")
            .expect("later client particle intake");

        assert!(movement < animate);
        assert!(animate < music);
        assert!(animate < particle_intake);
    }

    #[test]
    fn firefly_light_and_height_gates_match_vanilla_boundaries() {
        let light = bbb_world::TerrainLight { block: 6, sky: 12 };
        assert_eq!(light.block.max(light.sky.saturating_sub(4)), 8);
        assert_eq!(light.block.max(light.sky.saturating_sub(14)), 6);
        assert!(firefly_sound_height_allows(Some(20), 20));
        assert!(firefly_sound_height_allows(Some(19), 20));
        assert!(!firefly_sound_height_allows(Some(21), 20));
        assert!(!firefly_sound_height_allows(None, 20));
    }

    fn ambient_particle(resource_id: &str, probability: f32) -> ClientAmbientParticle {
        ClientAmbientParticle {
            particle: simple_particle_options(resource_id),
            probability_bits: probability.to_bits(),
        }
    }

    fn world_with_ambient_particles() -> WorldStore {
        world_with_ambient_particles_and_block_state(0)
    }

    fn world_with_ambient_particles_and_block_state(block_state_id: i32) -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension::default());
        world.begin_registry_configuration();
        world.record_registry_entries(
            "minecraft:worldgen/biome",
            0,
            vec![
                RegistryPacketEntry::with_raw_data(
                    "example:biome_a",
                    biome_with_ambient_particle_nbt("minecraft:ash"),
                ),
                RegistryPacketEntry::with_raw_data(
                    "example:biome_b",
                    biome_with_ambient_particle_nbt("minecraft:white_ash"),
                ),
            ],
        );
        for data in crate::runtime::minimal_non_empty_synchronized_registry_data() {
            if data.registry != "minecraft:worldgen/biome" {
                world.record_registry_data(data);
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
        world.insert_decoded_chunk(chunk_with_biome(world.dimension(), 0, block_state_id));
        world
    }

    fn biome_with_ambient_particle_nbt(resource_id: &str) -> Vec<u8> {
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
        out.push(9);
        write_nbt_utf(&mut out, "minecraft:visual/ambient_particles");
        out.push(10);
        out.extend_from_slice(&1_i32.to_be_bytes());
        out.push(10);
        write_nbt_utf(&mut out, "particle");
        write_nbt_string(&mut out, "type", resource_id);
        out.push(0);
        write_nbt_float(&mut out, "probability", 1.0);
        out.push(0);
        out.push(0);
        out.push(0);
        out
    }

    fn chunk_with_biome(
        dimension: WorldDimension,
        biome_id: i32,
        block_state_id: i32,
    ) -> ChunkColumn {
        ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..dimension.section_count())
                .map(|_| ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        block_state_id,
                    ),
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

    fn set_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
        assert!(world.apply_block_update(BlockUpdate {
            pos: bbb_protocol::packets::BlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
        }));
    }

    fn block_state_id(name: &str, properties: &[(&str, &str)]) -> i32 {
        let properties = properties
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect::<BTreeMap<_, _>>();
        bbb_world::vanilla_block_state_id(name, &properties)
            .unwrap_or_else(|| panic!("missing test block state {name} {properties:?}"))
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
}
