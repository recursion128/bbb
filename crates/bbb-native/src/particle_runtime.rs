use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use bbb_pack::{
    AtlasLayout, AtlasPacker, AtlasSprite, PackRoots, ParticleDefinitionCatalog,
    ParticleSpriteCatalog, SpriteImage,
};
use bbb_protocol::codec::Decoder;
use bbb_protocol::packets::{ClientParticleStatus, LevelEvent, LevelParticles, Vec3d};
use bbb_renderer::{
    ParticleChildSpawnTemplate, ParticleSpawnBatch, ParticleSpawnCommand, ParticleSpriteUv,
    ParticleUvRect, Renderer,
};
use bbb_world::LevelEventSoundRandomState;

use crate::particle_registry::{vanilla_particle_type, ParticleTypeInfo};

pub(crate) trait ParticleEventSink {
    fn spawn_level_particles(
        &mut self,
        packet: &LevelParticles,
        context: LevelParticleSpawnContext,
    ) -> ParticleSpawnBatch;
    fn spawn_level_event_particles(
        &mut self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch;
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct LevelParticleSpawnContext {
    pub(crate) camera_position: Option<[f64; 3]>,
}

pub(crate) struct NativeParticleRuntime {
    resolver: ParticleCommandResolver,
    atlas: NativeParticleAtlas,
}

impl NativeParticleRuntime {
    pub(crate) fn load(roots: &PackRoots, particle_status: ClientParticleStatus) -> Result<Self> {
        let definitions = roots
            .load_particle_definition_catalog()
            .context("load particle definition catalog")?;
        let sprites = roots
            .load_particle_sprite_catalog()
            .context("load particle sprite catalog")?;
        let atlas = particle_atlas_from_images(sprites.sprites().values().cloned().collect())
            .context("stitch particle atlas")?;
        Ok(Self {
            resolver: ParticleCommandResolver::new(definitions, sprites, particle_status),
            atlas,
        })
    }

    pub(crate) fn upload_particle_atlas(&self, renderer: &mut Renderer) -> Result<()> {
        renderer.upload_particle_atlas(
            self.atlas.width,
            self.atlas.height,
            &self.atlas.rgba,
            self.atlas.sprite_uvs.clone(),
        )
    }
}

impl ParticleEventSink for NativeParticleRuntime {
    fn spawn_level_particles(
        &mut self,
        packet: &LevelParticles,
        context: LevelParticleSpawnContext,
    ) -> ParticleSpawnBatch {
        self.resolver
            .resolve_level_particles_with_context(packet, context)
    }

    fn spawn_level_event_particles(
        &mut self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        self.resolver.resolve_level_event_particles(event, random)
    }
}

#[derive(Debug, Clone)]
struct ParticleCommandResolver {
    definitions: ParticleDefinitionCatalog,
    sprites: ParticleSpriteCatalog,
    random: LegacyRandom,
    particle_level_random: LegacyRandom,
    particle_status: ClientParticleStatus,
}

#[derive(Debug, Clone)]
struct SimpleParticleTemplate {
    particle_type: ParticleTypeInfo,
    sprite_ids: Vec<String>,
    missing_sprite_count: usize,
}

#[derive(Debug, Clone)]
struct NativeParticleAtlas {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    sprite_uvs: Vec<ParticleSpriteUv>,
}

fn particle_atlas_from_images(images: Vec<SpriteImage>) -> Result<NativeParticleAtlas> {
    let atlas = AtlasPacker::new(4096, 1)?.stitch(&images)?;
    let sprite_uvs = atlas
        .layout
        .sprites
        .iter()
        .map(|sprite| ParticleSpriteUv {
            id: sprite.id.clone(),
            uv: particle_uv_rect(&atlas.layout, sprite),
        })
        .collect();
    Ok(NativeParticleAtlas {
        width: atlas.layout.width,
        height: atlas.layout.height,
        rgba: atlas.rgba,
        sprite_uvs,
    })
}

fn particle_uv_rect(layout: &AtlasLayout, sprite: &AtlasSprite) -> ParticleUvRect {
    let width = layout.width as f32;
    let height = layout.height as f32;
    let x0 = sprite.content.x as f32;
    let y0 = sprite.content.y as f32;
    let x1 = (sprite.content.x + sprite.content.width) as f32;
    let y1 = (sprite.content.y + sprite.content.height) as f32;
    ParticleUvRect {
        min: [(x0 + 0.5) / width, (y0 + 0.5) / height],
        max: [(x1 - 0.5) / width, (y1 - 0.5) / height],
    }
}

impl ParticleCommandResolver {
    fn new(
        definitions: ParticleDefinitionCatalog,
        sprites: ParticleSpriteCatalog,
        particle_status: ClientParticleStatus,
    ) -> Self {
        Self {
            definitions,
            sprites,
            random: LegacyRandom::new(default_particle_seed()),
            particle_level_random: LegacyRandom::new(default_particle_seed()),
            particle_status,
        }
    }

    #[cfg(test)]
    fn with_seed_and_particle_status(
        definitions: ParticleDefinitionCatalog,
        sprites: ParticleSpriteCatalog,
        seed: i64,
        particle_status: ClientParticleStatus,
    ) -> Self {
        Self {
            definitions,
            sprites,
            random: LegacyRandom::new(seed),
            particle_level_random: LegacyRandom::new(seed),
            particle_status,
        }
    }

    fn resolve_level_particles(&mut self, packet: &LevelParticles) -> ParticleSpawnBatch {
        self.resolve_level_particles_with_context(packet, LevelParticleSpawnContext::default())
    }

    fn resolve_level_particles_with_context(
        &mut self,
        packet: &LevelParticles,
        context: LevelParticleSpawnContext,
    ) -> ParticleSpawnBatch {
        if packet.count < 0 {
            return ParticleSpawnBatch::default();
        }

        let Some(particle_type) = vanilla_particle_type(packet.particle.particle_type_id) else {
            return ParticleSpawnBatch {
                unknown_particle_type_count: 1,
                ..ParticleSpawnBatch::default()
            };
        };
        let Some(definition) = self.definitions.definition(particle_type.name) else {
            return ParticleSpawnBatch {
                missing_definition_count: 1,
                ..ParticleSpawnBatch::default()
            };
        };

        let sprite_ids = definition.textures.clone();
        let missing_sprite_count = sprite_ids
            .iter()
            .filter(|sprite_id| self.sprites.sprite(sprite_id).is_none())
            .count();
        let override_limiter = particle_type.override_limiter || packet.override_limiter;
        let raw_options_len = packet.particle.raw_options.len();
        let option_state =
            particle_option_render_state(particle_type.id, &packet.particle.raw_options);
        let initial_delay_ticks = initial_delay_ticks_for_particle_options(
            particle_type.id,
            &packet.particle.raw_options,
        );
        let command_count = if packet.count == 0 {
            1
        } else {
            packet.count as usize
        };
        let mut commands = Vec::with_capacity(command_count);

        if packet.count == 0 {
            let position = packet.position;
            let velocity = Vec3d {
                x: packet.offset.x * f64::from(packet.max_speed),
                y: packet.offset.y * f64::from(packet.max_speed),
                z: packet.offset.z * f64::from(packet.max_speed),
            };
            if self.should_spawn_level_particle(
                override_limiter,
                packet.always_show,
                position,
                context.camera_position,
            ) {
                commands.push(self.command(
                    packet,
                    particle_type,
                    &sprite_ids,
                    position,
                    velocity,
                    override_limiter,
                    raw_options_len,
                    initial_delay_ticks,
                    option_state,
                ));
            }
        } else {
            for _ in 0..packet.count {
                let position = Vec3d {
                    x: packet.position.x + self.random.next_gaussian() * packet.offset.x,
                    y: packet.position.y + self.random.next_gaussian() * packet.offset.y,
                    z: packet.position.z + self.random.next_gaussian() * packet.offset.z,
                };
                let velocity = Vec3d {
                    x: self.random.next_gaussian() * f64::from(packet.max_speed),
                    y: self.random.next_gaussian() * f64::from(packet.max_speed),
                    z: self.random.next_gaussian() * f64::from(packet.max_speed),
                };
                if self.should_spawn_level_particle(
                    override_limiter,
                    packet.always_show,
                    position,
                    context.camera_position,
                ) {
                    commands.push(self.command(
                        packet,
                        particle_type,
                        &sprite_ids,
                        position,
                        velocity,
                        override_limiter,
                        raw_options_len,
                        initial_delay_ticks,
                        option_state,
                    ));
                }
            }
        }

        ParticleSpawnBatch {
            commands,
            missing_sprite_count,
            ..ParticleSpawnBatch::default()
        }
    }

    fn should_spawn_level_particle(
        &mut self,
        override_limiter: bool,
        always_show: bool,
        position: Vec3d,
        camera_position: Option<[f64; 3]>,
    ) -> bool {
        let particle_level = self.calculate_particle_level(always_show);
        if override_limiter {
            return true;
        }
        if let Some(camera_position) = camera_position {
            let dx = position.x - camera_position[0];
            let dy = position.y - camera_position[1];
            let dz = position.z - camera_position[2];
            if dx * dx + dy * dy + dz * dz > 1024.0 {
                return false;
            }
        }
        particle_level != ClientParticleStatus::Minimal
    }

    fn calculate_particle_level(&mut self, always_show: bool) -> ClientParticleStatus {
        let mut particle_level = self.particle_status;
        if always_show
            && particle_level == ClientParticleStatus::Minimal
            && self.particle_level_random.next_i32(10) == 0
        {
            particle_level = ClientParticleStatus::Decreased;
        }
        if particle_level == ClientParticleStatus::Decreased
            && self.particle_level_random.next_i32(3) == 0
        {
            particle_level = ClientParticleStatus::Minimal;
        }
        particle_level
    }

    fn resolve_level_event_particles(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        match event.event_type {
            LAVA_EXTINGUISH_LEVEL_EVENT => {
                let mut spawns = Vec::with_capacity(8);
                for _ in 0..8 {
                    spawns.push((
                        Vec3d {
                            x: f64::from(event.pos.x) + random.next_double(),
                            y: f64::from(event.pos.y) + 1.2,
                            z: f64::from(event.pos.z) + random.next_double(),
                        },
                        Vec3d::default(),
                    ));
                }
                self.simple_particle_batch(LARGE_SMOKE_PARTICLE_TYPE_ID, spawns)
            }
            REDSTONE_TORCH_BURNOUT_LEVEL_EVENT => {
                let mut spawns = Vec::with_capacity(5);
                for _ in 0..5 {
                    spawns.push((
                        Vec3d {
                            x: f64::from(event.pos.x) + random.next_double() * 0.6 + 0.2,
                            y: f64::from(event.pos.y) + random.next_double() * 0.6 + 0.2,
                            z: f64::from(event.pos.z) + random.next_double() * 0.6 + 0.2,
                        },
                        Vec3d::default(),
                    ));
                }
                self.simple_particle_batch(SMOKE_PARTICLE_TYPE_ID, spawns)
            }
            END_PORTAL_FRAME_FILL_LEVEL_EVENT => {
                let mut spawns = Vec::with_capacity(16);
                for _ in 0..16 {
                    spawns.push((
                        Vec3d {
                            x: f64::from(event.pos.x) + (5.0 + random.next_double() * 6.0) / 16.0,
                            y: f64::from(event.pos.y) + 0.8125,
                            z: f64::from(event.pos.z) + (5.0 + random.next_double() * 6.0) / 16.0,
                        },
                        Vec3d::default(),
                    ));
                }
                self.simple_particle_batch(SMOKE_PARTICLE_TYPE_ID, spawns)
            }
            DISPENSER_SMOKE_LEVEL_EVENT => {
                self.shoot_particles(event, SMOKE_PARTICLE_TYPE_ID, random)
            }
            BLAZE_SMOKE_LEVEL_EVENT => {
                let mut batch = ParticleSpawnBatch::default();
                let smoke = self.simple_particle_template(SMOKE_PARTICLE_TYPE_ID);
                let flame = self.simple_particle_template(FLAME_PARTICLE_TYPE_ID);
                let smoke = self.append_template_result(&mut batch, smoke);
                let flame = self.append_template_result(&mut batch, flame);

                for _ in 0..20 {
                    let position = Vec3d {
                        x: f64::from(event.pos.x) + 0.5 + (random.next_double() - 0.5) * 2.0,
                        y: f64::from(event.pos.y) + 0.5 + (random.next_double() - 0.5) * 2.0,
                        z: f64::from(event.pos.z) + 0.5 + (random.next_double() - 0.5) * 2.0,
                    };
                    if let Some(smoke) = smoke.as_ref() {
                        batch.commands.push(self.command_from_template(
                            smoke,
                            position,
                            Vec3d::default(),
                            false,
                        ));
                    }
                    if let Some(flame) = flame.as_ref() {
                        batch.commands.push(self.command_from_template(
                            flame,
                            position,
                            Vec3d::default(),
                            false,
                        ));
                    }
                }
                batch
            }
            DRAGON_FIREBALL_EXPLODE_LEVEL_EVENT => self.dragon_breath_particle_batch(event, random),
            ENDER_EYE_BREAK_LEVEL_EVENT => self.ender_eye_break_particle_batch(event, random),
            EXPLOSION_LEVEL_EVENT => self.simple_particle_batch(
                EXPLOSION_PARTICLE_TYPE_ID,
                vec![(
                    Vec3d {
                        x: f64::from(event.pos.x) + 0.5,
                        y: f64::from(event.pos.y) + 0.5,
                        z: f64::from(event.pos.z) + 0.5,
                    },
                    Vec3d::default(),
                )],
            ),
            END_GATEWAY_SPAWN_LEVEL_EVENT => self.simple_particle_batch_with_visibility(
                EXPLOSION_EMITTER_PARTICLE_TYPE_ID,
                vec![(
                    Vec3d {
                        x: f64::from(event.pos.x) + 0.5,
                        y: f64::from(event.pos.y) + 0.5,
                        z: f64::from(event.pos.z) + 0.5,
                    },
                    Vec3d::default(),
                )],
                true,
            ),
            ELECTRIC_SPARK_LEVEL_EVENT => match event.data {
                0..=2 => self.axis_particle_batch(
                    event,
                    ELECTRIC_SPARK_PARTICLE_TYPE_ID,
                    event.data,
                    ELECTRIC_SPARK_AXIS_RADIUS,
                    ELECTRIC_SPARK_AXIS_MIN,
                    ELECTRIC_SPARK_AXIS_MAX,
                    random,
                ),
                _ => self.block_face_particle_batch(
                    event,
                    ELECTRIC_SPARK_PARTICLE_TYPE_ID,
                    BLOCK_FACE_PARTICLE_MIN,
                    BLOCK_FACE_PARTICLE_MAX,
                    random,
                ),
            },
            WAX_ON_LEVEL_EVENT => self.block_face_particle_batch(
                event,
                WAX_ON_PARTICLE_TYPE_ID,
                BLOCK_FACE_PARTICLE_MIN,
                BLOCK_FACE_PARTICLE_MAX,
                random,
            ),
            WAX_OFF_LEVEL_EVENT => self.block_face_particle_batch(
                event,
                WAX_OFF_PARTICLE_TYPE_ID,
                BLOCK_FACE_PARTICLE_MIN,
                BLOCK_FACE_PARTICLE_MAX,
                random,
            ),
            SCRAPE_LEVEL_EVENT => self.block_face_particle_batch(
                event,
                SCRAPE_PARTICLE_TYPE_ID,
                BLOCK_FACE_PARTICLE_MIN,
                BLOCK_FACE_PARTICLE_MAX,
                random,
            ),
            EGG_CRACK_LEVEL_EVENT => self.block_face_particle_batch(
                event,
                EGG_CRACK_PARTICLE_TYPE_ID,
                BLOCK_FACE_PARTICLE_MIN,
                EGG_CRACK_PARTICLE_MAX,
                random,
            ),
            SCULK_SHRIEK_PARTICLES_LEVEL_EVENT => self.sculk_shriek_particle_batch(event),
            TRIAL_SPAWNER_SPAWN_PARTICLES_LEVEL_EVENT
            | TRIAL_SPAWNER_SPAWN_MOB_LEVEL_EVENT
            | TRIAL_SPAWNER_SPAWN_ITEM_LEVEL_EVENT => {
                self.trial_spawn_particle_batch(event, random)
            }
            TRIAL_SPAWNER_DETECT_PLAYER_LEVEL_EVENT => self.trial_detect_player_particle_batch(
                event,
                TRIAL_SPAWNER_DETECTED_PLAYER_PARTICLE_TYPE_ID,
                event.data,
                random,
            ),
            TRIAL_SPAWNER_EJECT_ITEM_LEVEL_EVENT
            | TRIAL_SPAWNER_EJECT_ITEM_PARTICLES_LEVEL_EVENT => {
                self.trial_eject_item_particle_batch(event, random)
            }
            TRIAL_SPAWNER_DETECT_PLAYER_OMINOUS_LEVEL_EVENT => self
                .trial_detect_player_particle_batch(
                    event,
                    TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS_PARTICLE_TYPE_ID,
                    event.data,
                    random,
                ),
            TRIAL_SPAWNER_OMINOUS_ACTIVATE_LEVEL_EVENT => {
                self.trial_ominous_activate_particle_batch(event, random)
            }
            COBWEB_PLACE_PARTICLES_LEVEL_EVENT => self.cobweb_poof_particle_batch(event, random),
            SPLASH_CLOUD_LEVEL_EVENT => {
                let mut spawns = Vec::with_capacity(8);
                for _ in 0..8 {
                    spawns.push((
                        Vec3d {
                            x: f64::from(event.pos.x) + random.next_double(),
                            y: f64::from(event.pos.y) + 1.2,
                            z: f64::from(event.pos.z) + random.next_double(),
                        },
                        Vec3d::default(),
                    ));
                }
                self.simple_particle_batch(CLOUD_PARTICLE_TYPE_ID, spawns)
            }
            BEE_GROWTH_PARTICLES_LEVEL_EVENT | TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT => self
                .particle_in_block_batch(
                    event,
                    HAPPY_VILLAGER_PARTICLE_TYPE_ID,
                    event.data,
                    random,
                ),
            DISPENSER_WHITE_SMOKE_LEVEL_EVENT => {
                self.shoot_particles(event, WHITE_SMOKE_PARTICLE_TYPE_ID, random)
            }
            _ => ParticleSpawnBatch::default(),
        }
    }

    fn shoot_particles(
        &self,
        event: &LevelEvent,
        particle_type_id: i32,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let (normal_x, normal_y, normal_z) = direction_normal_from_3d_data_value(event.data);
        let mut spawns = Vec::with_capacity(10);
        for _ in 0..10 {
            let pow = random.next_double() * 0.2 + 0.01;
            let position = Vec3d {
                x: f64::from(event.pos.x)
                    + f64::from(normal_x) * 0.6
                    + 0.5
                    + f64::from(normal_x) * 0.01
                    + (random.next_double() - 0.5) * f64::from(normal_z) * 0.5,
                y: f64::from(event.pos.y)
                    + f64::from(normal_y) * 0.6
                    + 0.5
                    + f64::from(normal_y) * 0.01
                    + (random.next_double() - 0.5) * f64::from(normal_y) * 0.5,
                z: f64::from(event.pos.z)
                    + f64::from(normal_z) * 0.6
                    + 0.5
                    + f64::from(normal_z) * 0.01
                    + (random.next_double() - 0.5) * f64::from(normal_x) * 0.5,
            };
            let velocity = Vec3d {
                x: f64::from(normal_x) * pow + random.next_gaussian() * 0.01,
                y: f64::from(normal_y) * pow + random.next_gaussian() * 0.01,
                z: f64::from(normal_z) * pow + random.next_gaussian() * 0.01,
            };
            spawns.push((position, velocity));
        }
        self.simple_particle_batch(particle_type_id, spawns)
    }

    fn sculk_shriek_particle_batch(&self, event: &LevelEvent) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(SHRIEK_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let position = Vec3d {
            x: f64::from(event.pos.x) + 0.5,
            y: f64::from(event.pos.y) + SCULK_SHRIEKER_TOP_Y,
            z: f64::from(event.pos.z) + 0.5,
        };

        for index in 0..SCULK_SHRIEK_PARTICLE_COUNT {
            let mut command =
                self.command_from_template(&template, position, Vec3d::default(), false);
            command.initial_delay_ticks = index * SCULK_SHRIEK_DELAY_STEP_TICKS;
            batch.commands.push(command);
        }

        batch
    }

    fn trial_spawn_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let flame_particle_type_id = match event.data {
            1 => SOUL_FIRE_FLAME_PARTICLE_TYPE_ID,
            _ => FLAME_PARTICLE_TYPE_ID,
        };
        let mut batch = ParticleSpawnBatch::default();
        let smoke = self.simple_particle_template(SMOKE_PARTICLE_TYPE_ID);
        let flame = self.simple_particle_template(flame_particle_type_id);
        let smoke = self.append_template_result(&mut batch, smoke);
        let flame = self.append_template_result(&mut batch, flame);

        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + 0.5 + (random.next_double() - 0.5) * 2.0,
                y: f64::from(event.pos.y) + 0.5 + (random.next_double() - 0.5) * 2.0,
                z: f64::from(event.pos.z) + 0.5 + (random.next_double() - 0.5) * 2.0,
            };
            if let Some(smoke) = smoke.as_ref() {
                batch.commands.push(self.command_from_template(
                    smoke,
                    position,
                    Vec3d::default(),
                    false,
                ));
            }
            if let Some(flame) = flame.as_ref() {
                batch.commands.push(self.command_from_template(
                    flame,
                    position,
                    Vec3d::default(),
                    false,
                ));
            }
        }

        batch
    }

    fn dragon_breath_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(DRAGON_BREATH_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..200 {
            let dist = random.next_float() * 4.0;
            let angle = random.next_float() * std::f32::consts::TAU;
            let velocity_x = f64::from(angle.cos() * dist);
            let velocity_y = 0.01 + random.next_double() * 0.5;
            let velocity_z = f64::from(angle.sin() * dist);
            let position = Vec3d {
                x: f64::from(event.pos.x) + velocity_x * 0.1,
                y: f64::from(event.pos.y) + 0.3,
                z: f64::from(event.pos.z) + velocity_z * 0.1,
            };
            // `PowerParticleOption.create(..., dist)` calls `Particle.setPower(dist)`
            // after the provider creates DragonBreathParticle.
            let powered_velocity = Vec3d {
                x: velocity_x * f64::from(dist),
                y: (velocity_y - 0.1) * f64::from(dist) + 0.1,
                z: velocity_z * f64::from(dist),
            };
            batch.commands.push(self.command_from_template(
                &template,
                position,
                powered_velocity,
                false,
            ));
        }

        batch
    }

    fn ender_eye_break_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        for _ in 0..ENDER_EYE_BREAK_ITEM_PARTICLE_COUNT {
            random.next_gaussian();
            random.next_double();
            random.next_gaussian();
        }

        let template = match self.simple_particle_template(PORTAL_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        let center_x = f64::from(event.pos.x) + 0.5;
        let y = f64::from(event.pos.y);
        let center_z = f64::from(event.pos.z) + 0.5;
        let mut angle = 0.0_f64;
        while angle < std::f64::consts::TAU {
            let angle_cos = angle.cos();
            let angle_sin = angle.sin();
            let position = Vec3d {
                x: center_x + angle_cos * 5.0,
                y: y - 0.4,
                z: center_z + angle_sin * 5.0,
            };
            batch.commands.push(self.command_from_template(
                &template,
                position,
                Vec3d {
                    x: angle_cos * -5.0,
                    y: 0.0,
                    z: angle_sin * -5.0,
                },
                false,
            ));
            batch.commands.push(self.command_from_template(
                &template,
                position,
                Vec3d {
                    x: angle_cos * -7.0,
                    y: 0.0,
                    z: angle_sin * -7.0,
                },
                false,
            ));
            angle += std::f64::consts::PI / 20.0;
        }

        batch
    }

    fn trial_eject_item_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let mut batch = ParticleSpawnBatch::default();
        let small_flame = self.simple_particle_template(SMALL_FLAME_PARTICLE_TYPE_ID);
        let smoke = self.simple_particle_template(SMOKE_PARTICLE_TYPE_ID);
        let small_flame = self.append_template_result(&mut batch, small_flame);
        let smoke = self.append_template_result(&mut batch, smoke);

        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + 0.4 + random.next_double() * 0.2,
                y: f64::from(event.pos.y) + 0.4 + random.next_double() * 0.2,
                z: f64::from(event.pos.z) + 0.4 + random.next_double() * 0.2,
            };
            let velocity = Vec3d {
                x: random.next_gaussian() * 0.02,
                y: random.next_gaussian() * 0.02,
                z: random.next_gaussian() * 0.02,
            };
            if let Some(small_flame) = small_flame.as_ref() {
                batch.commands.push(self.command_from_template(
                    small_flame,
                    position,
                    Vec3d {
                        z: velocity.z * 0.25,
                        ..velocity
                    },
                    false,
                ));
            }
            if let Some(smoke) = smoke.as_ref() {
                batch
                    .commands
                    .push(self.command_from_template(smoke, position, velocity, false));
            }
        }

        batch
    }

    fn trial_detect_player_particle_batch(
        &self,
        event: &LevelEvent,
        particle_type_id: i32,
        data: i32,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let mut batch = ParticleSpawnBatch::default();
        self.append_trial_detect_player_particles(
            &mut batch,
            event,
            particle_type_id,
            data,
            random,
        );
        batch
    }

    fn trial_ominous_activate_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let mut batch = ParticleSpawnBatch::default();
        self.append_trial_detect_player_particles(
            &mut batch,
            event,
            TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS_PARTICLE_TYPE_ID,
            0,
            random,
        );
        self.append_trial_become_ominous_particles(&mut batch, event, random);
        batch
    }

    fn append_trial_detect_player_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        event: &LevelEvent,
        particle_type_id: i32,
        data: i32,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(template) =
            self.append_template_result(batch, self.simple_particle_template(particle_type_id))
        else {
            return;
        };
        let count = 30_i64 + i64::from(data.min(10)) * 5;
        for _ in 0..count.max(0) {
            let spread_x = (2.0 * f64::from(random.next_float()) - 1.0) * 0.65;
            let spread_z = (2.0 * f64::from(random.next_float()) - 1.0) * 0.65;
            let position = Vec3d {
                x: f64::from(event.pos.x) + 0.5 + spread_x,
                y: f64::from(event.pos.y) + 0.1 + f64::from(random.next_float()) * 0.8,
                z: f64::from(event.pos.z) + 0.5 + spread_z,
            };
            batch.commands.push(self.command_from_template(
                &template,
                position,
                Vec3d::default(),
                false,
            ));
        }
    }

    fn append_trial_become_ominous_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) {
        let trial_omen = self.simple_particle_template(TRIAL_OMEN_PARTICLE_TYPE_ID);
        let soul_fire_flame = self.simple_particle_template(SOUL_FIRE_FLAME_PARTICLE_TYPE_ID);
        let trial_omen = self.append_template_result(batch, trial_omen);
        let soul_fire_flame = self.append_template_result(batch, soul_fire_flame);

        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + 0.5 + (random.next_double() - 0.5) * 2.0,
                y: f64::from(event.pos.y) + 0.5 + (random.next_double() - 0.5) * 2.0,
                z: f64::from(event.pos.z) + 0.5 + (random.next_double() - 0.5) * 2.0,
            };
            let velocity = Vec3d {
                x: random.next_gaussian() * 0.02,
                y: random.next_gaussian() * 0.02,
                z: random.next_gaussian() * 0.02,
            };
            if let Some(trial_omen) = trial_omen.as_ref() {
                batch
                    .commands
                    .push(self.command_from_template(trial_omen, position, velocity, false));
            }
            if let Some(soul_fire_flame) = soul_fire_flame.as_ref() {
                batch.commands.push(self.command_from_template(
                    soul_fire_flame,
                    position,
                    velocity,
                    false,
                ));
            }
        }
    }

    fn cobweb_poof_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        self.particle_in_block_batch(event, POOF_PARTICLE_TYPE_ID, 10, random)
    }

    fn particle_in_block_batch(
        &self,
        event: &LevelEvent,
        particle_type_id: i32,
        count: i32,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        if count <= 0 {
            return ParticleSpawnBatch::default();
        }
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..count {
            let velocity = Vec3d {
                x: random.next_gaussian() * 0.02,
                y: random.next_gaussian() * 0.02,
                z: random.next_gaussian() * 0.02,
            };
            let position = Vec3d {
                x: f64::from(event.pos.x) + random.next_double(),
                y: f64::from(event.pos.y) + random.next_double(),
                z: f64::from(event.pos.z) + random.next_double(),
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }

        batch
    }

    fn block_face_particle_batch(
        &self,
        event: &LevelEvent,
        particle_type_id: i32,
        min_particles_per_face: i32,
        max_particles_per_face: i32,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for direction in BLOCK_FACE_DIRECTIONS {
            let particle_count = random
                .next_int_bound(max_particles_per_face - min_particles_per_face + 1)
                + min_particles_per_face;
            for _ in 0..particle_count {
                let speed = Vec3d {
                    x: random_between(random, -0.5, 0.5),
                    y: random_between(random, -0.5, 0.5),
                    z: random_between(random, -0.5, 0.5),
                };
                let (position, velocity) =
                    block_face_particle(event, *direction, speed, BLOCK_FACE_STEP_FACTOR, random);
                batch
                    .commands
                    .push(self.command_from_template(&template, position, velocity, false));
            }
        }

        batch
    }

    fn axis_particle_batch(
        &self,
        event: &LevelEvent,
        particle_type_id: i32,
        axis: i32,
        radius: f64,
        min_particles: i32,
        max_particles: i32,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let particle_count =
            random.next_int_bound(max_particles - min_particles + 1) + min_particles;

        for _ in 0..particle_count {
            let (position, velocity) = axis_particle(event, axis, radius, random);
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }

        batch
    }

    fn simple_particle_batch(
        &self,
        particle_type_id: i32,
        spawns: Vec<(Vec3d, Vec3d)>,
    ) -> ParticleSpawnBatch {
        self.simple_particle_batch_with_visibility(particle_type_id, spawns, false)
    }

    fn simple_particle_batch_with_visibility(
        &self,
        particle_type_id: i32,
        spawns: Vec<(Vec3d, Vec3d)>,
        always_show: bool,
    ) -> ParticleSpawnBatch {
        if spawns.is_empty() {
            return ParticleSpawnBatch::default();
        }
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let commands = spawns
            .into_iter()
            .map(|(position, velocity)| {
                self.command_from_template(&template, position, velocity, always_show)
            })
            .collect();

        ParticleSpawnBatch {
            commands,
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        }
    }

    fn simple_particle_template(
        &self,
        particle_type_id: i32,
    ) -> Result<SimpleParticleTemplate, ParticleSpawnBatch> {
        let Some(particle_type) = vanilla_particle_type(particle_type_id) else {
            return Err(ParticleSpawnBatch {
                unknown_particle_type_count: 1,
                ..ParticleSpawnBatch::default()
            });
        };
        let Some(definition) = self.definitions.definition(particle_type.name) else {
            return Err(ParticleSpawnBatch {
                missing_definition_count: 1,
                ..ParticleSpawnBatch::default()
            });
        };

        let sprite_ids = definition.textures.clone();
        let missing_sprite_count = sprite_ids
            .iter()
            .filter(|sprite_id| self.sprites.sprite(sprite_id).is_none())
            .count();
        Ok(SimpleParticleTemplate {
            particle_type,
            sprite_ids,
            missing_sprite_count,
        })
    }

    fn append_template_result(
        &self,
        batch: &mut ParticleSpawnBatch,
        result: Result<SimpleParticleTemplate, ParticleSpawnBatch>,
    ) -> Option<SimpleParticleTemplate> {
        match result {
            Ok(template) => {
                batch.missing_sprite_count += template.missing_sprite_count;
                Some(template)
            }
            Err(diagnostic) => {
                append_particle_batch(batch, diagnostic);
                None
            }
        }
    }

    fn command_from_template(
        &self,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        velocity: Vec3d,
        always_show: bool,
    ) -> ParticleSpawnCommand {
        self.command_for_type(
            template.particle_type,
            &template.sprite_ids,
            position,
            velocity,
            template.particle_type.override_limiter,
            always_show,
            0,
            0,
            ParticleOptionRenderState::default(),
        )
    }

    fn command(
        &self,
        packet: &LevelParticles,
        particle_type: ParticleTypeInfo,
        sprite_ids: &[String],
        position: Vec3d,
        velocity: Vec3d,
        override_limiter: bool,
        raw_options_len: usize,
        initial_delay_ticks: u32,
        option_state: ParticleOptionRenderState,
    ) -> ParticleSpawnCommand {
        self.command_for_type(
            particle_type,
            sprite_ids,
            position,
            velocity,
            override_limiter,
            packet.always_show,
            raw_options_len,
            initial_delay_ticks,
            option_state,
        )
    }

    fn command_for_type(
        &self,
        particle_type: ParticleTypeInfo,
        sprite_ids: &[String],
        position: Vec3d,
        velocity: Vec3d,
        override_limiter: bool,
        always_show: bool,
        raw_options_len: usize,
        initial_delay_ticks: u32,
        option_state: ParticleOptionRenderState,
    ) -> ParticleSpawnCommand {
        let child_spawn_templates = self.child_spawn_templates_for_type(particle_type);
        ParticleSpawnCommand {
            particle_type_id: particle_type.id,
            particle_id: particle_type.name.to_string(),
            sprite_ids: sprite_ids.to_vec(),
            position: [position.x, position.y, position.z],
            velocity: [velocity.x, velocity.y, velocity.z],
            override_limiter,
            always_show,
            raw_options_len,
            initial_delay_ticks,
            child_spawn_templates,
            option_color: option_state.color,
            option_color_to: option_state.color_to,
            option_scale: option_state.scale,
            option_power: option_state.power,
            option_target: option_state.target,
            option_duration_ticks: option_state.duration_ticks,
            option_roll: option_state.roll,
        }
    }

    fn child_spawn_templates_for_type(
        &self,
        particle_type: ParticleTypeInfo,
    ) -> Vec<ParticleChildSpawnTemplate> {
        if particle_type.id != LAVA_PARTICLE_TYPE_ID {
            return Vec::new();
        }
        self.simple_particle_template(SMOKE_PARTICLE_TYPE_ID)
            .ok()
            .map(|template| {
                vec![ParticleChildSpawnTemplate {
                    particle_type_id: template.particle_type.id,
                    particle_id: template.particle_type.name.to_string(),
                    sprite_ids: template.sprite_ids,
                }]
            })
            .unwrap_or_default()
    }
}

fn initial_delay_ticks_for_particle_options(particle_type_id: i32, raw_options: &[u8]) -> u32 {
    if particle_type_id != SHRIEK_PARTICLE_TYPE_ID {
        return 0;
    }
    let mut decoder = Decoder::new(raw_options);
    match decoder.read_var_i32() {
        Ok(delay) if decoder.is_empty() => u32::try_from(delay).unwrap_or(0),
        _ => 0,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct ParticleOptionRenderState {
    color: Option<[f32; 4]>,
    color_to: Option<[f32; 4]>,
    scale: Option<f32>,
    power: Option<f32>,
    target: Option<[f64; 3]>,
    duration_ticks: Option<u32>,
    roll: Option<f32>,
}

fn particle_option_render_state(
    particle_type_id: i32,
    raw_options: &[u8],
) -> ParticleOptionRenderState {
    let mut decoder = Decoder::new(raw_options);
    match particle_type_id {
        EFFECT_PARTICLE_TYPE_ID | INSTANT_EFFECT_PARTICLE_TYPE_ID => {
            let Ok(color) = decoder.read_i32() else {
                return ParticleOptionRenderState::default();
            };
            let Ok(power) = decoder.read_f32() else {
                return ParticleOptionRenderState::default();
            };
            if !decoder.is_empty() {
                return ParticleOptionRenderState::default();
            }
            ParticleOptionRenderState {
                color: Some(rgb_particle_color(color)),
                power: Some(power),
                ..ParticleOptionRenderState::default()
            }
        }
        DUST_PARTICLE_TYPE_ID => {
            let Ok(color) = decoder.read_i32() else {
                return ParticleOptionRenderState::default();
            };
            let Ok(scale) = decoder.read_f32() else {
                return ParticleOptionRenderState::default();
            };
            if !decoder.is_empty() {
                return ParticleOptionRenderState::default();
            }
            ParticleOptionRenderState {
                color: Some(rgb_particle_color(color)),
                scale: Some(clamp_particle_option_scale(scale)),
                ..ParticleOptionRenderState::default()
            }
        }
        DUST_COLOR_TRANSITION_PARTICLE_TYPE_ID => {
            let Ok(from_color) = decoder.read_i32() else {
                return ParticleOptionRenderState::default();
            };
            let Ok(to_color) = decoder.read_i32() else {
                return ParticleOptionRenderState::default();
            };
            let Ok(scale) = decoder.read_f32() else {
                return ParticleOptionRenderState::default();
            };
            if !decoder.is_empty() {
                return ParticleOptionRenderState::default();
            }
            ParticleOptionRenderState {
                color: Some(rgb_particle_color(from_color)),
                color_to: Some(rgb_particle_color(to_color)),
                scale: Some(clamp_particle_option_scale(scale)),
                ..ParticleOptionRenderState::default()
            }
        }
        ENTITY_EFFECT_PARTICLE_TYPE_ID | FLASH_PARTICLE_TYPE_ID => {
            let Ok(color) = decoder.read_i32() else {
                return ParticleOptionRenderState::default();
            };
            if !decoder.is_empty() {
                return ParticleOptionRenderState::default();
            }
            ParticleOptionRenderState {
                color: Some(argb_particle_color(color)),
                ..ParticleOptionRenderState::default()
            }
        }
        SCULK_CHARGE_PARTICLE_TYPE_ID => {
            let Ok(roll) = decoder.read_f32() else {
                return ParticleOptionRenderState::default();
            };
            if !decoder.is_empty() {
                return ParticleOptionRenderState::default();
            }
            ParticleOptionRenderState {
                roll: Some(roll),
                ..ParticleOptionRenderState::default()
            }
        }
        VIBRATION_PARTICLE_TYPE_ID => {
            let Ok(target) = decode_vibration_position_source_target(&mut decoder) else {
                return ParticleOptionRenderState::default();
            };
            let Ok(arrival_ticks) = decoder.read_var_i32() else {
                return ParticleOptionRenderState::default();
            };
            if !decoder.is_empty() {
                return ParticleOptionRenderState::default();
            }
            ParticleOptionRenderState {
                target,
                duration_ticks: u32::try_from(arrival_ticks)
                    .ok()
                    .filter(|duration| *duration > 0),
                ..ParticleOptionRenderState::default()
            }
        }
        TRAIL_PARTICLE_TYPE_ID => {
            let Ok(target) = decode_option_vec3d(&mut decoder) else {
                return ParticleOptionRenderState::default();
            };
            let Ok(color) = decoder.read_i32() else {
                return ParticleOptionRenderState::default();
            };
            let Ok(duration) = decoder.read_var_i32() else {
                return ParticleOptionRenderState::default();
            };
            if !decoder.is_empty() {
                return ParticleOptionRenderState::default();
            }
            ParticleOptionRenderState {
                color: Some(rgb_particle_color(color)),
                target: Some(target),
                duration_ticks: u32::try_from(duration)
                    .ok()
                    .filter(|duration| *duration > 0),
                ..ParticleOptionRenderState::default()
            }
        }
        _ => ParticleOptionRenderState::default(),
    }
}

fn decode_vibration_position_source_target(
    decoder: &mut Decoder<'_>,
) -> bbb_protocol::codec::Result<Option<[f64; 3]>> {
    match decoder.read_var_i32()? {
        0 => {
            let packed = decoder.read_i64()?;
            let x = (packed >> 38) as i32;
            let y = ((packed << 52) >> 52) as i32;
            let z = ((packed << 26) >> 38) as i32;
            Ok(Some([
                f64::from(x) + 0.5,
                f64::from(y) + 0.5,
                f64::from(z) + 0.5,
            ]))
        }
        1 => {
            decoder.read_var_i32()?;
            decoder.read_f32()?;
            Ok(None)
        }
        other => Err(bbb_protocol::codec::ProtocolError::InvalidData(format!(
            "unknown position source type id {other}"
        ))),
    }
}

fn decode_option_vec3d(decoder: &mut Decoder<'_>) -> bbb_protocol::codec::Result<[f64; 3]> {
    Ok([
        decoder.read_f64()?,
        decoder.read_f64()?,
        decoder.read_f64()?,
    ])
}

fn rgb_particle_color(color: i32) -> [f32; 4] {
    let color = color as u32;
    [
        ((color >> 16) & 0xff) as f32 / 255.0,
        ((color >> 8) & 0xff) as f32 / 255.0,
        (color & 0xff) as f32 / 255.0,
        1.0,
    ]
}

fn clamp_particle_option_scale(scale: f32) -> f32 {
    scale.clamp(0.01, 4.0)
}

fn argb_particle_color(color: i32) -> [f32; 4] {
    let color = color as u32;
    [
        ((color >> 16) & 0xff) as f32 / 255.0,
        ((color >> 8) & 0xff) as f32 / 255.0,
        (color & 0xff) as f32 / 255.0,
        ((color >> 24) & 0xff) as f32 / 255.0,
    ]
}

fn append_particle_batch(batch: &mut ParticleSpawnBatch, mut other: ParticleSpawnBatch) {
    batch.commands.append(&mut other.commands);
    batch.missing_definition_count += other.missing_definition_count;
    batch.missing_sprite_count += other.missing_sprite_count;
    batch.unknown_particle_type_count += other.unknown_particle_type_count;
}

fn direction_normal_from_3d_data_value(data: i32) -> (i32, i32, i32) {
    match (data % 6).abs() {
        0 => (0, -1, 0),
        1 => (0, 1, 0),
        2 => (0, 0, -1),
        3 => (0, 0, 1),
        4 => (-1, 0, 0),
        _ => (1, 0, 0),
    }
}

fn block_face_particle(
    event: &LevelEvent,
    (step_x, step_y, step_z): (i32, i32, i32),
    speed: Vec3d,
    step_factor: f64,
    random: &mut LevelEventSoundRandomState,
) -> (Vec3d, Vec3d) {
    let center = Vec3d {
        x: f64::from(event.pos.x) + 0.5,
        y: f64::from(event.pos.y) + 0.5,
        z: f64::from(event.pos.z) + 0.5,
    };
    let position = Vec3d {
        x: center.x
            + if step_x == 0 {
                random_between(random, -0.5, 0.5)
            } else {
                f64::from(step_x) * step_factor
            },
        y: center.y
            + if step_y == 0 {
                random_between(random, -0.5, 0.5)
            } else {
                f64::from(step_y) * step_factor
            },
        z: center.z
            + if step_z == 0 {
                random_between(random, -0.5, 0.5)
            } else {
                f64::from(step_z) * step_factor
            },
    };
    let velocity = Vec3d {
        x: if step_x == 0 { speed.x } else { 0.0 },
        y: if step_y == 0 { speed.y } else { 0.0 },
        z: if step_z == 0 { speed.z } else { 0.0 },
    };
    (position, velocity)
}

fn axis_particle(
    event: &LevelEvent,
    axis: i32,
    radius: f64,
    random: &mut LevelEventSoundRandomState,
) -> (Vec3d, Vec3d) {
    let center = Vec3d {
        x: f64::from(event.pos.x) + 0.5,
        y: f64::from(event.pos.y) + 0.5,
        z: f64::from(event.pos.z) + 0.5,
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

fn random_between(random: &mut LevelEventSoundRandomState, min: f64, max: f64) -> f64 {
    random.next_double() * (max - min) + min
}

const LAVA_EXTINGUISH_LEVEL_EVENT: i32 = 1501;
const REDSTONE_TORCH_BURNOUT_LEVEL_EVENT: i32 = 1502;
const END_PORTAL_FRAME_FILL_LEVEL_EVENT: i32 = 1503;
const DISPENSER_SMOKE_LEVEL_EVENT: i32 = 2000;
const ENDER_EYE_BREAK_LEVEL_EVENT: i32 = 2003;
const BLAZE_SMOKE_LEVEL_EVENT: i32 = 2004;
const DRAGON_FIREBALL_EXPLODE_LEVEL_EVENT: i32 = 2006;
const EXPLOSION_LEVEL_EVENT: i32 = 2008;
const SPLASH_CLOUD_LEVEL_EVENT: i32 = 2009;
const DISPENSER_WHITE_SMOKE_LEVEL_EVENT: i32 = 2010;
const BEE_GROWTH_PARTICLES_LEVEL_EVENT: i32 = 2011;
const TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT: i32 = 2012;
const END_GATEWAY_SPAWN_LEVEL_EVENT: i32 = 3000;
const ELECTRIC_SPARK_LEVEL_EVENT: i32 = 3002;
const WAX_ON_LEVEL_EVENT: i32 = 3003;
const WAX_OFF_LEVEL_EVENT: i32 = 3004;
const SCRAPE_LEVEL_EVENT: i32 = 3005;
const SCULK_SHRIEK_PARTICLES_LEVEL_EVENT: i32 = 3007;
const EGG_CRACK_LEVEL_EVENT: i32 = 3009;
const TRIAL_SPAWNER_SPAWN_PARTICLES_LEVEL_EVENT: i32 = 3011;
const TRIAL_SPAWNER_SPAWN_MOB_LEVEL_EVENT: i32 = 3012;
const TRIAL_SPAWNER_DETECT_PLAYER_LEVEL_EVENT: i32 = 3013;
const TRIAL_SPAWNER_EJECT_ITEM_LEVEL_EVENT: i32 = 3014;
const TRIAL_SPAWNER_EJECT_ITEM_PARTICLES_LEVEL_EVENT: i32 = 3017;
const COBWEB_PLACE_PARTICLES_LEVEL_EVENT: i32 = 3018;
const TRIAL_SPAWNER_DETECT_PLAYER_OMINOUS_LEVEL_EVENT: i32 = 3019;
const TRIAL_SPAWNER_OMINOUS_ACTIVATE_LEVEL_EVENT: i32 = 3020;
const TRIAL_SPAWNER_SPAWN_ITEM_LEVEL_EVENT: i32 = 3021;
const CLOUD_PARTICLE_TYPE_ID: i32 = 4;
const DRAGON_BREATH_PARTICLE_TYPE_ID: i32 = 8;
const DUST_PARTICLE_TYPE_ID: i32 = 14;
const DUST_COLOR_TRANSITION_PARTICLE_TYPE_ID: i32 = 15;
const EFFECT_PARTICLE_TYPE_ID: i32 = 16;
const ENTITY_EFFECT_PARTICLE_TYPE_ID: i32 = 21;
const EXPLOSION_EMITTER_PARTICLE_TYPE_ID: i32 = 22;
const EXPLOSION_PARTICLE_TYPE_ID: i32 = 23;
const FLAME_PARTICLE_TYPE_ID: i32 = 32;
const SCULK_CHARGE_PARTICLE_TYPE_ID: i32 = 38;
const SOUL_FIRE_FLAME_PARTICLE_TYPE_ID: i32 = 40;
const FLASH_PARTICLE_TYPE_ID: i32 = 42;
const HAPPY_VILLAGER_PARTICLE_TYPE_ID: i32 = 43;
const INSTANT_EFFECT_PARTICLE_TYPE_ID: i32 = 46;
const VIBRATION_PARTICLE_TYPE_ID: i32 = 48;
const TRAIL_PARTICLE_TYPE_ID: i32 = 49;
const LARGE_SMOKE_PARTICLE_TYPE_ID: i32 = 55;
const LAVA_PARTICLE_TYPE_ID: i32 = 56;
const POOF_PARTICLE_TYPE_ID: i32 = 59;
const PORTAL_PARTICLE_TYPE_ID: i32 = 60;
const SMOKE_PARTICLE_TYPE_ID: i32 = 62;
const WHITE_SMOKE_PARTICLE_TYPE_ID: i32 = 63;
const SMALL_FLAME_PARTICLE_TYPE_ID: i32 = 93;
const ELECTRIC_SPARK_PARTICLE_TYPE_ID: i32 = 103;
const WAX_ON_PARTICLE_TYPE_ID: i32 = 101;
const WAX_OFF_PARTICLE_TYPE_ID: i32 = 102;
const SCRAPE_PARTICLE_TYPE_ID: i32 = 104;
const SHRIEK_PARTICLE_TYPE_ID: i32 = 105;
const EGG_CRACK_PARTICLE_TYPE_ID: i32 = 106;
const TRIAL_SPAWNER_DETECTED_PLAYER_PARTICLE_TYPE_ID: i32 = 108;
const TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS_PARTICLE_TYPE_ID: i32 = 109;
const TRIAL_OMEN_PARTICLE_TYPE_ID: i32 = 114;
const BLOCK_FACE_DIRECTIONS: &[(i32, i32, i32)] = &[
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
    (-1, 0, 0),
    (1, 0, 0),
];
const BLOCK_FACE_STEP_FACTOR: f64 = 0.55;
const BLOCK_FACE_PARTICLE_MIN: i32 = 3;
const BLOCK_FACE_PARTICLE_MAX: i32 = 5;
const ELECTRIC_SPARK_AXIS_RADIUS: f64 = 0.125;
const ELECTRIC_SPARK_AXIS_MIN: i32 = 10;
const ELECTRIC_SPARK_AXIS_MAX: i32 = 19;
const EGG_CRACK_PARTICLE_MAX: i32 = 6;
const ENDER_EYE_BREAK_ITEM_PARTICLE_COUNT: i32 = 8;
const SCULK_SHRIEKER_TOP_Y: f64 = 0.5;
const SCULK_SHRIEK_PARTICLE_COUNT: u32 = 10;
const SCULK_SHRIEK_DELAY_STEP_TICKS: u32 = 5;

fn default_particle_seed() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as i64)
        .unwrap_or(0)
}

const RANDOM_MULTIPLIER: u64 = 25_214_903_917;
const RANDOM_INCREMENT: u64 = 11;
const RANDOM_MASK: u64 = (1_u64 << 48) - 1;

#[derive(Debug, Clone)]
struct LegacyRandom {
    seed: u64,
    next_gaussian: Option<f64>,
}

impl LegacyRandom {
    fn new(seed: i64) -> Self {
        Self {
            seed: ((seed as u64) ^ RANDOM_MULTIPLIER) & RANDOM_MASK,
            next_gaussian: None,
        }
    }

    fn next_gaussian(&mut self) -> f64 {
        if let Some(value) = self.next_gaussian.take() {
            return value;
        }

        loop {
            let v1 = 2.0 * self.next_f64() - 1.0;
            let v2 = 2.0 * self.next_f64() - 1.0;
            let s = v1 * v1 + v2 * v2;
            if s < 1.0 && s != 0.0 {
                let multiplier = (-2.0 * s.ln() / s).sqrt();
                self.next_gaussian = Some(v2 * multiplier);
                return v1 * multiplier;
            }
        }
    }

    fn next_f64(&mut self) -> f64 {
        let high = (self.next_bits(26) as u64) << 27;
        let low = self.next_bits(27) as u64;
        (high + low) as f64 / ((1_u64 << 53) as f64)
    }

    fn next_i32(&mut self, bound: i32) -> i32 {
        assert!(bound > 0);
        if (bound & -bound) == bound {
            return ((i64::from(bound) * i64::from(self.next_bits(31))) >> 31) as i32;
        }
        loop {
            let bits = self.next_bits(31) as i32;
            let value = bits % bound;
            if bits - value + (bound - 1) >= 0 {
                return value;
            }
        }
    }

    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(RANDOM_MULTIPLIER)
            .wrapping_add(RANDOM_INCREMENT)
            & RANDOM_MASK;
        (self.seed >> (48 - bits)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn legacy_random_gaussian_matches_java_samples() {
        let mut random = LegacyRandom::new(0);
        assert_close(random.next_gaussian(), 0.8025330637390305);
        assert_close(random.next_gaussian(), -0.9015460884175122);
        assert_close(random.next_gaussian(), 2.080920790428163);
    }

    #[test]
    fn count_zero_emits_single_spawn_with_offset_velocity() {
        let mut resolver = test_resolver(0);
        let batch = resolver.resolve_level_particles(&level_particles_packet(4, 0));

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.missing_definition_count, 0);
        assert_eq!(batch.unknown_particle_type_count, 0);
        let command = &batch.commands[0];
        assert_eq!(command.particle_type_id, 4);
        assert_eq!(command.particle_id, "minecraft:cloud");
        assert_eq!(
            command.sprite_ids,
            vec![
                "minecraft:generic_7".to_string(),
                "minecraft:generic_6".to_string(),
            ]
        );
        assert_eq!(command.position, [10.0, 64.5, -3.25]);
        assert_close(command.velocity[0], 0.15);
        assert_close(command.velocity[1], 0.30);
        assert_close(command.velocity[2], 0.45);
        assert!(command.override_limiter);
        assert!(command.always_show);
        assert_eq!(command.raw_options_len, 2);
        assert_eq!(command.initial_delay_ticks, 0);
    }

    #[test]
    fn lava_level_particle_command_carries_smoke_child_template() {
        let mut resolver = test_resolver(0);
        let batch =
            resolver.resolve_level_particles(&level_particles_packet(LAVA_PARTICLE_TYPE_ID, 0));

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_type_id, LAVA_PARTICLE_TYPE_ID);
        assert_eq!(command.particle_id, "minecraft:lava");
        assert_eq!(command.child_spawn_templates.len(), 1);
        let child = &command.child_spawn_templates[0];
        assert_eq!(child.particle_type_id, SMOKE_PARTICLE_TYPE_ID);
        assert_eq!(child.particle_id, "minecraft:smoke");
        assert_eq!(child.sprite_ids, vec!["minecraft:smoke_0".to_string()]);
    }

    #[test]
    fn spell_particle_options_decode_color_and_power_into_spawn_command() {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(EFFECT_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = spell_particle_options(0x0011_2233, 0.5);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_id, "minecraft:effect");
        assert_eq!(
            command.option_color,
            Some([
                0x11 as f32 / 255.0,
                0x22 as f32 / 255.0,
                0x33 as f32 / 255.0,
                1.0,
            ])
        );
        assert_eq!(command.option_power, Some(0.5));

        let mut instant_packet = level_particles_packet(INSTANT_EFFECT_PARTICLE_TYPE_ID, 0);
        instant_packet.particle.raw_options = spell_particle_options(0x00aa_bbcc, 1.25);
        let instant = resolver.resolve_level_particles(&instant_packet);
        assert_eq!(instant.len(), 1);
        assert_eq!(instant.commands[0].particle_id, "minecraft:instant_effect");
        assert_eq!(instant.commands[0].option_power, Some(1.25));
    }

    #[test]
    fn entity_effect_particle_options_decode_argb_color_into_spawn_command() {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(ENTITY_EFFECT_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = 0x8011_2233_u32.to_be_bytes().to_vec();

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_id, "minecraft:entity_effect");
        assert_eq!(
            command.option_color,
            Some([
                0x11 as f32 / 255.0,
                0x22 as f32 / 255.0,
                0x33 as f32 / 255.0,
                0x80 as f32 / 255.0,
            ])
        );
        assert_eq!(command.option_power, None);
    }

    #[test]
    fn flash_particle_options_decode_argb_color_into_spawn_command() {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(FLASH_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = 0x6612_3456_u32.to_be_bytes().to_vec();

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_id, "minecraft:flash");
        assert_eq!(
            command.option_color,
            Some([
                0x12 as f32 / 255.0,
                0x34 as f32 / 255.0,
                0x56 as f32 / 255.0,
                0x66 as f32 / 255.0,
            ])
        );
        assert_eq!(command.option_power, None);
    }

    #[test]
    fn dust_particle_options_decode_color_scale_and_transition_into_spawn_command() {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = dust_particle_options(0x0012_3456, 2.5);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_id, "minecraft:dust");
        assert_eq!(command.sprite_ids, vec!["minecraft:generic_0".to_string()]);
        assert_eq!(
            command.option_color,
            Some([
                0x12 as f32 / 255.0,
                0x34 as f32 / 255.0,
                0x56 as f32 / 255.0,
                1.0,
            ])
        );
        assert_eq!(command.option_scale, Some(2.5));
        assert_eq!(command.option_color_to, None);

        let mut transition_packet =
            level_particles_packet(DUST_COLOR_TRANSITION_PARTICLE_TYPE_ID, 0);
        transition_packet.particle.raw_options =
            dust_color_transition_options(0x0001_0203, 0x00a0_b0c0, 9.0);
        let transition = resolver.resolve_level_particles(&transition_packet);
        assert_eq!(transition.len(), 1);
        let transition_command = &transition.commands[0];
        assert_eq!(
            transition_command.particle_id,
            "minecraft:dust_color_transition"
        );
        assert_eq!(
            transition_command.option_color,
            Some([1.0 / 255.0, 2.0 / 255.0, 3.0 / 255.0, 1.0])
        );
        assert_eq!(
            transition_command.option_color_to,
            Some([160.0 / 255.0, 176.0 / 255.0, 192.0 / 255.0, 1.0])
        );
        assert_eq!(transition_command.option_scale, Some(4.0));
    }

    #[test]
    fn sculk_charge_particle_options_decode_roll_into_spawn_command() {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(SCULK_CHARGE_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = 0.75_f32.to_be_bytes().to_vec();

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_id, "minecraft:sculk_charge");
        assert_eq!(
            command.sprite_ids,
            vec!["minecraft:sculk_charge_0".to_string()]
        );
        assert_eq!(command.option_roll, Some(0.75));
        assert_eq!(command.option_color, None);
        assert_eq!(command.option_power, None);
    }

    #[test]
    fn trail_particle_options_decode_target_color_and_duration_into_spawn_command() {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(TRAIL_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = trail_particle_options([1.5, 65.25, -4.75], 0x0012_3456, 27);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_id, "minecraft:trail");
        assert_eq!(command.sprite_ids, vec!["minecraft:generic_0".to_string()]);
        assert_eq!(command.option_target, Some([1.5, 65.25, -4.75]));
        assert_eq!(
            command.option_color,
            Some([
                0x12 as f32 / 255.0,
                0x34 as f32 / 255.0,
                0x56 as f32 / 255.0,
                1.0,
            ])
        );
        assert_eq!(command.option_duration_ticks, Some(27));
        assert_eq!(command.option_power, None);
    }

    #[test]
    fn vibration_particle_options_decode_block_target_and_arrival_into_spawn_command() {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(VIBRATION_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = vibration_particle_block_options([1, 64, -2], 27);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_id, "minecraft:vibration");
        assert_eq!(command.sprite_ids, vec!["minecraft:vibration".to_string()]);
        assert_eq!(command.option_target, Some([1.5, 64.5, -1.5]));
        assert_eq!(command.option_duration_ticks, Some(27));
        assert_eq!(command.option_color, None);
        assert_eq!(command.option_power, None);
    }

    #[test]
    fn vibration_particle_options_keep_entity_source_unresolved_for_later_lookup() {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(VIBRATION_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = vibration_particle_entity_options(123, 0.75, 27);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_id, "minecraft:vibration");
        assert_eq!(command.option_target, None);
        assert_eq!(command.option_duration_ticks, Some(27));
    }

    #[test]
    fn level_particles_decodes_shriek_delay_option_into_spawn_command() {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(SHRIEK_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = vec![17];

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_type_id, SHRIEK_PARTICLE_TYPE_ID);
        assert_eq!(command.particle_id, "minecraft:shriek");
        assert_eq!(command.sprite_ids, vec!["minecraft:shriek_0".to_string()]);
        assert_eq!(command.raw_options_len, 1);
        assert_eq!(command.initial_delay_ticks, 17);
    }

    #[test]
    fn positive_count_emits_deterministic_gaussian_scatter() {
        let mut resolver = test_resolver(0);
        let batch = resolver.resolve_level_particles(&level_particles_packet(4, 2));

        assert_eq!(batch.len(), 2);
        let first = &batch.commands[0];
        assert_close(first.position[0], 10.080253306373904);
        assert_close(first.position[1], 64.3196907823165);
        assert_close(first.position[2], -2.625723762871551);
        assert_close(first.velocity[0], 1.1456561526547341);
        assert_close(first.velocity[1], 1.4768617993237692);
        assert_close(first.velocity[2], -2.525118388151014);
    }

    #[test]
    fn negative_count_emits_no_spawn_commands() {
        let mut resolver = test_resolver(0);
        let batch = resolver.resolve_level_particles(&level_particles_packet(4, -1));

        assert!(batch.is_empty());
    }

    #[test]
    fn missing_definition_records_diagnostic_without_spawn_commands() {
        let mut resolver = test_resolver(0);
        let batch = resolver.resolve_level_particles(&level_particles_packet(47, 1));

        assert!(batch.commands.is_empty());
        assert_eq!(batch.missing_definition_count, 1);
        assert_eq!(batch.unknown_particle_type_count, 0);
    }

    #[test]
    fn unknown_particle_type_records_diagnostic_without_spawn_commands() {
        let mut resolver = test_resolver(0);
        let batch = resolver.resolve_level_particles(&level_particles_packet(999, 1));

        assert!(batch.commands.is_empty());
        assert_eq!(batch.missing_definition_count, 0);
        assert_eq!(batch.unknown_particle_type_count, 1);
    }

    #[test]
    fn missing_sprite_records_diagnostic_without_dropping_spawn_command() {
        let mut resolver = test_resolver_with_cloud_textures(
            0,
            ClientParticleStatus::All,
            &["minecraft:generic_7", "minecraft:missing_particle"],
            &["generic_7"],
        );
        let batch = resolver.resolve_level_particles(&level_particles_packet(4, 1));

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.missing_definition_count, 0);
        assert_eq!(batch.missing_sprite_count, 1);
        assert_eq!(
            batch.commands[0].sprite_ids,
            vec![
                "minecraft:generic_7".to_string(),
                "minecraft:missing_particle".to_string(),
            ]
        );
    }

    #[test]
    fn level_particles_drop_non_override_spawns_beyond_vanilla_camera_distance() {
        let mut packet = level_particles_packet(4, 0);
        packet.override_limiter = false;
        packet.always_show = false;
        packet.position = Vec3d {
            x: 33.0,
            y: 0.0,
            z: 0.0,
        };
        let context = LevelParticleSpawnContext {
            camera_position: Some([0.0, 0.0, 0.0]),
        };
        let mut resolver = test_resolver(0);

        let batch = resolver.resolve_level_particles_with_context(&packet, context);

        assert!(batch.commands.is_empty());
        assert_eq!(batch.missing_definition_count, 0);
        assert_eq!(batch.missing_sprite_count, 0);
    }

    #[test]
    fn level_particles_override_limiter_bypasses_distance_and_particle_status() {
        let mut packet = level_particles_packet(4, 0);
        packet.override_limiter = true;
        packet.always_show = false;
        packet.position = Vec3d {
            x: 33.0,
            y: 0.0,
            z: 0.0,
        };
        let context = LevelParticleSpawnContext {
            camera_position: Some([0.0, 0.0, 0.0]),
        };
        let mut resolver = test_resolver_with_particle_status(0, ClientParticleStatus::Minimal);

        let batch = resolver.resolve_level_particles_with_context(&packet, context);

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.commands[0].position, [33.0, 0.0, 0.0]);
        assert!(batch.commands[0].override_limiter);
    }

    #[test]
    fn decreased_particle_status_uses_vanilla_next_int_three() {
        let mut packet = level_particles_packet(4, 0);
        packet.override_limiter = false;
        packet.always_show = false;
        packet.position = Vec3d::default();
        let context = LevelParticleSpawnContext {
            camera_position: Some([0.0, 0.0, 0.0]),
        };
        let mut dropping_resolver =
            test_resolver_with_particle_status(0, ClientParticleStatus::Decreased);
        let mut keeping_resolver =
            test_resolver_with_particle_status(2, ClientParticleStatus::Decreased);

        assert!(dropping_resolver
            .resolve_level_particles_with_context(&packet, context)
            .commands
            .is_empty());
        assert_eq!(
            keeping_resolver
                .resolve_level_particles_with_context(&packet, context)
                .len(),
            1
        );
    }

    #[test]
    fn minimal_particle_status_only_keeps_always_show_promoted_particles() {
        let mut packet = level_particles_packet(4, 0);
        packet.override_limiter = false;
        packet.always_show = false;
        packet.position = Vec3d::default();
        let context = LevelParticleSpawnContext {
            camera_position: Some([0.0, 0.0, 0.0]),
        };
        let mut plain_minimal =
            test_resolver_with_particle_status(0, ClientParticleStatus::Minimal);
        assert!(plain_minimal
            .resolve_level_particles_with_context(&packet, context)
            .commands
            .is_empty());

        packet.always_show = true;
        let mut promoted = test_resolver_with_particle_status(0, ClientParticleStatus::Minimal);
        assert_eq!(
            promoted
                .resolve_level_particles_with_context(&packet, context)
                .len(),
            1
        );

        let mut promoted_then_dropped =
            test_resolver_with_particle_status(42, ClientParticleStatus::Minimal);
        assert!(promoted_then_dropped
            .resolve_level_particles_with_context(&packet, context)
            .commands
            .is_empty());
    }

    #[test]
    fn level_event_particles_map_vanilla_simple_side_effects() {
        let resolver = test_resolver(0);

        let mut lava_random = LevelEventSoundRandomState::with_seed(0);
        let lava =
            resolver.resolve_level_event_particles(&level_event_packet(1501), &mut lava_random);
        assert_eq!(lava.len(), 8);
        assert_particle_command(
            &lava.commands[0],
            55,
            "minecraft:large_smoke",
            [10.730_967_787_376_657, 65.2, -2.759_463_584_328_514],
            [0.0, 0.0, 0.0],
            false,
        );

        let mut burnout_random = LevelEventSoundRandomState::with_seed(0);
        burnout_random.next_float();
        burnout_random.next_float();
        let burnout = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 1502,
                ..level_event_packet(1502)
            },
            &mut burnout_random,
        );
        assert_eq!(burnout.len(), 5);
        assert_particle_command(
            &burnout.commands[0],
            62,
            "minecraft:smoke",
            [
                10.344_321_849_402_891,
                64.582_450_455_210_06,
                -2.469_737_796_929_42,
            ],
            [0.0, 0.0, 0.0],
            false,
        );

        let mut frame_fill_random = LevelEventSoundRandomState::with_seed(0);
        let frame_fill = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 1503,
                ..level_event_packet(1503)
            },
            &mut frame_fill_random,
        );
        assert_eq!(frame_fill.len(), 16);
        assert_particle_command(
            &frame_fill.commands[0],
            62,
            "minecraft:smoke",
            [10.586_612_920_266_246, 64.8125, -2.597_298_844_123_192_6],
            [0.0, 0.0, 0.0],
            false,
        );

        let mut shriek_random = LevelEventSoundRandomState::with_seed(0);
        let shriek = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3007,
                ..level_event_packet(3007)
            },
            &mut shriek_random,
        );
        assert_eq!(shriek.len(), 10);
        assert_particle_command(
            &shriek.commands[0],
            SHRIEK_PARTICLE_TYPE_ID,
            "minecraft:shriek",
            [10.5, 64.5, -2.5],
            [0.0, 0.0, 0.0],
            false,
        );
        assert_eq!(shriek.commands[0].initial_delay_ticks, 0);
        assert_particle_command_with_delay(
            &shriek.commands[9],
            SHRIEK_PARTICLE_TYPE_ID,
            "minecraft:shriek",
            [10.5, 64.5, -2.5],
            [0.0, 0.0, 0.0],
            false,
            45,
        );

        let mut blaze_random = LevelEventSoundRandomState::with_seed(0);
        let blaze = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2004,
                ..level_event_packet(2004)
            },
            &mut blaze_random,
        );
        assert_eq!(blaze.len(), 40);
        assert_particle_command(
            &blaze.commands[0],
            62,
            "minecraft:smoke",
            [
                10.961_935_574_753_314,
                63.981_072_831_342_97,
                -2.225_165_149_299_783_7,
            ],
            [0.0, 0.0, 0.0],
            false,
        );
        assert_particle_command(
            &blaze.commands[1],
            32,
            "minecraft:flame",
            [
                10.961_935_574_753_314,
                63.981_072_831_342_97,
                -2.225_165_149_299_783_7,
            ],
            [0.0, 0.0, 0.0],
            false,
        );

        let mut dragon_breath_random = LevelEventSoundRandomState::with_seed(0);
        let dragon_breath = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2006,
                ..level_event_packet(2006)
            },
            &mut dragon_breath_random,
        );
        assert_eq!(dragon_breath.len(), 200);
        assert_particle_command(
            &dragon_breath.commands[0],
            8,
            "minecraft:dragon_breath",
            [10.143_172_562_122_345, 64.3, -3.254_934_978_485_107_6],
            [
                4.186_181_081_614_109,
                0.188_500_336_334_049_33,
                -7.453_970_007_633_87,
            ],
            false,
        );

        let mut ender_eye_random = LevelEventSoundRandomState::with_seed(0);
        let ender_eye = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2003,
                ..level_event_packet(2003)
            },
            &mut ender_eye_random,
        );
        assert_eq!(ender_eye.len(), 80);
        assert_eq!(
            ender_eye.commands[0].sprite_ids,
            vec![
                "minecraft:generic_0".to_string(),
                "minecraft:generic_1".to_string(),
                "minecraft:generic_2".to_string(),
                "minecraft:generic_3".to_string(),
                "minecraft:generic_4".to_string(),
                "minecraft:generic_5".to_string(),
                "minecraft:generic_6".to_string(),
                "minecraft:generic_7".to_string(),
            ]
        );
        assert_particle_command(
            &ender_eye.commands[0],
            60,
            "minecraft:portal",
            [15.5, 63.6, -2.5],
            [-5.0, 0.0, -0.0],
            false,
        );
        assert_particle_command(
            &ender_eye.commands[1],
            60,
            "minecraft:portal",
            [15.5, 63.6, -2.5],
            [-7.0, 0.0, -0.0],
            false,
        );
        assert_particle_command(
            &ender_eye.commands[20],
            60,
            "minecraft:portal",
            [10.5, 63.6, 2.5],
            [-0.0, 0.0, -5.0],
            false,
        );

        let mut explosion_random = LevelEventSoundRandomState::with_seed(0);
        let explosion = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2008,
                ..level_event_packet(2008)
            },
            &mut explosion_random,
        );
        assert_eq!(explosion.len(), 1);
        assert_particle_command(
            &explosion.commands[0],
            23,
            "minecraft:explosion",
            [10.5, 64.5, -2.5],
            [0.0, 0.0, 0.0],
            true,
        );

        let mut gateway_random = LevelEventSoundRandomState::with_seed(0);
        let gateway = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3000,
                ..level_event_packet(3000)
            },
            &mut gateway_random,
        );
        assert_eq!(gateway.len(), 1);
        assert_particle_command_with_visibility(
            &gateway.commands[0],
            22,
            "minecraft:explosion_emitter",
            [10.5, 64.5, -2.5],
            [0.0, 0.0, 0.0],
            true,
            true,
        );

        let mut electric_x_random = LevelEventSoundRandomState::with_seed(0);
        let electric_x = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3002,
                data: 0,
                ..level_event_packet(3002)
            },
            &mut electric_x_random,
        );
        assert_eq!(electric_x.len(), 10);
        assert_particle_command(
            &electric_x.commands[0],
            103,
            "minecraft:electric_spark",
            [
                10.831_440_988_787_062,
                64.526_586_303_999_34,
                -2.547_737_357_950_073,
            ],
            [-0.765_986_782_385_549_7, 0.0, 0.0],
            true,
        );

        let mut electric_z_random = LevelEventSoundRandomState::with_seed(0);
        let electric_z = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3002,
                data: 2,
                ..level_event_packet(3002)
            },
            &mut electric_z_random,
        );
        assert_eq!(electric_z.len(), 10);
        assert_particle_command(
            &electric_z.commands[0],
            103,
            "minecraft:electric_spark",
            [
                10.582_860_247_196_765,
                64.526_586_303_999_34,
                -2.690_949_431_800_291,
            ],
            [0.0, 0.0, -0.765_986_782_385_549_7],
            true,
        );

        let mut electric_fallback_random = LevelEventSoundRandomState::with_seed(0);
        let electric_fallback = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3002,
                data: 3,
                ..level_event_packet(3002)
            },
            &mut electric_fallback_random,
        );
        assert_eq!(electric_fallback.len(), 23);
        assert_particle_command(
            &electric_fallback.commands[0],
            103,
            "minecraft:electric_spark",
            [10.117_006_608_807_225, 63.95, -2.218_465_367_954_695_3],
            [0.331_440_988_787_061_2, 0.0, -0.190_949_431_800_290_78],
            true,
        );

        let mut wax_on_random = LevelEventSoundRandomState::with_seed(0);
        let wax_on = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3003,
                ..level_event_packet(3003)
            },
            &mut wax_on_random,
        );
        assert_eq!(wax_on.len(), 23);
        assert_particle_command(
            &wax_on.commands[0],
            101,
            "minecraft:wax_on",
            [10.117_006_608_807_225, 63.95, -2.218_465_367_954_695_3],
            [0.331_440_988_787_061_2, 0.0, -0.190_949_431_800_290_78],
            true,
        );

        let mut wax_off_random = LevelEventSoundRandomState::with_seed(0);
        let wax_off = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3004,
                ..level_event_packet(3004)
            },
            &mut wax_off_random,
        );
        assert_eq!(wax_off.len(), 23);
        assert_eq!(wax_off.commands[0].particle_type_id, 102);
        assert_eq!(wax_off.commands[0].particle_id, "minecraft:wax_off");

        let mut scrape_random = LevelEventSoundRandomState::with_seed(0);
        let scrape = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3005,
                ..level_event_packet(3005)
            },
            &mut scrape_random,
        );
        assert_eq!(scrape.len(), 23);
        assert_eq!(scrape.commands[0].particle_type_id, 104);
        assert_eq!(scrape.commands[0].particle_id, "minecraft:scrape");

        let mut egg_crack_random = LevelEventSoundRandomState::with_seed(0);
        let egg_crack = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3009,
                ..level_event_packet(3009)
            },
            &mut egg_crack_random,
        );
        assert_eq!(egg_crack.len(), 30);
        assert_eq!(egg_crack.commands[0].particle_type_id, 106);
        assert_eq!(egg_crack.commands[0].particle_id, "minecraft:egg_crack");
        assert!(!egg_crack.commands[0].override_limiter);

        let mut trial_spawn_random = LevelEventSoundRandomState::with_seed(0);
        let trial_spawn = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3011,
                data: 0,
                ..level_event_packet(3011)
            },
            &mut trial_spawn_random,
        );
        assert_eq!(trial_spawn.len(), 40);
        assert_particle_command(
            &trial_spawn.commands[0],
            62,
            "minecraft:smoke",
            [
                10.961_935_574_753_314,
                63.981_072_831_342_97,
                -2.225_165_149_299_783_7,
            ],
            [0.0, 0.0, 0.0],
            false,
        );
        assert_particle_command(
            &trial_spawn.commands[1],
            32,
            "minecraft:flame",
            [
                10.961_935_574_753_314,
                63.981_072_831_342_97,
                -2.225_165_149_299_783_7,
            ],
            [0.0, 0.0, 0.0],
            false,
        );

        let mut trial_spawn_ominous_random = LevelEventSoundRandomState::with_seed(0);
        let trial_spawn_ominous = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3011,
                data: 1,
                ..level_event_packet(3011)
            },
            &mut trial_spawn_ominous_random,
        );
        assert_eq!(trial_spawn_ominous.len(), 40);
        assert_eq!(trial_spawn_ominous.commands[1].particle_type_id, 40);
        assert_eq!(
            trial_spawn_ominous.commands[1].particle_id,
            "minecraft:soul_fire_flame"
        );

        let mut trial_spawn_mob_random = LevelEventSoundRandomState::with_seed(0);
        let trial_spawn_mob = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3012,
                data: 1,
                ..level_event_packet(3012)
            },
            &mut trial_spawn_mob_random,
        );
        assert_eq!(trial_spawn_mob.len(), 40);
        assert_eq!(trial_spawn_mob.commands[1].particle_type_id, 40);
        assert_eq!(
            trial_spawn_mob.commands[1].particle_id,
            "minecraft:soul_fire_flame"
        );

        let mut trial_detect_random = LevelEventSoundRandomState::with_seed(0);
        let trial_detect = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3013,
                data: 2,
                ..level_event_packet(3013)
            },
            &mut trial_detect_random,
        );
        assert_eq!(trial_detect.len(), 40);
        assert_particle_command(
            &trial_detect.commands[0],
            108,
            "minecraft:trial_spawner_detection",
            [
                10.800_258_088_111_878,
                64.292_429_113_388_05,
                -2.069_126_719_236_374,
            ],
            [0.0, 0.0, 0.0],
            true,
        );

        let mut trial_eject_random = LevelEventSoundRandomState::with_seed(0);
        let trial_eject = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3017,
                ..level_event_packet(3017)
            },
            &mut trial_eject_random,
        );
        assert_eq!(trial_eject.len(), 40);
        assert_particle_command(
            &trial_eject.commands[0],
            93,
            "minecraft:small_flame",
            [
                10.546_193_557_475_332,
                64.448_107_283_134_3,
                -2.472_516_514_929_978_4,
            ],
            [
                0.022_619_280_994_487_918,
                0.043_745_738_729_615_43,
                -0.007_831_529_827_929_628,
            ],
            false,
        );
        assert_particle_command(
            &trial_eject.commands[1],
            62,
            "minecraft:smoke",
            [
                10.546_193_557_475_332,
                64.448_107_283_134_3,
                -2.472_516_514_929_978_4,
            ],
            [
                0.022_619_280_994_487_918,
                0.043_745_738_729_615_43,
                -0.031_326_119_311_718_51,
            ],
            false,
        );

        let mut trial_eject_sound_event_random = LevelEventSoundRandomState::with_seed(0);
        let trial_eject_sound_event = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3014,
                ..level_event_packet(3014)
            },
            &mut trial_eject_sound_event_random,
        );
        assert_eq!(trial_eject_sound_event.len(), 40);
        assert_eq!(trial_eject_sound_event.commands[0].particle_type_id, 93);
        assert_eq!(trial_eject_sound_event.commands[1].particle_type_id, 62);

        let mut cobweb_poof_random = LevelEventSoundRandomState::with_seed(0);
        let cobweb_poof = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3018,
                ..level_event_packet(3018)
            },
            &mut cobweb_poof_random,
        );
        assert_eq!(cobweb_poof.len(), 10);
        assert_particle_command(
            &cobweb_poof.commands[0],
            59,
            "minecraft:poof",
            [
                10.597_545_277_797_202,
                64.333_218_399_476_65,
                -2.614_810_815_259_281_7,
            ],
            [
                0.016_050_661_274_780_612,
                -0.018_030_921_768_350_243,
                0.041_618_415_808_563_26,
            ],
            true,
        );

        let mut bee_growth_random = LevelEventSoundRandomState::with_seed(0);
        let bee_growth = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2011,
                data: 3,
                ..level_event_packet(2011)
            },
            &mut bee_growth_random,
        );
        assert_eq!(bee_growth.len(), 3);
        assert_particle_command(
            &bee_growth.commands[0],
            43,
            "minecraft:happy_villager",
            [
                10.597_545_277_797_202,
                64.333_218_399_476_65,
                -2.614_810_815_259_281_7,
            ],
            [
                0.016_050_661_274_780_612,
                -0.018_030_921_768_350_243,
                0.041_618_415_808_563_26,
            ],
            false,
        );

        let mut turtle_egg_placement_random = LevelEventSoundRandomState::with_seed(0);
        let turtle_egg_placement = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2012,
                data: 2,
                ..level_event_packet(2012)
            },
            &mut turtle_egg_placement_random,
        );
        assert_eq!(turtle_egg_placement.len(), 2);
        assert_eq!(turtle_egg_placement.commands[0].particle_type_id, 43);
        assert_eq!(
            turtle_egg_placement.commands[0].particle_id,
            "minecraft:happy_villager"
        );

        let mut zero_bee_growth_random = LevelEventSoundRandomState::with_seed(0);
        let zero_bee_growth = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2011,
                data: 0,
                ..level_event_packet(2011)
            },
            &mut zero_bee_growth_random,
        );
        assert!(zero_bee_growth.is_empty());

        let mut trial_detect_ominous_random = LevelEventSoundRandomState::with_seed(0);
        let trial_detect_ominous = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3019,
                data: 10,
                ..level_event_packet(3019)
            },
            &mut trial_detect_ominous_random,
        );
        assert_eq!(trial_detect_ominous.len(), 80);
        assert_eq!(trial_detect_ominous.commands[0].particle_type_id, 109);
        assert_eq!(
            trial_detect_ominous.commands[0].particle_id,
            "minecraft:trial_spawner_detection_ominous"
        );

        let mut trial_ominous_activate_random = LevelEventSoundRandomState::with_seed(0);
        let trial_ominous_activate = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3020,
                ..level_event_packet(3020)
            },
            &mut trial_ominous_activate_random,
        );
        assert_eq!(trial_ominous_activate.len(), 70);
        assert_eq!(trial_ominous_activate.commands[0].particle_type_id, 109);
        assert_particle_command(
            &trial_ominous_activate.commands[30],
            114,
            "minecraft:trial_omen",
            [
                11.208_974_334_084_582,
                63.519_346_994_601_946,
                -2.115_413_986_094_133_7,
            ],
            [
                0.019_195_505_076_083_332,
                0.015_047_723_904_287_527,
                -0.013_159_128_311_470_1,
            ],
            false,
        );
        assert_eq!(trial_ominous_activate.commands[31].particle_type_id, 40);
        assert_eq!(
            trial_ominous_activate.commands[31].particle_id,
            "minecraft:soul_fire_flame"
        );

        let mut trial_spawn_item_random = LevelEventSoundRandomState::with_seed(0);
        let trial_spawn_item = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3021,
                data: 0,
                ..level_event_packet(3021)
            },
            &mut trial_spawn_item_random,
        );
        assert_eq!(trial_spawn_item.len(), 40);
        assert_eq!(trial_spawn_item.commands[1].particle_type_id, 32);

        let mut cloud_random = LevelEventSoundRandomState::with_seed(0);
        let cloud = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2009,
                ..level_event_packet(2009)
            },
            &mut cloud_random,
        );
        assert_eq!(cloud.len(), 8);
        assert_particle_command(
            &cloud.commands[0],
            4,
            "minecraft:cloud",
            [10.730_967_787_376_657, 65.2, -2.759_463_584_328_514],
            [0.0, 0.0, 0.0],
            false,
        );

        let mut dispenser_random = LevelEventSoundRandomState::with_seed(0);
        let dispenser = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2000,
                data: 5,
                ..level_event_packet(2000)
            },
            &mut dispenser_random,
        );
        assert_eq!(dispenser.len(), 10);
        assert_particle_command(
            &dispenser.commands[0],
            62,
            "minecraft:smoke",
            [11.11, 64.5, -2.474_781_497_441_183],
            [
                0.166_039_302_804_156_55,
                -0.016_834_122_587_673_427,
                -0.000_272_902_629_078_872_87,
            ],
            false,
        );

        let mut white_smoke_random = LevelEventSoundRandomState::with_seed(0);
        let white_smoke = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2010,
                data: 2,
                ..level_event_packet(2010)
            },
            &mut white_smoke_random,
        );
        assert_eq!(white_smoke.len(), 10);
        assert_particle_command(
            &white_smoke.commands[0],
            63,
            "minecraft:white_smoke",
            [10.629_731_792_164_257, 64.5, -3.11],
            [
                0.009_845_745_328_825_128,
                -0.016_834_122_587_673_427,
                -0.156_466_460_104_410_3,
            ],
            false,
        );
    }

    #[test]
    fn direction_normal_from_3d_data_value_matches_vanilla_wrapping() {
        assert_eq!(direction_normal_from_3d_data_value(0), (0, -1, 0));
        assert_eq!(direction_normal_from_3d_data_value(1), (0, 1, 0));
        assert_eq!(direction_normal_from_3d_data_value(2), (0, 0, -1));
        assert_eq!(direction_normal_from_3d_data_value(3), (0, 0, 1));
        assert_eq!(direction_normal_from_3d_data_value(4), (-1, 0, 0));
        assert_eq!(direction_normal_from_3d_data_value(5), (1, 0, 0));
        assert_eq!(direction_normal_from_3d_data_value(7), (0, 1, 0));
        assert_eq!(direction_normal_from_3d_data_value(-1), (0, 1, 0));
    }

    #[test]
    fn particle_atlas_from_images_exports_renderer_uvs() {
        let image = SpriteImage::new(
            "minecraft:generic_0",
            2,
            2,
            vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        )
        .unwrap();

        let atlas = particle_atlas_from_images(vec![image]).unwrap();

        assert_eq!(atlas.width, 4);
        assert_eq!(atlas.height, 4);
        assert_eq!(atlas.rgba.len(), 4 * 4 * 4);
        assert_eq!(
            atlas.sprite_uvs,
            vec![ParticleSpriteUv {
                id: "minecraft:generic_0".to_string(),
                uv: ParticleUvRect {
                    min: [0.375, 0.375],
                    max: [0.625, 0.625],
                },
            }]
        );
    }

    fn test_resolver(seed: i64) -> ParticleCommandResolver {
        test_resolver_with_particle_status(seed, ClientParticleStatus::All)
    }

    fn test_resolver_with_particle_status(
        seed: i64,
        particle_status: ClientParticleStatus,
    ) -> ParticleCommandResolver {
        test_resolver_with_cloud_textures(
            seed,
            particle_status,
            &["minecraft:generic_7", "minecraft:generic_6"],
            &[
                "generic_7",
                "generic_6",
                "generic_0",
                "generic_1",
                "generic_2",
                "generic_3",
                "generic_4",
                "generic_5",
                "effect_0",
                "spell_0",
                "dragon_breath_0",
                "flash",
                "vibration",
                "sculk_charge_0",
                "flame",
                "soul_fire_flame",
                "explosion_emitter_0",
                "explosion_0",
                "smoke_0",
                "large_smoke_0",
                "lava",
                "white_smoke_0",
                "poof_0",
                "happy_villager_0",
                "small_flame",
                "electric_spark_0",
                "wax_on_0",
                "wax_off_0",
                "scrape_0",
                "shriek_0",
                "egg_crack_0",
                "trial_spawner_detection_0",
                "trial_spawner_detection_ominous_0",
                "trial_omen_0",
            ],
        )
    }

    fn test_resolver_with_cloud_textures(
        seed: i64,
        particle_status: ClientParticleStatus,
        cloud_textures: &[&str],
        particle_textures: &[&str],
    ) -> ParticleCommandResolver {
        let root = unique_temp_dir("particle-runtime");
        let assets_dir = assets_dir(&root);
        write_particle_atlas(&assets_dir);
        for texture in particle_textures {
            write_test_png(
                &assets_dir
                    .join("textures")
                    .join("particle")
                    .join(format!("{texture}.png")),
                8,
                8,
            );
        }
        write_json(
            &particle_dir(&root).join("cloud.json"),
            &particle_definition_json(cloud_textures),
        );
        write_json(
            &particle_dir(&root).join("flame.json"),
            r#"{
              "textures": [
                "minecraft:flame"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("effect.json"),
            r#"{
              "textures": [
                "minecraft:effect_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("entity_effect.json"),
            r#"{
              "textures": [
                "minecraft:effect_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("instant_effect.json"),
            r#"{
              "textures": [
                "minecraft:spell_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("dragon_breath.json"),
            r#"{
              "textures": [
                "minecraft:dragon_breath_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("flash.json"),
            r#"{
              "textures": [
                "minecraft:flash"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("vibration.json"),
            r#"{
              "textures": [
                "minecraft:vibration"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("dust.json"),
            r#"{
              "textures": [
                "minecraft:generic_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("dust_color_transition.json"),
            r#"{
              "textures": [
                "minecraft:generic_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("trail.json"),
            r#"{
              "textures": [
                "minecraft:generic_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("sculk_charge.json"),
            r#"{
              "textures": [
                "minecraft:sculk_charge_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("soul_fire_flame.json"),
            r#"{
              "textures": [
                "minecraft:soul_fire_flame"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("explosion.json"),
            r#"{
              "textures": [
                "minecraft:explosion_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("explosion_emitter.json"),
            r#"{
              "textures": [
                "minecraft:explosion_emitter_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("smoke.json"),
            r#"{
              "textures": [
                "minecraft:smoke_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("large_smoke.json"),
            r#"{
              "textures": [
                "minecraft:large_smoke_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("lava.json"),
            r#"{
              "textures": [
                "minecraft:lava"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("white_smoke.json"),
            r#"{
              "textures": [
                "minecraft:white_smoke_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("poof.json"),
            r#"{
              "textures": [
                "minecraft:poof_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("portal.json"),
            r#"{
              "textures": [
                "minecraft:generic_0",
                "minecraft:generic_1",
                "minecraft:generic_2",
                "minecraft:generic_3",
                "minecraft:generic_4",
                "minecraft:generic_5",
                "minecraft:generic_6",
                "minecraft:generic_7"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("happy_villager.json"),
            r#"{
              "textures": [
                "minecraft:happy_villager_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("small_flame.json"),
            r#"{
              "textures": [
                "minecraft:small_flame"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("electric_spark.json"),
            r#"{
              "textures": [
                "minecraft:electric_spark_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("wax_on.json"),
            r#"{
              "textures": [
                "minecraft:wax_on_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("wax_off.json"),
            r#"{
              "textures": [
                "minecraft:wax_off_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("scrape.json"),
            r#"{
              "textures": [
                "minecraft:scrape_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("shriek.json"),
            r#"{
              "textures": [
                "minecraft:shriek_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("egg_crack.json"),
            r#"{
              "textures": [
                "minecraft:egg_crack_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("trial_spawner_detection.json"),
            r#"{
              "textures": [
                "minecraft:trial_spawner_detection_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("trial_spawner_detection_ominous.json"),
            r#"{
              "textures": [
                "minecraft:trial_spawner_detection_ominous_0"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("trial_omen.json"),
            r#"{
              "textures": [
                "minecraft:trial_omen_0"
              ]
            }"#,
        );

        let catalog = PackRoots::from_root(&root)
            .unwrap()
            .load_particle_definition_catalog()
            .unwrap();
        let sprites = PackRoots::from_root(&root)
            .unwrap()
            .load_particle_sprite_catalog()
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
        ParticleCommandResolver::with_seed_and_particle_status(
            catalog,
            sprites,
            seed,
            particle_status,
        )
    }

    fn level_particles_packet(particle_type_id: i32, count: i32) -> LevelParticles {
        LevelParticles {
            override_limiter: true,
            always_show: true,
            position: Vec3d {
                x: 10.0,
                y: 64.5,
                z: -3.25,
            },
            offset: Vec3d {
                x: 0.1,
                y: 0.2,
                z: 0.3,
            },
            max_speed: 1.5,
            count,
            particle: bbb_protocol::packets::ParticlePayload {
                particle_type_id,
                raw_options: vec![0xaa, 0xbb],
            },
        }
    }

    fn spell_particle_options(color: i32, power: f32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&color.to_be_bytes());
        out.extend_from_slice(&power.to_be_bytes());
        out
    }

    fn dust_particle_options(color: i32, scale: f32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&color.to_be_bytes());
        out.extend_from_slice(&scale.to_be_bytes());
        out
    }

    fn dust_color_transition_options(from_color: i32, to_color: i32, scale: f32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&from_color.to_be_bytes());
        out.extend_from_slice(&to_color.to_be_bytes());
        out.extend_from_slice(&scale.to_be_bytes());
        out
    }

    fn trail_particle_options(target: [f64; 3], color: i32, duration: i32) -> Vec<u8> {
        let mut out = Vec::new();
        for coordinate in target {
            out.extend_from_slice(&coordinate.to_be_bytes());
        }
        out.extend_from_slice(&color.to_be_bytes());
        write_positive_var_i32(&mut out, duration);
        out
    }

    fn vibration_particle_block_options(pos: [i32; 3], arrival_ticks: i32) -> Vec<u8> {
        let mut out = Vec::new();
        write_positive_var_i32(&mut out, 0);
        out.extend_from_slice(&encode_test_block_pos(pos).to_be_bytes());
        write_positive_var_i32(&mut out, arrival_ticks);
        out
    }

    fn vibration_particle_entity_options(
        entity_id: i32,
        y_offset: f32,
        arrival_ticks: i32,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        write_positive_var_i32(&mut out, 1);
        write_positive_var_i32(&mut out, entity_id);
        out.extend_from_slice(&y_offset.to_be_bytes());
        write_positive_var_i32(&mut out, arrival_ticks);
        out
    }

    fn encode_test_block_pos(pos: [i32; 3]) -> i64 {
        (((pos[0] as i64) & 0x3ffffff) << 38)
            | (((pos[2] as i64) & 0x3ffffff) << 12)
            | ((pos[1] as i64) & 0xfff)
    }

    fn write_positive_var_i32(out: &mut Vec<u8>, value: i32) {
        let mut value = value as u32;
        loop {
            if value & !0x7f == 0 {
                out.push(value as u8);
                return;
            }
            out.push(((value & 0x7f) | 0x80) as u8);
            value >>= 7;
        }
    }

    fn level_event_packet(event_type: i32) -> LevelEvent {
        LevelEvent {
            event_type,
            pos: bbb_protocol::packets::BlockPos {
                x: 10,
                y: 64,
                z: -3,
            },
            data: 0,
            global: false,
        }
    }

    fn assert_particle_command(
        command: &ParticleSpawnCommand,
        particle_type_id: i32,
        particle_id: &str,
        position: [f64; 3],
        velocity: [f64; 3],
        override_limiter: bool,
    ) {
        assert_particle_command_with_visibility_and_delay(
            command,
            particle_type_id,
            particle_id,
            position,
            velocity,
            override_limiter,
            false,
            0,
        );
    }

    fn assert_particle_command_with_delay(
        command: &ParticleSpawnCommand,
        particle_type_id: i32,
        particle_id: &str,
        position: [f64; 3],
        velocity: [f64; 3],
        override_limiter: bool,
        initial_delay_ticks: u32,
    ) {
        assert_particle_command_with_visibility_and_delay(
            command,
            particle_type_id,
            particle_id,
            position,
            velocity,
            override_limiter,
            false,
            initial_delay_ticks,
        );
    }

    fn assert_particle_command_with_visibility(
        command: &ParticleSpawnCommand,
        particle_type_id: i32,
        particle_id: &str,
        position: [f64; 3],
        velocity: [f64; 3],
        override_limiter: bool,
        always_show: bool,
    ) {
        assert_particle_command_with_visibility_and_delay(
            command,
            particle_type_id,
            particle_id,
            position,
            velocity,
            override_limiter,
            always_show,
            0,
        );
    }

    fn assert_particle_command_with_visibility_and_delay(
        command: &ParticleSpawnCommand,
        particle_type_id: i32,
        particle_id: &str,
        position: [f64; 3],
        velocity: [f64; 3],
        override_limiter: bool,
        always_show: bool,
        initial_delay_ticks: u32,
    ) {
        assert_eq!(command.particle_type_id, particle_type_id);
        assert_eq!(command.particle_id, particle_id);
        for (actual, expected) in command.position.iter().zip(position) {
            assert_close(*actual, expected);
        }
        for (actual, expected) in command.velocity.iter().zip(velocity) {
            assert_close(*actual, expected);
        }
        assert_eq!(command.override_limiter, override_limiter);
        assert_eq!(command.always_show, always_show);
        assert_eq!(command.raw_options_len, 0);
        assert_eq!(command.initial_delay_ticks, initial_delay_ticks);
    }

    fn particle_dir(root: &Path) -> PathBuf {
        assets_dir(root).join("particles")
    }

    fn assets_dir(root: &Path) -> PathBuf {
        root.join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("assets")
            .join("minecraft")
    }

    fn particle_definition_json(textures: &[&str]) -> String {
        let textures = textures
            .iter()
            .map(|texture| format!("\"{texture}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!(r#"{{ "textures": [{textures}] }}"#)
    }

    fn write_particle_atlas(assets_dir: &Path) {
        write_json(
            &assets_dir.join("atlases").join("particles.json"),
            r#"{
              "sources": [
                {
                  "type": "minecraft:directory",
                  "prefix": "",
                  "source": "particle"
                }
              ]
            }"#,
        );
    }

    fn write_json(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn write_test_png(path: &Path, width: u32, height: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut image = image::RgbaImage::new(width, height);
        for (index, pixel) in image.pixels_mut().enumerate() {
            let shade = (index % 255) as u8;
            *pixel = image::Rgba([shade, 255 - shade, 64, 255]);
        }
        image.save(path).unwrap();
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("bbb-native-{label}-{nanos}-{id}"))
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }
}
