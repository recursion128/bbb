use bbb_item_model::NativeItemRuntime;
use bbb_protocol::{
    block_state_types::vanilla_falling_dust_static_facts,
    packets::{vanilla_particle_command_options, VanillaParticleCommandOptions, Vec3d},
    particle_types::{
        vanilla_particle_type_id_for_resource_id, VANILLA_PARTICLE_TYPE_BLOCK_CRUMBLE_ID,
        VANILLA_PARTICLE_TYPE_BLOCK_ID, VANILLA_PARTICLE_TYPE_BLOCK_MARKER_ID,
        VANILLA_PARTICLE_TYPE_DRAGON_BREATH_ID, VANILLA_PARTICLE_TYPE_DUST_COLOR_TRANSITION_ID,
        VANILLA_PARTICLE_TYPE_DUST_ID, VANILLA_PARTICLE_TYPE_DUST_PILLAR_ID,
        VANILLA_PARTICLE_TYPE_EFFECT_ID, VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID,
        VANILLA_PARTICLE_TYPE_FALLING_DUST_ID, VANILLA_PARTICLE_TYPE_FLASH_ID,
        VANILLA_PARTICLE_TYPE_INSTANT_EFFECT_ID, VANILLA_PARTICLE_TYPE_ITEM_ID,
        VANILLA_PARTICLE_TYPE_SCULK_CHARGE_ID, VANILLA_PARTICLE_TYPE_SHRIEK_ID,
        VANILLA_PARTICLE_TYPE_TINTED_LEAVES_ID, VANILLA_PARTICLE_TYPE_TRAIL_ID,
        VANILLA_PARTICLE_TYPE_VIBRATION_ID,
    },
};
use bbb_renderer::{ParticleBlockOptionState, ParticleSpawnBatch, ParticleSpawnCommand};
use bbb_world::{
    BlockPos, ClientParticleOptionValue, ClientParticleOptions, LevelEventSoundRandomState,
};

use super::{
    argb_particle_color, particle_biome_blend_at, particle_item_option_state_for_stack,
    particle_provider_accepts_spawn, random_between, rgb_particle_color_u32,
    EnvironmentParticleColorSource, EnvironmentParticleSpawn, LevelParticleSpawnContext,
    ParticleBiomeSampler, ParticleCommandResolver, ParticleOptionRenderState,
    SimpleParticleTemplate,
};
use crate::biome_tint::is_foliage_tinted_block;

struct PreparedEnvironmentParticle {
    template: SimpleParticleTemplate,
    option_state: ParticleOptionRenderState,
    initial_delay_ticks: u32,
}

impl ParticleCommandResolver {
    pub(super) fn resolve_environment_particles(
        &mut self,
        spawns: &[EnvironmentParticleSpawn],
        context: LevelParticleSpawnContext,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        let mut batch = ParticleSpawnBatch::default();
        for spawn in spawns {
            let mut prepared = match self.prepare_environment_particle(&spawn.particle) {
                Ok(prepared) => prepared,
                Err(diagnostic) => {
                    super::append_particle_batch(&mut batch, diagnostic);
                    continue;
                }
            };
            if let Some(color_source) = spawn.color_source {
                prepared.option_state.color =
                    Some(self.environment_particle_color(color_source, biome_sampler));
            }
            batch.missing_sprite_count += prepared.template.missing_sprite_count;
            let position = Vec3d {
                x: spawn.position[0],
                y: spawn.position[1],
                z: spawn.position[2],
            };
            let velocity = Vec3d {
                x: spawn.velocity[0],
                y: spawn.velocity[1],
                z: spawn.velocity[2],
            };
            if !self.should_spawn_level_particle(
                prepared.template.particle_type.override_limiter,
                spawn.always_show,
                position,
                context.camera_position,
            ) || !particle_provider_accepts_spawn(
                prepared.template.particle_type.id,
                &prepared.option_state,
            ) {
                continue;
            }
            batch.commands.push(self.environment_particle_command(
                &prepared,
                position,
                velocity,
                spawn.always_show,
                biome_sampler,
                item_runtime,
            ));
        }
        batch
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
                    particle_biome_blend_at(block_pos, biome_sampler)
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

    pub(super) fn environment_particle_batch(
        &self,
        options: &ClientParticleOptions,
        position: Vec3d,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        let prepared = match self.prepare_environment_particle(options) {
            Ok(prepared) => prepared,
            Err(diagnostic) => return diagnostic,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: prepared.template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        if particle_provider_accepts_spawn(
            prepared.template.particle_type.id,
            &prepared.option_state,
        ) {
            batch.commands.push(self.environment_particle_command(
                &prepared,
                position,
                Vec3d::default(),
                false,
                biome_sampler,
                item_runtime,
            ));
        }
        batch
    }

    fn prepare_environment_particle(
        &self,
        options: &ClientParticleOptions,
    ) -> Result<PreparedEnvironmentParticle, ParticleSpawnBatch> {
        let Some(particle_type_id) = vanilla_particle_type_id_for_resource_id(&options.resource_id)
        else {
            return Err(ParticleSpawnBatch {
                unknown_particle_type_count: 1,
                ..ParticleSpawnBatch::default()
            });
        };
        let Some((option_state, initial_delay_ticks)) =
            environment_particle_option_render_state(particle_type_id, &options.option)
        else {
            return Err(ParticleSpawnBatch {
                unsupported_particle_option_count: 1,
                ..ParticleSpawnBatch::default()
            });
        };
        Ok(PreparedEnvironmentParticle {
            template: self.simple_particle_template(particle_type_id)?,
            option_state,
            initial_delay_ticks,
        })
    }

    fn environment_particle_command(
        &self,
        prepared: &PreparedEnvironmentParticle,
        position: Vec3d,
        velocity: Vec3d,
        always_show: bool,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnCommand {
        self.command_for_type(
            prepared.template.particle_type,
            &prepared.template.sprite_ids,
            position,
            velocity,
            prepared.template.particle_type.override_limiter,
            always_show,
            0,
            prepared.initial_delay_ticks,
            prepared.option_state.clone(),
            biome_sampler,
            item_runtime,
        )
    }
}

pub(crate) fn simple_client_particle_options(resource_id: &str) -> ClientParticleOptions {
    ClientParticleOptions {
        resource_id: resource_id.to_string(),
        fields: vec![bbb_protocol::NbtSummaryEntry {
            name: "type".to_string(),
            value: bbb_protocol::NbtSummaryValue::String(resource_id.to_string()),
        }],
        option: ClientParticleOptionValue::Empty,
    }
}

pub(crate) fn block_client_particle_options(
    resource_id: &str,
    block_state_id: i32,
) -> ClientParticleOptions {
    ClientParticleOptions {
        resource_id: resource_id.to_string(),
        fields: vec![bbb_protocol::NbtSummaryEntry {
            name: "type".to_string(),
            value: bbb_protocol::NbtSummaryValue::String(resource_id.to_string()),
        }],
        option: ClientParticleOptionValue::Block { block_state_id },
    }
}

pub(crate) fn color_client_particle_options(resource_id: &str, argb: u32) -> ClientParticleOptions {
    ClientParticleOptions {
        resource_id: resource_id.to_string(),
        fields: vec![
            bbb_protocol::NbtSummaryEntry {
                name: "type".to_string(),
                value: bbb_protocol::NbtSummaryValue::String(resource_id.to_string()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "color".to_string(),
                value: bbb_protocol::NbtSummaryValue::Int(argb as i32),
            },
        ],
        option: ClientParticleOptionValue::Color { argb },
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

fn environment_particle_option_render_state(
    particle_type_id: i32,
    option: &ClientParticleOptionValue,
) -> Option<(ParticleOptionRenderState, u32)> {
    let state = match option {
        ClientParticleOptionValue::Empty
            if vanilla_particle_command_options(
                bbb_protocol::particle_types::vanilla_particle_type(particle_type_id)?.resource_id,
            ) == Some(VanillaParticleCommandOptions::Empty) =>
        {
            ParticleOptionRenderState::default()
        }
        ClientParticleOptionValue::Block { block_state_id }
            if matches!(
                particle_type_id,
                VANILLA_PARTICLE_TYPE_BLOCK_ID
                    | VANILLA_PARTICLE_TYPE_BLOCK_MARKER_ID
                    | VANILLA_PARTICLE_TYPE_FALLING_DUST_ID
                    | VANILLA_PARTICLE_TYPE_DUST_PILLAR_ID
                    | VANILLA_PARTICLE_TYPE_BLOCK_CRUMBLE_ID
            ) =>
        {
            ParticleOptionRenderState {
                color: if particle_type_id == VANILLA_PARTICLE_TYPE_FALLING_DUST_ID {
                    vanilla_falling_dust_static_facts(*block_state_id)
                        .and_then(|facts| facts.falling_block_dust_color_rgb())
                        .map(rgb_particle_color_u32)
                } else {
                    None
                },
                block: Some(ParticleBlockOptionState {
                    block_state_id: *block_state_id,
                }),
                ..ParticleOptionRenderState::default()
            }
        }
        ClientParticleOptionValue::Power { power_bits }
            if particle_type_id == VANILLA_PARTICLE_TYPE_DRAGON_BREATH_ID =>
        {
            ParticleOptionRenderState {
                power: Some(f32::from_bits(*power_bits)),
                ..ParticleOptionRenderState::default()
            }
        }
        ClientParticleOptionValue::Dust { color, scale_bits }
            if particle_type_id == VANILLA_PARTICLE_TYPE_DUST_ID =>
        {
            ParticleOptionRenderState {
                color: Some(rgb_particle_color_u32(*color)),
                scale: Some(f32::from_bits(*scale_bits)),
                ..ParticleOptionRenderState::default()
            }
        }
        ClientParticleOptionValue::DustColorTransition {
            from_color,
            to_color,
            scale_bits,
        } if particle_type_id == VANILLA_PARTICLE_TYPE_DUST_COLOR_TRANSITION_ID => {
            ParticleOptionRenderState {
                color: Some(rgb_particle_color_u32(*from_color)),
                color_to: Some(rgb_particle_color_u32(*to_color)),
                scale: Some(f32::from_bits(*scale_bits)),
                ..ParticleOptionRenderState::default()
            }
        }
        ClientParticleOptionValue::Spell { color, power_bits }
            if matches!(
                particle_type_id,
                VANILLA_PARTICLE_TYPE_EFFECT_ID | VANILLA_PARTICLE_TYPE_INSTANT_EFFECT_ID
            ) =>
        {
            ParticleOptionRenderState {
                color: Some(rgb_particle_color_u32(*color)),
                power: Some(f32::from_bits(*power_bits)),
                ..ParticleOptionRenderState::default()
            }
        }
        ClientParticleOptionValue::Color { argb }
            if matches!(
                particle_type_id,
                VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID
                    | VANILLA_PARTICLE_TYPE_TINTED_LEAVES_ID
                    | VANILLA_PARTICLE_TYPE_FLASH_ID
            ) =>
        {
            ParticleOptionRenderState {
                color: Some(argb_particle_color(*argb as i32)),
                ..ParticleOptionRenderState::default()
            }
        }
        ClientParticleOptionValue::SculkCharge { roll_bits }
            if particle_type_id == VANILLA_PARTICLE_TYPE_SCULK_CHARGE_ID =>
        {
            ParticleOptionRenderState {
                roll: Some(f32::from_bits(*roll_bits)),
                ..ParticleOptionRenderState::default()
            }
        }
        ClientParticleOptionValue::Item { stack }
            if particle_type_id == VANILLA_PARTICLE_TYPE_ITEM_ID =>
        {
            ParticleOptionRenderState {
                item: Some(particle_item_option_state_for_stack(stack)?),
                item_stack: Some(stack.clone()),
                item_component_patch_empty: stack.component_patch == Default::default(),
                ..ParticleOptionRenderState::default()
            }
        }
        ClientParticleOptionValue::Vibration {
            block_pos,
            arrival_in_ticks,
        } if particle_type_id == VANILLA_PARTICLE_TYPE_VIBRATION_ID => ParticleOptionRenderState {
            target: Some([
                f64::from(block_pos[0]) + 0.5,
                f64::from(block_pos[1]) + 0.5,
                f64::from(block_pos[2]) + 0.5,
            ]),
            duration_ticks: u32::try_from(*arrival_in_ticks)
                .ok()
                .filter(|duration| *duration > 0),
            ..ParticleOptionRenderState::default()
        },
        ClientParticleOptionValue::Trail {
            target_bits,
            color,
            duration,
        } if particle_type_id == VANILLA_PARTICLE_TYPE_TRAIL_ID => ParticleOptionRenderState {
            color: Some(rgb_particle_color_u32(*color)),
            target: Some(target_bits.map(f64::from_bits)),
            duration_ticks: u32::try_from(*duration)
                .ok()
                .filter(|duration| *duration > 0),
            ..ParticleOptionRenderState::default()
        },
        ClientParticleOptionValue::Shriek { delay }
            if particle_type_id == VANILLA_PARTICLE_TYPE_SHRIEK_ID =>
        {
            return Some((
                ParticleOptionRenderState::default(),
                u32::try_from(*delay).unwrap_or(0),
            ));
        }
        _ => return None,
    };
    Some((state, 0))
}
