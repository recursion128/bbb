use bbb_client_world::{
    block_name_is_air, BlockPos, ExplosionEventState, LevelEventSoundRandomState, WorldStore,
};
use bbb_protocol::packets::{ClientParticleStatus, Vec3d};

use super::{ClientParticle, ParticleBiomeSampler};

const MAX_PARTICLES_PER_TICK: i32 = 512;
const MAX_TRACKED_EXPLOSIONS: usize = 4_096;

#[derive(Debug, Clone, Default)]
pub(super) struct ClientExplosionTracker {
    explosions: Vec<TrackedExplosion>,
    level_generation: Option<u64>,
}

#[derive(Debug, Clone)]
struct TrackedExplosion {
    center: Vec3d,
    radius: f32,
    block_count: i32,
    block_particles: Vec<TrackedExplosionParticle>,
    block_particle_weight: i32,
}

/// One `ExplosionParticleInfo`, decoded when the explosion is tracked.
#[derive(Debug, Clone)]
struct TrackedExplosionParticle {
    particle: Option<ClientParticle>,
    scaling: f32,
    speed: f32,
    weight: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ExplosionParticleSpawn {
    pub(super) particle: Option<ClientParticle>,
    pub(super) position: Vec3d,
    pub(super) velocity: Vec3d,
}

pub(super) struct ExplosionWorldParticleSampler<'a> {
    pub(super) world: &'a WorldStore,
}

impl ParticleBiomeSampler for ExplosionWorldParticleSampler<'_> {
    fn biome_id_at(&self, pos: BlockPos) -> Option<i32> {
        self.world.probe_block(pos).and_then(|probe| probe.biome_id)
    }
}

impl ClientExplosionTracker {
    pub(super) fn track(
        &mut self,
        state: &ExplosionEventState,
        level_generation: u64,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) {
        if self.level_generation != Some(level_generation) {
            self.explosions.clear();
            self.level_generation = Some(level_generation);
        }
        if self.explosions.len() >= MAX_TRACKED_EXPLOSIONS
            || state.block_count <= 0
            || !state.radius.is_finite()
            || state.radius <= 0.0
            || !vec3_is_finite(state.center)
        {
            return;
        }

        let block_particles: Vec<_> = state
            .block_particles
            .iter()
            .filter(|entry| {
                entry.weight > 0 && entry.scaling.is_finite() && entry.speed.is_finite()
            })
            .map(|entry| TrackedExplosionParticle {
                particle: ClientParticle::from_payload_with_registry(
                    &entry.particle,
                    content_registry,
                ),
                scaling: entry.scaling,
                speed: entry.speed,
                weight: entry.weight,
            })
            .collect();
        let Some(block_particle_weight) =
            checked_total_weight(block_particles.iter().map(|entry| entry.weight))
        else {
            return;
        };
        if block_particle_weight == 0 {
            return;
        }

        self.explosions.push(TrackedExplosion {
            center: state.center,
            radius: state.radius,
            block_count: state.block_count,
            block_particles,
            block_particle_weight,
        });
    }

    pub(super) fn tick(
        &mut self,
        world: &WorldStore,
        level_generation: u64,
        particle_status: ClientParticleStatus,
        random: &mut LevelEventSoundRandomState,
    ) -> Vec<ExplosionParticleSpawn> {
        if self.level_generation != Some(level_generation) {
            self.explosions.clear();
            self.level_generation = Some(level_generation);
            return Vec::new();
        }
        let explosions = std::mem::take(&mut self.explosions);
        if particle_status != ClientParticleStatus::All || explosions.is_empty() {
            return Vec::new();
        }
        let Some(total_blocks) =
            checked_total_weight(explosions.iter().map(|explosion| explosion.block_count))
        else {
            return Vec::new();
        };
        let total_particles = total_blocks.min(MAX_PARTICLES_PER_TICK);
        let mut spawns = Vec::with_capacity(total_particles as usize);
        for _ in 0..total_particles {
            let Some(explosion) = choose_weighted(
                &explosions,
                total_blocks,
                |explosion| explosion.block_count,
                random,
            ) else {
                continue;
            };
            if let Some(spawn) = sample_particle(world, explosion, random) {
                spawns.push(spawn);
            }
        }
        spawns
    }
}

fn sample_particle(
    world: &WorldStore,
    explosion: &TrackedExplosion,
    random: &mut LevelEventSoundRandomState,
) -> Option<ExplosionParticleSpawn> {
    let mut direction = Vec3d {
        x: f64::from(random.next_float() * 2.0 - 1.0),
        y: f64::from(random.next_float() * 2.0 - 1.0),
        z: f64::from(random.next_float() * 2.0 - 1.0),
    };
    let direction_length =
        (direction.x * direction.x + direction.y * direction.y + direction.z * direction.z).sqrt();
    if direction_length < f64::from(1.0e-5_f32) {
        direction = Vec3d::default();
    } else {
        direction.x /= direction_length;
        direction.y /= direction_length;
        direction.z /= direction_length;
    }

    let radius = (f64::from(random.next_float()).cbrt() as f32) * explosion.radius;
    let local_position = Vec3d {
        x: direction.x * f64::from(radius),
        y: direction.y * f64::from(radius),
        z: direction.z * f64::from(radius),
    };
    let sampled_position = Vec3d {
        x: explosion.center.x + local_position.x,
        y: explosion.center.y + local_position.y,
        z: explosion.center.z + local_position.z,
    };
    if !position_is_air(world, sampled_position) {
        return None;
    }

    let speed =
        0.5 / (radius / explosion.radius + 0.1) * random.next_float() * random.next_float() + 0.3;
    let particle = choose_weighted(
        &explosion.block_particles,
        explosion.block_particle_weight,
        |entry| entry.weight,
        random,
    )?;
    Some(ExplosionParticleSpawn {
        particle: particle.particle.clone(),
        position: Vec3d {
            x: explosion.center.x + local_position.x * f64::from(particle.scaling),
            y: explosion.center.y + local_position.y * f64::from(particle.scaling),
            z: explosion.center.z + local_position.z * f64::from(particle.scaling),
        },
        velocity: Vec3d {
            x: direction.x * f64::from(speed * particle.speed),
            y: direction.y * f64::from(speed * particle.speed),
            z: direction.z * f64::from(speed * particle.speed),
        },
    })
}

fn position_is_air(world: &WorldStore, position: Vec3d) -> bool {
    let pos = BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    };
    world
        .block_view(pos)
        .is_none_or(|probe| probe.block_name.is_some_and(block_name_is_air))
}

fn checked_total_weight(mut weights: impl Iterator<Item = i32>) -> Option<i32> {
    weights.try_fold(0_i32, i32::checked_add)
}

fn choose_weighted<'a, T>(
    entries: &'a [T],
    total_weight: i32,
    weight: impl Fn(&T) -> i32,
    random: &mut LevelEventSoundRandomState,
) -> Option<&'a T> {
    if total_weight <= 0 {
        return None;
    }
    let mut selection = random.next_int_bound(total_weight);
    entries.iter().find(|entry| {
        selection -= weight(entry);
        selection < 0
    })
}

fn vec3_is_finite(value: Vec3d) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}
