use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::scene::biome_tint::is_foliage_tinted_block;
use bbb_client_world::particles::ParticleEngine;
use bbb_client_world::{BlockPos, LevelEventSoundRandomState};
use bbb_protocol::packets::Vec3d;

use super::{
    particle_biome_blend_at, random_between, EnvironmentParticleColorSource,
    EnvironmentParticleSpawn, NativeParticleRuntime, ParticleBiomeSampler,
};

impl NativeParticleRuntime {
    /// `ClientLevel.addParticle` for client-generated spawns (animate tick,
    /// block entities, weather).
    pub(crate) fn spawn_environment_particles(
        &mut self,
        out: &mut ParticleEngine,
        spawns: &[EnvironmentParticleSpawn],
        camera_position: Option<[f64; 3]>,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        for spawn in spawns {
            let Some(kind) = self.defined(spawn.particle.kind) else {
                continue;
            };
            let position = Vec3d {
                x: spawn.position[0],
                y: spawn.position[1],
                z: spawn.position[2],
            };
            if !self.should_spawn_level_particle(
                kind.override_limiter(),
                spawn.always_show,
                position,
                camera_position,
            ) {
                continue;
            }
            let mut prepared = self.prepare(&spawn.particle, item_runtime, None);
            if !prepared.accepts {
                continue;
            }
            if let Some(color_source) = spawn.color_source {
                let color = self.environment_particle_color(color_source, biome_sampler);
                prepared = prepared.with_color(Some(color));
            }
            let velocity = Vec3d {
                x: spawn.velocity[0],
                y: spawn.velocity[1],
                z: spawn.velocity[2],
            };
            self.spawn_prepared(out, &prepared, position, velocity, biome_sampler);
        }
    }

    fn environment_particle_color(
        &self,
        source: EnvironmentParticleColorSource,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
    ) -> [f32; 4] {
        match source {
            EnvironmentParticleColorSource::ClientLeaf {
                block_pos,
                block_resource_id,
            } => {
                let biome_id = biome_sampler.and_then(|sampler| sampler.biome_id_at(block_pos));
                let blend = if is_foliage_tinted_block(block_resource_id) {
                    particle_biome_blend_at(block_pos, self.biome_blend_radius, biome_sampler)
                } else {
                    None
                };
                self.terrain_particle_tint_catalog.client_leaf_tint_color(
                    block_resource_id,
                    biome_id,
                    blend.as_ref(),
                )
            }
        }
    }
}

pub(crate) fn particle_utils_axis_sample(
    pos: BlockPos,
    axis: i32,
    radius: f64,
    random: &mut LevelEventSoundRandomState,
) -> (Vec3d, Vec3d) {
    let center = Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    };
    let step_x = axis == 0;
    let step_y = axis == 1;
    let step_z = axis == 2;
    let position = Vec3d {
        x: center.x + random_between(random, -1.0, 1.0) * if step_x { 0.5 } else { radius },
        y: center.y + random_between(random, -1.0, 1.0) * if step_y { 0.5 } else { radius },
        z: center.z + random_between(random, -1.0, 1.0) * if step_z { 0.5 } else { radius },
    };
    let velocity = Vec3d {
        x: if step_x {
            random_between(random, -1.0, 1.0)
        } else {
            0.0
        },
        y: if step_y {
            random_between(random, -1.0, 1.0)
        } else {
            0.0
        },
        z: if step_z {
            random_between(random, -1.0, 1.0)
        } else {
            0.0
        },
    };
    (position, velocity)
}
