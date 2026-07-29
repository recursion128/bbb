use bbb_item_model::NativeItemRuntime;
use bbb_protocol::packets::{ClientParticleStatus, SoundSource, Vec3d as ProtocolVec3d};
use bbb_world::{
    biome_height_adjusted_temperature, block_name_is_campfire, BlockPos, ClientSoundEventHolder,
    LevelEventSoundRandomState, SoundEventState, TerrainFluidKind, WorldStore,
};

use crate::{
    audio_runtime::AudioEventSink,
    environment_audio_runtime::sound_holder_state,
    particle_runtime::{
        simple_client_particle_options, EnvironmentParticleSpawn, LevelParticleSpawnContext,
        ParticleEventSink,
    },
    simulation::effects::ParticleBatchSink,
};

const RAIN_PARTICLES_PER_BLOCK: f32 = 0.225;
const WEATHER_RADIUS: i32 = 10;
const RAIN_HEIGHT_RANGE: i32 = 10;
const WEATHER_RANDOM_SEED_MULTIPLIER: i64 = 312_987_231;
const RAIN_SOUND_CHANCE_BOUND: i32 = 3;
const RAIN_SOUND: &str = "minecraft:weather.rain";
const RAIN_ABOVE_SOUND: &str = "minecraft:weather.rain.above";
const RAIN_PARTICLE: &str = "minecraft:rain";
const SMOKE_PARTICLE: &str = "minecraft:smoke";
const RAIN_TEMPERATURE_THRESHOLD: f32 = 0.15;

/// Simulation-side owner of Vanilla `WeatherEffectRenderer`'s mutable tick
/// and rain-sound cursors.
///
/// The owner is independent from particle pack resources because Vanilla keeps
/// the renderer's cursors across resource reloads, and rain sounds still advance
/// when particle assets are unavailable.
#[derive(Debug)]
pub(crate) struct WeatherEffectRuntime {
    ticks: i32,
    rain_sound_time: i32,
    particle_status: ClientParticleStatus,
}

#[derive(Debug, Default)]
pub(crate) struct WeatherTickEffects {
    pub(crate) particles: Vec<EnvironmentParticleSpawn>,
    pub(crate) sound: Option<SoundEventState>,
}

impl WeatherEffectRuntime {
    pub(crate) fn new(particle_status: ClientParticleStatus) -> Self {
        Self {
            ticks: 0,
            rain_sound_time: 0,
            particle_status,
        }
    }

    /// Mirrors `GameRenderer.tick` → `LevelRenderer.tick`:
    /// `running_level_ticks` excludes paused/frozen client ticks, and each
    /// accepted tick increments the renderer cursor before sampling weather.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn advance_for_ticks(
        &mut self,
        world: &mut WorldStore,
        running_level_ticks: u32,
        camera_position: Option<[f64; 3]>,
        mut particle_events: Option<&mut dyn ParticleEventSink>,
        particle_batches: &mut dyn ParticleBatchSink,
        audio: &mut Option<&mut dyn AudioEventSink>,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        for _ in 0..running_level_ticks {
            let Some(camera_position) = camera_position else {
                self.ticks = self.ticks.wrapping_add(1);
                continue;
            };
            let effects = self.tick_running(world, camera_position);
            if !effects.particles.is_empty() {
                if let Some(particle_events) = particle_events.as_deref_mut() {
                    let batch = particle_events.spawn_environment_particles(
                        &effects.particles,
                        LevelParticleSpawnContext {
                            camera_position: Some(camera_position),
                            vibration_entity_position: None,
                        },
                        None,
                        item_runtime,
                    );
                    if !batch.is_empty() {
                        particle_batches.submit_particle_batch(batch);
                    }
                }
            }
            if let (Some(audio), Some(sound)) = (audio.as_deref_mut(), effects.sound.as_ref()) {
                audio.play_positioned_sound(sound);
            }
        }
    }

    pub(crate) fn tick_running(
        &mut self,
        world: &mut WorldStore,
        camera_position: [f64; 3],
    ) -> WeatherTickEffects {
        self.ticks = self.ticks.wrapping_add(1);
        let rain_level = world.weather().rain_level;
        if !rain_level.is_finite() || rain_level <= 0.0 {
            return WeatherTickEffects::default();
        }
        let camera_pos = BlockPos {
            x: containing_block_coordinate(camera_position[0]),
            y: containing_block_coordinate(camera_position[1]),
            z: containing_block_coordinate(camera_position[2]),
        };
        let dimension = world.dimension();
        let Some(level) = world.level_info() else {
            return WeatherTickEffects::default();
        };
        let sea_level = level.sea_level;
        let diameter = WEATHER_RADIUS * 2 + 1;
        let area = diameter * diameter;
        let mut particle_count =
            (RAIN_PARTICLES_PER_BLOCK * area as f32 * rain_level * rain_level) as i32;
        if self.particle_status == ClientParticleStatus::Decreased {
            particle_count /= 2;
        }
        let seed = i64::from(self.ticks).wrapping_mul(WEATHER_RANDOM_SEED_MULTIPLIER);
        let mut random = LevelEventSoundRandomState::with_seed(seed);
        let mut effects = WeatherTickEffects::default();
        let mut rain_particle_pos = None;

        for _ in 0..particle_count {
            let x = camera_pos.x + random.next_int_bound(diameter) - WEATHER_RADIUS;
            let z = camera_pos.z + random.next_int_bound(diameter) - WEATHER_RADIUS;
            let Some(height) = world.sample_motion_blocking_height(x, z) else {
                continue;
            };
            let height_pos = BlockPos { x, y: height, z };
            if height <= dimension.min_y
                || height > camera_pos.y + RAIN_HEIGHT_RANGE
                || height < camera_pos.y - RAIN_HEIGHT_RANGE
                || !weather_rains_at(world, height_pos, sea_level)
            {
                continue;
            }
            let surface_pos = BlockPos {
                y: height - 1,
                ..height_pos
            };
            rain_particle_pos = Some(surface_pos);
            if self.particle_status == ClientParticleStatus::Minimal {
                break;
            }

            let block_x = random.next_double();
            let block_z = random.next_double();
            let sample_position = [
                f64::from(surface_pos.x) + block_x,
                f64::from(surface_pos.y),
                f64::from(surface_pos.z) + block_z,
            ];
            let surface = world.particle_block_fluid_surface_sample(sample_position);
            let particle = if weather_surface_smokes(world, surface_pos, surface.fluid_kind) {
                SMOKE_PARTICLE
            } else {
                RAIN_PARTICLE
            };
            effects.particles.push(EnvironmentParticleSpawn {
                particle: simple_client_particle_options(particle),
                position: [
                    sample_position[0],
                    sample_position[1] + surface.max_surface_height(),
                    sample_position[2],
                ],
                velocity: [0.0; 3],
                always_show: false,
            });
        }

        if let Some(rain_particle_pos) = rain_particle_pos {
            let previous_sound_time = self.rain_sound_time;
            self.rain_sound_time = self.rain_sound_time.wrapping_add(1);
            if random.next_int_bound(RAIN_SOUND_CHANCE_BOUND) < previous_sound_time {
                self.rain_sound_time = 0;
                let camera_height = world.sample_motion_blocking_height(camera_pos.x, camera_pos.z);
                let above = rain_particle_pos.y > camera_pos.y + 1
                    && camera_height.is_some_and(|height| height > camera_pos.y);
                effects.sound = Some(weather_sound(
                    world,
                    if above { RAIN_ABOVE_SOUND } else { RAIN_SOUND },
                    rain_particle_pos,
                    if above { 0.1 } else { 0.2 },
                    if above { 0.5 } else { 1.0 },
                ));
            }
        }

        effects
    }
}

fn weather_rains_at(world: &WorldStore, pos: BlockPos, sea_level: i32) -> bool {
    let dimension = world.dimension();
    let biome_y = pos
        .y
        .clamp(dimension.min_y, dimension.min_y + dimension.height - 1);
    let biome_id = world
        .probe_block(BlockPos { y: biome_y, ..pos })
        .and_then(|block| block.biome_id);
    let Some(biome_id) = biome_id else {
        return false;
    };
    let Ok(Some(biome)) = world.registry_biome_client_projection(biome_id) else {
        return false;
    };
    biome.has_precipitation
        && biome_height_adjusted_temperature(
            biome.temperature,
            biome.temperature_modifier,
            pos,
            sea_level,
        ) >= RAIN_TEMPERATURE_THRESHOLD
}

fn weather_surface_smokes(
    world: &WorldStore,
    pos: BlockPos,
    fluid_kind: Option<TerrainFluidKind>,
) -> bool {
    if fluid_kind == Some(TerrainFluidKind::Lava) {
        return true;
    }
    world.probe_block(pos).is_some_and(|block| {
        block.block_name.as_deref() == Some("minecraft:magma_block")
            || block
                .block_name
                .as_deref()
                .is_some_and(block_name_is_campfire)
                && block.block_properties.get("lit").map(String::as_str) == Some("true")
    })
}

fn weather_sound(
    world: &mut WorldStore,
    resource_id: &str,
    pos: BlockPos,
    volume: f32,
    pitch: f32,
) -> SoundEventState {
    SoundEventState {
        sound: sound_holder_state(
            world,
            &ClientSoundEventHolder::Reference {
                resource_id: resource_id.to_string(),
            },
        ),
        source: SoundSource::Weather.as_str().to_string(),
        position: ProtocolVec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        },
        volume,
        pitch,
        seed: world.next_sound_seed(),
        distance_delay: false,
    }
}

fn containing_block_coordinate(value: f64) -> i32 {
    value.floor() as i32
}
