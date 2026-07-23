use bbb_protocol::{
    packets::{
        vanilla_particle_command_options, LevelParticles, ParticlePayload,
        VanillaParticleCommandOptions, Vec3d,
    },
    particle_types::vanilla_particle_type_id_for_resource_id,
    NbtSummaryValue,
};
use bbb_renderer::ParticleSpawnBatch;
use bbb_world::ClientParticleOptions;

use super::{
    append_particle_batch, EnvironmentParticleSpawn, LevelParticleSpawnContext,
    ParticleBiomeSampler, ParticleCommandResolver,
};

impl ParticleCommandResolver {
    pub(super) fn resolve_environment_particles(
        &mut self,
        spawns: &[EnvironmentParticleSpawn],
        context: LevelParticleSpawnContext,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
    ) -> ParticleSpawnBatch {
        let mut batch = ParticleSpawnBatch::default();
        for spawn in spawns {
            let Some(particle_type_id) =
                vanilla_particle_type_id_for_resource_id(&spawn.particle.resource_id)
            else {
                batch.unknown_particle_type_count += 1;
                continue;
            };
            if !simple_environment_particle_options(&spawn.particle) {
                batch.unsupported_particle_option_count += 1;
                continue;
            }
            append_particle_batch(
                &mut batch,
                self.resolve_level_particles_with_context(
                    &LevelParticles {
                        override_limiter: false,
                        always_show: false,
                        position: Vec3d {
                            x: spawn.position[0],
                            y: spawn.position[1],
                            z: spawn.position[2],
                        },
                        offset: Vec3d {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        },
                        max_speed: 0.0,
                        count: 0,
                        particle: ParticlePayload {
                            particle_type_id,
                            raw_options: Vec::new(),
                        },
                    },
                    context,
                    biome_sampler,
                    None,
                ),
            );
        }
        batch
    }
}

fn simple_environment_particle_options(options: &ClientParticleOptions) -> bool {
    if vanilla_particle_command_options(&options.resource_id)
        != Some(VanillaParticleCommandOptions::Empty)
    {
        return false;
    }
    matches!(
        options.fields.as_slice(),
        [field]
            if field.name == "type"
                && matches!(
                    &field.value,
                    NbtSummaryValue::String(resource_id)
                        if particle_resource_id_matches(resource_id, &options.resource_id)
                )
    )
}

fn particle_resource_id_matches(raw: &str, normalized: &str) -> bool {
    raw == normalized || (!raw.contains(':') && normalized.strip_prefix("minecraft:") == Some(raw))
}
