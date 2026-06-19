use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use bbb_pack::{
    AtlasLayout, AtlasPacker, AtlasSprite, PackRoots, ParticleDefinitionCatalog,
    ParticleSpriteCatalog, SpriteImage,
};
use bbb_protocol::packets::{LevelEvent, LevelParticles, Vec3d};
use bbb_renderer::{
    ParticleSpawnBatch, ParticleSpawnCommand, ParticleSpriteUv, ParticleUvRect, Renderer,
};
use bbb_world::LevelEventSoundRandomState;

use crate::particle_registry::{vanilla_particle_type, ParticleTypeInfo};

pub(crate) trait ParticleEventSink {
    fn spawn_level_particles(&mut self, packet: &LevelParticles) -> ParticleSpawnBatch;
    fn spawn_level_event_particles(
        &mut self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch;
}

pub(crate) struct NativeParticleRuntime {
    resolver: ParticleCommandResolver,
    atlas: NativeParticleAtlas,
}

impl NativeParticleRuntime {
    pub(crate) fn load(roots: &PackRoots) -> Result<Self> {
        let definitions = roots
            .load_particle_definition_catalog()
            .context("load particle definition catalog")?;
        let sprites = roots
            .load_particle_sprite_catalog()
            .context("load particle sprite catalog")?;
        let atlas = particle_atlas_from_images(sprites.sprites().values().cloned().collect())
            .context("stitch particle atlas")?;
        Ok(Self {
            resolver: ParticleCommandResolver::new(definitions, sprites),
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
    fn spawn_level_particles(&mut self, packet: &LevelParticles) -> ParticleSpawnBatch {
        self.resolver.resolve_level_particles(packet)
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
    fn new(definitions: ParticleDefinitionCatalog, sprites: ParticleSpriteCatalog) -> Self {
        Self {
            definitions,
            sprites,
            random: LegacyRandom::new(default_particle_seed()),
        }
    }

    #[cfg(test)]
    fn with_seed(
        definitions: ParticleDefinitionCatalog,
        sprites: ParticleSpriteCatalog,
        seed: i64,
    ) -> Self {
        Self {
            definitions,
            sprites,
            random: LegacyRandom::new(seed),
        }
    }

    fn resolve_level_particles(&mut self, packet: &LevelParticles) -> ParticleSpawnBatch {
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
        let command_count = if packet.count == 0 {
            1
        } else {
            packet.count as usize
        };
        let mut commands = Vec::with_capacity(command_count);

        if packet.count == 0 {
            commands.push(self.command(
                packet,
                particle_type,
                &sprite_ids,
                packet.position,
                Vec3d {
                    x: packet.offset.x * f64::from(packet.max_speed),
                    y: packet.offset.y * f64::from(packet.max_speed),
                    z: packet.offset.z * f64::from(packet.max_speed),
                },
                override_limiter,
                raw_options_len,
            ));
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
                commands.push(self.command(
                    packet,
                    particle_type,
                    &sprite_ids,
                    position,
                    velocity,
                    override_limiter,
                    raw_options_len,
                ));
            }
        }

        ParticleSpawnBatch {
            commands,
            missing_sprite_count,
            ..ParticleSpawnBatch::default()
        }
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
    ) -> ParticleSpawnCommand {
        self.command_for_type(
            particle_type,
            sprite_ids,
            position,
            velocity,
            override_limiter,
            packet.always_show,
            raw_options_len,
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
    ) -> ParticleSpawnCommand {
        ParticleSpawnCommand {
            particle_type_id: particle_type.id,
            particle_id: particle_type.name.to_string(),
            sprite_ids: sprite_ids.to_vec(),
            position: [position.x, position.y, position.z],
            velocity: [velocity.x, velocity.y, velocity.z],
            override_limiter,
            always_show,
            raw_options_len,
        }
    }
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
const BLAZE_SMOKE_LEVEL_EVENT: i32 = 2004;
const EXPLOSION_LEVEL_EVENT: i32 = 2008;
const SPLASH_CLOUD_LEVEL_EVENT: i32 = 2009;
const DISPENSER_WHITE_SMOKE_LEVEL_EVENT: i32 = 2010;
const END_GATEWAY_SPAWN_LEVEL_EVENT: i32 = 3000;
const ELECTRIC_SPARK_LEVEL_EVENT: i32 = 3002;
const WAX_ON_LEVEL_EVENT: i32 = 3003;
const WAX_OFF_LEVEL_EVENT: i32 = 3004;
const SCRAPE_LEVEL_EVENT: i32 = 3005;
const EGG_CRACK_LEVEL_EVENT: i32 = 3009;
const CLOUD_PARTICLE_TYPE_ID: i32 = 4;
const EXPLOSION_EMITTER_PARTICLE_TYPE_ID: i32 = 22;
const EXPLOSION_PARTICLE_TYPE_ID: i32 = 23;
const FLAME_PARTICLE_TYPE_ID: i32 = 32;
const LARGE_SMOKE_PARTICLE_TYPE_ID: i32 = 55;
const SMOKE_PARTICLE_TYPE_ID: i32 = 62;
const WHITE_SMOKE_PARTICLE_TYPE_ID: i32 = 63;
const ELECTRIC_SPARK_PARTICLE_TYPE_ID: i32 = 103;
const WAX_ON_PARTICLE_TYPE_ID: i32 = 101;
const WAX_OFF_PARTICLE_TYPE_ID: i32 = 102;
const SCRAPE_PARTICLE_TYPE_ID: i32 = 104;
const EGG_CRACK_PARTICLE_TYPE_ID: i32 = 106;
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
        test_resolver_with_cloud_textures(
            seed,
            &["minecraft:generic_7", "minecraft:generic_6"],
            &[
                "generic_7",
                "generic_6",
                "flame",
                "explosion_emitter_0",
                "explosion_0",
                "smoke_0",
                "large_smoke_0",
                "white_smoke_0",
                "electric_spark_0",
                "wax_on_0",
                "wax_off_0",
                "scrape_0",
                "egg_crack_0",
            ],
        )
    }

    fn test_resolver_with_cloud_textures(
        seed: i64,
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
            &particle_dir(&root).join("white_smoke.json"),
            r#"{
              "textures": [
                "minecraft:white_smoke_0"
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
            &particle_dir(&root).join("egg_crack.json"),
            r#"{
              "textures": [
                "minecraft:egg_crack_0"
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
        ParticleCommandResolver::with_seed(catalog, sprites, seed)
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
        assert_particle_command_with_visibility(
            command,
            particle_type_id,
            particle_id,
            position,
            velocity,
            override_limiter,
            false,
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
