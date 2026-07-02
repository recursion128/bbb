use std::{
    collections::HashMap,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use bbb_item_model::NativeItemRuntime;
use bbb_pack::{
    AtlasLayout, AtlasPacker, AtlasSprite, PackRoots, ParticleDefinitionCatalog,
    ParticleSpriteCatalog, SpriteImage,
};
use bbb_protocol::codec::Decoder;
use bbb_protocol::packets::{BlockPos, ClientParticleStatus, LevelEvent, LevelParticles, Vec3d};
use bbb_renderer::{
    ParticleBlockOptionState, ParticleChildSpawnTemplate, ParticleItemOptionState,
    ParticleSpawnBatch, ParticleSpawnCommand, ParticleSpriteUv, ParticleUvRect, Renderer,
};
use bbb_world::{
    block_name_has_invisible_render_shape, block_name_is_air,
    block_name_should_spawn_terrain_particles, LevelEventSoundRandomState,
};

use crate::{
    particle_registry::{vanilla_particle_type, ParticleTypeInfo},
    terrain_runtime::TerrainTextureState,
};

const PARTICLE_TEXTURE_ANIMATION_INTERVAL: Duration = Duration::from_millis(50);

pub(crate) trait ParticleEventSink {
    fn maybe_upload_particle_atlas_animation(&mut self, _renderer: &mut Renderer) {}

    fn spawn_level_particles(
        &mut self,
        packet: &LevelParticles,
        context: LevelParticleSpawnContext,
    ) -> ParticleSpawnBatch;
    fn spawn_level_event_particles(
        &mut self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch;
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct LevelParticleSpawnContext {
    pub(crate) camera_position: Option<[f64; 3]>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct LevelEventParticleContext {
    pub(crate) sculk_charge_pop_full_block: Option<bool>,
    pub(crate) block_state_id_at_event_pos: Option<i32>,
    pub(crate) vault_block_entity_at_event_pos: bool,
    pub(crate) dripstone_drip_particle: Option<LevelEventDripstoneDripParticle>,
    pub(crate) growth_particles: Option<LevelEventGrowthParticleContext>,
    pub(crate) in_block_particle_spread_height: Option<f64>,
    pub(crate) composter_fill_center_shape_max_y: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LevelEventDripstoneDripParticle {
    Water,
    Lava,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LevelEventGrowthParticleContext {
    pub(crate) pos: BlockPos,
    pub(crate) mode: LevelEventGrowthParticleMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LevelEventGrowthParticleMode {
    InBlock {
        spread_height: f64,
    },
    WideNoFloating {
        support: LevelEventGrowthParticleSupport,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LevelEventGrowthParticleSupport {
    bits: u64,
}

impl LevelEventGrowthParticleSupport {
    pub(crate) fn empty() -> Self {
        Self { bits: 0 }
    }

    #[cfg(test)]
    pub(crate) fn full() -> Self {
        let mut support = Self::empty();
        for dx in -GROWTH_PARTICLE_SUPPORT_RADIUS..=GROWTH_PARTICLE_SUPPORT_RADIUS {
            for dz in -GROWTH_PARTICLE_SUPPORT_RADIUS..=GROWTH_PARTICLE_SUPPORT_RADIUS {
                support.insert(dx, dz);
            }
        }
        support
    }

    pub(crate) fn insert(&mut self, dx: i32, dz: i32) {
        if let Some(bit) = growth_particle_support_bit(dx, dz) {
            self.bits |= bit;
        }
    }

    fn contains(self, dx: i32, dz: i32) -> bool {
        growth_particle_support_bit(dx, dz).is_some_and(|bit| self.bits & bit != 0)
    }
}

pub(crate) struct NativeParticleRuntime {
    resolver: ParticleCommandResolver,
    atlas: NativeParticleAtlas,
    texture_animation_tick: u64,
    last_texture_animation_at: Option<Instant>,
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
            texture_animation_tick: 0,
            last_texture_animation_at: None,
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

    pub(crate) fn set_terrain_particle_sprite_ids(&mut self, textures: &TerrainTextureState) {
        self.resolver.set_terrain_particle_sprite_ids(textures);
    }

    pub(crate) fn set_default_item_particle_sprite_ids(&mut self, items: &NativeItemRuntime) {
        self.resolver.set_default_item_particle_sprite_ids(items);
    }

    pub(crate) fn maybe_upload_particle_atlas_animation(&mut self, renderer: &mut Renderer) {
        if !self.atlas.has_animation() {
            return;
        }
        let Some(tick) = advance_particle_texture_animation_tick(self, Instant::now()) else {
            return;
        };
        match self.atlas.animation_atlas_frame(tick) {
            Ok(Some(frame)) => {
                if frame.width != self.atlas.width || frame.height != self.atlas.height {
                    tracing::warn!(
                        width = frame.width,
                        height = frame.height,
                        atlas_width = self.atlas.width,
                        atlas_height = self.atlas.height,
                        "animated particle atlas frame dimensions changed"
                    );
                    return;
                }
                if let Err(err) = renderer.update_particle_atlas(&frame.rgba) {
                    tracing::warn!(?err, "failed to update animated particle texture atlas");
                }
            }
            Ok(None) => {}
            Err(err) => {
                tracing::warn!(
                    ?err,
                    "failed to stitch animated particle texture atlas frame"
                );
            }
        }
    }
}

impl ParticleEventSink for NativeParticleRuntime {
    fn maybe_upload_particle_atlas_animation(&mut self, renderer: &mut Renderer) {
        NativeParticleRuntime::maybe_upload_particle_atlas_animation(self, renderer);
    }

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
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        self.resolver
            .resolve_level_event_particles_with_context(event, context, random)
    }
}

#[derive(Debug, Clone)]
struct ParticleCommandResolver {
    definitions: ParticleDefinitionCatalog,
    sprites: ParticleSpriteCatalog,
    terrain_particle_sprite_ids: HashMap<i32, String>,
    terrain_particle_tint_colors: HashMap<i32, [f32; 4]>,
    falling_dust_block_tint_colors: HashMap<i32, [f32; 4]>,
    default_item_particle_sprite_ids: HashMap<i32, Vec<String>>,
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
    animation: Option<NativeParticleAtlasAnimation>,
}

impl NativeParticleAtlas {
    fn has_animation(&self) -> bool {
        self.animation.is_some()
    }

    fn animation_atlas_frame(&self, tick: u64) -> Result<Option<NativeParticleAtlasFrame>> {
        self.animation
            .as_ref()
            .map(|animation| animation.atlas_frame(tick))
            .transpose()
    }
}

#[derive(Debug, Clone)]
struct NativeParticleAtlasAnimation {
    packer: AtlasPacker,
    images: Vec<SpriteImage>,
}

impl NativeParticleAtlasAnimation {
    fn new(packer: AtlasPacker, images: Vec<SpriteImage>) -> Option<Self> {
        images
            .iter()
            .any(|image| image.animation.is_some())
            .then_some(Self { packer, images })
    }

    fn atlas_frame(&self, tick: u64) -> Result<NativeParticleAtlasFrame> {
        let atlas = self.packer.stitch_animation_frame(&self.images, tick)?;
        Ok(NativeParticleAtlasFrame {
            width: atlas.layout.width,
            height: atlas.layout.height,
            rgba: atlas.rgba,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeParticleAtlasFrame {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn particle_atlas_from_images(images: Vec<SpriteImage>) -> Result<NativeParticleAtlas> {
    let packer = AtlasPacker::new(4096, 1)?;
    let atlas = packer.stitch(&images)?;
    let sprite_uvs = atlas
        .layout
        .sprites
        .iter()
        .map(|sprite| ParticleSpriteUv {
            id: sprite.id.clone(),
            uv: particle_uv_rect(&atlas.layout, sprite),
            has_translucent: sprite.transparency.has_translucent,
        })
        .collect();
    Ok(NativeParticleAtlas {
        width: atlas.layout.width,
        height: atlas.layout.height,
        rgba: atlas.rgba,
        sprite_uvs,
        animation: NativeParticleAtlasAnimation::new(packer, images),
    })
}

fn advance_particle_texture_animation_tick(
    runtime: &mut NativeParticleRuntime,
    now: Instant,
) -> Option<u64> {
    let Some(last) = runtime.last_texture_animation_at else {
        runtime.last_texture_animation_at = Some(now);
        return None;
    };
    let elapsed = now.saturating_duration_since(last);
    let ticks = elapsed.as_millis() / PARTICLE_TEXTURE_ANIMATION_INTERVAL.as_millis();
    if ticks == 0 {
        return None;
    }

    let ticks = u64::try_from(ticks).unwrap_or(u64::MAX);
    runtime.texture_animation_tick = runtime.texture_animation_tick.saturating_add(ticks);
    let advanced = Duration::from_millis(
        ticks.saturating_mul(PARTICLE_TEXTURE_ANIMATION_INTERVAL.as_millis() as u64),
    );
    runtime.last_texture_animation_at = last.checked_add(advanced).or(Some(now));
    Some(runtime.texture_animation_tick)
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
            terrain_particle_sprite_ids: HashMap::new(),
            terrain_particle_tint_colors: HashMap::new(),
            falling_dust_block_tint_colors: HashMap::new(),
            default_item_particle_sprite_ids: HashMap::new(),
            random: LegacyRandom::new(default_particle_seed()),
            particle_level_random: LegacyRandom::new(default_particle_seed()),
            particle_status,
        }
    }

    fn set_terrain_particle_sprite_ids(&mut self, textures: &TerrainTextureState) {
        let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
        self.terrain_particle_sprite_ids = block_states
            .iter()
            .filter_map(|block_state| {
                textures
                    .particle_sprite_id_for_block_state(block_state.id)
                    .map(|sprite_id| (block_state.id, sprite_id))
            })
            .collect();
        self.terrain_particle_tint_colors = block_states
            .iter()
            .filter_map(|block_state| {
                textures
                    .terrain_particle_tint_color_for_block_state(block_state.id)
                    .map(|color| (block_state.id, color))
            })
            .collect();
        self.falling_dust_block_tint_colors = block_states
            .iter()
            .filter_map(|block_state| {
                textures
                    .falling_dust_block_tint_color_for_block_state(block_state.id)
                    .map(|color| (block_state.id, color))
            })
            .collect();
    }

    fn set_default_item_particle_sprite_ids(&mut self, items: &NativeItemRuntime) {
        self.default_item_particle_sprite_ids = items
            .default_item_particle_sprite_ids_by_protocol_id()
            .into_iter()
            .collect();
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
            terrain_particle_sprite_ids: HashMap::new(),
            terrain_particle_tint_colors: HashMap::new(),
            falling_dust_block_tint_colors: HashMap::new(),
            default_item_particle_sprite_ids: HashMap::new(),
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
        let (sprite_ids, missing_sprite_count) =
            if let Some(definition) = self.definitions.definition(particle_type.name) {
                let sprite_ids = definition.textures.clone();
                let missing_sprite_count = sprite_ids
                    .iter()
                    .filter(|sprite_id| self.sprites.sprite(sprite_id).is_none())
                    .count();
                (sprite_ids, missing_sprite_count)
            } else if definitionless_particle_type(particle_type.id) {
                (Vec::new(), 0)
            } else {
                return ParticleSpawnBatch {
                    missing_definition_count: 1,
                    ..ParticleSpawnBatch::default()
                };
            };
        let override_limiter = particle_type.override_limiter || packet.override_limiter;
        let raw_options_len = packet.particle.raw_options.len();
        let option_state =
            particle_option_render_state(particle_type.id, &packet.particle.raw_options);
        let provider_accepts_spawn =
            particle_provider_accepts_spawn(particle_type.id, option_state);
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
            ) && provider_accepts_spawn
            {
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
                ) && provider_accepts_spawn
                {
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
        self.resolve_level_event_particles_with_context(
            event,
            LevelEventParticleContext::default(),
            random,
        )
    }

    fn resolve_level_event_particles_with_context(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        match event.event_type {
            COMPOSTER_FILL_LEVEL_EVENT => {
                self.composter_fill_particle_batch(event, context, random)
            }
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
            DRIPSTONE_DRIP_LEVEL_EVENT => self.dripstone_drip_particle_batch(event, context),
            PLANT_GROWTH_LEVEL_EVENT => self.growth_particle_batch(event, context, random),
            DISPENSER_SMOKE_LEVEL_EVENT => {
                self.shoot_particles(event, SMOKE_PARTICLE_TYPE_ID, random)
            }
            DESTROY_BLOCK_PARTICLES_LEVEL_EVENT | BRUSH_BLOCK_COMPLETE_LEVEL_EVENT => {
                self.destroy_block_particle_batch(event)
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
            POTION_BREAK_LEVEL_EVENT | INSTANT_POTION_BREAK_LEVEL_EVENT => {
                self.potion_break_spell_particle_batch(event, random)
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
            SCULK_CHARGE_LEVEL_EVENT => self.sculk_charge_particle_batch(event, context, random),
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
            VAULT_ACTIVATE_LEVEL_EVENT => {
                self.vault_activation_particle_batch(event, context, random)
            }
            VAULT_DEACTIVATE_LEVEL_EVENT => self.vault_deactivation_particle_batch(event, random),
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
                    context.in_block_particle_spread_height.unwrap_or(1.0),
                    random,
                ),
            SMASH_ATTACK_PARTICLES_LEVEL_EVENT => {
                self.smash_attack_particle_batch(event, context, random)
            }
            DISPENSER_WHITE_SMOKE_LEVEL_EVENT => {
                self.shoot_particles(event, WHITE_SMOKE_PARTICLE_TYPE_ID, random)
            }
            _ => ParticleSpawnBatch::default(),
        }
    }

    fn composter_fill_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(COMPOSTER_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let center_height = context.composter_fill_center_shape_max_y.unwrap_or(1.0)
            + COMPOSTER_FILL_CENTER_HEIGHT_OFFSET;
        let mut batch = ParticleSpawnBatch {
            commands: Vec::with_capacity(COMPOSTER_FILL_PARTICLE_COUNT),
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..COMPOSTER_FILL_PARTICLE_COUNT {
            let velocity = Vec3d {
                x: random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
                y: random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
                z: random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
            };
            let position = Vec3d {
                x: f64::from(event.pos.x)
                    + COMPOSTER_FILL_SIDE_OFFSET
                    + COMPOSTER_FILL_WIDTH * f64::from(random.next_float()),
                y: f64::from(event.pos.y)
                    + center_height
                    + f64::from(random.next_float()) * (1.0 - center_height),
                z: f64::from(event.pos.z)
                    + COMPOSTER_FILL_SIDE_OFFSET
                    + COMPOSTER_FILL_WIDTH * f64::from(random.next_float()),
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }

        batch
    }

    fn destroy_block_particle_batch(&self, event: &LevelEvent) -> ParticleSpawnBatch {
        let block_state_id = event.data;
        if block_state_id <= AIR_BLOCK_STATE_ID {
            return ParticleSpawnBatch::default();
        }
        if !destroy_block_effect_accepts_block_state(block_state_id) {
            return ParticleSpawnBatch::default();
        }
        let template = match self.simple_particle_template(BLOCK_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let shape_boxes = destroy_block_shape_boxes(block_state_id);
        let particle_count = shape_boxes
            .iter()
            .map(|(min, max)| destroy_block_box_particle_count(*min, *max))
            .sum();
        let mut batch = ParticleSpawnBatch {
            commands: Vec::with_capacity(particle_count),
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let option_state = ParticleOptionRenderState {
            block: Some(ParticleBlockOptionState { block_state_id }),
            ..ParticleOptionRenderState::default()
        };
        let raw_options_len = positive_var_i32_len(block_state_id);

        for (min, max) in shape_boxes {
            self.append_destroy_block_box_particles(
                &mut batch,
                &template,
                event,
                min,
                max,
                raw_options_len,
                option_state,
            );
        }

        batch
    }

    fn append_destroy_block_box_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        event: &LevelEvent,
        min: [f64; 3],
        max: [f64; 3],
        raw_options_len: usize,
        option_state: ParticleOptionRenderState,
    ) {
        let width_x = destroy_block_box_width(min[0], max[0]);
        let width_y = destroy_block_box_width(min[1], max[1]);
        let width_z = destroy_block_box_width(min[2], max[2]);
        let count_x = destroy_block_axis_count(width_x);
        let count_y = destroy_block_axis_count(width_y);
        let count_z = destroy_block_axis_count(width_z);

        for xx in 0..count_x {
            for yy in 0..count_y {
                for zz in 0..count_z {
                    let rel_x = (xx as f64 + 0.5) / count_x as f64;
                    let rel_y = (yy as f64 + 0.5) / count_y as f64;
                    let rel_z = (zz as f64 + 0.5) / count_z as f64;
                    batch.commands.push(self.command_for_type(
                        template.particle_type,
                        &template.sprite_ids,
                        Vec3d {
                            x: f64::from(event.pos.x) + rel_x * width_x + min[0],
                            y: f64::from(event.pos.y) + rel_y * width_y + min[1],
                            z: f64::from(event.pos.z) + rel_z * width_z + min[2],
                        },
                        Vec3d {
                            x: rel_x - 0.5,
                            y: rel_y - 0.5,
                            z: rel_z - 0.5,
                        },
                        template.particle_type.override_limiter,
                        false,
                        raw_options_len,
                        0,
                        option_state,
                    ));
                }
            }
        }
    }

    fn smash_attack_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(DUST_PILLAR_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let particle_count = smash_attack_particle_loop_count(event.data, 3.0)
            + smash_attack_particle_loop_count(event.data, 1.5);
        let mut batch = ParticleSpawnBatch {
            commands: Vec::with_capacity(particle_count),
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let block_state_id = context
            .block_state_id_at_event_pos
            .unwrap_or(AIR_BLOCK_STATE_ID);
        let provider_accepts_spawn = terrain_particle_provider_accepts_block_state(block_state_id);
        let option_state = ParticleOptionRenderState {
            block: Some(ParticleBlockOptionState { block_state_id }),
            ..ParticleOptionRenderState::default()
        };
        let center = Vec3d {
            x: f64::from(event.pos.x) + 0.5,
            y: f64::from(event.pos.y) + 1.0,
            z: f64::from(event.pos.z) + 0.5,
        };

        for _ in 0..smash_attack_particle_loop_count(event.data, 3.0) {
            let position = Vec3d {
                x: center.x + random.next_gaussian() / 2.0,
                y: center.y,
                z: center.z + random.next_gaussian() / 2.0,
            };
            let velocity = Vec3d {
                x: random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
                y: random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
                z: random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
            };
            if provider_accepts_spawn {
                batch.commands.push(self.command_for_type(
                    template.particle_type,
                    &template.sprite_ids,
                    position,
                    velocity,
                    template.particle_type.override_limiter,
                    false,
                    0,
                    0,
                    option_state,
                ));
            }
        }

        for i in 0..smash_attack_particle_loop_count(event.data, 1.5) {
            let angle = i as f64;
            let position = Vec3d {
                x: center.x + 3.5 * angle.cos() + random.next_gaussian() / 2.0,
                y: center.y,
                z: center.z + 3.5 * angle.sin() + random.next_gaussian() / 2.0,
            };
            let velocity = Vec3d {
                x: random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
                y: random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
                z: random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
            };
            if provider_accepts_spawn {
                batch.commands.push(self.command_for_type(
                    template.particle_type,
                    &template.sprite_ids,
                    position,
                    velocity,
                    template.particle_type.override_limiter,
                    false,
                    0,
                    0,
                    option_state,
                ));
            }
        }

        batch
    }

    fn dripstone_drip_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
    ) -> ParticleSpawnBatch {
        let particle_type_id = match context.dripstone_drip_particle {
            Some(LevelEventDripstoneDripParticle::Lava) => DRIPPING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID,
            Some(LevelEventDripstoneDripParticle::Water) => {
                DRIPPING_DRIPSTONE_WATER_PARTICLE_TYPE_ID
            }
            None => return ParticleSpawnBatch::default(),
        };
        self.simple_particle_batch(
            particle_type_id,
            vec![(pointed_dripstone_drip_position(event), Vec3d::default())],
        )
    }

    fn growth_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        if event.data <= 0 {
            return ParticleSpawnBatch::default();
        }
        let Some(growth) = context.growth_particles else {
            return ParticleSpawnBatch::default();
        };
        match growth.mode {
            LevelEventGrowthParticleMode::InBlock { spread_height } => self
                .particle_in_block_batch_at(
                    growth.pos,
                    HAPPY_VILLAGER_PARTICLE_TYPE_ID,
                    event.data,
                    spread_height,
                    random,
                ),
            LevelEventGrowthParticleMode::WideNoFloating { support } => self
                .growth_wide_particle_batch(
                    growth.pos,
                    event.data.wrapping_mul(3),
                    support,
                    random,
                ),
        }
    }

    fn sculk_charge_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let count = event.data >> 6;
        if count <= 0 {
            return self.sculk_charge_pop_particle_batch(
                event,
                context.sculk_charge_pop_full_block.unwrap_or(false),
                random,
            );
        }

        let template = match self.simple_particle_template(SCULK_CHARGE_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let particle_data = event.data & 63;

        if particle_data == 0 {
            for direction in BLOCK_FACE_DIRECTIONS {
                let roll = if *direction == BLOCK_FACE_DIRECTION_DOWN {
                    std::f32::consts::PI
                } else {
                    0.0
                };
                let step_factor = if direction.1 != 0 {
                    SCULK_CHARGE_FULL_BLOCK_Y_FACTOR
                } else {
                    SCULK_CHARGE_FULL_BLOCK_SIDE_FACTOR
                };
                self.append_sculk_charge_face_particles(
                    &mut batch,
                    &template,
                    event,
                    *direction,
                    step_factor,
                    roll,
                    count,
                    random,
                );
            }
        } else {
            for (direction_index, direction) in BLOCK_FACE_DIRECTIONS.iter().enumerate() {
                if particle_data & (1 << direction_index) == 0 {
                    continue;
                }
                let roll = if *direction == BLOCK_FACE_DIRECTION_UP {
                    std::f32::consts::PI
                } else {
                    0.0
                };
                self.append_sculk_charge_face_particles(
                    &mut batch,
                    &template,
                    event,
                    *direction,
                    SCULK_CHARGE_MULTIFACE_FACTOR,
                    roll,
                    count,
                    random,
                );
            }
        }

        batch
    }

    fn sculk_charge_pop_particle_batch(
        &self,
        event: &LevelEvent,
        is_full_block: bool,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(SCULK_CHARGE_POP_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let particle_count = if is_full_block { 40 } else { 20 };
        let spread = if is_full_block {
            SCULK_CHARGE_POP_FULL_BLOCK_SPREAD
        } else {
            SCULK_CHARGE_POP_PARTIAL_BLOCK_SPREAD
        };
        for _ in 0..particle_count {
            let velocity_x = 2.0 * f64::from(random.next_float()) - 1.0;
            let velocity_y = 2.0 * f64::from(random.next_float()) - 1.0;
            let velocity_z = 2.0 * f64::from(random.next_float()) - 1.0;
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                Vec3d {
                    x: f64::from(event.pos.x) + 0.5 + velocity_x * spread,
                    y: f64::from(event.pos.y) + 0.5 + velocity_y * spread,
                    z: f64::from(event.pos.z) + 0.5 + velocity_z * spread,
                },
                Vec3d {
                    x: velocity_x * SCULK_CHARGE_POP_SPEED,
                    y: velocity_y * SCULK_CHARGE_POP_SPEED,
                    z: velocity_z * SCULK_CHARGE_POP_SPEED,
                },
                template.particle_type.override_limiter,
                false,
                0,
                0,
                ParticleOptionRenderState::default(),
            ));
        }
        batch
    }

    fn append_sculk_charge_face_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        event: &LevelEvent,
        direction: (i32, i32, i32),
        step_factor: f64,
        roll: f32,
        count: i32,
        random: &mut LevelEventSoundRandomState,
    ) {
        let particle_count = random.next_int_bound(count + 1);
        for _ in 0..particle_count {
            let speed = Vec3d {
                x: random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
                y: random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
                z: random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
            };
            let (position, velocity) =
                block_face_particle(event, direction, speed, step_factor, random);
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                position,
                velocity,
                template.particle_type.override_limiter,
                false,
                0,
                0,
                ParticleOptionRenderState {
                    roll: Some(roll),
                    ..ParticleOptionRenderState::default()
                },
            ));
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

    fn potion_break_spell_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let mut batch = ParticleSpawnBatch::default();
        self.append_item_break_particles(
            &mut batch,
            VANILLA_SPLASH_POTION_ITEM_ID,
            Vec3d {
                x: f64::from(event.pos.x) + 0.5,
                y: f64::from(event.pos.y),
                z: f64::from(event.pos.z) + 0.5,
            },
            random,
        );

        let particle_type_id = if event.event_type == INSTANT_POTION_BREAK_LEVEL_EVENT {
            INSTANT_EFFECT_PARTICLE_TYPE_ID
        } else {
            EFFECT_PARTICLE_TYPE_ID
        };
        let Some(template) = self
            .append_template_result(&mut batch, self.simple_particle_template(particle_type_id))
        else {
            return batch;
        };

        let red = ((event.data >> 16) & 0xFF) as f32 / 255.0;
        let green = ((event.data >> 8) & 0xFF) as f32 / 255.0;
        let blue = (event.data & 0xFF) as f32 / 255.0;
        let base_x = f64::from(event.pos.x) + 0.5;
        let base_y = f64::from(event.pos.y);
        let base_z = f64::from(event.pos.z) + 0.5;

        for _ in 0..POTION_BREAK_SPELL_PARTICLE_COUNT {
            let dist = random.next_double() * 4.0;
            let angle = random.next_double() * std::f64::consts::TAU;
            let velocity = Vec3d {
                x: angle.cos() * dist,
                y: 0.01 + random.next_double() * 0.5,
                z: angle.sin() * dist,
            };
            let random_brightness = 0.75 + random.next_float() * 0.25;
            let option_state = ParticleOptionRenderState {
                color: Some([
                    red * random_brightness,
                    green * random_brightness,
                    blue * random_brightness,
                    1.0,
                ]),
                power: Some(dist as f32),
                ..ParticleOptionRenderState::default()
            };
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                Vec3d {
                    x: base_x + velocity.x * 0.1,
                    y: base_y + 0.3,
                    z: base_z + velocity.z * 0.1,
                },
                velocity,
                template.particle_type.override_limiter,
                false,
                0,
                0,
                option_state,
            ));
        }

        batch
    }

    fn ender_eye_break_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let center_x = f64::from(event.pos.x) + 0.5;
        let y = f64::from(event.pos.y);
        let center_z = f64::from(event.pos.z) + 0.5;
        let mut batch = ParticleSpawnBatch::default();
        self.append_item_break_particles(
            &mut batch,
            VANILLA_ENDER_EYE_ITEM_ID,
            Vec3d {
                x: center_x,
                y,
                z: center_z,
            },
            random,
        );
        let Some(template) = self.append_template_result(
            &mut batch,
            self.simple_particle_template(PORTAL_PARTICLE_TYPE_ID),
        ) else {
            return batch;
        };

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

    fn append_item_break_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        item_id: i32,
        position: Vec3d,
        random: &mut LevelEventSoundRandomState,
    ) {
        let Some(template) = self
            .append_template_result(batch, self.simple_particle_template(ITEM_PARTICLE_TYPE_ID))
        else {
            return;
        };
        let raw_options_len = item_particle_raw_options_len(item_id, 1);
        let option_state = ParticleOptionRenderState {
            item: Some(ParticleItemOptionState {
                item_id,
                count: 1,
                component_patch_len: EMPTY_ITEM_COMPONENT_PATCH_OPTION_LEN,
            }),
            item_component_patch_empty: true,
            ..ParticleOptionRenderState::default()
        };

        for _ in 0..ITEM_BREAK_PARTICLE_COUNT {
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                position,
                Vec3d {
                    x: random.next_gaussian() * ITEM_BREAK_HORIZONTAL_VELOCITY_SCALE,
                    y: random.next_double() * ITEM_BREAK_VERTICAL_VELOCITY_SCALE,
                    z: random.next_gaussian() * ITEM_BREAK_HORIZONTAL_VELOCITY_SCALE,
                },
                template.particle_type.override_limiter,
                false,
                raw_options_len,
                0,
                option_state,
            ));
        }
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

    fn vault_activation_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        if !context.vault_block_entity_at_event_pos {
            return ParticleSpawnBatch::default();
        }

        let flame_particle_type_id = if event.data == 0 {
            SMALL_FLAME_PARTICLE_TYPE_ID
        } else {
            SOUL_FIRE_FLAME_PARTICLE_TYPE_ID
        };
        let mut batch = ParticleSpawnBatch::default();
        let smoke = self.simple_particle_template(SMOKE_PARTICLE_TYPE_ID);
        let flame = self.simple_particle_template(flame_particle_type_id);
        let smoke = self.append_template_result(&mut batch, smoke);
        let flame = self.append_template_result(&mut batch, flame);

        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + random_between(random, 0.1, 0.9),
                y: f64::from(event.pos.y) + random_between(random, 0.25, 0.75),
                z: f64::from(event.pos.z) + random_between(random, 0.1, 0.9),
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

    fn vault_deactivation_particle_batch(
        &self,
        event: &LevelEvent,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        let flame_particle_type_id = if event.data == 0 {
            SMALL_FLAME_PARTICLE_TYPE_ID
        } else {
            SOUL_FIRE_FLAME_PARTICLE_TYPE_ID
        };
        let template = match self.simple_particle_template(flame_particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..20 {
            let position = Vec3d {
                x: f64::from(event.pos.x) + random_between(random, 0.4, 0.6),
                y: f64::from(event.pos.y) + random_between(random, 0.4, 0.6),
                z: f64::from(event.pos.z) + random_between(random, 0.4, 0.6),
            };
            let velocity = Vec3d {
                x: random.next_gaussian() * 0.02,
                y: random.next_gaussian() * 0.02,
                z: random.next_gaussian() * 0.02,
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }

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
        self.particle_in_block_batch(event, POOF_PARTICLE_TYPE_ID, 10, 1.0, random)
    }

    fn particle_in_block_batch(
        &self,
        event: &LevelEvent,
        particle_type_id: i32,
        count: i32,
        spread_height: f64,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        self.particle_in_block_batch_at(event.pos, particle_type_id, count, spread_height, random)
    }

    fn particle_in_block_batch_at(
        &self,
        pos: BlockPos,
        particle_type_id: i32,
        count: i32,
        spread_height: f64,
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
                x: f64::from(pos.x) + random.next_double(),
                y: f64::from(pos.y) + random.next_double() * spread_height,
                z: f64::from(pos.z) + random.next_double(),
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }

        batch
    }

    fn growth_wide_particle_batch(
        &self,
        pos: BlockPos,
        count: i32,
        support: LevelEventGrowthParticleSupport,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch {
        if count <= 0 {
            return ParticleSpawnBatch::default();
        }
        let template = match self.simple_particle_template(HAPPY_VILLAGER_PARTICLE_TYPE_ID) {
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
                x: f64::from(pos.x)
                    + GROWTH_PARTICLE_WIDE_START_OFFSET
                    + random.next_double() * GROWTH_PARTICLE_WIDE_SPREAD * 2.0,
                y: f64::from(pos.y) + random.next_double() * GROWTH_PARTICLE_WIDE_HEIGHT,
                z: f64::from(pos.z)
                    + GROWTH_PARTICLE_WIDE_START_OFFSET
                    + random.next_double() * GROWTH_PARTICLE_WIDE_SPREAD * 2.0,
            };
            if growth_particle_position_has_support(pos, position, support) {
                batch
                    .commands
                    .push(self.command_from_template(&template, position, velocity, false));
            }
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
        let (sprite_ids, missing_sprite_count) =
            if let Some(definition) = self.definitions.definition(particle_type.name) {
                let sprite_ids = definition.textures.clone();
                let missing_sprite_count = sprite_ids
                    .iter()
                    .filter(|sprite_id| self.sprites.sprite(sprite_id).is_none())
                    .count();
                (sprite_ids, missing_sprite_count)
            } else if definitionless_particle_type(particle_type.id) {
                (Vec::new(), 0)
            } else {
                return Err(ParticleSpawnBatch {
                    missing_definition_count: 1,
                    ..ParticleSpawnBatch::default()
                });
            };
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
        let sprite_ids = self.sprite_ids_for_command(particle_type.id, sprite_ids, option_state);
        let option_color = option_state
            .color
            .or_else(|| self.tint_color_for_command(particle_type.id, option_state));
        ParticleSpawnCommand {
            particle_type_id: particle_type.id,
            particle_id: particle_type.name.to_string(),
            sprite_ids,
            position: [position.x, position.y, position.z],
            velocity: [velocity.x, velocity.y, velocity.z],
            override_limiter,
            always_show,
            raw_options_len,
            initial_delay_ticks,
            child_spawn_templates,
            option_color,
            option_color_to: option_state.color_to,
            option_scale: option_state.scale,
            option_power: option_state.power,
            option_target: option_state.target,
            option_duration_ticks: option_state.duration_ticks,
            option_roll: option_state.roll,
            option_block: option_state.block,
            option_item: option_state.item,
        }
    }

    fn sprite_ids_for_command(
        &self,
        particle_type_id: i32,
        sprite_ids: &[String],
        option_state: ParticleOptionRenderState,
    ) -> Vec<String> {
        if matches!(
            particle_type_id,
            BLOCK_PARTICLE_TYPE_ID
                | BLOCK_MARKER_PARTICLE_TYPE_ID
                | DUST_PILLAR_PARTICLE_TYPE_ID
                | BLOCK_CRUMBLE_PARTICLE_TYPE_ID
        ) {
            if let Some(sprite_id) = option_state
                .block
                .and_then(|block| self.terrain_particle_sprite_ids.get(&block.block_state_id))
            {
                return vec![sprite_id.clone()];
            }
        }
        if particle_type_id == ITEM_PARTICLE_TYPE_ID {
            if option_state.item_component_patch_empty {
                if let Some(sprite_ids) = option_state
                    .item
                    .and_then(|item| self.default_item_particle_sprite_ids.get(&item.item_id))
                {
                    return sprite_ids.clone();
                }
            }
        }
        sprite_ids.to_vec()
    }

    fn tint_color_for_command(
        &self,
        particle_type_id: i32,
        option_state: ParticleOptionRenderState,
    ) -> Option<[f32; 4]> {
        let block_state_id = option_state.block?.block_state_id;
        match particle_type_id {
            BLOCK_PARTICLE_TYPE_ID
            | DUST_PILLAR_PARTICLE_TYPE_ID
            | BLOCK_CRUMBLE_PARTICLE_TYPE_ID => self
                .terrain_particle_tint_colors
                .get(&block_state_id)
                .copied(),
            FALLING_DUST_PARTICLE_TYPE_ID => self
                .falling_dust_block_tint_colors
                .get(&block_state_id)
                .copied()
                .or_else(|| falling_dust_map_color_for_block_state_id(block_state_id)),
            _ => None,
        }
    }

    fn child_spawn_templates_for_type(
        &self,
        particle_type: ParticleTypeInfo,
    ) -> Vec<ParticleChildSpawnTemplate> {
        let child_particle_type_id = match particle_type.id {
            EXPLOSION_EMITTER_PARTICLE_TYPE_ID => EXPLOSION_PARTICLE_TYPE_ID,
            LAVA_PARTICLE_TYPE_ID => SMOKE_PARTICLE_TYPE_ID,
            GUST_EMITTER_LARGE_PARTICLE_TYPE_ID | GUST_EMITTER_SMALL_PARTICLE_TYPE_ID => {
                GUST_PARTICLE_TYPE_ID
            }
            _ => return Vec::new(),
        };
        self.simple_particle_template(child_particle_type_id)
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

fn definitionless_particle_type(particle_type_id: i32) -> bool {
    matches!(
        particle_type_id,
        BLOCK_PARTICLE_TYPE_ID
            | BLOCK_MARKER_PARTICLE_TYPE_ID
            | ITEM_PARTICLE_TYPE_ID
            | ITEM_SLIME_PARTICLE_TYPE_ID
            | ITEM_COBWEB_PARTICLE_TYPE_ID
            | ITEM_SNOWBALL_PARTICLE_TYPE_ID
            | ELDER_GUARDIAN_PARTICLE_TYPE_ID
            | EXPLOSION_EMITTER_PARTICLE_TYPE_ID
            | GUST_EMITTER_LARGE_PARTICLE_TYPE_ID
            | GUST_EMITTER_SMALL_PARTICLE_TYPE_ID
            | DUST_PILLAR_PARTICLE_TYPE_ID
            | BLOCK_CRUMBLE_PARTICLE_TYPE_ID
    )
}

fn item_particle_raw_options_len(item_id: i32, count: i32) -> usize {
    positive_var_i32_len(item_id)
        + positive_var_i32_len(count)
        + EMPTY_ITEM_COMPONENT_PATCH_OPTION_LEN
}

fn positive_var_i32_len(value: i32) -> usize {
    debug_assert!(value >= 0);
    let mut value = value as u32;
    let mut len = 1;
    while value & !0x7f != 0 {
        value >>= 7;
        len += 1;
    }
    len
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
    block: Option<ParticleBlockOptionState>,
    item: Option<ParticleItemOptionState>,
    item_component_patch_empty: bool,
}

fn particle_option_render_state(
    particle_type_id: i32,
    raw_options: &[u8],
) -> ParticleOptionRenderState {
    let mut decoder = Decoder::new(raw_options);
    match particle_type_id {
        BLOCK_PARTICLE_TYPE_ID
        | BLOCK_MARKER_PARTICLE_TYPE_ID
        | FALLING_DUST_PARTICLE_TYPE_ID
        | DUST_PILLAR_PARTICLE_TYPE_ID
        | BLOCK_CRUMBLE_PARTICLE_TYPE_ID => {
            let Ok(block_state_id) = decoder.read_var_i32() else {
                return ParticleOptionRenderState::default();
            };
            if !decoder.is_empty() {
                return ParticleOptionRenderState::default();
            }
            ParticleOptionRenderState {
                color: if particle_type_id == FALLING_DUST_PARTICLE_TYPE_ID {
                    falling_dust_color_for_block_state_id(block_state_id)
                } else {
                    None
                },
                block: Some(ParticleBlockOptionState { block_state_id }),
                ..ParticleOptionRenderState::default()
            }
        }
        ITEM_PARTICLE_TYPE_ID => {
            let Ok(item_id) = decoder.read_var_i32() else {
                return ParticleOptionRenderState::default();
            };
            let Ok(count) = decoder.read_var_i32() else {
                return ParticleOptionRenderState::default();
            };
            if item_id < 0 || count <= 0 {
                return ParticleOptionRenderState::default();
            }
            let component_patch_empty = decoder.remaining() == [0, 0];
            ParticleOptionRenderState {
                item: Some(ParticleItemOptionState {
                    item_id,
                    count,
                    component_patch_len: decoder.remaining_len(),
                }),
                item_component_patch_empty: component_patch_empty,
                ..ParticleOptionRenderState::default()
            }
        }
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
        ENTITY_EFFECT_PARTICLE_TYPE_ID
        | FLASH_PARTICLE_TYPE_ID
        | TINTED_LEAVES_PARTICLE_TYPE_ID => {
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

fn particle_provider_accepts_spawn(
    particle_type_id: i32,
    option_state: ParticleOptionRenderState,
) -> bool {
    let Some(block) = option_state.block else {
        return true;
    };
    match particle_type_id {
        FALLING_DUST_PARTICLE_TYPE_ID => {
            falling_dust_provider_accepts_block_state(block.block_state_id)
        }
        BLOCK_PARTICLE_TYPE_ID | DUST_PILLAR_PARTICLE_TYPE_ID | BLOCK_CRUMBLE_PARTICLE_TYPE_ID => {
            terrain_particle_provider_accepts_block_state(block.block_state_id)
        }
        _ => true,
    }
}

fn falling_dust_provider_accepts_block_state(block_state_id: i32) -> bool {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let Some(block_state) = block_states.by_id(block_state_id) else {
        return true;
    };
    !falling_dust_provider_rejects_block_name(&block_state.name)
}

fn falling_dust_provider_rejects_block_name(name: &str) -> bool {
    !matches!(
        name,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    ) && block_name_has_invisible_render_shape(name)
}

fn falling_dust_color_for_block_state_id(block_state_id: i32) -> Option<[f32; 4]> {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let block_state = block_states.by_id(block_state_id)?;
    falling_dust_color_for_block_name(&block_state.name).map(rgb_particle_color_u32)
}

fn falling_dust_map_color_for_block_state_id(block_state_id: i32) -> Option<[f32; 4]> {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let block_state = block_states.by_id(block_state_id)?;
    vanilla_static_map_color_for_block_state(&block_state.name, &block_state.properties)
        .map(rgb_particle_color_u32)
}

fn falling_dust_color_for_block_name(name: &str) -> Option<u32> {
    match name {
        // Vanilla FallingDustParticle.Provider uses FallingBlock#getDustColor first.
        "minecraft:sand" => Some(0xDB_D3_A0),
        "minecraft:red_sand" => Some(0xA9_58_21),
        "minecraft:gravel" => Some(0x80_7C_7B),
        "minecraft:dragon_egg" => Some(0x00_00_00),
        "minecraft:anvil" | "minecraft:chipped_anvil" | "minecraft:damaged_anvil" => {
            Some(MAP_COLOR_METAL)
        }
        name => concrete_powder_map_color(name),
    }
}

fn concrete_powder_map_color(name: &str) -> Option<u32> {
    let color = name
        .strip_prefix("minecraft:")?
        .strip_suffix("_concrete_powder")?;
    dye_color_map_color(color)
}

fn dye_color_map_color(color: &str) -> Option<u32> {
    Some(match color {
        "white" => MAP_COLOR_SNOW,
        "orange" => MAP_COLOR_ORANGE,
        "magenta" => MAP_COLOR_MAGENTA,
        "light_blue" => MAP_COLOR_LIGHT_BLUE,
        "yellow" => MAP_COLOR_YELLOW,
        "lime" => MAP_COLOR_LIGHT_GREEN,
        "pink" => MAP_COLOR_PINK,
        "gray" => MAP_COLOR_GRAY,
        "light_gray" => MAP_COLOR_LIGHT_GRAY,
        "cyan" => MAP_COLOR_CYAN,
        "purple" => MAP_COLOR_PURPLE,
        "blue" => MAP_COLOR_BLUE,
        "brown" => MAP_COLOR_BROWN,
        "green" => MAP_COLOR_GREEN,
        "red" => MAP_COLOR_RED,
        "black" => MAP_COLOR_BLACK,
        _ => return None,
    })
}

fn terracotta_map_color(color: &str) -> Option<u32> {
    Some(match color {
        "white" => MAP_COLOR_TERRACOTTA_WHITE,
        "orange" => MAP_COLOR_TERRACOTTA_ORANGE,
        "magenta" => MAP_COLOR_TERRACOTTA_MAGENTA,
        "light_blue" => MAP_COLOR_TERRACOTTA_LIGHT_BLUE,
        "yellow" => MAP_COLOR_TERRACOTTA_YELLOW,
        "lime" => MAP_COLOR_TERRACOTTA_LIGHT_GREEN,
        "pink" => MAP_COLOR_TERRACOTTA_PINK,
        "gray" => MAP_COLOR_TERRACOTTA_GRAY,
        "light_gray" => MAP_COLOR_TERRACOTTA_LIGHT_GRAY,
        "cyan" => MAP_COLOR_TERRACOTTA_CYAN,
        "purple" => MAP_COLOR_TERRACOTTA_PURPLE,
        "blue" => MAP_COLOR_TERRACOTTA_BLUE,
        "brown" => MAP_COLOR_TERRACOTTA_BROWN,
        "green" => MAP_COLOR_TERRACOTTA_GREEN,
        "red" => MAP_COLOR_TERRACOTTA_RED,
        "black" => MAP_COLOR_TERRACOTTA_BLACK,
        _ => return None,
    })
}

fn colored_family_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    for suffix in [
        "_wool",
        "_carpet",
        "_concrete",
        "_stained_glass",
        "_stained_glass_pane",
        "_glazed_terracotta",
    ] {
        if let Some(color) = name.strip_suffix(suffix) {
            return dye_color_map_color(color);
        }
    }
    if let Some(color) = name.strip_suffix("_terracotta") {
        return terracotta_map_color(color);
    }
    None
}

fn banner_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let color = name
        .strip_suffix("_wall_banner")
        .or_else(|| name.strip_suffix("_banner"))?;
    dye_color_map_color(color).map(|_| MAP_COLOR_WOOD)
}

fn candle_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if name == "candle" {
        return Some(MAP_COLOR_SAND);
    }
    let color = name.strip_suffix("_candle")?;
    if color == "white" {
        Some(MAP_COLOR_WOOL)
    } else {
        dye_color_map_color(color)
    }
}

fn shulker_box_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if name == "shulker_box" {
        return Some(MAP_COLOR_PURPLE);
    }
    let color = name.strip_suffix("_shulker_box")?;
    if color == "purple" {
        Some(MAP_COLOR_TERRACOTTA_PURPLE)
    } else {
        dye_color_map_color(color)
    }
}

fn bed_map_color(
    name: &str,
    properties: &std::collections::BTreeMap<String, String>,
) -> Option<u32> {
    let color = name.strip_prefix("minecraft:")?.strip_suffix("_bed")?;
    match properties.get("part").map(String::as_str) {
        Some("head") => Some(MAP_COLOR_WOOL),
        Some("foot") => dye_color_map_color(color),
        _ => None,
    }
}

fn ore_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    match name {
        "nether_gold_ore" | "nether_quartz_ore" => return Some(MAP_COLOR_NETHER),
        _ => {}
    }
    if name
        .strip_prefix("deepslate_")
        .is_some_and(|ore| ore.ends_with("_ore"))
    {
        return Some(MAP_COLOR_DEEPSLATE);
    }
    if name.ends_with("_ore") {
        return Some(MAP_COLOR_STONE);
    }
    None
}

fn deepslate_family_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if name == "deepslate"
        || name == "cobbled_deepslate"
        || name == "cobbled_deepslate_stairs"
        || name == "cobbled_deepslate_slab"
        || name == "cobbled_deepslate_wall"
        || name == "polished_deepslate"
        || name == "polished_deepslate_stairs"
        || name == "polished_deepslate_slab"
        || name == "polished_deepslate_wall"
        || name == "deepslate_tiles"
        || name == "deepslate_tile_stairs"
        || name == "deepslate_tile_slab"
        || name == "deepslate_tile_wall"
        || name == "deepslate_bricks"
        || name == "deepslate_brick_stairs"
        || name == "deepslate_brick_slab"
        || name == "deepslate_brick_wall"
        || name == "chiseled_deepslate"
        || name == "cracked_deepslate_bricks"
        || name == "cracked_deepslate_tiles"
        || name == "infested_deepslate"
        || name == "reinforced_deepslate"
    {
        return Some(MAP_COLOR_DEEPSLATE);
    }
    None
}

fn copper_weathering_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if name == "raw_copper_block" {
        return Some(MAP_COLOR_ORANGE);
    }
    let name = name.strip_prefix("waxed_").unwrap_or(name);
    if let Some(rest) = name.strip_prefix("exposed_") {
        return copper_weathering_base_suffix(rest).then_some(MAP_COLOR_TERRACOTTA_LIGHT_GRAY);
    }
    if let Some(rest) = name.strip_prefix("weathered_") {
        return copper_weathering_base_suffix(rest).then_some(MAP_COLOR_WARPED_STEM);
    }
    if let Some(rest) = name.strip_prefix("oxidized_") {
        return copper_weathering_base_suffix(rest).then_some(MAP_COLOR_WARPED_NYLIUM);
    }
    copper_weathering_base_suffix(name).then_some(MAP_COLOR_ORANGE)
}

fn copper_weathering_base_suffix(name: &str) -> bool {
    matches!(
        name,
        "copper"
            | "copper_block"
            | "cut_copper"
            | "cut_copper_stairs"
            | "cut_copper_slab"
            | "chiseled_copper"
            | "copper_door"
            | "copper_trapdoor"
            | "copper_grate"
            | "copper_bulb"
            | "copper_chest"
            | "copper_golem_statue"
            | "lightning_rod"
    )
}

fn wooden_stairs_and_slabs_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let family = name
        .strip_suffix("_stairs")
        .or_else(|| name.strip_suffix("_slab"))?;
    wooden_plank_family_map_color(family)
}

fn wooden_pressure_plate_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let family = name.strip_suffix("_pressure_plate")?;
    wooden_plank_family_map_color(family)
}

fn wooden_door_trapdoor_fence_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let family = name
        .strip_suffix("_fence_gate")
        .or_else(|| name.strip_suffix("_trapdoor"))
        .or_else(|| name.strip_suffix("_door"))
        .or_else(|| name.strip_suffix("_fence"))?;
    wooden_plank_family_map_color(family)
}

fn button_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if matches!(name, "stone_button" | "polished_blackstone_button") {
        return Some(MAP_COLOR_NONE);
    }
    let family = name.strip_suffix("_button")?;
    wooden_plank_family_map_color(family).map(|_| MAP_COLOR_NONE)
}

fn potted_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "flower_pot"
        | "potted_torchflower"
        | "potted_oak_sapling"
        | "potted_spruce_sapling"
        | "potted_birch_sapling"
        | "potted_jungle_sapling"
        | "potted_acacia_sapling"
        | "potted_cherry_sapling"
        | "potted_dark_oak_sapling"
        | "potted_pale_oak_sapling"
        | "potted_mangrove_propagule"
        | "potted_dandelion"
        | "potted_golden_dandelion"
        | "potted_poppy"
        | "potted_blue_orchid"
        | "potted_allium"
        | "potted_azure_bluet"
        | "potted_red_tulip"
        | "potted_orange_tulip"
        | "potted_white_tulip"
        | "potted_pink_tulip"
        | "potted_oxeye_daisy"
        | "potted_cornflower"
        | "potted_lily_of_the_valley"
        | "potted_wither_rose"
        | "potted_red_mushroom"
        | "potted_brown_mushroom"
        | "potted_dead_bush"
        | "potted_cactus"
        | "potted_bamboo"
        | "potted_crimson_fungus"
        | "potted_warped_fungus"
        | "potted_crimson_roots"
        | "potted_warped_roots"
        | "potted_azalea_bush"
        | "potted_flowering_azalea_bush"
        | "potted_open_eyeblossom"
        | "potted_closed_eyeblossom" => MAP_COLOR_NONE,
        _ => return None,
    })
}

fn cake_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if matches!(name, "cake" | "candle_cake") {
        return Some(MAP_COLOR_NONE);
    }
    let color = name.strip_suffix("_candle_cake")?;
    dye_color_map_color(color).map(|_| MAP_COLOR_NONE)
}

fn wooden_sign_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if let Some(family) = name.strip_suffix("_wall_hanging_sign") {
        return if family == "spruce" {
            Some(MAP_COLOR_WOOD)
        } else {
            hanging_sign_family_map_color(family)
        };
    }
    if let Some(family) = name.strip_suffix("_hanging_sign") {
        return hanging_sign_family_map_color(family);
    }
    let family = name
        .strip_suffix("_wall_sign")
        .or_else(|| name.strip_suffix("_sign"))?;
    wooden_plank_family_map_color(family)
}

fn wooden_shelf_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let family = name.strip_suffix("_shelf")?;
    wooden_plank_family_map_color(family)
}

fn hanging_sign_family_map_color(family: &str) -> Option<u32> {
    if family == "cherry" {
        return Some(MAP_COLOR_TERRACOTTA_PINK);
    }
    wooden_plank_family_map_color(family)
}

fn wooden_plank_family_map_color(family: &str) -> Option<u32> {
    Some(match family {
        "oak" => MAP_COLOR_WOOD,
        "spruce" => MAP_COLOR_PODZOL,
        "birch" => MAP_COLOR_SAND,
        "jungle" => MAP_COLOR_DIRT,
        "acacia" => MAP_COLOR_ORANGE,
        "cherry" => MAP_COLOR_TERRACOTTA_WHITE,
        "dark_oak" => MAP_COLOR_BROWN,
        "pale_oak" => MAP_COLOR_QUARTZ,
        "mangrove" => MAP_COLOR_RED,
        "bamboo" | "bamboo_mosaic" => MAP_COLOR_YELLOW,
        "crimson" => MAP_COLOR_CRIMSON_STEM,
        "warped" => MAP_COLOR_WARPED_STEM,
        _ => return None,
    })
}

fn vanilla_static_map_color_for_block_state(
    name: &str,
    properties: &std::collections::BTreeMap<String, String>,
) -> Option<u32> {
    if let Some(color) = colored_family_map_color(name) {
        return Some(color);
    }
    if let Some(color) = banner_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = candle_map_color(name) {
        return Some(color);
    }
    if let Some(color) = shulker_box_map_color(name) {
        return Some(color);
    }
    if let Some(color) = bed_map_color(name, properties) {
        return Some(color);
    }
    if let Some(color) = ore_map_color(name) {
        return Some(color);
    }
    if let Some(color) = deepslate_family_map_color(name) {
        return Some(color);
    }
    if let Some(color) = copper_weathering_map_color(name) {
        return Some(color);
    }
    if let Some(color) = wooden_stairs_and_slabs_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = wooden_pressure_plate_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = wooden_door_trapdoor_fence_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = wooden_sign_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = wooden_shelf_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = button_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = potted_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = cake_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = construction_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = resin_and_pale_garden_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = crop_static_map_color(name, properties) {
        return Some(color);
    }
    if let Some(color) = produce_and_fungus_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = natural_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = aquatic_static_map_color(name) {
        return Some(color);
    }
    if let Some(color) = utility_static_map_color(name) {
        return Some(color);
    }
    match name {
        "minecraft:stone"
        | "minecraft:andesite"
        | "minecraft:polished_andesite"
        | "minecraft:cobblestone"
        | "minecraft:suspicious_gravel"
        | "minecraft:dispenser"
        | "minecraft:dropper"
        | "minecraft:furnace"
        | "minecraft:piston_head"
        | "minecraft:moving_piston" => Some(MAP_COLOR_STONE),
        "minecraft:granite"
        | "minecraft:polished_granite"
        | "minecraft:dirt"
        | "minecraft:coarse_dirt"
        | "minecraft:jungle_planks"
        | "minecraft:farmland"
        | "minecraft:jukebox" => Some(MAP_COLOR_DIRT),
        "minecraft:diorite" | "minecraft:polished_diorite" | "minecraft:pale_oak_planks" => {
            Some(MAP_COLOR_QUARTZ)
        }
        "minecraft:quartz_block"
        | "minecraft:chiseled_quartz_block"
        | "minecraft:quartz_pillar"
        | "minecraft:quartz_stairs"
        | "minecraft:quartz_slab"
        | "minecraft:smooth_quartz"
        | "minecraft:smooth_quartz_stairs"
        | "minecraft:smooth_quartz_slab"
        | "minecraft:quartz_bricks"
        | "minecraft:sea_lantern" => Some(MAP_COLOR_QUARTZ),
        "minecraft:pale_oak_wood" => Some(MAP_COLOR_STONE),
        "minecraft:oak_planks" => Some(MAP_COLOR_WOOD),
        "minecraft:spruce_planks"
        | "minecraft:podzol"
        | "minecraft:mangrove_roots"
        | "minecraft:muddy_mangrove_roots"
        | "minecraft:spruce_wood"
        | "minecraft:stripped_spruce_wood" => Some(MAP_COLOR_PODZOL),
        "minecraft:birch_planks" => Some(MAP_COLOR_SAND),
        "minecraft:acacia_planks" | "minecraft:terracotta" => Some(MAP_COLOR_ORANGE),
        "minecraft:cherry_planks" => Some(MAP_COLOR_TERRACOTTA_WHITE),
        "minecraft:dark_oak_planks" => Some(MAP_COLOR_BROWN),
        "minecraft:mangrove_planks" => Some(MAP_COLOR_RED),
        "minecraft:bamboo_planks" | "minecraft:bamboo_mosaic" => Some(MAP_COLOR_YELLOW),
        "minecraft:suspicious_sand"
        | "minecraft:sandstone"
        | "minecraft:chiseled_sandstone"
        | "minecraft:cut_sandstone"
        | "minecraft:end_stone"
        | "minecraft:end_stone_bricks"
        | "minecraft:end_stone_brick_stairs"
        | "minecraft:end_stone_brick_slab"
        | "minecraft:end_stone_brick_wall" => Some(MAP_COLOR_SAND),
        "minecraft:sponge" | "minecraft:wet_sponge" => Some(MAP_COLOR_YELLOW),
        "minecraft:snow" | "minecraft:snow_block" => Some(MAP_COLOR_SNOW),
        "minecraft:ice" | "minecraft:packed_ice" | "minecraft:blue_ice" => Some(MAP_COLOR_ICE),
        "minecraft:clay" => Some(MAP_COLOR_CLAY),
        "minecraft:infested_stone"
        | "minecraft:infested_cobblestone"
        | "minecraft:infested_stone_bricks"
        | "minecraft:infested_mossy_stone_bricks"
        | "minecraft:infested_cracked_stone_bricks"
        | "minecraft:infested_chiseled_stone_bricks" => Some(MAP_COLOR_CLAY),
        "minecraft:lapis_block" => Some(MAP_COLOR_LAPIS),
        "minecraft:diamond_block" => Some(MAP_COLOR_DIAMOND),
        "minecraft:emerald_block" => Some(MAP_COLOR_EMERALD),
        "minecraft:gold_block" | "minecraft:raw_gold_block" => Some(MAP_COLOR_GOLD),
        "minecraft:iron_block" => Some(MAP_COLOR_METAL),
        "minecraft:raw_iron_block" => Some(MAP_COLOR_RAW_IRON),
        "minecraft:coal_block"
        | "minecraft:basalt"
        | "minecraft:polished_basalt"
        | "minecraft:obsidian"
        | "minecraft:crying_obsidian"
        | "minecraft:ancient_debris"
        | "minecraft:netherite_block" => Some(MAP_COLOR_BLACK),
        "minecraft:netherrack"
        | "minecraft:nether_bricks"
        | "minecraft:red_nether_bricks"
        | "minecraft:chiseled_nether_bricks"
        | "minecraft:cracked_nether_bricks"
        | "minecraft:magma_block"
        | "minecraft:crimson_fungus"
        | "minecraft:weeping_vines"
        | "minecraft:weeping_vines_plant"
        | "minecraft:crimson_roots" => Some(MAP_COLOR_NETHER),
        "minecraft:soul_sand" | "minecraft:soul_soil" => Some(MAP_COLOR_BROWN),
        "minecraft:glow_lichen" => Some(MAP_COLOR_GLOW_LICHEN),
        "minecraft:prismarine"
        | "minecraft:prismarine_stairs"
        | "minecraft:prismarine_slab"
        | "minecraft:prismarine_wall" => Some(MAP_COLOR_CYAN),
        "minecraft:prismarine_bricks"
        | "minecraft:prismarine_brick_stairs"
        | "minecraft:prismarine_brick_slab"
        | "minecraft:dark_prismarine"
        | "minecraft:dark_prismarine_stairs"
        | "minecraft:dark_prismarine_slab" => Some(MAP_COLOR_DIAMOND),
        "minecraft:warped_nylium" => Some(MAP_COLOR_WARPED_NYLIUM),
        "minecraft:crimson_nylium" => Some(MAP_COLOR_CRIMSON_NYLIUM),
        "minecraft:warped_wart_block" => Some(MAP_COLOR_WARPED_WART_BLOCK),
        "minecraft:nether_wart_block" | "minecraft:shroomlight" => Some(MAP_COLOR_RED),
        "minecraft:warped_fungus"
        | "minecraft:warped_roots"
        | "minecraft:nether_sprouts"
        | "minecraft:twisting_vines"
        | "minecraft:twisting_vines_plant" => Some(MAP_COLOR_CYAN),
        "minecraft:amethyst_block"
        | "minecraft:budding_amethyst"
        | "minecraft:amethyst_cluster"
        | "minecraft:large_amethyst_bud"
        | "minecraft:medium_amethyst_bud"
        | "minecraft:small_amethyst_bud" => Some(MAP_COLOR_PURPLE),
        "minecraft:chorus_plant" | "minecraft:chorus_flower" => Some(MAP_COLOR_PURPLE),
        "minecraft:purpur_block"
        | "minecraft:purpur_pillar"
        | "minecraft:purpur_stairs"
        | "minecraft:purpur_slab" => Some(MAP_COLOR_MAGENTA),
        "minecraft:end_portal_frame" => Some(MAP_COLOR_GREEN),
        "minecraft:tuff"
        | "minecraft:tuff_slab"
        | "minecraft:tuff_stairs"
        | "minecraft:tuff_wall"
        | "minecraft:polished_tuff"
        | "minecraft:polished_tuff_slab"
        | "minecraft:polished_tuff_stairs"
        | "minecraft:polished_tuff_wall"
        | "minecraft:chiseled_tuff"
        | "minecraft:tuff_bricks"
        | "minecraft:tuff_brick_slab"
        | "minecraft:tuff_brick_stairs"
        | "minecraft:tuff_brick_wall"
        | "minecraft:chiseled_tuff_bricks" => Some(MAP_COLOR_TERRACOTTA_GRAY),
        "minecraft:calcite" => Some(MAP_COLOR_TERRACOTTA_WHITE),
        "minecraft:tinted_glass" => Some(MAP_COLOR_GRAY),
        "minecraft:powder_snow" => Some(MAP_COLOR_SNOW),
        "minecraft:sculk_sensor" | "minecraft:calibrated_sculk_sensor" => Some(MAP_COLOR_CYAN),
        "minecraft:sculk"
        | "minecraft:sculk_vein"
        | "minecraft:sculk_catalyst"
        | "minecraft:sculk_shrieker"
        | "minecraft:smooth_basalt"
        | "minecraft:respawn_anchor"
        | "minecraft:blackstone"
        | "minecraft:blackstone_stairs"
        | "minecraft:blackstone_wall"
        | "minecraft:blackstone_slab"
        | "minecraft:polished_blackstone"
        | "minecraft:polished_blackstone_stairs"
        | "minecraft:polished_blackstone_slab"
        | "minecraft:polished_blackstone_wall"
        | "minecraft:polished_blackstone_pressure_plate"
        | "minecraft:polished_blackstone_bricks"
        | "minecraft:polished_blackstone_brick_slab"
        | "minecraft:polished_blackstone_brick_stairs"
        | "minecraft:polished_blackstone_brick_wall"
        | "minecraft:cracked_polished_blackstone_bricks"
        | "minecraft:chiseled_polished_blackstone"
        | "minecraft:gilded_blackstone" => Some(MAP_COLOR_BLACK),
        "minecraft:ochre_froglight" => Some(MAP_COLOR_SAND),
        "minecraft:verdant_froglight" => Some(MAP_COLOR_GLOW_LICHEN),
        "minecraft:pearlescent_froglight" => Some(MAP_COLOR_PINK),
        "minecraft:crimson_planks" => Some(MAP_COLOR_CRIMSON_STEM),
        "minecraft:warped_planks" => Some(MAP_COLOR_WARPED_STEM),
        "minecraft:oak_wood" | "minecraft:stripped_oak_wood" => Some(MAP_COLOR_WOOD),
        "minecraft:birch_wood" | "minecraft:stripped_birch_wood" => Some(MAP_COLOR_SAND),
        "minecraft:jungle_wood" | "minecraft:stripped_jungle_wood" => Some(MAP_COLOR_DIRT),
        "minecraft:acacia_wood" => Some(MAP_COLOR_GRAY),
        "minecraft:stripped_acacia_wood" => Some(MAP_COLOR_ORANGE),
        "minecraft:cherry_wood" => Some(MAP_COLOR_TERRACOTTA_GRAY),
        "minecraft:stripped_cherry_wood" => Some(MAP_COLOR_TERRACOTTA_PINK),
        "minecraft:dark_oak_wood" | "minecraft:stripped_dark_oak_wood" => Some(MAP_COLOR_BROWN),
        "minecraft:mangrove_wood"
        | "minecraft:stripped_mangrove_wood"
        | "minecraft:stripped_mangrove_log" => Some(MAP_COLOR_RED),
        "minecraft:stripped_bamboo_block" => Some(MAP_COLOR_YELLOW),
        "minecraft:crimson_stem" | "minecraft:stripped_crimson_stem" => {
            Some(MAP_COLOR_CRIMSON_STEM)
        }
        "minecraft:warped_stem" | "minecraft:stripped_warped_stem" => Some(MAP_COLOR_WARPED_STEM),
        "minecraft:crimson_hyphae" | "minecraft:stripped_crimson_hyphae" => {
            Some(MAP_COLOR_CRIMSON_HYPHAE)
        }
        "minecraft:warped_hyphae" | "minecraft:stripped_warped_hyphae" => {
            Some(MAP_COLOR_WARPED_HYPHAE)
        }
        "minecraft:oak_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_WOOD,
            MAP_COLOR_PODZOL,
        )),
        "minecraft:spruce_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_PODZOL,
            MAP_COLOR_BROWN,
        )),
        "minecraft:birch_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_SAND,
            MAP_COLOR_QUARTZ,
        )),
        "minecraft:jungle_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_DIRT,
            MAP_COLOR_PODZOL,
        )),
        "minecraft:acacia_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_ORANGE,
            MAP_COLOR_STONE,
        )),
        "minecraft:cherry_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_TERRACOTTA_WHITE,
            MAP_COLOR_TERRACOTTA_GRAY,
        )),
        "minecraft:dark_oak_log" => Some(MAP_COLOR_BROWN),
        "minecraft:pale_oak_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_QUARTZ,
            MAP_COLOR_STONE,
        )),
        "minecraft:mangrove_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_RED,
            MAP_COLOR_PODZOL,
        )),
        "minecraft:bamboo_block" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_YELLOW,
            MAP_COLOR_PLANT,
        )),
        "minecraft:stripped_spruce_log" => Some(MAP_COLOR_PODZOL),
        "minecraft:stripped_birch_log" => Some(MAP_COLOR_SAND),
        "minecraft:stripped_jungle_log" => Some(MAP_COLOR_DIRT),
        "minecraft:stripped_acacia_log" => Some(MAP_COLOR_ORANGE),
        "minecraft:stripped_cherry_log" => Some(rotated_pillar_map_color(
            properties,
            MAP_COLOR_TERRACOTTA_WHITE,
            MAP_COLOR_TERRACOTTA_PINK,
        )),
        "minecraft:stripped_dark_oak_log" => Some(MAP_COLOR_BROWN),
        "minecraft:stripped_pale_oak_log" => Some(MAP_COLOR_QUARTZ),
        "minecraft:stripped_oak_log" => Some(MAP_COLOR_WOOD),
        _ => None,
    }
}

fn construction_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "mossy_cobblestone"
        | "cobblestone_stairs"
        | "cobblestone_slab"
        | "cobblestone_wall"
        | "mossy_cobblestone_stairs"
        | "mossy_cobblestone_slab"
        | "mossy_cobblestone_wall"
        | "stone_bricks"
        | "mossy_stone_bricks"
        | "cracked_stone_bricks"
        | "chiseled_stone_bricks"
        | "stone_brick_stairs"
        | "stone_brick_slab"
        | "stone_brick_wall"
        | "mossy_stone_brick_stairs"
        | "mossy_stone_brick_slab"
        | "mossy_stone_brick_wall"
        | "stone_stairs"
        | "stone_slab"
        | "smooth_stone"
        | "smooth_stone_slab"
        | "andesite_stairs"
        | "andesite_slab"
        | "andesite_wall"
        | "polished_andesite_stairs"
        | "polished_andesite_slab" => MAP_COLOR_STONE,
        "granite_stairs"
        | "granite_slab"
        | "granite_wall"
        | "polished_granite_stairs"
        | "polished_granite_slab" => MAP_COLOR_DIRT,
        "diorite_stairs"
        | "diorite_slab"
        | "diorite_wall"
        | "polished_diorite_stairs"
        | "polished_diorite_slab" => MAP_COLOR_QUARTZ,
        "sandstone_stairs"
        | "sandstone_slab"
        | "sandstone_wall"
        | "cut_sandstone_slab"
        | "smooth_sandstone"
        | "smooth_sandstone_stairs"
        | "smooth_sandstone_slab" => MAP_COLOR_SAND,
        "red_sandstone"
        | "chiseled_red_sandstone"
        | "cut_red_sandstone"
        | "red_sandstone_stairs"
        | "red_sandstone_slab"
        | "red_sandstone_wall"
        | "cut_red_sandstone_slab"
        | "smooth_red_sandstone"
        | "smooth_red_sandstone_stairs"
        | "smooth_red_sandstone_slab" => MAP_COLOR_ORANGE,
        "bricks" | "brick_stairs" | "brick_slab" | "brick_wall" => MAP_COLOR_RED,
        "mud_bricks" | "mud_brick_stairs" | "mud_brick_slab" | "mud_brick_wall" => {
            MAP_COLOR_TERRACOTTA_LIGHT_GRAY
        }
        "nether_brick_stairs"
        | "nether_brick_slab"
        | "nether_brick_wall"
        | "red_nether_brick_stairs"
        | "red_nether_brick_slab"
        | "red_nether_brick_wall" => MAP_COLOR_NETHER,
        _ => return None,
    })
}

fn resin_and_pale_garden_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "resin_block"
        | "resin_clump"
        | "resin_bricks"
        | "resin_brick_stairs"
        | "resin_brick_slab"
        | "resin_brick_wall"
        | "chiseled_resin_bricks" => MAP_COLOR_TERRACOTTA_ORANGE,
        "pale_moss_block" | "pale_moss_carpet" | "pale_hanging_moss" => MAP_COLOR_LIGHT_GRAY,
        "open_eyeblossom" => MAP_COLOR_ORANGE,
        "closed_eyeblossom" => MAP_COLOR_METAL,
        "firefly_bush" => MAP_COLOR_PLANT,
        _ => return None,
    })
}

fn crop_static_map_color(
    name: &str,
    properties: &std::collections::BTreeMap<String, String>,
) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "wheat" => {
            if properties
                .get("age")
                .and_then(|age| age.parse::<u8>().ok())
                .is_some_and(|age| age >= 6)
            {
                MAP_COLOR_YELLOW
            } else {
                MAP_COLOR_PLANT
            }
        }
        "carrots" | "potatoes" | "beetroots" | "torchflower_crop" | "pitcher_crop"
        | "pitcher_plant" | "cactus" => MAP_COLOR_PLANT,
        "cactus_flower" => MAP_COLOR_PINK,
        "nether_wart" => MAP_COLOR_RED,
        _ => return None,
    })
}

fn produce_and_fungus_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "brown_mushroom" => MAP_COLOR_BROWN,
        "red_mushroom" | "red_mushroom_block" => MAP_COLOR_RED,
        "brown_mushroom_block" => MAP_COLOR_DIRT,
        "mushroom_stem" => MAP_COLOR_WOOL,
        "pumpkin" | "carved_pumpkin" | "jack_o_lantern" => MAP_COLOR_ORANGE,
        "melon" => MAP_COLOR_LIGHT_GREEN,
        "hay_block" => MAP_COLOR_YELLOW,
        "dried_kelp_block" => MAP_COLOR_GREEN,
        _ => return None,
    })
}

fn natural_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "oak_sapling"
        | "spruce_sapling"
        | "birch_sapling"
        | "jungle_sapling"
        | "acacia_sapling"
        | "dark_oak_sapling"
        | "mangrove_propagule"
        | "azalea_leaves"
        | "flowering_azalea_leaves"
        | "cave_vines"
        | "cave_vines_plant"
        | "spore_blossom"
        | "azalea"
        | "flowering_azalea"
        | "big_dripleaf"
        | "big_dripleaf_stem"
        | "small_dripleaf"
        | "bamboo"
        | "sweet_berry_bush"
        | "cocoa"
        | "dandelion"
        | "golden_dandelion"
        | "torchflower"
        | "poppy"
        | "blue_orchid"
        | "allium"
        | "azure_bluet"
        | "red_tulip"
        | "orange_tulip"
        | "white_tulip"
        | "pink_tulip"
        | "oxeye_daisy"
        | "cornflower"
        | "wither_rose"
        | "lily_of_the_valley"
        | "sunflower"
        | "lilac"
        | "rose_bush"
        | "peony" => MAP_COLOR_PLANT,
        "seagrass" | "tall_seagrass" | "kelp" | "kelp_plant" | "frogspawn" => MAP_COLOR_WATER,
        "cherry_sapling" | "cherry_leaves" => MAP_COLOR_PINK,
        "pale_oak_sapling" | "pale_oak_leaves" => MAP_COLOR_METAL,
        "dead_bush" => MAP_COLOR_WOOD,
        "bamboo_sapling" => MAP_COLOR_WOOD,
        "turtle_egg" => MAP_COLOR_SAND,
        "short_dry_grass" | "tall_dry_grass" => MAP_COLOR_YELLOW,
        "pointed_dripstone" | "dripstone_block" => MAP_COLOR_TERRACOTTA_BROWN,
        "moss_carpet" | "moss_block" => MAP_COLOR_GREEN,
        "hanging_roots" | "rooted_dirt" => MAP_COLOR_DIRT,
        "mud" => MAP_COLOR_TERRACOTTA_CYAN,
        "sniffer_egg" => MAP_COLOR_RED,
        "dried_ghast" => MAP_COLOR_GRAY,
        _ => return None,
    })
}

fn aquatic_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    Some(match name {
        "sea_pickle" => MAP_COLOR_GREEN,
        "conduit" => MAP_COLOR_DIAMOND,
        _ => coral_static_map_color(name)?,
    })
}

fn coral_static_map_color(name: &str) -> Option<u32> {
    let (dead, name) = name
        .strip_prefix("dead_")
        .map(|name| (true, name))
        .unwrap_or((false, name));
    let family = name
        .strip_suffix("_coral_wall_fan")
        .or_else(|| name.strip_suffix("_coral_fan"))
        .or_else(|| name.strip_suffix("_coral_block"))
        .or_else(|| name.strip_suffix("_coral"))?;
    if dead {
        return matches!(family, "tube" | "brain" | "bubble" | "fire" | "horn")
            .then_some(MAP_COLOR_GRAY);
    }
    Some(match family {
        "tube" => MAP_COLOR_BLUE,
        "brain" => MAP_COLOR_PINK,
        "bubble" => MAP_COLOR_PURPLE,
        "fire" => MAP_COLOR_RED,
        "horn" => MAP_COLOR_YELLOW,
        _ => return None,
    })
}

fn utility_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    if copper_bars_or_chain_default_none(name) {
        return Some(MAP_COLOR_NONE);
    }
    Some(match name {
        "bedrock"
        | "sticky_piston"
        | "piston"
        | "spawner"
        | "crafter"
        | "trial_spawner"
        | "vault"
        | "stone_pressure_plate"
        | "cauldron"
        | "lava_cauldron"
        | "powder_snow_cauldron"
        | "hopper"
        | "smoker"
        | "blast_furnace"
        | "ender_chest"
        | "observer"
        | "stonecutter" => MAP_COLOR_STONE,
        "note_block" | "bookshelf" | "chiseled_bookshelf" | "chest" | "crafting_table" | "loom"
        | "barrel" | "cartography_table" | "fletching_table" | "lectern" | "smithing_table"
        | "composter" | "beehive" | "trapped_chest" | "daylight_detector" => MAP_COLOR_WOOD,
        "scaffolding" => MAP_COLOR_SAND,
        "glowstone" => MAP_COLOR_SAND,
        "campfire" | "soul_campfire" => MAP_COLOR_PODZOL,
        "cobweb" => MAP_COLOR_WOOL,
        "tnt" => MAP_COLOR_FIRE,
        "fire" => MAP_COLOR_FIRE,
        "soul_fire" => MAP_COLOR_LIGHT_BLUE,
        "creaking_heart" => MAP_COLOR_ORANGE,
        "decorated_pot" => MAP_COLOR_TERRACOTTA_RED,
        "honey_block" | "honeycomb_block" => MAP_COLOR_ORANGE,
        "redstone_lamp" => MAP_COLOR_TERRACOTTA_ORANGE,
        "target" => MAP_COLOR_QUARTZ,
        "enchanting_table" => MAP_COLOR_RED,
        "bee_nest" => MAP_COLOR_YELLOW,
        "beacon" => MAP_COLOR_DIAMOND,
        "command_block" => MAP_COLOR_BROWN,
        "repeating_command_block" => MAP_COLOR_PURPLE,
        "chain_command_block" => MAP_COLOR_GREEN,
        "structure_block" | "jigsaw" | "test_block" => MAP_COLOR_LIGHT_GRAY,
        "glass"
        | "glass_pane"
        | "iron_bars"
        | "iron_chain"
        | "ladder"
        | "torch"
        | "wall_torch"
        | "redstone_torch"
        | "redstone_wall_torch"
        | "soul_torch"
        | "soul_wall_torch"
        | "copper_torch"
        | "copper_wall_torch"
        | "end_rod"
        | "powered_rail"
        | "detector_rail"
        | "rail"
        | "lever"
        | "repeater"
        | "tripwire_hook"
        | "tripwire"
        | "comparator"
        | "activator_rail"
        | "skeleton_skull"
        | "skeleton_wall_skull"
        | "wither_skeleton_skull"
        | "wither_skeleton_wall_skull"
        | "zombie_head"
        | "zombie_wall_head"
        | "player_head"
        | "player_wall_head"
        | "creeper_head"
        | "creeper_wall_head"
        | "dragon_head"
        | "dragon_wall_head"
        | "piglin_head"
        | "piglin_wall_head" => MAP_COLOR_NONE,
        "light_weighted_pressure_plate" | "bell" => MAP_COLOR_GOLD,
        "heavy_weighted_pressure_plate"
        | "iron_door"
        | "iron_trapdoor"
        | "brewing_stand"
        | "lantern"
        | "soul_lantern"
        | "grindstone"
        | "lodestone"
        | "heavy_core" => MAP_COLOR_METAL,
        _ => return None,
    })
}

fn copper_bars_or_chain_default_none(name: &str) -> bool {
    let name = name.strip_prefix("waxed_").unwrap_or(name);
    let name = name
        .strip_prefix("exposed_")
        .or_else(|| name.strip_prefix("weathered_"))
        .or_else(|| name.strip_prefix("oxidized_"))
        .unwrap_or(name);
    matches!(name, "copper_bars" | "copper_chain")
}

fn rotated_pillar_map_color(
    properties: &std::collections::BTreeMap<String, String>,
    top_color: u32,
    side_color: u32,
) -> u32 {
    if properties.get("axis").is_some_and(|axis| axis == "y") {
        top_color
    } else {
        side_color
    }
}

fn terrain_particle_provider_accepts_block_state(block_state_id: i32) -> bool {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let Some(block_state) = block_states.by_id(block_state_id) else {
        return true;
    };
    !block_name_is_air(&block_state.name)
        && block_state.name != "minecraft:moving_piston"
        && block_name_should_spawn_terrain_particles(&block_state.name)
}

fn destroy_block_effect_accepts_block_state(block_state_id: i32) -> bool {
    let block_states = bbb_world::BlockStateRegistry::vanilla_26_1();
    let Some(block_state) = block_states.by_id(block_state_id) else {
        return true;
    };
    !block_name_is_air(&block_state.name)
        && block_name_should_spawn_terrain_particles(&block_state.name)
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
    rgb_particle_color_u32(color as u32)
}

fn rgb_particle_color_u32(color: u32) -> [f32; 4] {
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

fn destroy_block_axis_count(width: f64) -> usize {
    ((width / DESTROY_BLOCK_PARTICLE_DENSITY).ceil() as usize).max(2)
}

fn destroy_block_box_width(min: f64, max: f64) -> f64 {
    (max - min).min(DESTROY_BLOCK_FULL_BOX_WIDTH)
}

fn destroy_block_box_particle_count(min: [f64; 3], max: [f64; 3]) -> usize {
    destroy_block_axis_count(destroy_block_box_width(min[0], max[0]))
        * destroy_block_axis_count(destroy_block_box_width(min[1], max[1]))
        * destroy_block_axis_count(destroy_block_box_width(min[2], max[2]))
}

fn destroy_block_shape_boxes(block_state_id: i32) -> Vec<([f64; 3], [f64; 3])> {
    bbb_world::BlockStateRegistry::vanilla_26_1()
        .by_id(block_state_id)
        .and_then(crate::block_outline::block_state_shape_boxes)
        .filter(|boxes| !boxes.is_empty())
        .unwrap_or_else(|| vec![([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])])
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

fn growth_particle_position_has_support(
    pos: BlockPos,
    position: Vec3d,
    support: LevelEventGrowthParticleSupport,
) -> bool {
    let support_x = position.x.floor() as i32;
    let Some(support_y) = (position.y.floor() as i32).checked_sub(1) else {
        return false;
    };
    let support_z = position.z.floor() as i32;
    let (Some(dx), Some(dz)) = (support_x.checked_sub(pos.x), support_z.checked_sub(pos.z)) else {
        return false;
    };
    pos.y
        .checked_sub(1)
        .is_some_and(|below_y| support_y == below_y && support.contains(dx, dz))
}

fn growth_particle_support_bit(dx: i32, dz: i32) -> Option<u64> {
    if !(-GROWTH_PARTICLE_SUPPORT_RADIUS..=GROWTH_PARTICLE_SUPPORT_RADIUS).contains(&dx)
        || !(-GROWTH_PARTICLE_SUPPORT_RADIUS..=GROWTH_PARTICLE_SUPPORT_RADIUS).contains(&dz)
    {
        return None;
    }
    let index = (dz + GROWTH_PARTICLE_SUPPORT_RADIUS) * GROWTH_PARTICLE_SUPPORT_WIDTH
        + (dx + GROWTH_PARTICLE_SUPPORT_RADIUS);
    Some(1_u64 << index)
}

fn pointed_dripstone_drip_position(event: &LevelEvent) -> Vec3d {
    let (x_offset, z_offset) = pointed_dripstone_xz_offset(event.pos.x, event.pos.z);
    Vec3d {
        x: f64::from(event.pos.x) + 0.5 + x_offset,
        y: f64::from(event.pos.y) + POINTED_DRIPSTONE_DRIP_Y_OFFSET,
        z: f64::from(event.pos.z) + 0.5 + z_offset,
    }
}

fn pointed_dripstone_xz_offset(x: i32, z: i32) -> (f64, f64) {
    let seed = java_block_position_seed(x, 0, z);
    let x_offset = ((((seed & 15) as f32) / 15.0 - 0.5) * 0.5).clamp(
        -POINTED_DRIPSTONE_MAX_HORIZONTAL_OFFSET,
        POINTED_DRIPSTONE_MAX_HORIZONTAL_OFFSET,
    );
    let z_offset = ((((seed >> 8 & 15) as f32) / 15.0 - 0.5) * 0.5).clamp(
        -POINTED_DRIPSTONE_MAX_HORIZONTAL_OFFSET,
        POINTED_DRIPSTONE_MAX_HORIZONTAL_OFFSET,
    );
    (f64::from(x_offset), f64::from(z_offset))
}

fn java_block_position_seed(x: i32, y: i32, z: i32) -> i64 {
    let seed = i64::from(x.wrapping_mul(3_129_871))
        ^ i64::from(z).wrapping_mul(116_129_781)
        ^ i64::from(y);
    seed.wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11))
        >> 16
}

const COMPOSTER_FILL_LEVEL_EVENT: i32 = 1500;
const LAVA_EXTINGUISH_LEVEL_EVENT: i32 = 1501;
const REDSTONE_TORCH_BURNOUT_LEVEL_EVENT: i32 = 1502;
const END_PORTAL_FRAME_FILL_LEVEL_EVENT: i32 = 1503;
const DRIPSTONE_DRIP_LEVEL_EVENT: i32 = 1504;
const PLANT_GROWTH_LEVEL_EVENT: i32 = 1505;
const DISPENSER_SMOKE_LEVEL_EVENT: i32 = 2000;
const DESTROY_BLOCK_PARTICLES_LEVEL_EVENT: i32 = 2001;
const POTION_BREAK_LEVEL_EVENT: i32 = 2002;
const ENDER_EYE_BREAK_LEVEL_EVENT: i32 = 2003;
const BLAZE_SMOKE_LEVEL_EVENT: i32 = 2004;
const DRAGON_FIREBALL_EXPLODE_LEVEL_EVENT: i32 = 2006;
const INSTANT_POTION_BREAK_LEVEL_EVENT: i32 = 2007;
const EXPLOSION_LEVEL_EVENT: i32 = 2008;
const SPLASH_CLOUD_LEVEL_EVENT: i32 = 2009;
const DISPENSER_WHITE_SMOKE_LEVEL_EVENT: i32 = 2010;
const BEE_GROWTH_PARTICLES_LEVEL_EVENT: i32 = 2011;
const TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT: i32 = 2012;
const SMASH_ATTACK_PARTICLES_LEVEL_EVENT: i32 = 2013;
const END_GATEWAY_SPAWN_LEVEL_EVENT: i32 = 3000;
const ELECTRIC_SPARK_LEVEL_EVENT: i32 = 3002;
const WAX_ON_LEVEL_EVENT: i32 = 3003;
const WAX_OFF_LEVEL_EVENT: i32 = 3004;
const SCRAPE_LEVEL_EVENT: i32 = 3005;
const SCULK_CHARGE_LEVEL_EVENT: i32 = 3006;
const SCULK_SHRIEK_PARTICLES_LEVEL_EVENT: i32 = 3007;
const BRUSH_BLOCK_COMPLETE_LEVEL_EVENT: i32 = 3008;
const EGG_CRACK_LEVEL_EVENT: i32 = 3009;
const TRIAL_SPAWNER_SPAWN_PARTICLES_LEVEL_EVENT: i32 = 3011;
const TRIAL_SPAWNER_SPAWN_MOB_LEVEL_EVENT: i32 = 3012;
const TRIAL_SPAWNER_DETECT_PLAYER_LEVEL_EVENT: i32 = 3013;
const TRIAL_SPAWNER_EJECT_ITEM_LEVEL_EVENT: i32 = 3014;
const VAULT_ACTIVATE_LEVEL_EVENT: i32 = 3015;
const VAULT_DEACTIVATE_LEVEL_EVENT: i32 = 3016;
const TRIAL_SPAWNER_EJECT_ITEM_PARTICLES_LEVEL_EVENT: i32 = 3017;
const COBWEB_PLACE_PARTICLES_LEVEL_EVENT: i32 = 3018;
const TRIAL_SPAWNER_DETECT_PLAYER_OMINOUS_LEVEL_EVENT: i32 = 3019;
const TRIAL_SPAWNER_OMINOUS_ACTIVATE_LEVEL_EVENT: i32 = 3020;
const TRIAL_SPAWNER_SPAWN_ITEM_LEVEL_EVENT: i32 = 3021;
const BLOCK_PARTICLE_TYPE_ID: i32 = 1;
const BLOCK_MARKER_PARTICLE_TYPE_ID: i32 = 2;
const CLOUD_PARTICLE_TYPE_ID: i32 = 4;
const DRAGON_BREATH_PARTICLE_TYPE_ID: i32 = 8;
const DUST_PARTICLE_TYPE_ID: i32 = 14;
const DUST_COLOR_TRANSITION_PARTICLE_TYPE_ID: i32 = 15;
const EFFECT_PARTICLE_TYPE_ID: i32 = 16;
const ELDER_GUARDIAN_PARTICLE_TYPE_ID: i32 = 17;
const ENTITY_EFFECT_PARTICLE_TYPE_ID: i32 = 21;
const EXPLOSION_EMITTER_PARTICLE_TYPE_ID: i32 = 22;
const EXPLOSION_PARTICLE_TYPE_ID: i32 = 23;
const GUST_PARTICLE_TYPE_ID: i32 = 24;
const GUST_EMITTER_LARGE_PARTICLE_TYPE_ID: i32 = 26;
const GUST_EMITTER_SMALL_PARTICLE_TYPE_ID: i32 = 27;
const FALLING_DUST_PARTICLE_TYPE_ID: i32 = 29;
const FLAME_PARTICLE_TYPE_ID: i32 = 32;
const TINTED_LEAVES_PARTICLE_TYPE_ID: i32 = 36;
const SCULK_CHARGE_PARTICLE_TYPE_ID: i32 = 38;
const SCULK_CHARGE_POP_PARTICLE_TYPE_ID: i32 = 39;
const SOUL_FIRE_FLAME_PARTICLE_TYPE_ID: i32 = 40;
const FLASH_PARTICLE_TYPE_ID: i32 = 42;
const HAPPY_VILLAGER_PARTICLE_TYPE_ID: i32 = 43;
const COMPOSTER_PARTICLE_TYPE_ID: i32 = 44;
const INSTANT_EFFECT_PARTICLE_TYPE_ID: i32 = 46;
const ITEM_PARTICLE_TYPE_ID: i32 = 47;
const VIBRATION_PARTICLE_TYPE_ID: i32 = 48;
const TRAIL_PARTICLE_TYPE_ID: i32 = 49;
const ITEM_SLIME_PARTICLE_TYPE_ID: i32 = 52;
const ITEM_COBWEB_PARTICLE_TYPE_ID: i32 = 53;
const ITEM_SNOWBALL_PARTICLE_TYPE_ID: i32 = 54;
const LARGE_SMOKE_PARTICLE_TYPE_ID: i32 = 55;
const LAVA_PARTICLE_TYPE_ID: i32 = 56;
const POOF_PARTICLE_TYPE_ID: i32 = 59;
const PORTAL_PARTICLE_TYPE_ID: i32 = 60;
const SMOKE_PARTICLE_TYPE_ID: i32 = 62;
const WHITE_SMOKE_PARTICLE_TYPE_ID: i32 = 63;
const SMALL_FLAME_PARTICLE_TYPE_ID: i32 = 93;
const DRIPPING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID: i32 = 95;
const DRIPPING_DRIPSTONE_WATER_PARTICLE_TYPE_ID: i32 = 97;
const ELECTRIC_SPARK_PARTICLE_TYPE_ID: i32 = 103;
const WAX_ON_PARTICLE_TYPE_ID: i32 = 101;
const WAX_OFF_PARTICLE_TYPE_ID: i32 = 102;
const SCRAPE_PARTICLE_TYPE_ID: i32 = 104;
const SHRIEK_PARTICLE_TYPE_ID: i32 = 105;
const EGG_CRACK_PARTICLE_TYPE_ID: i32 = 106;
const TRIAL_SPAWNER_DETECTED_PLAYER_PARTICLE_TYPE_ID: i32 = 108;
const TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS_PARTICLE_TYPE_ID: i32 = 109;
const DUST_PILLAR_PARTICLE_TYPE_ID: i32 = 111;
const TRIAL_OMEN_PARTICLE_TYPE_ID: i32 = 114;
const BLOCK_CRUMBLE_PARTICLE_TYPE_ID: i32 = 115;
const BLOCK_FACE_DIRECTIONS: &[(i32, i32, i32)] = &[
    (0, -1, 0),
    (0, 1, 0),
    (0, 0, -1),
    (0, 0, 1),
    (-1, 0, 0),
    (1, 0, 0),
];
const BLOCK_FACE_DIRECTION_DOWN: (i32, i32, i32) = (0, -1, 0);
const BLOCK_FACE_DIRECTION_UP: (i32, i32, i32) = (0, 1, 0);
const BLOCK_FACE_STEP_FACTOR: f64 = 0.55;
const BLOCK_FACE_PARTICLE_MIN: i32 = 3;
const BLOCK_FACE_PARTICLE_MAX: i32 = 5;
const SCULK_CHARGE_FULL_BLOCK_Y_FACTOR: f64 = 0.65;
const SCULK_CHARGE_FULL_BLOCK_SIDE_FACTOR: f64 = 0.57;
const SCULK_CHARGE_MULTIFACE_FACTOR: f64 = 0.35;
const SCULK_CHARGE_SPEED_VAR: f64 = 0.005;
const SCULK_CHARGE_POP_PARTIAL_BLOCK_SPREAD: f64 = 0.25;
const SCULK_CHARGE_POP_FULL_BLOCK_SPREAD: f64 = 0.45;
const SCULK_CHARGE_POP_SPEED: f64 = 0.07;
const ELECTRIC_SPARK_AXIS_RADIUS: f64 = 0.125;
const ELECTRIC_SPARK_AXIS_MIN: i32 = 10;
const ELECTRIC_SPARK_AXIS_MAX: i32 = 19;
const EGG_CRACK_PARTICLE_MAX: i32 = 6;
const ITEM_BREAK_PARTICLE_COUNT: i32 = 8;
const POTION_BREAK_ITEM_PARTICLE_COUNT: i32 = ITEM_BREAK_PARTICLE_COUNT;
const POTION_BREAK_SPELL_PARTICLE_COUNT: i32 = 100;
// Vanilla 26.1 BuiltInRegistries.ITEM ids from Items.java order.
const VANILLA_ENDER_EYE_ITEM_ID: i32 = 1129;
const VANILLA_SPLASH_POTION_ITEM_ID: i32 = 1292;
const EMPTY_ITEM_COMPONENT_PATCH_OPTION_LEN: usize = 2;
const ITEM_BREAK_HORIZONTAL_VELOCITY_SCALE: f64 = 0.15;
const ITEM_BREAK_VERTICAL_VELOCITY_SCALE: f64 = 0.2;
const DESTROY_BLOCK_PARTICLE_DENSITY: f64 = 0.25;
const DESTROY_BLOCK_FULL_BOX_WIDTH: f64 = 1.0;
const COMPOSTER_FILL_PARTICLE_COUNT: usize = 10;
const COMPOSTER_FILL_CENTER_HEIGHT_OFFSET: f64 = 0.03125;
const COMPOSTER_FILL_SIDE_OFFSET: f64 = 0.1875;
const COMPOSTER_FILL_WIDTH: f64 = 0.625;
const COMPOSTER_FILL_VELOCITY_SCALE: f64 = 0.02;
const SCULK_SHRIEKER_TOP_Y: f64 = 0.5;
const SCULK_SHRIEK_PARTICLE_COUNT: u32 = 10;
const SCULK_SHRIEK_DELAY_STEP_TICKS: u32 = 5;
const AIR_BLOCK_STATE_ID: i32 = 0;
const MAP_COLOR_NONE: u32 = 0;
const MAP_COLOR_SAND: u32 = 16_247_203;
const MAP_COLOR_WOOL: u32 = 13_092_807;
const MAP_COLOR_FIRE: u32 = 16_711_680;
const MAP_COLOR_ICE: u32 = 10_526_975;
const MAP_COLOR_SNOW: u32 = 16_777_215;
const MAP_COLOR_METAL: u32 = 10_987_431;
const MAP_COLOR_PLANT: u32 = 31_744;
const MAP_COLOR_CLAY: u32 = 10_791_096;
const MAP_COLOR_DIRT: u32 = 9_923_917;
const MAP_COLOR_STONE: u32 = 7_368_816;
const MAP_COLOR_WATER: u32 = 4_210_943;
const MAP_COLOR_WOOD: u32 = 9_402_184;
const MAP_COLOR_QUARTZ: u32 = 16_776_437;
const MAP_COLOR_ORANGE: u32 = 14_188_339;
const MAP_COLOR_MAGENTA: u32 = 11_685_080;
const MAP_COLOR_LIGHT_BLUE: u32 = 6_724_056;
const MAP_COLOR_YELLOW: u32 = 15_066_419;
const MAP_COLOR_LIGHT_GREEN: u32 = 8_375_321;
const MAP_COLOR_PINK: u32 = 15_892_389;
const MAP_COLOR_GRAY: u32 = 5_000_268;
const MAP_COLOR_LIGHT_GRAY: u32 = 10_066_329;
const MAP_COLOR_CYAN: u32 = 5_013_401;
const MAP_COLOR_PURPLE: u32 = 8_339_378;
const MAP_COLOR_BLUE: u32 = 3_361_970;
const MAP_COLOR_BROWN: u32 = 6_704_179;
const MAP_COLOR_GREEN: u32 = 6_717_235;
const MAP_COLOR_RED: u32 = 10_040_115;
const MAP_COLOR_BLACK: u32 = 1_644_825;
const MAP_COLOR_GOLD: u32 = 16_445_005;
const MAP_COLOR_DIAMOND: u32 = 6_085_589;
const MAP_COLOR_LAPIS: u32 = 4_882_687;
const MAP_COLOR_EMERALD: u32 = 55_610;
const MAP_COLOR_TERRACOTTA_WHITE: u32 = 13_742_497;
const MAP_COLOR_TERRACOTTA_ORANGE: u32 = 10_441_252;
const MAP_COLOR_TERRACOTTA_MAGENTA: u32 = 9_787_244;
const MAP_COLOR_TERRACOTTA_LIGHT_BLUE: u32 = 7_367_818;
const MAP_COLOR_TERRACOTTA_YELLOW: u32 = 12_223_780;
const MAP_COLOR_TERRACOTTA_LIGHT_GREEN: u32 = 6_780_213;
const MAP_COLOR_TERRACOTTA_GRAY: u32 = 3_746_083;
const MAP_COLOR_TERRACOTTA_PINK: u32 = 10_505_550;
const MAP_COLOR_TERRACOTTA_LIGHT_GRAY: u32 = 8_874_850;
const MAP_COLOR_TERRACOTTA_CYAN: u32 = 5_725_276;
const MAP_COLOR_TERRACOTTA_PURPLE: u32 = 8_014_168;
const MAP_COLOR_TERRACOTTA_BLUE: u32 = 4_996_700;
const MAP_COLOR_TERRACOTTA_BROWN: u32 = 4_993_571;
const MAP_COLOR_TERRACOTTA_GREEN: u32 = 5_001_770;
const MAP_COLOR_TERRACOTTA_RED: u32 = 9_321_518;
const MAP_COLOR_TERRACOTTA_BLACK: u32 = 2_430_480;
const MAP_COLOR_PODZOL: u32 = 8_476_209;
const MAP_COLOR_NETHER: u32 = 7_340_544;
const MAP_COLOR_CRIMSON_NYLIUM: u32 = 12_398_641;
const MAP_COLOR_CRIMSON_STEM: u32 = 9_715_553;
const MAP_COLOR_CRIMSON_HYPHAE: u32 = 6_035_741;
const MAP_COLOR_WARPED_NYLIUM: u32 = 1_474_182;
const MAP_COLOR_WARPED_STEM: u32 = 3_837_580;
const MAP_COLOR_WARPED_HYPHAE: u32 = 5_647_422;
const MAP_COLOR_WARPED_WART_BLOCK: u32 = 1_356_933;
const MAP_COLOR_DEEPSLATE: u32 = 6_579_300;
const MAP_COLOR_RAW_IRON: u32 = 14_200_723;
const MAP_COLOR_GLOW_LICHEN: u32 = 8_365_974;
const SMASH_ATTACK_CENTER_SPEED_SCALE: f64 = 0.2_f32 as f64;
const SMASH_ATTACK_RING_SPEED_SCALE: f64 = 0.05_f32 as f64;
const POINTED_DRIPSTONE_DRIP_Y_OFFSET: f64 = 0.25;
const POINTED_DRIPSTONE_MAX_HORIZONTAL_OFFSET: f32 = 0.125;
const GROWTH_PARTICLE_WIDE_SPREAD: f64 = 3.0;
const GROWTH_PARTICLE_WIDE_HEIGHT: f64 = 1.0;
const GROWTH_PARTICLE_WIDE_START_OFFSET: f64 = 0.5 - GROWTH_PARTICLE_WIDE_SPREAD;
const GROWTH_PARTICLE_SUPPORT_RADIUS: i32 = 3;
const GROWTH_PARTICLE_SUPPORT_WIDTH: i32 = GROWTH_PARTICLE_SUPPORT_RADIUS * 2 + 1;

fn smash_attack_particle_loop_count(count: i32, divisor: f32) -> usize {
    let limit = count as f32 / divisor;
    let mut loop_count = 0;
    while (loop_count as f32) < limit {
        loop_count += 1;
    }
    loop_count
}

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
    use bbb_pack::{SpriteAnimation, SpriteAnimationFrame};
    use std::{
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
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
    fn explosion_emitter_particle_commands_carry_explosion_child_template_without_definition() {
        let mut resolver = test_resolver(0);
        let batch = resolver.resolve_level_particles(&level_particles_packet(
            EXPLOSION_EMITTER_PARTICLE_TYPE_ID,
            0,
        ));

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.missing_definition_count, 0);
        let command = &batch.commands[0];
        assert_eq!(command.particle_type_id, EXPLOSION_EMITTER_PARTICLE_TYPE_ID);
        assert_eq!(command.particle_id, "minecraft:explosion_emitter");
        assert!(command.sprite_ids.is_empty());
        assert_eq!(command.child_spawn_templates.len(), 1);
        let child = &command.child_spawn_templates[0];
        assert_eq!(child.particle_type_id, EXPLOSION_PARTICLE_TYPE_ID);
        assert_eq!(child.particle_id, "minecraft:explosion");
        assert_eq!(child.sprite_ids, vec!["minecraft:explosion_0".to_string()]);
    }

    #[test]
    fn elder_guardian_particle_command_is_definitionless_special_group_input() {
        let mut resolver = test_resolver(0);
        let batch = resolver
            .resolve_level_particles(&level_particles_packet(ELDER_GUARDIAN_PARTICLE_TYPE_ID, 0));

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.missing_definition_count, 0);
        let command = &batch.commands[0];
        assert_eq!(command.particle_type_id, ELDER_GUARDIAN_PARTICLE_TYPE_ID);
        assert_eq!(command.particle_id, "minecraft:elder_guardian");
        assert!(command.sprite_ids.is_empty());
        assert!(command.child_spawn_templates.is_empty());
    }

    #[test]
    fn terrain_and_item_atlas_particles_are_definitionless_submission_inputs() {
        let mut resolver = test_resolver(0);
        for (particle_type_id, particle_id, raw_options, block_state_id, item) in [
            (
                BLOCK_PARTICLE_TYPE_ID,
                "minecraft:block",
                block_particle_options(129),
                Some(129),
                None,
            ),
            (
                BLOCK_MARKER_PARTICLE_TYPE_ID,
                "minecraft:block_marker",
                block_particle_options(2),
                Some(2),
                None,
            ),
            (
                DUST_PILLAR_PARTICLE_TYPE_ID,
                "minecraft:dust_pillar",
                block_particle_options(3),
                Some(3),
                None,
            ),
            (
                BLOCK_CRUMBLE_PARTICLE_TYPE_ID,
                "minecraft:block_crumble",
                block_particle_options(4),
                Some(4),
                None,
            ),
            (
                ITEM_PARTICLE_TYPE_ID,
                "minecraft:item",
                item_particle_options(5, 6, 0),
                None,
                Some(ParticleItemOptionState {
                    item_id: 5,
                    count: 6,
                    component_patch_len: 2,
                }),
            ),
            (
                ITEM_SLIME_PARTICLE_TYPE_ID,
                "minecraft:item_slime",
                Vec::new(),
                None,
                None,
            ),
            (
                ITEM_COBWEB_PARTICLE_TYPE_ID,
                "minecraft:item_cobweb",
                Vec::new(),
                None,
                None,
            ),
            (
                ITEM_SNOWBALL_PARTICLE_TYPE_ID,
                "minecraft:item_snowball",
                Vec::new(),
                None,
                None,
            ),
        ] {
            let mut packet = level_particles_packet(particle_type_id, 0);
            packet.particle.raw_options = raw_options.clone();
            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{particle_id}");
            assert_eq!(batch.missing_definition_count, 0, "{particle_id}");
            assert_eq!(batch.unknown_particle_type_count, 0, "{particle_id}");
            let command = &batch.commands[0];
            assert_eq!(command.particle_type_id, particle_type_id, "{particle_id}");
            assert_eq!(command.particle_id, particle_id, "{particle_id}");
            assert!(command.sprite_ids.is_empty(), "{particle_id}");
            assert_eq!(command.raw_options_len, raw_options.len(), "{particle_id}");
            assert_eq!(
                command.option_block.map(|option| option.block_state_id),
                block_state_id,
                "{particle_id}"
            );
            assert_eq!(command.option_item, item, "{particle_id}");
        }
    }

    #[test]
    fn terrain_particle_commands_use_installed_block_sprite_ids() {
        let mut resolver = test_resolver(0);
        let stone_id = test_block_state_id("minecraft:stone", []);
        resolver
            .terrain_particle_sprite_ids
            .insert(stone_id, "minecraft:block/stone".to_string());

        for (particle_type_id, particle_id) in [
            (BLOCK_PARTICLE_TYPE_ID, "minecraft:block"),
            (BLOCK_MARKER_PARTICLE_TYPE_ID, "minecraft:block_marker"),
            (DUST_PILLAR_PARTICLE_TYPE_ID, "minecraft:dust_pillar"),
            (BLOCK_CRUMBLE_PARTICLE_TYPE_ID, "minecraft:block_crumble"),
        ] {
            let mut packet = level_particles_packet(particle_type_id, 0);
            packet.particle.raw_options = block_particle_options(stone_id);
            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{particle_id}");
            assert_eq!(
                batch.commands[0].sprite_ids,
                vec!["minecraft:block/stone".to_string()],
                "{particle_id}"
            );
        }
    }

    #[test]
    fn terrain_particle_commands_use_installed_block_tint_colors() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());
        let redstone_id = test_block_state_id(
            "minecraft:redstone_wire",
            [
                ("east", "up"),
                ("north", "up"),
                ("power", "15"),
                ("south", "up"),
                ("west", "up"),
            ],
        );

        for (particle_type_id, particle_id) in [
            (BLOCK_PARTICLE_TYPE_ID, "minecraft:block"),
            (DUST_PILLAR_PARTICLE_TYPE_ID, "minecraft:dust_pillar"),
            (BLOCK_CRUMBLE_PARTICLE_TYPE_ID, "minecraft:block_crumble"),
        ] {
            let mut packet = level_particles_packet(particle_type_id, 0);
            packet.particle.raw_options = block_particle_options(redstone_id);
            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{particle_id}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option_06(255, 50, 0)),
                "{particle_id}"
            );
        }

        let mut marker = level_particles_packet(BLOCK_MARKER_PARTICLE_TYPE_ID, 0);
        marker.particle.raw_options = block_particle_options(redstone_id);
        let marker_batch = resolver.resolve_level_particles(&marker);
        assert_eq!(marker_batch.len(), 1);
        assert_eq!(marker_batch.commands[0].option_color, None);
    }

    #[test]
    fn generic_item_particle_uses_installed_default_item_sprite_for_empty_component_patch() {
        let mut resolver = test_resolver(0);
        resolver.default_item_particle_sprite_ids.insert(
            5,
            vec![
                "minecraft:item/apple".to_string(),
                "minecraft:item/apple_overlay".to_string(),
            ],
        );

        let mut empty_patch = level_particles_packet(ITEM_PARTICLE_TYPE_ID, 0);
        empty_patch.particle.raw_options = item_particle_options(5, 6, 0);
        let empty_batch = resolver.resolve_level_particles(&empty_patch);

        assert_eq!(empty_batch.len(), 1);
        assert_eq!(
            empty_batch.commands[0].sprite_ids,
            vec![
                "minecraft:item/apple".to_string(),
                "minecraft:item/apple_overlay".to_string()
            ]
        );
        assert_eq!(
            empty_batch.commands[0].option_item,
            Some(ParticleItemOptionState {
                item_id: 5,
                count: 6,
                component_patch_len: EMPTY_ITEM_COMPONENT_PATCH_OPTION_LEN,
            })
        );

        let mut non_empty_patch = level_particles_packet(ITEM_PARTICLE_TYPE_ID, 0);
        non_empty_patch.particle.raw_options = item_particle_options(5, 6, 1);
        let non_empty_batch = resolver.resolve_level_particles(&non_empty_patch);

        assert_eq!(non_empty_batch.len(), 1);
        assert!(non_empty_batch.commands[0].sprite_ids.is_empty());
        assert_eq!(
            non_empty_batch.commands[0].option_item,
            Some(ParticleItemOptionState {
                item_id: 5,
                count: 6,
                component_patch_len: EMPTY_ITEM_COMPONENT_PATCH_OPTION_LEN,
            })
        );
    }

    #[test]
    fn terrain_particle_providers_reject_vanilla_filtered_block_states() {
        let air_id = test_block_state_id("minecraft:air", []);
        let moving_piston_id = test_block_state_id(
            "minecraft:moving_piston",
            [("facing", "north"), ("type", "normal")],
        );
        let barrier_id = test_block_state_id("minecraft:barrier", [("waterlogged", "false")]);
        let structure_void_id = test_block_state_id("minecraft:structure_void", []);
        let stone_id = test_block_state_id("minecraft:stone", []);

        for particle_type_id in [
            BLOCK_PARTICLE_TYPE_ID,
            DUST_PILLAR_PARTICLE_TYPE_ID,
            BLOCK_CRUMBLE_PARTICLE_TYPE_ID,
        ] {
            for block_state_id in [air_id, moving_piston_id, barrier_id, structure_void_id] {
                let mut resolver = test_resolver(0);
                let mut packet = level_particles_packet(particle_type_id, 0);
                packet.particle.raw_options = block_particle_options(block_state_id);

                let batch = resolver.resolve_level_particles(&packet);

                assert_eq!(batch.len(), 0, "{particle_type_id} {block_state_id}");
                assert_eq!(batch.missing_definition_count, 0);
                assert_eq!(batch.unknown_particle_type_count, 0);
            }

            let mut resolver = test_resolver(0);
            let mut packet = level_particles_packet(particle_type_id, 0);
            packet.particle.raw_options = block_particle_options(stone_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{particle_type_id}");
            assert_eq!(
                batch.commands[0].option_block,
                Some(ParticleBlockOptionState {
                    block_state_id: stone_id
                })
            );
        }
    }

    #[test]
    fn block_marker_provider_keeps_invisible_and_no_terrain_particle_states() {
        let barrier_id = test_block_state_id("minecraft:barrier", [("waterlogged", "false")]);
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(BLOCK_MARKER_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(barrier_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.commands[0].particle_id, "minecraft:block_marker");
        assert_eq!(
            batch.commands[0].option_block,
            Some(ParticleBlockOptionState {
                block_state_id: barrier_id
            })
        );
    }

    #[test]
    fn terrain_particle_provider_rejection_preserves_packet_random_sequence() {
        let barrier_id = test_block_state_id("minecraft:barrier", [("waterlogged", "false")]);
        let stone_id = test_block_state_id("minecraft:stone", []);
        let mut rejected_resolver = test_resolver(42);
        let mut accepted_resolver = test_resolver(42);
        let mut rejected = level_particles_packet(BLOCK_PARTICLE_TYPE_ID, 2);
        rejected.particle.raw_options = block_particle_options(barrier_id);
        let mut accepted = level_particles_packet(BLOCK_PARTICLE_TYPE_ID, 2);
        accepted.particle.raw_options = block_particle_options(stone_id);

        let rejected_batch = rejected_resolver.resolve_level_particles(&rejected);
        let accepted_batch = accepted_resolver.resolve_level_particles(&accepted);
        assert_eq!(rejected_batch.len(), 0);
        assert_eq!(accepted_batch.len(), 2);

        let next_rejected = rejected_resolver
            .resolve_level_particles(&level_particles_packet(SMOKE_PARTICLE_TYPE_ID, 1));
        let next_accepted = accepted_resolver
            .resolve_level_particles(&level_particles_packet(SMOKE_PARTICLE_TYPE_ID, 1));

        assert_eq!(next_rejected.len(), 1);
        assert_eq!(next_accepted.len(), 1);
        assert_eq!(
            next_rejected.commands[0].position,
            next_accepted.commands[0].position
        );
        assert_eq!(
            next_rejected.commands[0].velocity,
            next_accepted.commands[0].velocity
        );
    }

    #[test]
    fn falling_dust_decodes_block_particle_option_metadata() {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(321);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        assert_eq!(batch.missing_definition_count, 0);
        let command = &batch.commands[0];
        assert_eq!(command.particle_type_id, FALLING_DUST_PARTICLE_TYPE_ID);
        assert_eq!(command.particle_id, "minecraft:falling_dust");
        assert_eq!(command.sprite_ids, vec!["minecraft:generic_0".to_string()]);
        assert_eq!(
            command.option_block,
            Some(ParticleBlockOptionState {
                block_state_id: 321
            })
        );
        assert_eq!(command.option_item, None);
        assert_eq!(command.option_color, None);
        assert_eq!(command.raw_options_len, block_particle_options(321).len());
    }

    #[test]
    fn falling_dust_decodes_falling_block_dust_colors() {
        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:sand", []),
                "minecraft:sand",
                rgb_option(0xDB, 0xD3, 0xA0),
            ),
            (
                test_block_state_id("minecraft:red_sand", []),
                "minecraft:red_sand",
                rgb_option(0xA9, 0x58, 0x21),
            ),
            (
                test_block_state_id("minecraft:gravel", []),
                "minecraft:gravel",
                rgb_option(0x80, 0x7C, 0x7B),
            ),
            (
                test_block_state_id("minecraft:dragon_egg", []),
                "minecraft:dragon_egg",
                rgb_option(0x00, 0x00, 0x00),
            ),
            (
                test_block_state_id("minecraft:anvil", [("facing", "north")]),
                "minecraft:anvil",
                rgb_option(0xA7, 0xA7, 0xA7),
            ),
            (
                test_block_state_id("minecraft:red_concrete_powder", []),
                "minecraft:red_concrete_powder",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id("minecraft:black_concrete_powder", []),
                "minecraft:black_concrete_powder",
                rgb_option(0x19, 0x19, 0x19),
            ),
        ] {
            let mut resolver = test_resolver(0);
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_block,
                Some(ParticleBlockOptionState { block_state_id }),
                "{block_name}"
            );
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_installed_block_tint_for_non_falling_block_colors() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());
        let lily_pad_id = test_block_state_id("minecraft:lily_pad", []);
        let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = block_particle_options(lily_pad_id);

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        assert_eq!(
            batch.commands[0].option_color,
            Some(rgb_option(0x20, 0x80, 0x30))
        );
    }

    #[test]
    fn falling_dust_uses_map_color_fallback_for_non_tinted_blocks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:stone", []),
                "minecraft:stone",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:dirt", []),
                "minecraft:dirt",
                rgb_option(0x97, 0x6d, 0x4d),
            ),
            (
                test_block_state_id("minecraft:oak_planks", []),
                "minecraft:oak_planks",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id("minecraft:birch_planks", []),
                "minecraft:birch_planks",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id("minecraft:acacia_planks", []),
                "minecraft:acacia_planks",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:cherry_planks", []),
                "minecraft:cherry_planks",
                rgb_option(0xd1, 0xb1, 0xa1),
            ),
            (
                test_block_state_id("minecraft:dark_oak_planks", []),
                "minecraft:dark_oak_planks",
                rgb_option(0x66, 0x4c, 0x33),
            ),
            (
                test_block_state_id("minecraft:bamboo_mosaic", []),
                "minecraft:bamboo_mosaic",
                rgb_option(0xe5, 0xe5, 0x33),
            ),
            (
                test_block_state_id("minecraft:oak_log", [("axis", "y")]),
                "minecraft:oak_log axis=y",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id("minecraft:oak_log", [("axis", "x")]),
                "minecraft:oak_log axis=x",
                rgb_option(0x81, 0x56, 0x31),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_wood_log_and_stem_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:spruce_log", [("axis", "y")]),
                "minecraft:spruce_log axis=y",
                rgb_option(0x81, 0x56, 0x31),
            ),
            (
                test_block_state_id("minecraft:spruce_log", [("axis", "x")]),
                "minecraft:spruce_log axis=x",
                rgb_option(0x66, 0x4c, 0x33),
            ),
            (
                test_block_state_id("minecraft:birch_log", [("axis", "y")]),
                "minecraft:birch_log axis=y",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id("minecraft:birch_log", [("axis", "z")]),
                "minecraft:birch_log axis=z",
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                test_block_state_id("minecraft:acacia_log", [("axis", "y")]),
                "minecraft:acacia_log axis=y",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:acacia_log", [("axis", "x")]),
                "minecraft:acacia_log axis=x",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:cherry_log", [("axis", "y")]),
                "minecraft:cherry_log axis=y",
                rgb_option(0xd1, 0xb1, 0xa1),
            ),
            (
                test_block_state_id("minecraft:cherry_log", [("axis", "x")]),
                "minecraft:cherry_log axis=x",
                rgb_option(0x39, 0x29, 0x23),
            ),
            (
                test_block_state_id("minecraft:pale_oak_log", [("axis", "y")]),
                "minecraft:pale_oak_log axis=y",
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                test_block_state_id("minecraft:pale_oak_log", [("axis", "z")]),
                "minecraft:pale_oak_log axis=z",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:mangrove_log", [("axis", "y")]),
                "minecraft:mangrove_log axis=y",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id("minecraft:mangrove_log", [("axis", "x")]),
                "minecraft:mangrove_log axis=x",
                rgb_option(0x81, 0x56, 0x31),
            ),
            (
                test_block_state_id("minecraft:bamboo_block", [("axis", "y")]),
                "minecraft:bamboo_block axis=y",
                rgb_option(0xe5, 0xe5, 0x33),
            ),
            (
                test_block_state_id("minecraft:bamboo_block", [("axis", "x")]),
                "minecraft:bamboo_block axis=x",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:acacia_wood", [("axis", "z")]),
                "minecraft:acacia_wood",
                rgb_option(0x4c, 0x4c, 0x4c),
            ),
            (
                test_block_state_id("minecraft:stripped_cherry_log", [("axis", "x")]),
                "minecraft:stripped_cherry_log axis=x",
                rgb_option(0xa0, 0x4d, 0x4e),
            ),
            (
                test_block_state_id("minecraft:stripped_cherry_wood", [("axis", "y")]),
                "minecraft:stripped_cherry_wood",
                rgb_option(0xa0, 0x4d, 0x4e),
            ),
            (
                test_block_state_id("minecraft:crimson_planks", []),
                "minecraft:crimson_planks",
                rgb_option(0x94, 0x3f, 0x61),
            ),
            (
                test_block_state_id("minecraft:crimson_hyphae", [("axis", "x")]),
                "minecraft:crimson_hyphae",
                rgb_option(0x5c, 0x19, 0x1d),
            ),
            (
                test_block_state_id("minecraft:warped_stem", [("axis", "z")]),
                "minecraft:warped_stem",
                rgb_option(0x3a, 0x8e, 0x8c),
            ),
            (
                test_block_state_id("minecraft:warped_hyphae", [("axis", "y")]),
                "minecraft:warped_hyphae",
                rgb_option(0x56, 0x2c, 0x3e),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_wooden_stairs_and_slabs_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id(
                    "minecraft:oak_stairs",
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("shape", "straight"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:oak_stairs",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id(
                    "minecraft:spruce_slab",
                    [("type", "top"), ("waterlogged", "true")],
                ),
                "minecraft:spruce_slab",
                rgb_option(0x81, 0x56, 0x31),
            ),
            (
                test_block_state_id(
                    "minecraft:birch_stairs",
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("shape", "straight"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:birch_stairs",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id(
                    "minecraft:jungle_slab",
                    [("type", "top"), ("waterlogged", "true")],
                ),
                "minecraft:jungle_slab",
                rgb_option(0x97, 0x6d, 0x4d),
            ),
            (
                test_block_state_id(
                    "minecraft:acacia_stairs",
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("shape", "straight"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:acacia_stairs",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:cherry_slab",
                    [("type", "top"), ("waterlogged", "true")],
                ),
                "minecraft:cherry_slab",
                rgb_option(0xd1, 0xb1, 0xa1),
            ),
            (
                test_block_state_id(
                    "minecraft:dark_oak_stairs",
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("shape", "straight"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:dark_oak_stairs",
                rgb_option(0x66, 0x4c, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:pale_oak_slab",
                    [("type", "top"), ("waterlogged", "true")],
                ),
                "minecraft:pale_oak_slab",
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                test_block_state_id(
                    "minecraft:mangrove_stairs",
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("shape", "straight"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:mangrove_stairs",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:bamboo_slab",
                    [("type", "top"), ("waterlogged", "true")],
                ),
                "minecraft:bamboo_slab",
                rgb_option(0xe5, 0xe5, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:bamboo_mosaic_stairs",
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("shape", "straight"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:bamboo_mosaic_stairs",
                rgb_option(0xe5, 0xe5, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:crimson_stairs",
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("shape", "straight"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:crimson_stairs",
                rgb_option(0x94, 0x3f, 0x61),
            ),
            (
                test_block_state_id(
                    "minecraft:warped_slab",
                    [("type", "top"), ("waterlogged", "true")],
                ),
                "minecraft:warped_slab",
                rgb_option(0x3a, 0x8e, 0x8c),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_wooden_pressure_plate_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_name, expected_color) in [
            ("minecraft:oak_pressure_plate", rgb_option(0x8f, 0x77, 0x48)),
            (
                "minecraft:spruce_pressure_plate",
                rgb_option(0x81, 0x56, 0x31),
            ),
            (
                "minecraft:birch_pressure_plate",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                "minecraft:jungle_pressure_plate",
                rgb_option(0x97, 0x6d, 0x4d),
            ),
            (
                "minecraft:acacia_pressure_plate",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                "minecraft:cherry_pressure_plate",
                rgb_option(0xd1, 0xb1, 0xa1),
            ),
            (
                "minecraft:dark_oak_pressure_plate",
                rgb_option(0x66, 0x4c, 0x33),
            ),
            (
                "minecraft:pale_oak_pressure_plate",
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                "minecraft:mangrove_pressure_plate",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                "minecraft:bamboo_pressure_plate",
                rgb_option(0xe5, 0xe5, 0x33),
            ),
            (
                "minecraft:crimson_pressure_plate",
                rgb_option(0x94, 0x3f, 0x61),
            ),
            (
                "minecraft:warped_pressure_plate",
                rgb_option(0x3a, 0x8e, 0x8c),
            ),
        ] {
            let block_state_id = test_block_state_id(block_name, [("powered", "true")]);
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_wooden_door_trapdoor_fence_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (family, expected_color) in [
            ("oak", rgb_option(0x8f, 0x77, 0x48)),
            ("spruce", rgb_option(0x81, 0x56, 0x31)),
            ("birch", rgb_option(0xf7, 0xe9, 0xa3)),
            ("jungle", rgb_option(0x97, 0x6d, 0x4d)),
            ("acacia", rgb_option(0xd8, 0x7f, 0x33)),
            ("cherry", rgb_option(0xd1, 0xb1, 0xa1)),
            ("dark_oak", rgb_option(0x66, 0x4c, 0x33)),
            ("pale_oak", rgb_option(0xff, 0xfc, 0xf5)),
            ("mangrove", rgb_option(0x99, 0x33, 0x33)),
            ("bamboo", rgb_option(0xe5, 0xe5, 0x33)),
            ("crimson", rgb_option(0x94, 0x3f, 0x61)),
            ("warped", rgb_option(0x3a, 0x8e, 0x8c)),
        ] {
            for kind in ["door", "trapdoor", "fence", "fence_gate"] {
                let block_name = format!("minecraft:{family}_{kind}");
                let block_state_id = match kind {
                    "door" => test_block_state_id(
                        &block_name,
                        [
                            ("facing", "north"),
                            ("half", "upper"),
                            ("hinge", "left"),
                            ("open", "true"),
                            ("powered", "true"),
                        ],
                    ),
                    "trapdoor" => test_block_state_id(
                        &block_name,
                        [
                            ("facing", "north"),
                            ("half", "top"),
                            ("open", "true"),
                            ("powered", "true"),
                            ("waterlogged", "true"),
                        ],
                    ),
                    "fence" => test_block_state_id(
                        &block_name,
                        [
                            ("east", "true"),
                            ("north", "true"),
                            ("south", "true"),
                            ("waterlogged", "true"),
                            ("west", "true"),
                        ],
                    ),
                    "fence_gate" => test_block_state_id(
                        &block_name,
                        [
                            ("facing", "north"),
                            ("in_wall", "true"),
                            ("open", "true"),
                            ("powered", "true"),
                        ],
                    ),
                    _ => unreachable!("covered test kinds"),
                };
                let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
                packet.particle.raw_options = block_particle_options(block_state_id);

                let batch = resolver.resolve_level_particles(&packet);

                assert_eq!(batch.len(), 1, "{block_name}");
                assert_eq!(
                    batch.commands[0].option_color,
                    Some(expected_color),
                    "{block_name}"
                );
            }
        }
    }

    #[test]
    fn falling_dust_uses_wooden_sign_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (family, expected_color) in [
            ("oak", rgb_option(0x8f, 0x77, 0x48)),
            ("spruce", rgb_option(0x81, 0x56, 0x31)),
            ("birch", rgb_option(0xf7, 0xe9, 0xa3)),
            ("jungle", rgb_option(0x97, 0x6d, 0x4d)),
            ("acacia", rgb_option(0xd8, 0x7f, 0x33)),
            ("cherry", rgb_option(0xd1, 0xb1, 0xa1)),
            ("dark_oak", rgb_option(0x66, 0x4c, 0x33)),
            ("pale_oak", rgb_option(0xff, 0xfc, 0xf5)),
            ("mangrove", rgb_option(0x99, 0x33, 0x33)),
            ("bamboo", rgb_option(0xe5, 0xe5, 0x33)),
            ("crimson", rgb_option(0x94, 0x3f, 0x61)),
            ("warped", rgb_option(0x3a, 0x8e, 0x8c)),
        ] {
            for kind in ["sign", "wall_sign"] {
                let block_name = format!("minecraft:{family}_{kind}");
                let block_state_id = match kind {
                    "sign" => test_block_state_id(
                        &block_name,
                        [("rotation", "0"), ("waterlogged", "true")],
                    ),
                    "wall_sign" => test_block_state_id(
                        &block_name,
                        [("facing", "north"), ("waterlogged", "true")],
                    ),
                    _ => unreachable!("covered test kinds"),
                };
                let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
                packet.particle.raw_options = block_particle_options(block_state_id);

                let batch = resolver.resolve_level_particles(&packet);

                assert_eq!(batch.len(), 1, "{block_name}");
                assert_eq!(
                    batch.commands[0].option_color,
                    Some(expected_color),
                    "{block_name}"
                );
            }
        }
    }

    #[test]
    fn falling_dust_uses_wooden_shelf_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (family, expected_color) in [
            ("oak", rgb_option(0x8f, 0x77, 0x48)),
            ("spruce", rgb_option(0x81, 0x56, 0x31)),
            ("birch", rgb_option(0xf7, 0xe9, 0xa3)),
            ("jungle", rgb_option(0x97, 0x6d, 0x4d)),
            ("acacia", rgb_option(0xd8, 0x7f, 0x33)),
            ("cherry", rgb_option(0xd1, 0xb1, 0xa1)),
            ("dark_oak", rgb_option(0x66, 0x4c, 0x33)),
            ("pale_oak", rgb_option(0xff, 0xfc, 0xf5)),
            ("mangrove", rgb_option(0x99, 0x33, 0x33)),
            ("bamboo", rgb_option(0xe5, 0xe5, 0x33)),
            ("crimson", rgb_option(0x94, 0x3f, 0x61)),
            ("warped", rgb_option(0x3a, 0x8e, 0x8c)),
        ] {
            let block_name = format!("minecraft:{family}_shelf");
            let block_state_id = test_block_state_id(
                &block_name,
                [
                    ("facing", "north"),
                    ("powered", "true"),
                    ("side_chain", "unconnected"),
                    ("waterlogged", "true"),
                ],
            );
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_hanging_sign_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (family, ceiling_color, wall_color) in [
            (
                "oak",
                rgb_option(0x8f, 0x77, 0x48),
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                "spruce",
                rgb_option(0x81, 0x56, 0x31),
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                "birch",
                rgb_option(0xf7, 0xe9, 0xa3),
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                "jungle",
                rgb_option(0x97, 0x6d, 0x4d),
                rgb_option(0x97, 0x6d, 0x4d),
            ),
            (
                "acacia",
                rgb_option(0xd8, 0x7f, 0x33),
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                "cherry",
                rgb_option(0xa0, 0x4d, 0x4e),
                rgb_option(0xa0, 0x4d, 0x4e),
            ),
            (
                "dark_oak",
                rgb_option(0x66, 0x4c, 0x33),
                rgb_option(0x66, 0x4c, 0x33),
            ),
            (
                "pale_oak",
                rgb_option(0xff, 0xfc, 0xf5),
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                "mangrove",
                rgb_option(0x99, 0x33, 0x33),
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                "bamboo",
                rgb_option(0xe5, 0xe5, 0x33),
                rgb_option(0xe5, 0xe5, 0x33),
            ),
            (
                "crimson",
                rgb_option(0x94, 0x3f, 0x61),
                rgb_option(0x94, 0x3f, 0x61),
            ),
            (
                "warped",
                rgb_option(0x3a, 0x8e, 0x8c),
                rgb_option(0x3a, 0x8e, 0x8c),
            ),
        ] {
            for (kind, expected_color) in [
                ("hanging_sign", ceiling_color),
                ("wall_hanging_sign", wall_color),
            ] {
                let block_name = format!("minecraft:{family}_{kind}");
                let block_state_id = match kind {
                    "hanging_sign" => test_block_state_id(
                        &block_name,
                        [
                            ("attached", "true"),
                            ("rotation", "0"),
                            ("waterlogged", "true"),
                        ],
                    ),
                    "wall_hanging_sign" => test_block_state_id(
                        &block_name,
                        [("facing", "north"), ("waterlogged", "true")],
                    ),
                    _ => unreachable!("covered test kinds"),
                };
                let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
                packet.particle.raw_options = block_particle_options(block_state_id);

                let batch = resolver.resolve_level_particles(&packet);

                assert_eq!(batch.len(), 1, "{block_name}");
                assert_eq!(
                    batch.commands[0].option_color,
                    Some(expected_color),
                    "{block_name}"
                );
            }
        }
    }

    #[test]
    fn falling_dust_uses_button_none_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for block_name in [
            "minecraft:stone_button",
            "minecraft:oak_button",
            "minecraft:spruce_button",
            "minecraft:birch_button",
            "minecraft:jungle_button",
            "minecraft:acacia_button",
            "minecraft:cherry_button",
            "minecraft:dark_oak_button",
            "minecraft:pale_oak_button",
            "minecraft:mangrove_button",
            "minecraft:bamboo_button",
            "minecraft:crimson_button",
            "minecraft:warped_button",
            "minecraft:polished_blackstone_button",
        ] {
            let block_state_id = test_block_state_id(
                block_name,
                [("face", "floor"), ("facing", "north"), ("powered", "true")],
            );
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x00, 0x00, 0x00)),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_potted_none_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for block_name in [
            "minecraft:flower_pot",
            "minecraft:potted_torchflower",
            "minecraft:potted_oak_sapling",
            "minecraft:potted_spruce_sapling",
            "minecraft:potted_birch_sapling",
            "minecraft:potted_jungle_sapling",
            "minecraft:potted_acacia_sapling",
            "minecraft:potted_cherry_sapling",
            "minecraft:potted_dark_oak_sapling",
            "minecraft:potted_pale_oak_sapling",
            "minecraft:potted_mangrove_propagule",
            "minecraft:potted_dandelion",
            "minecraft:potted_golden_dandelion",
            "minecraft:potted_poppy",
            "minecraft:potted_blue_orchid",
            "minecraft:potted_allium",
            "minecraft:potted_azure_bluet",
            "minecraft:potted_red_tulip",
            "minecraft:potted_orange_tulip",
            "minecraft:potted_white_tulip",
            "minecraft:potted_pink_tulip",
            "minecraft:potted_oxeye_daisy",
            "minecraft:potted_cornflower",
            "minecraft:potted_lily_of_the_valley",
            "minecraft:potted_wither_rose",
            "minecraft:potted_red_mushroom",
            "minecraft:potted_brown_mushroom",
            "minecraft:potted_dead_bush",
            "minecraft:potted_cactus",
            "minecraft:potted_bamboo",
            "minecraft:potted_crimson_fungus",
            "minecraft:potted_warped_fungus",
            "minecraft:potted_crimson_roots",
            "minecraft:potted_warped_roots",
            "minecraft:potted_azalea_bush",
            "minecraft:potted_flowering_azalea_bush",
            "minecraft:potted_open_eyeblossom",
            "minecraft:potted_closed_eyeblossom",
        ] {
            let block_state_id = test_block_state_id(block_name, []);
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x00, 0x00, 0x00)),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_cake_none_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name) in [
            (
                test_block_state_id("minecraft:cake", [("bites", "6")]),
                "minecraft:cake",
            ),
            (
                test_block_state_id("minecraft:candle_cake", [("lit", "true")]),
                "minecraft:candle_cake",
            ),
            (
                test_block_state_id("minecraft:white_candle_cake", [("lit", "false")]),
                "minecraft:white_candle_cake",
            ),
            (
                test_block_state_id("minecraft:orange_candle_cake", [("lit", "true")]),
                "minecraft:orange_candle_cake",
            ),
            (
                test_block_state_id("minecraft:magenta_candle_cake", [("lit", "false")]),
                "minecraft:magenta_candle_cake",
            ),
            (
                test_block_state_id("minecraft:light_blue_candle_cake", [("lit", "true")]),
                "minecraft:light_blue_candle_cake",
            ),
            (
                test_block_state_id("minecraft:yellow_candle_cake", [("lit", "false")]),
                "minecraft:yellow_candle_cake",
            ),
            (
                test_block_state_id("minecraft:lime_candle_cake", [("lit", "true")]),
                "minecraft:lime_candle_cake",
            ),
            (
                test_block_state_id("minecraft:pink_candle_cake", [("lit", "false")]),
                "minecraft:pink_candle_cake",
            ),
            (
                test_block_state_id("minecraft:gray_candle_cake", [("lit", "true")]),
                "minecraft:gray_candle_cake",
            ),
            (
                test_block_state_id("minecraft:light_gray_candle_cake", [("lit", "false")]),
                "minecraft:light_gray_candle_cake",
            ),
            (
                test_block_state_id("minecraft:cyan_candle_cake", [("lit", "true")]),
                "minecraft:cyan_candle_cake",
            ),
            (
                test_block_state_id("minecraft:purple_candle_cake", [("lit", "false")]),
                "minecraft:purple_candle_cake",
            ),
            (
                test_block_state_id("minecraft:blue_candle_cake", [("lit", "true")]),
                "minecraft:blue_candle_cake",
            ),
            (
                test_block_state_id("minecraft:brown_candle_cake", [("lit", "false")]),
                "minecraft:brown_candle_cake",
            ),
            (
                test_block_state_id("minecraft:green_candle_cake", [("lit", "true")]),
                "minecraft:green_candle_cake",
            ),
            (
                test_block_state_id("minecraft:red_candle_cake", [("lit", "false")]),
                "minecraft:red_candle_cake",
            ),
            (
                test_block_state_id("minecraft:black_candle_cake", [("lit", "true")]),
                "minecraft:black_candle_cake",
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x00, 0x00, 0x00)),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_colored_family_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:white_wool", []),
                "minecraft:white_wool",
                rgb_option(0xff, 0xff, 0xff),
            ),
            (
                test_block_state_id("minecraft:lime_wool", []),
                "minecraft:lime_wool",
                rgb_option(0x7f, 0xcc, 0x19),
            ),
            (
                test_block_state_id("minecraft:blue_carpet", []),
                "minecraft:blue_carpet",
                rgb_option(0x33, 0x4c, 0xb2),
            ),
            (
                test_block_state_id("minecraft:cyan_stained_glass", []),
                "minecraft:cyan_stained_glass",
                rgb_option(0x4c, 0x7f, 0x99),
            ),
            (
                test_block_state_id("minecraft:purple_glazed_terracotta", [("facing", "north")]),
                "minecraft:purple_glazed_terracotta",
                rgb_option(0x7f, 0x3f, 0xb2),
            ),
            (
                test_block_state_id("minecraft:orange_concrete", []),
                "minecraft:orange_concrete",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:black_concrete", []),
                "minecraft:black_concrete",
                rgb_option(0x19, 0x19, 0x19),
            ),
            (
                test_block_state_id("minecraft:terracotta", []),
                "minecraft:terracotta",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:white_terracotta", []),
                "minecraft:white_terracotta",
                rgb_option(0xd1, 0xb1, 0xa1),
            ),
            (
                test_block_state_id("minecraft:light_blue_terracotta", []),
                "minecraft:light_blue_terracotta",
                rgb_option(0x70, 0x6c, 0x8a),
            ),
            (
                test_block_state_id("minecraft:red_terracotta", []),
                "minecraft:red_terracotta",
                rgb_option(0x8e, 0x3c, 0x2e),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_banner_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for color in [
            "white",
            "orange",
            "magenta",
            "light_blue",
            "yellow",
            "lime",
            "pink",
            "gray",
            "light_gray",
            "cyan",
            "purple",
            "blue",
            "brown",
            "green",
            "red",
            "black",
        ] {
            for kind in ["banner", "wall_banner"] {
                let block_name = format!("minecraft:{color}_{kind}");
                let block_state_id = match kind {
                    "banner" => test_block_state_id(&block_name, [("rotation", "0")]),
                    "wall_banner" => test_block_state_id(&block_name, [("facing", "north")]),
                    _ => unreachable!("covered test kinds"),
                };
                let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
                packet.particle.raw_options = block_particle_options(block_state_id);

                let batch = resolver.resolve_level_particles(&packet);

                assert_eq!(batch.len(), 1, "{block_name}");
                assert_eq!(
                    batch.commands[0].option_color,
                    Some(rgb_option(0x8f, 0x77, 0x48)),
                    "{block_name}"
                );
            }
        }
    }

    #[test]
    fn falling_dust_uses_mineral_and_natural_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:gold_ore", []),
                "minecraft:gold_ore",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:deepslate_iron_ore", []),
                "minecraft:deepslate_iron_ore",
                rgb_option(0x64, 0x64, 0x64),
            ),
            (
                test_block_state_id("minecraft:nether_quartz_ore", []),
                "minecraft:nether_quartz_ore",
                rgb_option(0x70, 0x02, 0x00),
            ),
            (
                test_block_state_id("minecraft:lapis_block", []),
                "minecraft:lapis_block",
                rgb_option(0x4a, 0x80, 0xff),
            ),
            (
                test_block_state_id("minecraft:diamond_block", []),
                "minecraft:diamond_block",
                rgb_option(0x5c, 0xdb, 0xd5),
            ),
            (
                test_block_state_id("minecraft:emerald_block", []),
                "minecraft:emerald_block",
                rgb_option(0x00, 0xd9, 0x3a),
            ),
            (
                test_block_state_id("minecraft:raw_iron_block", []),
                "minecraft:raw_iron_block",
                rgb_option(0xd8, 0xaf, 0x93),
            ),
            (
                test_block_state_id("minecraft:suspicious_gravel", [("dusted", "0")]),
                "minecraft:suspicious_gravel",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:suspicious_sand", [("dusted", "0")]),
                "minecraft:suspicious_sand",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id("minecraft:sandstone", []),
                "minecraft:sandstone",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id("minecraft:snow_block", []),
                "minecraft:snow_block",
                rgb_option(0xff, 0xff, 0xff),
            ),
            (
                test_block_state_id("minecraft:ice", []),
                "minecraft:ice",
                rgb_option(0xa0, 0xa0, 0xff),
            ),
            (
                test_block_state_id("minecraft:clay", []),
                "minecraft:clay",
                rgb_option(0xa4, 0xa8, 0xb8),
            ),
            (
                test_block_state_id("minecraft:deepslate", [("axis", "y")]),
                "minecraft:deepslate",
                rgb_option(0x64, 0x64, 0x64),
            ),
            (
                test_block_state_id("minecraft:netherrack", []),
                "minecraft:netherrack",
                rgb_option(0x70, 0x02, 0x00),
            ),
            (
                test_block_state_id("minecraft:red_nether_bricks", []),
                "minecraft:red_nether_bricks",
                rgb_option(0x70, 0x02, 0x00),
            ),
            (
                test_block_state_id("minecraft:soul_sand", []),
                "minecraft:soul_sand",
                rgb_option(0x66, 0x4c, 0x33),
            ),
            (
                test_block_state_id("minecraft:basalt", [("axis", "z")]),
                "minecraft:basalt",
                rgb_option(0x19, 0x19, 0x19),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_deepslate_construction_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        let wall = [
            ("east", "low"),
            ("north", "none"),
            ("south", "none"),
            ("up", "true"),
            ("waterlogged", "false"),
            ("west", "none"),
        ];

        for (block_state_id, block_name) in [
            (
                test_block_state_id(
                    "minecraft:cobbled_deepslate_stairs",
                    [
                        ("facing", "east"),
                        ("half", "bottom"),
                        ("shape", "straight"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:cobbled_deepslate_stairs",
            ),
            (
                test_block_state_id(
                    "minecraft:polished_deepslate_slab",
                    [("type", "bottom"), ("waterlogged", "false")],
                ),
                "minecraft:polished_deepslate_slab",
            ),
            (
                test_block_state_id("minecraft:deepslate_tile_wall", wall),
                "minecraft:deepslate_tile_wall",
            ),
            (
                test_block_state_id(
                    "minecraft:deepslate_brick_stairs",
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("shape", "inner_left"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:deepslate_brick_stairs",
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x64, 0x64, 0x64)),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_infested_stone_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:infested_stone", []),
                "minecraft:infested_stone",
                rgb_option(0xa4, 0xa8, 0xb8),
            ),
            (
                test_block_state_id("minecraft:infested_cobblestone", []),
                "minecraft:infested_cobblestone",
                rgb_option(0xa4, 0xa8, 0xb8),
            ),
            (
                test_block_state_id("minecraft:infested_chiseled_stone_bricks", []),
                "minecraft:infested_chiseled_stone_bricks",
                rgb_option(0xa4, 0xa8, 0xb8),
            ),
            (
                test_block_state_id("minecraft:infested_deepslate", [("axis", "y")]),
                "minecraft:infested_deepslate",
                rgb_option(0x64, 0x64, 0x64),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_natural_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:oak_sapling", [("stage", "0")]),
                "minecraft:oak_sapling",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:cherry_sapling", [("stage", "1")]),
                "minecraft:cherry_sapling",
                rgb_option(0xf2, 0x7f, 0xa5),
            ),
            (
                test_block_state_id("minecraft:pale_oak_sapling", [("stage", "0")]),
                "minecraft:pale_oak_sapling",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
            (
                test_block_state_id("minecraft:short_dry_grass", []),
                "minecraft:short_dry_grass",
                rgb_option(0xe5, 0xe5, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:pointed_dripstone",
                    [
                        ("thickness", "tip"),
                        ("vertical_direction", "down"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:pointed_dripstone",
                rgb_option(0x4c, 0x32, 0x23),
            ),
            (
                test_block_state_id("minecraft:cave_vines", [("age", "25"), ("berries", "true")]),
                "minecraft:cave_vines",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:moss_block", []),
                "minecraft:moss_block",
                rgb_option(0x66, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:hanging_roots", [("waterlogged", "false")]),
                "minecraft:hanging_roots",
                rgb_option(0x97, 0x6d, 0x4d),
            ),
            (
                test_block_state_id("minecraft:mud", []),
                "minecraft:mud",
                rgb_option(0x57, 0x5c, 0x5c),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_crop_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:wheat", [("age", "0")]),
                "minecraft:wheat age=0",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:wheat", [("age", "7")]),
                "minecraft:wheat age=7",
                rgb_option(0xe5, 0xe5, 0x33),
            ),
            (
                test_block_state_id("minecraft:carrots", [("age", "0")]),
                "minecraft:carrots",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:potatoes", [("age", "0")]),
                "minecraft:potatoes",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:beetroots", [("age", "0")]),
                "minecraft:beetroots",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:nether_wart", [("age", "0")]),
                "minecraft:nether_wart",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id("minecraft:torchflower_crop", [("age", "0")]),
                "minecraft:torchflower_crop",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:pitcher_crop", [("age", "0"), ("half", "upper")]),
                "minecraft:pitcher_crop",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:pitcher_plant", [("half", "upper")]),
                "minecraft:pitcher_plant",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:cactus", [("age", "0")]),
                "minecraft:cactus",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:cactus_flower", []),
                "minecraft:cactus_flower",
                rgb_option(0xf2, 0x7f, 0xa5),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_produce_and_fungus_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:brown_mushroom", []),
                "minecraft:brown_mushroom",
                rgb_option(0x66, 0x4c, 0x33),
            ),
            (
                test_block_state_id("minecraft:red_mushroom", []),
                "minecraft:red_mushroom",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:brown_mushroom_block",
                    [
                        ("down", "true"),
                        ("east", "true"),
                        ("north", "true"),
                        ("south", "true"),
                        ("up", "true"),
                        ("west", "true"),
                    ],
                ),
                "minecraft:brown_mushroom_block",
                rgb_option(0x97, 0x6d, 0x4d),
            ),
            (
                test_block_state_id(
                    "minecraft:red_mushroom_block",
                    [
                        ("down", "true"),
                        ("east", "true"),
                        ("north", "true"),
                        ("south", "true"),
                        ("up", "true"),
                        ("west", "true"),
                    ],
                ),
                "minecraft:red_mushroom_block",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:mushroom_stem",
                    [
                        ("down", "true"),
                        ("east", "true"),
                        ("north", "true"),
                        ("south", "true"),
                        ("up", "true"),
                        ("west", "true"),
                    ],
                ),
                "minecraft:mushroom_stem",
                rgb_option(0xc7, 0xc7, 0xc7),
            ),
            (
                test_block_state_id("minecraft:pumpkin", []),
                "minecraft:pumpkin",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:carved_pumpkin", [("facing", "north")]),
                "minecraft:carved_pumpkin",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:jack_o_lantern", [("facing", "north")]),
                "minecraft:jack_o_lantern",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:melon", []),
                "minecraft:melon",
                rgb_option(0x7f, 0xcc, 0x19),
            ),
            (
                test_block_state_id("minecraft:hay_block", [("axis", "x")]),
                "minecraft:hay_block",
                rgb_option(0xe5, 0xe5, 0x33),
            ),
            (
                test_block_state_id("minecraft:dried_kelp_block", []),
                "minecraft:dried_kelp_block",
                rgb_option(0x66, 0x7f, 0x33),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_static_foliage_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id(
                    "minecraft:cherry_leaves",
                    [
                        ("distance", "1"),
                        ("persistent", "true"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:cherry_leaves",
                rgb_option(0xf2, 0x7f, 0xa5),
            ),
            (
                test_block_state_id(
                    "minecraft:pale_oak_leaves",
                    [
                        ("distance", "1"),
                        ("persistent", "true"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:pale_oak_leaves",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
            (
                test_block_state_id(
                    "minecraft:azalea_leaves",
                    [
                        ("distance", "1"),
                        ("persistent", "true"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:azalea_leaves",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id(
                    "minecraft:flowering_azalea_leaves",
                    [
                        ("distance", "1"),
                        ("persistent", "true"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:flowering_azalea_leaves",
                rgb_option(0x00, 0x7c, 0x00),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_utility_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:bedrock", []),
                "minecraft:bedrock",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:stone_pressure_plate", [("powered", "true")]),
                "minecraft:stone_pressure_plate",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id(
                    "minecraft:sticky_piston",
                    [("extended", "true"), ("facing", "north")],
                ),
                "minecraft:sticky_piston",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id(
                    "minecraft:note_block",
                    [("instrument", "harp"), ("note", "0"), ("powered", "false")],
                ),
                "minecraft:note_block",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id(
                    "minecraft:chiseled_bookshelf",
                    [
                        ("facing", "north"),
                        ("slot_0_occupied", "false"),
                        ("slot_1_occupied", "false"),
                        ("slot_2_occupied", "false"),
                        ("slot_3_occupied", "false"),
                        ("slot_4_occupied", "false"),
                        ("slot_5_occupied", "false"),
                    ],
                ),
                "minecraft:chiseled_bookshelf",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id(
                    "minecraft:chest",
                    [
                        ("facing", "north"),
                        ("type", "single"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:chest",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id("minecraft:cobweb", []),
                "minecraft:cobweb",
                rgb_option(0xc7, 0xc7, 0xc7),
            ),
            (
                test_block_state_id("minecraft:tnt", [("unstable", "false")]),
                "minecraft:tnt",
                rgb_option(0xff, 0x00, 0x00),
            ),
            (
                test_block_state_id("minecraft:light_weighted_pressure_plate", [("power", "0")]),
                "minecraft:light_weighted_pressure_plate",
                rgb_option(0xfa, 0xee, 0x4d),
            ),
            (
                test_block_state_id("minecraft:heavy_weighted_pressure_plate", [("power", "0")]),
                "minecraft:heavy_weighted_pressure_plate",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
            (
                test_block_state_id(
                    "minecraft:iron_door",
                    [
                        ("facing", "north"),
                        ("half", "upper"),
                        ("hinge", "left"),
                        ("open", "true"),
                        ("powered", "true"),
                    ],
                ),
                "minecraft:iron_door",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
            (
                test_block_state_id(
                    "minecraft:iron_trapdoor",
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("open", "true"),
                        ("powered", "true"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:iron_trapdoor",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
            (
                test_block_state_id(
                    "minecraft:brewing_stand",
                    [
                        ("has_bottle_0", "true"),
                        ("has_bottle_1", "true"),
                        ("has_bottle_2", "true"),
                    ],
                ),
                "minecraft:brewing_stand",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
            (
                test_block_state_id("minecraft:cauldron", []),
                "minecraft:cauldron",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:lava_cauldron", []),
                "minecraft:lava_cauldron",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:powder_snow_cauldron", [("level", "1")]),
                "minecraft:powder_snow_cauldron",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id(
                    "minecraft:hopper",
                    [("enabled", "true"), ("facing", "down")],
                ),
                "minecraft:hopper",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:stonecutter", [("facing", "north")]),
                "minecraft:stonecutter",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id(
                    "minecraft:bell",
                    [
                        ("attachment", "floor"),
                        ("facing", "north"),
                        ("powered", "true"),
                    ],
                ),
                "minecraft:bell",
                rgb_option(0xfa, 0xee, 0x4d),
            ),
            (
                test_block_state_id(
                    "minecraft:lantern",
                    [("hanging", "true"), ("waterlogged", "true")],
                ),
                "minecraft:lantern",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
            (
                test_block_state_id(
                    "minecraft:soul_lantern",
                    [("hanging", "true"), ("waterlogged", "true")],
                ),
                "minecraft:soul_lantern",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
            (
                test_block_state_id(
                    "minecraft:decorated_pot",
                    [
                        ("cracked", "false"),
                        ("facing", "north"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:decorated_pot",
                rgb_option(0x8e, 0x3c, 0x2e),
            ),
            (
                test_block_state_id(
                    "minecraft:crafter",
                    [
                        ("crafting", "false"),
                        ("orientation", "down_east"),
                        ("triggered", "false"),
                    ],
                ),
                "minecraft:crafter",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id(
                    "minecraft:vault",
                    [
                        ("facing", "north"),
                        ("ominous", "false"),
                        ("vault_state", "inactive"),
                    ],
                ),
                "minecraft:vault",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:heavy_core", [("waterlogged", "false")]),
                "minecraft:heavy_core",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_skull_and_head_none_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name) in [
            (
                test_block_state_id(
                    "minecraft:skeleton_skull",
                    [("powered", "true"), ("rotation", "15")],
                ),
                "minecraft:skeleton_skull",
            ),
            (
                test_block_state_id(
                    "minecraft:skeleton_wall_skull",
                    [("facing", "east"), ("powered", "false")],
                ),
                "minecraft:skeleton_wall_skull",
            ),
            (
                test_block_state_id(
                    "minecraft:wither_skeleton_skull",
                    [("powered", "false"), ("rotation", "0")],
                ),
                "minecraft:wither_skeleton_skull",
            ),
            (
                test_block_state_id(
                    "minecraft:wither_skeleton_wall_skull",
                    [("facing", "north"), ("powered", "true")],
                ),
                "minecraft:wither_skeleton_wall_skull",
            ),
            (
                test_block_state_id(
                    "minecraft:zombie_head",
                    [("powered", "true"), ("rotation", "7")],
                ),
                "minecraft:zombie_head",
            ),
            (
                test_block_state_id(
                    "minecraft:zombie_wall_head",
                    [("facing", "west"), ("powered", "false")],
                ),
                "minecraft:zombie_wall_head",
            ),
            (
                test_block_state_id(
                    "minecraft:player_head",
                    [("powered", "false"), ("rotation", "12")],
                ),
                "minecraft:player_head",
            ),
            (
                test_block_state_id(
                    "minecraft:player_wall_head",
                    [("facing", "south"), ("powered", "true")],
                ),
                "minecraft:player_wall_head",
            ),
            (
                test_block_state_id(
                    "minecraft:creeper_head",
                    [("powered", "true"), ("rotation", "2")],
                ),
                "minecraft:creeper_head",
            ),
            (
                test_block_state_id(
                    "minecraft:creeper_wall_head",
                    [("facing", "east"), ("powered", "true")],
                ),
                "minecraft:creeper_wall_head",
            ),
            (
                test_block_state_id(
                    "minecraft:dragon_head",
                    [("powered", "false"), ("rotation", "10")],
                ),
                "minecraft:dragon_head",
            ),
            (
                test_block_state_id(
                    "minecraft:dragon_wall_head",
                    [("facing", "north"), ("powered", "false")],
                ),
                "minecraft:dragon_wall_head",
            ),
            (
                test_block_state_id(
                    "minecraft:piglin_head",
                    [("powered", "true"), ("rotation", "5")],
                ),
                "minecraft:piglin_head",
            ),
            (
                test_block_state_id(
                    "minecraft:piglin_wall_head",
                    [("facing", "west"), ("powered", "true")],
                ),
                "minecraft:piglin_wall_head",
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x00, 0x00, 0x00)),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_redstone_fixture_none_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name) in [
            (
                test_block_state_id(
                    "minecraft:powered_rail",
                    [
                        ("powered", "true"),
                        ("shape", "ascending_east"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:powered_rail",
            ),
            (
                test_block_state_id(
                    "minecraft:detector_rail",
                    [
                        ("powered", "false"),
                        ("shape", "ascending_south"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:detector_rail",
            ),
            (
                test_block_state_id(
                    "minecraft:rail",
                    [("shape", "north_east"), ("waterlogged", "true")],
                ),
                "minecraft:rail",
            ),
            (
                test_block_state_id(
                    "minecraft:activator_rail",
                    [
                        ("powered", "true"),
                        ("shape", "ascending_west"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:activator_rail",
            ),
            (
                test_block_state_id(
                    "minecraft:lever",
                    [("face", "ceiling"), ("facing", "east"), ("powered", "true")],
                ),
                "minecraft:lever",
            ),
            (
                test_block_state_id(
                    "minecraft:repeater",
                    [
                        ("delay", "4"),
                        ("facing", "north"),
                        ("locked", "true"),
                        ("powered", "false"),
                    ],
                ),
                "minecraft:repeater",
            ),
            (
                test_block_state_id(
                    "minecraft:comparator",
                    [
                        ("facing", "east"),
                        ("mode", "subtract"),
                        ("powered", "false"),
                    ],
                ),
                "minecraft:comparator",
            ),
            (
                test_block_state_id(
                    "minecraft:tripwire_hook",
                    [
                        ("attached", "true"),
                        ("facing", "north"),
                        ("powered", "true"),
                    ],
                ),
                "minecraft:tripwire_hook",
            ),
            (
                test_block_state_id(
                    "minecraft:tripwire",
                    [
                        ("attached", "true"),
                        ("disarmed", "false"),
                        ("east", "true"),
                        ("north", "false"),
                        ("powered", "true"),
                        ("south", "true"),
                        ("west", "false"),
                    ],
                ),
                "minecraft:tripwire",
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x00, 0x00, 0x00)),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_redstone_utility_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:redstone_lamp", [("lit", "true")]),
                "minecraft:redstone_lamp",
                rgb_option(0x9f, 0x52, 0x24),
            ),
            (
                test_block_state_id(
                    "minecraft:ender_chest",
                    [("facing", "north"), ("waterlogged", "true")],
                ),
                "minecraft:ender_chest",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id(
                    "minecraft:observer",
                    [("facing", "up"), ("powered", "true")],
                ),
                "minecraft:observer",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id(
                    "minecraft:trapped_chest",
                    [
                        ("facing", "east"),
                        ("type", "right"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:trapped_chest",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id(
                    "minecraft:daylight_detector",
                    [("inverted", "true"), ("power", "15")],
                ),
                "minecraft:daylight_detector",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id(
                    "minecraft:command_block",
                    [("conditional", "true"), ("facing", "up")],
                ),
                "minecraft:command_block",
                rgb_option(0x66, 0x4c, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:repeating_command_block",
                    [("conditional", "false"), ("facing", "down")],
                ),
                "minecraft:repeating_command_block",
                rgb_option(0x7f, 0x3f, 0xb2),
            ),
            (
                test_block_state_id(
                    "minecraft:chain_command_block",
                    [("conditional", "true"), ("facing", "east")],
                ),
                "minecraft:chain_command_block",
                rgb_option(0x66, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:structure_block", [("mode", "data")]),
                "minecraft:structure_block",
                rgb_option(0x99, 0x99, 0x99),
            ),
            (
                test_block_state_id("minecraft:jigsaw", [("orientation", "up_north")]),
                "minecraft:jigsaw",
                rgb_option(0x99, 0x99, 0x99),
            ),
            (
                test_block_state_id("minecraft:test_block", [("mode", "fail")]),
                "minecraft:test_block",
                rgb_option(0x99, 0x99, 0x99),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_aquatic_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:tube_coral_block", []),
                "minecraft:tube_coral_block",
                rgb_option(0x33, 0x4c, 0xb2),
            ),
            (
                test_block_state_id("minecraft:brain_coral", [("waterlogged", "true")]),
                "minecraft:brain_coral",
                rgb_option(0xf2, 0x7f, 0xa5),
            ),
            (
                test_block_state_id("minecraft:bubble_coral_fan", [("waterlogged", "false")]),
                "minecraft:bubble_coral_fan",
                rgb_option(0x7f, 0x3f, 0xb2),
            ),
            (
                test_block_state_id(
                    "minecraft:fire_coral_wall_fan",
                    [("facing", "east"), ("waterlogged", "true")],
                ),
                "minecraft:fire_coral_wall_fan",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id("minecraft:horn_coral_block", []),
                "minecraft:horn_coral_block",
                rgb_option(0xe5, 0xe5, 0x33),
            ),
            (
                test_block_state_id("minecraft:dead_tube_coral", [("waterlogged", "false")]),
                "minecraft:dead_tube_coral",
                rgb_option(0x4c, 0x4c, 0x4c),
            ),
            (
                test_block_state_id(
                    "minecraft:dead_horn_coral_wall_fan",
                    [("facing", "south"), ("waterlogged", "true")],
                ),
                "minecraft:dead_horn_coral_wall_fan",
                rgb_option(0x4c, 0x4c, 0x4c),
            ),
            (
                test_block_state_id(
                    "minecraft:sea_pickle",
                    [("pickles", "4"), ("waterlogged", "false")],
                ),
                "minecraft:sea_pickle",
                rgb_option(0x66, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:conduit", [("waterlogged", "true")]),
                "minecraft:conduit",
                rgb_option(0x5c, 0xdb, 0xd5),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_bamboo_honey_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:bamboo_sapling", []),
                "minecraft:bamboo_sapling",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id(
                    "minecraft:bamboo",
                    [("age", "1"), ("leaves", "large"), ("stage", "1")],
                ),
                "minecraft:bamboo",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id("minecraft:sweet_berry_bush", [("age", "3")]),
                "minecraft:sweet_berry_bush",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id(
                    "minecraft:campfire",
                    [
                        ("facing", "east"),
                        ("lit", "false"),
                        ("signal_fire", "true"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:campfire",
                rgb_option(0x81, 0x56, 0x31),
            ),
            (
                test_block_state_id(
                    "minecraft:soul_campfire",
                    [
                        ("facing", "south"),
                        ("lit", "true"),
                        ("signal_fire", "false"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:soul_campfire",
                rgb_option(0x81, 0x56, 0x31),
            ),
            (
                test_block_state_id("minecraft:honey_block", []),
                "minecraft:honey_block",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:honeycomb_block", []),
                "minecraft:honeycomb_block",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:lodestone", []),
                "minecraft:lodestone",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_water_plant_and_egg_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:seagrass", []),
                "minecraft:seagrass",
                rgb_option(0x40, 0x40, 0xff),
            ),
            (
                test_block_state_id("minecraft:tall_seagrass", [("half", "upper")]),
                "minecraft:tall_seagrass",
                rgb_option(0x40, 0x40, 0xff),
            ),
            (
                test_block_state_id("minecraft:kelp", [("age", "25")]),
                "minecraft:kelp",
                rgb_option(0x40, 0x40, 0xff),
            ),
            (
                test_block_state_id("minecraft:kelp_plant", []),
                "minecraft:kelp_plant",
                rgb_option(0x40, 0x40, 0xff),
            ),
            (
                test_block_state_id("minecraft:frogspawn", []),
                "minecraft:frogspawn",
                rgb_option(0x40, 0x40, 0xff),
            ),
            (
                test_block_state_id("minecraft:turtle_egg", [("eggs", "4"), ("hatch", "2")]),
                "minecraft:turtle_egg",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id("minecraft:sniffer_egg", [("hatch", "2")]),
                "minecraft:sniffer_egg",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:dried_ghast",
                    [
                        ("facing", "east"),
                        ("hydration", "3"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:dried_ghast",
                rgb_option(0x4c, 0x4c, 0x4c),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_flower_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for flower in [
            "dandelion",
            "golden_dandelion",
            "torchflower",
            "poppy",
            "blue_orchid",
            "allium",
            "azure_bluet",
            "red_tulip",
            "orange_tulip",
            "white_tulip",
            "pink_tulip",
            "oxeye_daisy",
            "cornflower",
            "wither_rose",
            "lily_of_the_valley",
        ] {
            let block_name = format!("minecraft:{flower}");
            let block_state_id = test_block_state_id(&block_name, []);
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x00, 0x7c, 0x00)),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_tall_flower_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (flower, half) in [
            ("sunflower", "upper"),
            ("lilac", "lower"),
            ("rose_bush", "upper"),
            ("peony", "lower"),
        ] {
            let block_name = format!("minecraft:{flower}");
            let block_state_id = test_block_state_id(&block_name, [("half", half)]);
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x00, 0x7c, 0x00)),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_fire_cocoa_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id(
                    "minecraft:fire",
                    [
                        ("age", "0"),
                        ("east", "true"),
                        ("north", "true"),
                        ("south", "true"),
                        ("up", "true"),
                        ("west", "true"),
                    ],
                ),
                "minecraft:fire",
                rgb_option(0xff, 0x00, 0x00),
            ),
            (
                test_block_state_id("minecraft:soul_fire", []),
                "minecraft:soul_fire",
                rgb_option(0x66, 0x99, 0xd8),
            ),
            (
                test_block_state_id("minecraft:cocoa", [("age", "0"), ("facing", "north")]),
                "minecraft:cocoa",
                rgb_option(0x00, 0x7c, 0x00),
            ),
            (
                test_block_state_id(
                    "minecraft:creaking_heart",
                    [
                        ("axis", "x"),
                        ("creaking_heart_state", "uprooted"),
                        ("natural", "true"),
                    ],
                ),
                "minecraft:creaking_heart",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_default_none_fixture_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name) in [
            (
                test_block_state_id(
                    "minecraft:ladder",
                    [("facing", "north"), ("waterlogged", "true")],
                ),
                "minecraft:ladder",
            ),
            (
                test_block_state_id("minecraft:torch", []),
                "minecraft:torch",
            ),
            (
                test_block_state_id("minecraft:wall_torch", [("facing", "north")]),
                "minecraft:wall_torch",
            ),
            (
                test_block_state_id("minecraft:redstone_torch", [("lit", "false")]),
                "minecraft:redstone_torch",
            ),
            (
                test_block_state_id(
                    "minecraft:redstone_wall_torch",
                    [("facing", "north"), ("lit", "true")],
                ),
                "minecraft:redstone_wall_torch",
            ),
            (
                test_block_state_id("minecraft:soul_torch", []),
                "minecraft:soul_torch",
            ),
            (
                test_block_state_id("minecraft:soul_wall_torch", [("facing", "north")]),
                "minecraft:soul_wall_torch",
            ),
            (
                test_block_state_id("minecraft:copper_torch", []),
                "minecraft:copper_torch",
            ),
            (
                test_block_state_id("minecraft:copper_wall_torch", [("facing", "north")]),
                "minecraft:copper_wall_torch",
            ),
            (
                test_block_state_id("minecraft:end_rod", [("facing", "up")]),
                "minecraft:end_rod",
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x00, 0x00, 0x00)),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_default_none_pane_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name) in [
            (
                test_block_state_id("minecraft:glass", []),
                "minecraft:glass",
            ),
            (
                test_block_state_id(
                    "minecraft:glass_pane",
                    [
                        ("east", "true"),
                        ("north", "true"),
                        ("south", "true"),
                        ("waterlogged", "true"),
                        ("west", "true"),
                    ],
                ),
                "minecraft:glass_pane",
            ),
            (
                test_block_state_id(
                    "minecraft:iron_bars",
                    [
                        ("east", "true"),
                        ("north", "true"),
                        ("south", "true"),
                        ("waterlogged", "true"),
                        ("west", "true"),
                    ],
                ),
                "minecraft:iron_bars",
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x00, 0x00, 0x00)),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_default_none_metal_bars_chain_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for block_name in [
            "minecraft:copper_bars",
            "minecraft:exposed_copper_bars",
            "minecraft:weathered_copper_bars",
            "minecraft:oxidized_copper_bars",
            "minecraft:waxed_copper_bars",
            "minecraft:waxed_exposed_copper_bars",
            "minecraft:waxed_weathered_copper_bars",
            "minecraft:waxed_oxidized_copper_bars",
        ] {
            let block_state_id = test_block_state_id(
                block_name,
                [
                    ("east", "true"),
                    ("north", "true"),
                    ("south", "true"),
                    ("waterlogged", "true"),
                    ("west", "true"),
                ],
            );
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x00, 0x00, 0x00)),
                "{block_name}"
            );
        }

        for block_name in [
            "minecraft:iron_chain",
            "minecraft:copper_chain",
            "minecraft:exposed_copper_chain",
            "minecraft:weathered_copper_chain",
            "minecraft:oxidized_copper_chain",
            "minecraft:waxed_copper_chain",
            "minecraft:waxed_exposed_copper_chain",
            "minecraft:waxed_weathered_copper_chain",
            "minecraft:waxed_oxidized_copper_chain",
        ] {
            let block_state_id =
                test_block_state_id(block_name, [("axis", "x"), ("waterlogged", "true")]);
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(rgb_option(0x00, 0x00, 0x00)),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_functional_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id(
                    "minecraft:scaffolding",
                    [
                        ("bottom", "true"),
                        ("distance", "0"),
                        ("waterlogged", "true"),
                    ],
                ),
                "minecraft:scaffolding",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id("minecraft:loom", [("facing", "north")]),
                "minecraft:loom",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id("minecraft:barrel", [("facing", "north"), ("open", "true")]),
                "minecraft:barrel",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id("minecraft:smoker", [("facing", "north"), ("lit", "true")]),
                "minecraft:smoker",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id(
                    "minecraft:blast_furnace",
                    [("facing", "north"), ("lit", "true")],
                ),
                "minecraft:blast_furnace",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:cartography_table", []),
                "minecraft:cartography_table",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id("minecraft:fletching_table", []),
                "minecraft:fletching_table",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id(
                    "minecraft:grindstone",
                    [("face", "floor"), ("facing", "north")],
                ),
                "minecraft:grindstone",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
            (
                test_block_state_id(
                    "minecraft:lectern",
                    [
                        ("facing", "north"),
                        ("has_book", "true"),
                        ("powered", "true"),
                    ],
                ),
                "minecraft:lectern",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id("minecraft:smithing_table", []),
                "minecraft:smithing_table",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id("minecraft:composter", [("level", "0")]),
                "minecraft:composter",
                rgb_option(0x8f, 0x77, 0x48),
            ),
            (
                test_block_state_id("minecraft:target", [("power", "0")]),
                "minecraft:target",
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                test_block_state_id(
                    "minecraft:bee_nest",
                    [("facing", "north"), ("honey_level", "0")],
                ),
                "minecraft:bee_nest",
                rgb_option(0xe5, 0xe5, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:beehive",
                    [("facing", "north"), ("honey_level", "0")],
                ),
                "minecraft:beehive",
                rgb_option(0x8f, 0x77, 0x48),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_magic_utility_static_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:glowstone", []),
                "minecraft:glowstone",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id("minecraft:enchanting_table", []),
                "minecraft:enchanting_table",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id("minecraft:beacon", []),
                "minecraft:beacon",
                rgb_option(0x5c, 0xdb, 0xd5),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_decorative_colored_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id(
                    "minecraft:candle",
                    [("candles", "4"), ("lit", "false"), ("waterlogged", "false")],
                ),
                "minecraft:candle",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id(
                    "minecraft:white_candle",
                    [("candles", "2"), ("lit", "true"), ("waterlogged", "false")],
                ),
                "minecraft:white_candle",
                rgb_option(0xc7, 0xc7, 0xc7),
            ),
            (
                test_block_state_id(
                    "minecraft:purple_candle",
                    [("candles", "1"), ("lit", "false"), ("waterlogged", "false")],
                ),
                "minecraft:purple_candle",
                rgb_option(0x7f, 0x3f, 0xb2),
            ),
            (
                test_block_state_id(
                    "minecraft:white_bed",
                    [("facing", "north"), ("occupied", "false"), ("part", "foot")],
                ),
                "minecraft:white_bed foot",
                rgb_option(0xff, 0xff, 0xff),
            ),
            (
                test_block_state_id(
                    "minecraft:white_bed",
                    [("facing", "north"), ("occupied", "false"), ("part", "head")],
                ),
                "minecraft:white_bed head",
                rgb_option(0xc7, 0xc7, 0xc7),
            ),
            (
                test_block_state_id(
                    "minecraft:red_bed",
                    [("facing", "east"), ("occupied", "true"), ("part", "foot")],
                ),
                "minecraft:red_bed foot",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id("minecraft:shulker_box", [("facing", "up")]),
                "minecraft:shulker_box",
                rgb_option(0x7f, 0x3f, 0xb2),
            ),
            (
                test_block_state_id("minecraft:white_shulker_box", [("facing", "north")]),
                "minecraft:white_shulker_box",
                rgb_option(0xff, 0xff, 0xff),
            ),
            (
                test_block_state_id("minecraft:purple_shulker_box", [("facing", "down")]),
                "minecraft:purple_shulker_box",
                rgb_option(0x7a, 0x49, 0x58),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_cave_and_emissive_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:amethyst_block", []),
                "minecraft:amethyst_block",
                rgb_option(0x7f, 0x3f, 0xb2),
            ),
            (
                test_block_state_id(
                    "minecraft:small_amethyst_bud",
                    [("facing", "down"), ("waterlogged", "false")],
                ),
                "minecraft:small_amethyst_bud",
                rgb_option(0x7f, 0x3f, 0xb2),
            ),
            (
                test_block_state_id("minecraft:tuff_bricks", []),
                "minecraft:tuff_bricks",
                rgb_option(0x39, 0x29, 0x23),
            ),
            (
                test_block_state_id("minecraft:calcite", []),
                "minecraft:calcite",
                rgb_option(0xd1, 0xb1, 0xa1),
            ),
            (
                test_block_state_id("minecraft:tinted_glass", []),
                "minecraft:tinted_glass",
                rgb_option(0x4c, 0x4c, 0x4c),
            ),
            (
                test_block_state_id("minecraft:powder_snow", []),
                "minecraft:powder_snow",
                rgb_option(0xff, 0xff, 0xff),
            ),
            (
                test_block_state_id(
                    "minecraft:sculk_sensor",
                    [
                        ("power", "3"),
                        ("sculk_sensor_phase", "active"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:sculk_sensor",
                rgb_option(0x4c, 0x7f, 0x99),
            ),
            (
                test_block_state_id("minecraft:sculk", []),
                "minecraft:sculk",
                rgb_option(0x19, 0x19, 0x19),
            ),
            (
                test_block_state_id(
                    "minecraft:sculk_shrieker",
                    [
                        ("can_summon", "false"),
                        ("shrieking", "false"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:sculk_shrieker",
                rgb_option(0x19, 0x19, 0x19),
            ),
            (
                test_block_state_id("minecraft:ochre_froglight", [("axis", "x")]),
                "minecraft:ochre_froglight",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id("minecraft:verdant_froglight", [("axis", "y")]),
                "minecraft:verdant_froglight",
                rgb_option(0x7f, 0xa7, 0x96),
            ),
            (
                test_block_state_id("minecraft:pearlescent_froglight", [("axis", "z")]),
                "minecraft:pearlescent_froglight",
                rgb_option(0xf2, 0x7f, 0xa5),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_copper_weathering_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:copper_block", []),
                "minecraft:copper_block",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:exposed_copper", []),
                "minecraft:exposed_copper",
                rgb_option(0x87, 0x6b, 0x62),
            ),
            (
                test_block_state_id("minecraft:weathered_copper", []),
                "minecraft:weathered_copper",
                rgb_option(0x3a, 0x8e, 0x8c),
            ),
            (
                test_block_state_id("minecraft:oxidized_copper", []),
                "minecraft:oxidized_copper",
                rgb_option(0x16, 0x7e, 0x86),
            ),
            (
                test_block_state_id("minecraft:raw_copper_block", []),
                "minecraft:raw_copper_block",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:waxed_oxidized_cut_copper_slab",
                    [("type", "bottom"), ("waterlogged", "false")],
                ),
                "minecraft:waxed_oxidized_cut_copper_slab",
                rgb_option(0x16, 0x7e, 0x86),
            ),
            (
                test_block_state_id(
                    "minecraft:copper_door",
                    [
                        ("facing", "east"),
                        ("half", "lower"),
                        ("hinge", "right"),
                        ("open", "false"),
                        ("powered", "false"),
                    ],
                ),
                "minecraft:copper_door",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id(
                    "minecraft:waxed_weathered_copper_bulb",
                    [("lit", "false"), ("powered", "true")],
                ),
                "minecraft:waxed_weathered_copper_bulb",
                rgb_option(0x3a, 0x8e, 0x8c),
            ),
            (
                test_block_state_id(
                    "minecraft:waxed_exposed_copper_grate",
                    [("waterlogged", "false")],
                ),
                "minecraft:waxed_exposed_copper_grate",
                rgb_option(0x87, 0x6b, 0x62),
            ),
            (
                test_block_state_id(
                    "minecraft:oxidized_copper_chest",
                    [
                        ("facing", "east"),
                        ("type", "single"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:oxidized_copper_chest",
                rgb_option(0x16, 0x7e, 0x86),
            ),
            (
                test_block_state_id(
                    "minecraft:waxed_oxidized_copper_golem_statue",
                    [
                        ("copper_golem_pose", "sitting"),
                        ("facing", "west"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:waxed_oxidized_copper_golem_statue",
                rgb_option(0x16, 0x7e, 0x86),
            ),
            (
                test_block_state_id(
                    "minecraft:weathered_lightning_rod",
                    [
                        ("facing", "up"),
                        ("powered", "false"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:weathered_lightning_rod",
                rgb_option(0x3a, 0x8e, 0x8c),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_nether_flora_and_blackstone_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:crimson_nylium", []),
                "minecraft:crimson_nylium",
                rgb_option(0xbd, 0x30, 0x31),
            ),
            (
                test_block_state_id("minecraft:warped_nylium", []),
                "minecraft:warped_nylium",
                rgb_option(0x16, 0x7e, 0x86),
            ),
            (
                test_block_state_id("minecraft:warped_wart_block", []),
                "minecraft:warped_wart_block",
                rgb_option(0x14, 0xb4, 0x85),
            ),
            (
                test_block_state_id("minecraft:nether_wart_block", []),
                "minecraft:nether_wart_block",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id("minecraft:warped_fungus", []),
                "minecraft:warped_fungus",
                rgb_option(0x4c, 0x7f, 0x99),
            ),
            (
                test_block_state_id("minecraft:crimson_fungus", []),
                "minecraft:crimson_fungus",
                rgb_option(0x70, 0x02, 0x00),
            ),
            (
                test_block_state_id("minecraft:shroomlight", []),
                "minecraft:shroomlight",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id("minecraft:weeping_vines", [("age", "13")]),
                "minecraft:weeping_vines",
                rgb_option(0x70, 0x02, 0x00),
            ),
            (
                test_block_state_id("minecraft:twisting_vines", [("age", "13")]),
                "minecraft:twisting_vines",
                rgb_option(0x4c, 0x7f, 0x99),
            ),
            (
                test_block_state_id("minecraft:magma_block", []),
                "minecraft:magma_block",
                rgb_option(0x70, 0x02, 0x00),
            ),
            (
                test_block_state_id("minecraft:respawn_anchor", [("charges", "4")]),
                "minecraft:respawn_anchor",
                rgb_option(0x19, 0x19, 0x19),
            ),
            (
                test_block_state_id("minecraft:smooth_basalt", []),
                "minecraft:smooth_basalt",
                rgb_option(0x19, 0x19, 0x19),
            ),
            (
                test_block_state_id("minecraft:blackstone", []),
                "minecraft:blackstone",
                rgb_option(0x19, 0x19, 0x19),
            ),
            (
                test_block_state_id(
                    "minecraft:polished_blackstone_pressure_plate",
                    [("powered", "false")],
                ),
                "minecraft:polished_blackstone_pressure_plate",
                rgb_option(0x19, 0x19, 0x19),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_quartz_prismarine_and_end_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:quartz_block", []),
                "minecraft:quartz_block",
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                test_block_state_id("minecraft:quartz_pillar", [("axis", "x")]),
                "minecraft:quartz_pillar",
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                test_block_state_id(
                    "minecraft:smooth_quartz_stairs",
                    [
                        ("facing", "east"),
                        ("half", "bottom"),
                        ("shape", "straight"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:smooth_quartz_stairs",
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                test_block_state_id("minecraft:quartz_bricks", []),
                "minecraft:quartz_bricks",
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                test_block_state_id("minecraft:sea_lantern", []),
                "minecraft:sea_lantern",
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                test_block_state_id(
                    "minecraft:prismarine_wall",
                    [
                        ("east", "low"),
                        ("north", "none"),
                        ("south", "none"),
                        ("up", "true"),
                        ("waterlogged", "false"),
                        ("west", "none"),
                    ],
                ),
                "minecraft:prismarine_wall",
                rgb_option(0x4c, 0x7f, 0x99),
            ),
            (
                test_block_state_id(
                    "minecraft:dark_prismarine_slab",
                    [("type", "top"), ("waterlogged", "false")],
                ),
                "minecraft:dark_prismarine_slab",
                rgb_option(0x5c, 0xdb, 0xd5),
            ),
            (
                test_block_state_id(
                    "minecraft:prismarine_brick_stairs",
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("shape", "inner_left"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:prismarine_brick_stairs",
                rgb_option(0x5c, 0xdb, 0xd5),
            ),
            (
                test_block_state_id("minecraft:end_stone", []),
                "minecraft:end_stone",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id(
                    "minecraft:end_stone_brick_wall",
                    [
                        ("east", "low"),
                        ("north", "none"),
                        ("south", "none"),
                        ("up", "true"),
                        ("waterlogged", "false"),
                        ("west", "none"),
                    ],
                ),
                "minecraft:end_stone_brick_wall",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id(
                    "minecraft:end_portal_frame",
                    [("eye", "true"), ("facing", "north")],
                ),
                "minecraft:end_portal_frame",
                rgb_option(0x66, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:purpur_pillar", [("axis", "z")]),
                "minecraft:purpur_pillar",
                rgb_option(0xb2, 0x4c, 0xd8),
            ),
            (
                test_block_state_id(
                    "minecraft:purpur_slab",
                    [("type", "bottom"), ("waterlogged", "false")],
                ),
                "minecraft:purpur_slab",
                rgb_option(0xb2, 0x4c, 0xd8),
            ),
            (
                test_block_state_id("minecraft:chorus_flower", [("age", "5")]),
                "minecraft:chorus_flower",
                rgb_option(0x7f, 0x3f, 0xb2),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_construction_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        let wall = [
            ("east", "low"),
            ("north", "none"),
            ("south", "none"),
            ("up", "true"),
            ("waterlogged", "false"),
            ("west", "none"),
        ];

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id(
                    "minecraft:stone_stairs",
                    [
                        ("facing", "east"),
                        ("half", "bottom"),
                        ("shape", "straight"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:stone_stairs",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id(
                    "minecraft:mossy_cobblestone_slab",
                    [("type", "bottom"), ("waterlogged", "false")],
                ),
                "minecraft:mossy_cobblestone_slab",
                rgb_option(0x70, 0x70, 0x70),
            ),
            (
                test_block_state_id("minecraft:granite_wall", wall),
                "minecraft:granite_wall",
                rgb_option(0x97, 0x6d, 0x4d),
            ),
            (
                test_block_state_id(
                    "minecraft:diorite_slab",
                    [("type", "bottom"), ("waterlogged", "false")],
                ),
                "minecraft:diorite_slab",
                rgb_option(0xff, 0xfc, 0xf5),
            ),
            (
                test_block_state_id(
                    "minecraft:smooth_sandstone_stairs",
                    [
                        ("facing", "north"),
                        ("half", "top"),
                        ("shape", "inner_left"),
                        ("waterlogged", "false"),
                    ],
                ),
                "minecraft:smooth_sandstone_stairs",
                rgb_option(0xf7, 0xe9, 0xa3),
            ),
            (
                test_block_state_id("minecraft:red_sandstone_wall", wall),
                "minecraft:red_sandstone_wall",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:bricks", []),
                "minecraft:bricks",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id("minecraft:brick_wall", wall),
                "minecraft:brick_wall",
                rgb_option(0x99, 0x33, 0x33),
            ),
            (
                test_block_state_id("minecraft:mud_bricks", []),
                "minecraft:mud_bricks",
                rgb_option(0x87, 0x6b, 0x62),
            ),
            (
                test_block_state_id(
                    "minecraft:nether_brick_slab",
                    [("type", "bottom"), ("waterlogged", "false")],
                ),
                "minecraft:nether_brick_slab",
                rgb_option(0x70, 0x02, 0x00),
            ),
            (
                test_block_state_id("minecraft:red_nether_brick_wall", wall),
                "minecraft:red_nether_brick_wall",
                rgb_option(0x70, 0x02, 0x00),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_uses_resin_and_pale_garden_map_color_fallbacks() {
        let mut resolver = test_resolver(0);
        resolver.set_terrain_particle_sprite_ids(&TerrainTextureState::default());

        let wall = [
            ("east", "low"),
            ("north", "none"),
            ("south", "none"),
            ("up", "true"),
            ("waterlogged", "false"),
            ("west", "none"),
        ];

        for (block_state_id, block_name, expected_color) in [
            (
                test_block_state_id("minecraft:resin_block", []),
                "minecraft:resin_block",
                rgb_option(0x9f, 0x52, 0x24),
            ),
            (
                test_block_state_id(
                    "minecraft:resin_clump",
                    [
                        ("down", "false"),
                        ("east", "false"),
                        ("north", "true"),
                        ("south", "false"),
                        ("up", "false"),
                        ("waterlogged", "false"),
                        ("west", "false"),
                    ],
                ),
                "minecraft:resin_clump",
                rgb_option(0x9f, 0x52, 0x24),
            ),
            (
                test_block_state_id("minecraft:resin_brick_wall", wall),
                "minecraft:resin_brick_wall",
                rgb_option(0x9f, 0x52, 0x24),
            ),
            (
                test_block_state_id("minecraft:chiseled_resin_bricks", []),
                "minecraft:chiseled_resin_bricks",
                rgb_option(0x9f, 0x52, 0x24),
            ),
            (
                test_block_state_id("minecraft:pale_moss_block", []),
                "minecraft:pale_moss_block",
                rgb_option(0x99, 0x99, 0x99),
            ),
            (
                test_block_state_id(
                    "minecraft:pale_moss_carpet",
                    [
                        ("bottom", "false"),
                        ("east", "none"),
                        ("north", "none"),
                        ("south", "none"),
                        ("west", "none"),
                    ],
                ),
                "minecraft:pale_moss_carpet",
                rgb_option(0x99, 0x99, 0x99),
            ),
            (
                test_block_state_id("minecraft:pale_hanging_moss", [("tip", "true")]),
                "minecraft:pale_hanging_moss",
                rgb_option(0x99, 0x99, 0x99),
            ),
            (
                test_block_state_id("minecraft:open_eyeblossom", []),
                "minecraft:open_eyeblossom",
                rgb_option(0xd8, 0x7f, 0x33),
            ),
            (
                test_block_state_id("minecraft:closed_eyeblossom", []),
                "minecraft:closed_eyeblossom",
                rgb_option(0xa7, 0xa7, 0xa7),
            ),
            (
                test_block_state_id("minecraft:firefly_bush", []),
                "minecraft:firefly_bush",
                rgb_option(0x00, 0x7c, 0x00),
            ),
        ] {
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), 1, "{block_name}");
            assert_eq!(
                batch.commands[0].option_color,
                Some(expected_color),
                "{block_name}"
            );
        }
    }

    #[test]
    fn falling_dust_rejects_non_air_invisible_render_shape_blocks() {
        let barrier_id = test_block_state_id("minecraft:barrier", [("waterlogged", "false")]);
        let water_id = test_block_state_id("minecraft:water", [("level", "0")]);
        let air_id = test_block_state_id("minecraft:air", []);
        let stone_id = test_block_state_id("minecraft:stone", []);

        for (block_state_id, block_name, expected_commands) in [
            (barrier_id, "minecraft:barrier", 0),
            (water_id, "minecraft:water", 0),
            (air_id, "minecraft:air", 1),
            (stone_id, "minecraft:stone", 1),
        ] {
            let mut resolver = test_resolver(0);
            let mut packet = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 0);
            packet.particle.raw_options = block_particle_options(block_state_id);

            let batch = resolver.resolve_level_particles(&packet);

            assert_eq!(batch.len(), expected_commands, "{block_name}");
            assert_eq!(batch.missing_definition_count, 0, "{block_name}");
            assert_eq!(batch.unknown_particle_type_count, 0, "{block_name}");
            if expected_commands == 1 {
                assert_eq!(
                    batch.commands[0].option_block,
                    Some(ParticleBlockOptionState { block_state_id }),
                    "{block_name}"
                );
            }
        }
    }

    #[test]
    fn falling_dust_provider_rejection_preserves_packet_random_sequence() {
        let barrier_id = test_block_state_id("minecraft:barrier", [("waterlogged", "false")]);
        let stone_id = test_block_state_id("minecraft:stone", []);
        let mut rejected_resolver = test_resolver(42);
        let mut accepted_resolver = test_resolver(42);
        let mut rejected = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 2);
        rejected.particle.raw_options = block_particle_options(barrier_id);
        let mut accepted = level_particles_packet(FALLING_DUST_PARTICLE_TYPE_ID, 2);
        accepted.particle.raw_options = block_particle_options(stone_id);

        let rejected_batch = rejected_resolver.resolve_level_particles(&rejected);
        let accepted_batch = accepted_resolver.resolve_level_particles(&accepted);
        assert_eq!(rejected_batch.len(), 0);
        assert_eq!(accepted_batch.len(), 2);

        let next_rejected = rejected_resolver
            .resolve_level_particles(&level_particles_packet(SMOKE_PARTICLE_TYPE_ID, 1));
        let next_accepted = accepted_resolver
            .resolve_level_particles(&level_particles_packet(SMOKE_PARTICLE_TYPE_ID, 1));

        assert_eq!(next_rejected.len(), 1);
        assert_eq!(next_accepted.len(), 1);
        assert_eq!(
            next_rejected.commands[0].position,
            next_accepted.commands[0].position
        );
        assert_eq!(
            next_rejected.commands[0].velocity,
            next_accepted.commands[0].velocity
        );
    }

    #[test]
    fn gust_seed_particle_commands_carry_gust_child_template() {
        let mut resolver = test_resolver(0);
        for particle_type_id in [
            GUST_EMITTER_LARGE_PARTICLE_TYPE_ID,
            GUST_EMITTER_SMALL_PARTICLE_TYPE_ID,
        ] {
            let batch =
                resolver.resolve_level_particles(&level_particles_packet(particle_type_id, 0));

            assert_eq!(batch.len(), 1);
            let command = &batch.commands[0];
            assert_eq!(command.particle_type_id, particle_type_id);
            assert_eq!(command.child_spawn_templates.len(), 1);
            let child = &command.child_spawn_templates[0];
            assert_eq!(child.particle_type_id, GUST_PARTICLE_TYPE_ID);
            assert_eq!(child.particle_id, "minecraft:gust");
            assert_eq!(child.sprite_ids, vec!["minecraft:gust_0".to_string()]);
        }
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
    fn tinted_leaves_particle_options_decode_argb_color_into_spawn_command() {
        let mut resolver = test_resolver(0);
        let mut packet = level_particles_packet(TINTED_LEAVES_PARTICLE_TYPE_ID, 0);
        packet.particle.raw_options = 0x7f44_6688_u32.to_be_bytes().to_vec();

        let batch = resolver.resolve_level_particles(&packet);

        assert_eq!(batch.len(), 1);
        let command = &batch.commands[0];
        assert_eq!(command.particle_id, "minecraft:tinted_leaves");
        assert_eq!(
            command.sprite_ids,
            vec!["minecraft:tinted_leaf_0".to_string()]
        );
        assert_eq!(
            command.option_color,
            Some([
                0x44 as f32 / 255.0,
                0x66 as f32 / 255.0,
                0x88 as f32 / 255.0,
                0x7f as f32 / 255.0,
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
        let batch = resolver.resolve_level_particles(&level_particles_packet(18, 1));

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

        let mut composter_random = LevelEventSoundRandomState::with_seed(0);
        let composter = resolver.resolve_level_event_particles_with_context(
            &LevelEvent {
                event_type: COMPOSTER_FILL_LEVEL_EVENT,
                ..level_event_packet(COMPOSTER_FILL_LEVEL_EVENT)
            },
            LevelEventParticleContext {
                composter_fill_center_shape_max_y: Some(13.0 / 16.0),
                ..LevelEventParticleContext::default()
            },
            &mut composter_random,
        );
        assert_eq!(composter.len(), 10);
        let (expected_position, expected_velocity) = first_composter_fill_particle(13.0 / 16.0);
        assert_particle_command(
            &composter.commands[0],
            COMPOSTER_PARTICLE_TYPE_ID,
            "minecraft:composter",
            expected_position,
            expected_velocity,
            false,
        );

        let mut fallback_composter_random = LevelEventSoundRandomState::with_seed(0);
        let fallback_composter = resolver.resolve_level_event_particles(
            &level_event_packet(COMPOSTER_FILL_LEVEL_EVENT),
            &mut fallback_composter_random,
        );
        assert_eq!(fallback_composter.len(), 10);
        let (expected_position, expected_velocity) = first_composter_fill_particle(1.0);
        assert_particle_command(
            &fallback_composter.commands[0],
            COMPOSTER_PARTICLE_TYPE_ID,
            "minecraft:composter",
            expected_position,
            expected_velocity,
            false,
        );

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

        let dripstone_event = LevelEvent {
            event_type: DRIPSTONE_DRIP_LEVEL_EVENT,
            ..level_event_packet(DRIPSTONE_DRIP_LEVEL_EVENT)
        };
        let mut water_drip_random = LevelEventSoundRandomState::with_seed(0);
        let water_drip = resolver.resolve_level_event_particles_with_context(
            &dripstone_event,
            LevelEventParticleContext {
                dripstone_drip_particle: Some(LevelEventDripstoneDripParticle::Water),
                ..LevelEventParticleContext::default()
            },
            &mut water_drip_random,
        );
        assert_eq!(water_drip.len(), 1);
        assert_particle_command(
            &water_drip.commands[0],
            DRIPPING_DRIPSTONE_WATER_PARTICLE_TYPE_ID,
            "minecraft:dripping_dripstone_water",
            [10.583_333_343_267_44, 64.25, -2.416_666_656_732_56],
            [0.0, 0.0, 0.0],
            false,
        );

        let mut lava_drip_random = LevelEventSoundRandomState::with_seed(0);
        let lava_drip = resolver.resolve_level_event_particles_with_context(
            &dripstone_event,
            LevelEventParticleContext {
                dripstone_drip_particle: Some(LevelEventDripstoneDripParticle::Lava),
                ..LevelEventParticleContext::default()
            },
            &mut lava_drip_random,
        );
        assert_eq!(lava_drip.len(), 1);
        assert_eq!(
            lava_drip.commands[0].particle_type_id,
            DRIPPING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID
        );
        assert_eq!(
            lava_drip.commands[0].particle_id,
            "minecraft:dripping_dripstone_lava"
        );

        let mut missing_context_random = LevelEventSoundRandomState::with_seed(0);
        let missing_context_drip =
            resolver.resolve_level_event_particles(&dripstone_event, &mut missing_context_random);
        assert!(missing_context_drip.commands.is_empty());

        let growth_event = LevelEvent {
            event_type: PLANT_GROWTH_LEVEL_EVENT,
            data: 2,
            ..level_event_packet(PLANT_GROWTH_LEVEL_EVENT)
        };
        let mut growth_in_block_random = LevelEventSoundRandomState::with_seed(0);
        let growth_in_block = resolver.resolve_level_event_particles_with_context(
            &growth_event,
            LevelEventParticleContext {
                growth_particles: Some(LevelEventGrowthParticleContext {
                    pos: growth_event.pos,
                    mode: LevelEventGrowthParticleMode::InBlock { spread_height: 1.0 },
                }),
                ..LevelEventParticleContext::default()
            },
            &mut growth_in_block_random,
        );
        assert_eq!(growth_in_block.len(), 2);
        assert_particle_command(
            &growth_in_block.commands[0],
            HAPPY_VILLAGER_PARTICLE_TYPE_ID,
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

        let mut growth_wide_random = LevelEventSoundRandomState::with_seed(0);
        let growth_wide = resolver.resolve_level_event_particles_with_context(
            &growth_event,
            LevelEventParticleContext {
                growth_particles: Some(LevelEventGrowthParticleContext {
                    pos: growth_event.pos,
                    mode: LevelEventGrowthParticleMode::WideNoFloating {
                        support: LevelEventGrowthParticleSupport::full(),
                    },
                }),
                ..LevelEventParticleContext::default()
            },
            &mut growth_wide_random,
        );
        let (growth_wide_position, growth_wide_velocity) =
            first_growth_wide_particle(growth_event.pos);
        assert_eq!(growth_wide.len(), 6);
        assert_particle_command(
            &growth_wide.commands[0],
            HAPPY_VILLAGER_PARTICLE_TYPE_ID,
            "minecraft:happy_villager",
            growth_wide_position,
            growth_wide_velocity,
            false,
        );

        let mut empty_support_growth_random = LevelEventSoundRandomState::with_seed(0);
        let empty_support_growth = resolver.resolve_level_event_particles_with_context(
            &growth_event,
            LevelEventParticleContext {
                growth_particles: Some(LevelEventGrowthParticleContext {
                    pos: growth_event.pos,
                    mode: LevelEventGrowthParticleMode::WideNoFloating {
                        support: LevelEventGrowthParticleSupport::empty(),
                    },
                }),
                ..LevelEventParticleContext::default()
            },
            &mut empty_support_growth_random,
        );
        assert!(empty_support_growth.commands.is_empty());

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

        let mut potion_random = LevelEventSoundRandomState::with_seed(0);
        let potion = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2002,
                data: 0x0033_66cc,
                ..level_event_packet(2002)
            },
            &mut potion_random,
        );
        assert_eq!(potion.len(), 108);
        assert_item_break_particle_command(
            &potion.commands[0],
            VANILLA_SPLASH_POTION_ITEM_ID,
            [10.5, 64.0, -2.5],
            first_item_break_particle_velocity(0),
        );
        let (expected_position, expected_velocity, expected_color, expected_power) =
            first_potion_break_spell_particle(0x0033_66cc);
        assert_particle_command(
            &potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize],
            EFFECT_PARTICLE_TYPE_ID,
            "minecraft:effect",
            expected_position,
            expected_velocity,
            false,
        );
        assert_eq!(
            potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize].option_color,
            Some(expected_color)
        );
        assert_eq!(
            potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize].option_power,
            Some(expected_power)
        );

        let mut instant_potion_random = LevelEventSoundRandomState::with_seed(0);
        let instant_potion = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2007,
                data: 0x00aa_bbcc,
                ..level_event_packet(2007)
            },
            &mut instant_potion_random,
        );
        assert_eq!(instant_potion.len(), 108);
        assert_item_break_particle_command(
            &instant_potion.commands[0],
            VANILLA_SPLASH_POTION_ITEM_ID,
            [10.5, 64.0, -2.5],
            first_item_break_particle_velocity(0),
        );
        assert_eq!(
            instant_potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize].particle_type_id,
            INSTANT_EFFECT_PARTICLE_TYPE_ID
        );
        assert_eq!(
            instant_potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize].particle_id,
            "minecraft:instant_effect"
        );
        assert_eq!(
            instant_potion.commands[POTION_BREAK_ITEM_PARTICLE_COUNT as usize].option_power,
            Some(first_potion_break_spell_particle(0x00aa_bbcc).3)
        );

        let mut ender_eye_random = LevelEventSoundRandomState::with_seed(0);
        let ender_eye = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 2003,
                ..level_event_packet(2003)
            },
            &mut ender_eye_random,
        );
        assert_eq!(ender_eye.len(), 88);
        assert_item_break_particle_command(
            &ender_eye.commands[0],
            VANILLA_ENDER_EYE_ITEM_ID,
            [10.5, 64.0, -2.5],
            first_item_break_particle_velocity(0),
        );
        let first_portal_index = ITEM_BREAK_PARTICLE_COUNT as usize;
        assert_eq!(
            ender_eye.commands[first_portal_index].sprite_ids,
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
            &ender_eye.commands[first_portal_index],
            60,
            "minecraft:portal",
            [15.5, 63.6, -2.5],
            [-5.0, 0.0, -0.0],
            false,
        );
        assert_particle_command(
            &ender_eye.commands[first_portal_index + 1],
            60,
            "minecraft:portal",
            [15.5, 63.6, -2.5],
            [-7.0, 0.0, -0.0],
            false,
        );
        assert_particle_command(
            &ender_eye.commands[first_portal_index + 20],
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

        let mut sculk_charge_full_random = LevelEventSoundRandomState::with_seed(0);
        let sculk_charge_full_data = 2 << 6;
        let sculk_charge_full = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3006,
                data: sculk_charge_full_data,
                ..level_event_packet(3006)
            },
            &mut sculk_charge_full_random,
        );
        let expected_sculk_charge_full = expected_sculk_charge_particles(sculk_charge_full_data);
        assert_eq!(sculk_charge_full.len(), expected_sculk_charge_full.len());
        assert_sculk_charge_command(
            &sculk_charge_full.commands[0],
            &expected_sculk_charge_full[0],
        );
        assert_eq!(
            sculk_charge_full.commands[0].sprite_ids,
            vec!["minecraft:sculk_charge_0".to_string()]
        );
        assert_eq!(
            sculk_charge_full.commands[0].option_roll,
            Some(expected_sculk_charge_full[0].roll)
        );

        let mut sculk_charge_mask_random = LevelEventSoundRandomState::with_seed(0);
        let sculk_charge_mask_data = (3 << 6) | (1 << 1) | (1 << 4);
        let sculk_charge_mask = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3006,
                data: sculk_charge_mask_data,
                ..level_event_packet(3006)
            },
            &mut sculk_charge_mask_random,
        );
        let expected_sculk_charge_mask = expected_sculk_charge_particles(sculk_charge_mask_data);
        assert_eq!(sculk_charge_mask.len(), expected_sculk_charge_mask.len());
        assert_sculk_charge_command(
            &sculk_charge_mask.commands[0],
            &expected_sculk_charge_mask[0],
        );
        assert_eq!(
            sculk_charge_mask.commands[0].option_roll,
            Some(std::f32::consts::PI)
        );
        let first_west_expected = expected_sculk_charge_mask
            .iter()
            .find(|expected| expected.direction == (-1, 0, 0))
            .expect("west multiface particle");
        let first_west_actual = sculk_charge_mask
            .commands
            .iter()
            .find(|command| {
                (command.position[0] - (10.5 - SCULK_CHARGE_MULTIFACE_FACTOR)).abs() < 1.0e-12
            })
            .expect("west multiface command");
        assert_sculk_charge_command(first_west_actual, first_west_expected);
        assert_eq!(first_west_actual.option_roll, Some(0.0));

        let mut sculk_charge_pop_random = LevelEventSoundRandomState::with_seed(0);
        let sculk_charge_pop = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3006,
                data: 0,
                ..level_event_packet(3006)
            },
            &mut sculk_charge_pop_random,
        );
        let expected_sculk_charge_pop = expected_sculk_charge_pop_particles(false);
        assert_eq!(sculk_charge_pop.len(), expected_sculk_charge_pop.len());
        assert_sculk_charge_pop_command(
            &sculk_charge_pop.commands[0],
            &expected_sculk_charge_pop[0],
        );
        assert_eq!(
            sculk_charge_pop.commands[0].sprite_ids,
            vec!["minecraft:sculk_charge_pop_0".to_string()]
        );

        let mut sculk_charge_pop_full_random = LevelEventSoundRandomState::with_seed(0);
        let sculk_charge_pop_full = resolver.resolve_level_event_particles_with_context(
            &LevelEvent {
                event_type: 3006,
                data: 0,
                ..level_event_packet(3006)
            },
            LevelEventParticleContext {
                sculk_charge_pop_full_block: Some(true),
                ..LevelEventParticleContext::default()
            },
            &mut sculk_charge_pop_full_random,
        );
        let expected_sculk_charge_pop_full = expected_sculk_charge_pop_particles(true);
        assert_eq!(
            sculk_charge_pop_full.len(),
            expected_sculk_charge_pop_full.len()
        );
        assert_sculk_charge_pop_command(
            &sculk_charge_pop_full.commands[0],
            &expected_sculk_charge_pop_full[0],
        );

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

        let mut missing_vault_activation_random = LevelEventSoundRandomState::with_seed(0);
        let missing_vault_activation = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3015,
                data: 0,
                ..level_event_packet(3015)
            },
            &mut missing_vault_activation_random,
        );
        assert!(missing_vault_activation.is_empty());

        let vault_activation_position = first_vault_activation_particle();
        let mut vault_activation_random = LevelEventSoundRandomState::with_seed(0);
        let vault_activation = resolver.resolve_level_event_particles_with_context(
            &LevelEvent {
                event_type: 3015,
                data: 0,
                ..level_event_packet(3015)
            },
            LevelEventParticleContext {
                vault_block_entity_at_event_pos: true,
                ..LevelEventParticleContext::default()
            },
            &mut vault_activation_random,
        );
        assert_eq!(vault_activation.len(), 40);
        assert_particle_command(
            &vault_activation.commands[0],
            SMOKE_PARTICLE_TYPE_ID,
            "minecraft:smoke",
            vault_activation_position,
            [0.0, 0.0, 0.0],
            false,
        );
        assert_particle_command(
            &vault_activation.commands[1],
            SMALL_FLAME_PARTICLE_TYPE_ID,
            "minecraft:small_flame",
            vault_activation_position,
            [0.0, 0.0, 0.0],
            false,
        );

        let mut ominous_vault_activation_random = LevelEventSoundRandomState::with_seed(0);
        let ominous_vault_activation = resolver.resolve_level_event_particles_with_context(
            &LevelEvent {
                event_type: 3015,
                data: 1,
                ..level_event_packet(3015)
            },
            LevelEventParticleContext {
                vault_block_entity_at_event_pos: true,
                ..LevelEventParticleContext::default()
            },
            &mut ominous_vault_activation_random,
        );
        assert_eq!(ominous_vault_activation.len(), 40);
        assert_eq!(
            ominous_vault_activation.commands[1].particle_type_id,
            SOUL_FIRE_FLAME_PARTICLE_TYPE_ID
        );
        assert_eq!(
            ominous_vault_activation.commands[1].particle_id,
            "minecraft:soul_fire_flame"
        );

        let (vault_deactivation_position, vault_deactivation_velocity) =
            first_vault_deactivation_particle();
        let mut vault_deactivation_random = LevelEventSoundRandomState::with_seed(0);
        let vault_deactivation = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3016,
                data: 0,
                ..level_event_packet(3016)
            },
            &mut vault_deactivation_random,
        );
        assert_eq!(vault_deactivation.len(), 20);
        assert_particle_command(
            &vault_deactivation.commands[0],
            SMALL_FLAME_PARTICLE_TYPE_ID,
            "minecraft:small_flame",
            vault_deactivation_position,
            vault_deactivation_velocity,
            false,
        );

        let mut ominous_vault_deactivation_random = LevelEventSoundRandomState::with_seed(0);
        let ominous_vault_deactivation = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: 3016,
                data: 1,
                ..level_event_packet(3016)
            },
            &mut ominous_vault_deactivation_random,
        );
        assert_eq!(ominous_vault_deactivation.len(), 20);
        assert_particle_command(
            &ominous_vault_deactivation.commands[0],
            SOUL_FIRE_FLAME_PARTICLE_TYPE_ID,
            "minecraft:soul_fire_flame",
            vault_deactivation_position,
            vault_deactivation_velocity,
            false,
        );

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

        let mut bee_growth_half_height_random = LevelEventSoundRandomState::with_seed(0);
        let bee_growth_half_height = resolver.resolve_level_event_particles_with_context(
            &LevelEvent {
                event_type: 2011,
                data: 3,
                ..level_event_packet(2011)
            },
            LevelEventParticleContext {
                in_block_particle_spread_height: Some(0.5),
                ..LevelEventParticleContext::default()
            },
            &mut bee_growth_half_height_random,
        );
        assert_eq!(bee_growth_half_height.len(), 3);
        assert_particle_command(
            &bee_growth_half_height.commands[0],
            43,
            "minecraft:happy_villager",
            [
                10.597_545_277_797_202,
                64.166_609_199_738_33,
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
    fn level_event_destroy_block_particles_use_block_particle_options() {
        let resolver = test_resolver(0);
        let stone_id = test_block_state_id("minecraft:stone", []);
        let bottom_slab_id = test_block_state_id(
            "minecraft:oak_slab",
            [("type", "bottom"), ("waterlogged", "false")],
        );
        let barrier_id = test_block_state_id("minecraft:barrier", [("waterlogged", "false")]);
        let structure_void_id = test_block_state_id("minecraft:structure_void", []);
        let moving_piston_id = test_block_state_id(
            "minecraft:moving_piston",
            [("facing", "north"), ("type", "normal")],
        );
        let event = LevelEvent {
            event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
            data: stone_id,
            ..level_event_packet(DESTROY_BLOCK_PARTICLES_LEVEL_EVENT)
        };
        let mut random = LevelEventSoundRandomState::with_seed(0);

        let batch = resolver.resolve_level_event_particles(&event, &mut random);

        assert_eq!(batch.len(), 64);
        assert_eq!(batch.missing_definition_count, 0);
        assert_eq!(batch.missing_sprite_count, 0);
        assert_block_destroy_particle_command(
            &batch.commands[0],
            stone_id,
            [10.125, 64.125, -2.875],
            [-0.375, -0.375, -0.375],
        );
        assert_block_destroy_particle_command(
            &batch.commands[63],
            stone_id,
            [10.875, 64.875, -2.125],
            [0.375, 0.375, 0.375],
        );

        let mut brush_random = LevelEventSoundRandomState::with_seed(0);
        let brush = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: BRUSH_BLOCK_COMPLETE_LEVEL_EVENT,
                data: stone_id,
                ..level_event_packet(BRUSH_BLOCK_COMPLETE_LEVEL_EVENT)
            },
            &mut brush_random,
        );
        assert_eq!(brush.len(), 64);
        assert_block_destroy_particle_command(
            &brush.commands[0],
            stone_id,
            [10.125, 64.125, -2.875],
            [-0.375, -0.375, -0.375],
        );

        let mut slab_random = LevelEventSoundRandomState::with_seed(0);
        let slab = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
                data: bottom_slab_id,
                ..level_event_packet(DESTROY_BLOCK_PARTICLES_LEVEL_EVENT)
            },
            &mut slab_random,
        );
        assert_eq!(slab.len(), 32);
        assert_block_destroy_particle_command(
            &slab.commands[0],
            bottom_slab_id,
            [10.125, 64.125, -2.875],
            [-0.375, -0.25, -0.375],
        );
        assert_block_destroy_particle_command(
            &slab.commands[31],
            bottom_slab_id,
            [10.875, 64.375, -2.125],
            [0.375, 0.25, 0.375],
        );

        let mut air_random = LevelEventSoundRandomState::with_seed(0);
        let air = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
                data: AIR_BLOCK_STATE_ID,
                ..level_event_packet(DESTROY_BLOCK_PARTICLES_LEVEL_EVENT)
            },
            &mut air_random,
        );
        assert!(air.is_empty());

        for block_state_id in [barrier_id, structure_void_id] {
            let mut random = LevelEventSoundRandomState::with_seed(0);
            let batch = resolver.resolve_level_event_particles(
                &LevelEvent {
                    event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
                    data: block_state_id,
                    ..level_event_packet(DESTROY_BLOCK_PARTICLES_LEVEL_EVENT)
                },
                &mut random,
            );
            assert!(batch.is_empty(), "{block_state_id}");
        }

        let mut moving_piston_random = LevelEventSoundRandomState::with_seed(0);
        let moving_piston = resolver.resolve_level_event_particles(
            &LevelEvent {
                event_type: DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
                data: moving_piston_id,
                ..level_event_packet(DESTROY_BLOCK_PARTICLES_LEVEL_EVENT)
            },
            &mut moving_piston_random,
        );
        assert_eq!(moving_piston.len(), 64);
        assert_block_destroy_particle_command(
            &moving_piston.commands[0],
            moving_piston_id,
            [10.125, 64.125, -2.875],
            [-0.375, -0.375, -0.375],
        );
    }

    #[test]
    fn level_event_smash_attack_particles_use_vanilla_dust_pillar_context() {
        let resolver = test_resolver(0);
        let event = LevelEvent {
            event_type: SMASH_ATTACK_PARTICLES_LEVEL_EVENT,
            data: 6,
            ..level_event_packet(SMASH_ATTACK_PARTICLES_LEVEL_EVENT)
        };
        let context = LevelEventParticleContext {
            block_state_id_at_event_pos: Some(9),
            ..LevelEventParticleContext::default()
        };
        let mut random = LevelEventSoundRandomState::with_seed(0);

        let batch =
            resolver.resolve_level_event_particles_with_context(&event, context, &mut random);

        let expected = expected_smash_attack_particles(event.data);
        assert_eq!(batch.len(), 6);
        assert_eq!(batch.len(), expected.len());
        assert_eq!(batch.missing_sprite_count, 0);
        assert_particle_command(
            &batch.commands[0],
            DUST_PILLAR_PARTICLE_TYPE_ID,
            "minecraft:dust_pillar",
            expected[0].0,
            expected[0].1,
            false,
        );
        assert_eq!(batch.commands[0].sprite_ids, Vec::<String>::new());
        assert_particle_command(
            &batch.commands[2],
            DUST_PILLAR_PARTICLE_TYPE_ID,
            "minecraft:dust_pillar",
            expected[2].0,
            expected[2].1,
            false,
        );
        for command in &batch.commands {
            assert_eq!(
                command.option_block,
                Some(ParticleBlockOptionState { block_state_id: 9 })
            );
            assert_eq!(command.option_item, None);
        }

        let mut rejected_random = LevelEventSoundRandomState::with_seed(0);
        let rejected = resolver.resolve_level_event_particles_with_context(
            &LevelEvent { data: 1, ..event },
            LevelEventParticleContext::default(),
            &mut rejected_random,
        );
        assert!(rejected.is_empty());

        let mut accepted_random = LevelEventSoundRandomState::with_seed(0);
        let accepted = resolver.resolve_level_event_particles_with_context(
            &LevelEvent { data: 1, ..event },
            context,
            &mut accepted_random,
        );
        assert_eq!(accepted.len(), 2);

        let cloud_event = LevelEvent {
            event_type: SPLASH_CLOUD_LEVEL_EVENT,
            ..level_event_packet(SPLASH_CLOUD_LEVEL_EVENT)
        };
        let rejected_cloud =
            resolver.resolve_level_event_particles(&cloud_event, &mut rejected_random);
        let accepted_cloud =
            resolver.resolve_level_event_particles(&cloud_event, &mut accepted_random);
        assert_eq!(rejected_cloud.len(), 8);
        assert_eq!(accepted_cloud.len(), 8);
        assert_eq!(
            rejected_cloud.commands[0].position,
            accepted_cloud.commands[0].position
        );
        assert_eq!(
            rejected_cloud.commands[0].velocity,
            accepted_cloud.commands[0].velocity
        );
    }

    #[test]
    fn level_event_particle_resolver_covers_vanilla_26_1_particle_events() {
        let resolver = test_resolver(0);
        let stone_id = test_block_state_id("minecraft:stone", []);
        let cases = [
            (COMPOSTER_FILL_LEVEL_EVENT, 0, "composter fill"),
            (LAVA_EXTINGUISH_LEVEL_EVENT, 0, "lava extinguish"),
            (
                REDSTONE_TORCH_BURNOUT_LEVEL_EVENT,
                0,
                "redstone torch burnout",
            ),
            (
                END_PORTAL_FRAME_FILL_LEVEL_EVENT,
                0,
                "end portal frame fill",
            ),
            (DRIPSTONE_DRIP_LEVEL_EVENT, 0, "pointed dripstone drip"),
            (PLANT_GROWTH_LEVEL_EVENT, 2, "plant growth"),
            (DISPENSER_SMOKE_LEVEL_EVENT, 0, "dispenser smoke"),
            (
                DESTROY_BLOCK_PARTICLES_LEVEL_EVENT,
                stone_id,
                "destroy block",
            ),
            (POTION_BREAK_LEVEL_EVENT, 0x0033_66cc, "potion break"),
            (
                INSTANT_POTION_BREAK_LEVEL_EVENT,
                0x0033_66cc,
                "instant potion break",
            ),
            (ENDER_EYE_BREAK_LEVEL_EVENT, 0, "ender eye break"),
            (BLAZE_SMOKE_LEVEL_EVENT, 0, "blaze smoke"),
            (
                DRAGON_FIREBALL_EXPLODE_LEVEL_EVENT,
                0,
                "dragon fireball explode",
            ),
            (EXPLOSION_LEVEL_EVENT, 0, "explosion"),
            (SPLASH_CLOUD_LEVEL_EVENT, 0, "splash cloud"),
            (
                DISPENSER_WHITE_SMOKE_LEVEL_EVENT,
                0,
                "dispenser white smoke",
            ),
            (BEE_GROWTH_PARTICLES_LEVEL_EVENT, 1, "bee growth"),
            (
                TURTLE_EGG_PLACEMENT_PARTICLES_LEVEL_EVENT,
                1,
                "turtle egg placement",
            ),
            (SMASH_ATTACK_PARTICLES_LEVEL_EVENT, 3, "smash attack"),
            (END_GATEWAY_SPAWN_LEVEL_EVENT, 0, "end gateway spawn"),
            (ELECTRIC_SPARK_LEVEL_EVENT, 0, "electric spark"),
            (WAX_ON_LEVEL_EVENT, 0, "wax on"),
            (WAX_OFF_LEVEL_EVENT, 0, "wax off"),
            (SCRAPE_LEVEL_EVENT, 0, "scrape"),
            (SCULK_CHARGE_LEVEL_EVENT, 2 << 6, "sculk charge"),
            (SCULK_SHRIEK_PARTICLES_LEVEL_EVENT, 0, "sculk shriek"),
            (
                BRUSH_BLOCK_COMPLETE_LEVEL_EVENT,
                stone_id,
                "brush block complete",
            ),
            (EGG_CRACK_LEVEL_EVENT, 0, "egg crack"),
            (
                TRIAL_SPAWNER_SPAWN_PARTICLES_LEVEL_EVENT,
                0,
                "trial spawner spawn particles",
            ),
            (
                TRIAL_SPAWNER_SPAWN_MOB_LEVEL_EVENT,
                1,
                "trial spawner spawn mob",
            ),
            (
                TRIAL_SPAWNER_DETECT_PLAYER_LEVEL_EVENT,
                2,
                "trial spawner detect player",
            ),
            (
                TRIAL_SPAWNER_EJECT_ITEM_LEVEL_EVENT,
                0,
                "trial spawner eject item",
            ),
            (VAULT_ACTIVATE_LEVEL_EVENT, 0, "vault activate"),
            (VAULT_DEACTIVATE_LEVEL_EVENT, 0, "vault deactivate"),
            (
                TRIAL_SPAWNER_EJECT_ITEM_PARTICLES_LEVEL_EVENT,
                0,
                "trial spawner eject item particles",
            ),
            (COBWEB_PLACE_PARTICLES_LEVEL_EVENT, 0, "cobweb place"),
            (
                TRIAL_SPAWNER_DETECT_PLAYER_OMINOUS_LEVEL_EVENT,
                2,
                "ominous trial spawner detect player",
            ),
            (
                TRIAL_SPAWNER_OMINOUS_ACTIVATE_LEVEL_EVENT,
                1,
                "trial spawner ominous activate",
            ),
            (
                TRIAL_SPAWNER_SPAWN_ITEM_LEVEL_EVENT,
                1,
                "trial spawner spawn item",
            ),
        ];

        for (event_type, data, label) in cases {
            let event = LevelEvent {
                event_type,
                data,
                ..level_event_packet(event_type)
            };
            let mut random = LevelEventSoundRandomState::with_seed(0);
            let batch = resolver.resolve_level_event_particles_with_context(
                &event,
                representative_level_event_particle_context(&event, stone_id),
                &mut random,
            );

            assert!(
                !batch.commands.is_empty(),
                "vanilla LevelEvent particle case {event_type} ({label}) must be mapped"
            );
            assert_eq!(batch.missing_definition_count, 0, "{label}");
            assert_eq!(batch.missing_sprite_count, 0, "{label}");
            assert_eq!(batch.unknown_particle_type_count, 0, "{label}");
        }
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
                has_translucent: false,
            }]
        );
    }

    #[test]
    fn particle_atlas_animation_frame_uses_sprite_animation_tick() {
        let mut image = SpriteImage::new("minecraft:vibration", 1, 1, vec![10, 0, 0, 255]).unwrap();
        image.animation = Some(SpriteAnimation {
            frame_count: 2,
            default_frame_time: 1,
            interpolate: false,
            frames: vec![
                SpriteAnimationFrame { index: 0, time: 2 },
                SpriteAnimationFrame { index: 1, time: 1 },
            ],
        });
        image.animation_frames_rgba = vec![vec![10, 0, 0, 255], vec![20, 0, 0, 255]];

        let atlas = particle_atlas_from_images(vec![image]).unwrap();
        let tick_zero = atlas.animation_atlas_frame(0).unwrap().unwrap();
        let tick_two = atlas.animation_atlas_frame(2).unwrap().unwrap();

        assert!(atlas.has_animation());
        assert_eq!(
            (tick_zero.width, tick_zero.height),
            (atlas.width, atlas.height)
        );
        assert_eq!(
            (tick_two.width, tick_two.height),
            (atlas.width, atlas.height)
        );
        assert_eq!(
            atlas_pixel(&tick_zero.rgba, tick_zero.width, 1, 1),
            [10, 0, 0, 255]
        );
        assert_eq!(
            atlas_pixel(&tick_two.rgba, tick_two.width, 1, 1),
            [20, 0, 0, 255]
        );
    }

    #[test]
    fn particle_texture_animation_tick_advances_at_vanilla_interval() {
        let atlas = particle_atlas_from_images(vec![SpriteImage::new(
            "minecraft:generic_0",
            1,
            1,
            vec![10, 0, 0, 255],
        )
        .unwrap()])
        .unwrap();
        let mut runtime = NativeParticleRuntime {
            resolver: test_resolver(0),
            atlas,
            texture_animation_tick: 0,
            last_texture_animation_at: None,
        };
        let start = Instant::now();

        assert_eq!(
            advance_particle_texture_animation_tick(&mut runtime, start),
            None
        );
        assert_eq!(
            advance_particle_texture_animation_tick(
                &mut runtime,
                start + PARTICLE_TEXTURE_ANIMATION_INTERVAL - Duration::from_millis(1),
            ),
            None
        );
        assert_eq!(
            advance_particle_texture_animation_tick(
                &mut runtime,
                start + PARTICLE_TEXTURE_ANIMATION_INTERVAL * 3,
            ),
            Some(3)
        );
        assert_eq!(runtime.texture_animation_tick, 3);
    }

    fn atlas_pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let offset = usize::try_from((y * width + x) * 4).unwrap();
        rgba[offset..offset + 4].try_into().unwrap()
    }

    fn test_resolver(seed: i64) -> ParticleCommandResolver {
        test_resolver_with_particle_status(seed, ClientParticleStatus::All)
    }

    fn representative_level_event_particle_context(
        event: &LevelEvent,
        block_state_id: i32,
    ) -> LevelEventParticleContext {
        match event.event_type {
            DRIPSTONE_DRIP_LEVEL_EVENT => LevelEventParticleContext {
                dripstone_drip_particle: Some(LevelEventDripstoneDripParticle::Water),
                ..LevelEventParticleContext::default()
            },
            PLANT_GROWTH_LEVEL_EVENT => LevelEventParticleContext {
                growth_particles: Some(LevelEventGrowthParticleContext {
                    pos: event.pos,
                    mode: LevelEventGrowthParticleMode::InBlock { spread_height: 1.0 },
                }),
                ..LevelEventParticleContext::default()
            },
            SMASH_ATTACK_PARTICLES_LEVEL_EVENT => LevelEventParticleContext {
                block_state_id_at_event_pos: Some(block_state_id),
                ..LevelEventParticleContext::default()
            },
            VAULT_ACTIVATE_LEVEL_EVENT => LevelEventParticleContext {
                vault_block_entity_at_event_pos: true,
                ..LevelEventParticleContext::default()
            },
            _ => LevelEventParticleContext::default(),
        }
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
                "tinted_leaf_0",
                "dragon_breath_0",
                "flash",
                "vibration",
                "sculk_charge_0",
                "sculk_charge_pop_0",
                "flame",
                "soul_fire_flame",
                "explosion_0",
                "gust_0",
                "smoke_0",
                "large_smoke_0",
                "lava",
                "white_smoke_0",
                "dripping_dripstone_lava",
                "dripping_dripstone_water",
                "poof_0",
                "happy_villager_0",
                "composter_0",
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
            &particle_dir(&root).join("tinted_leaves.json"),
            r#"{
              "textures": [
                "minecraft:tinted_leaf_0"
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
            &particle_dir(&root).join("gust.json"),
            r#"{
              "textures": [
                "minecraft:gust_0"
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
            &particle_dir(&root).join("falling_dust.json"),
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
            &particle_dir(&root).join("sculk_charge_pop.json"),
            r#"{
              "textures": [
                "minecraft:sculk_charge_pop_0"
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
            &particle_dir(&root).join("dripping_dripstone_lava.json"),
            r#"{
              "textures": [
                "minecraft:dripping_dripstone_lava"
              ]
            }"#,
        );
        write_json(
            &particle_dir(&root).join("dripping_dripstone_water.json"),
            r#"{
              "textures": [
                "minecraft:dripping_dripstone_water"
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
            &particle_dir(&root).join("composter.json"),
            r#"{
              "textures": [
                "minecraft:composter_0"
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

    fn block_particle_options(block_state_id: i32) -> Vec<u8> {
        let mut out = Vec::new();
        write_positive_var_i32(&mut out, block_state_id);
        out
    }

    fn item_particle_options(item_id: i32, count: i32, added_components: i32) -> Vec<u8> {
        let mut out = Vec::new();
        write_positive_var_i32(&mut out, item_id);
        write_positive_var_i32(&mut out, count);
        write_positive_var_i32(&mut out, added_components);
        write_positive_var_i32(&mut out, 0);
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

    fn test_block_state_id<const N: usize>(name: &str, props: [(&str, &str); N]) -> i32 {
        let properties = props
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        bbb_world::BlockStateRegistry::vanilla_26_1()
            .find_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("missing test block state {name} {properties:?}"))
            .id
    }

    fn first_composter_fill_particle(center_shape_max_y: f64) -> ([f64; 3], [f64; 3]) {
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let center_height = center_shape_max_y + COMPOSTER_FILL_CENTER_HEIGHT_OFFSET;
        let velocity = [
            random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
            random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
            random.next_gaussian() * COMPOSTER_FILL_VELOCITY_SCALE,
        ];
        let position = [
            10.0 + COMPOSTER_FILL_SIDE_OFFSET
                + COMPOSTER_FILL_WIDTH * f64::from(random.next_float()),
            64.0 + center_height + f64::from(random.next_float()) * (1.0 - center_height),
            -3.0 + COMPOSTER_FILL_SIDE_OFFSET
                + COMPOSTER_FILL_WIDTH * f64::from(random.next_float()),
        ];
        (position, velocity)
    }

    fn first_potion_break_spell_particle(data: i32) -> ([f64; 3], [f64; 3], [f32; 4], f32) {
        let mut random = LevelEventSoundRandomState::with_seed(0);
        for _ in 0..POTION_BREAK_ITEM_PARTICLE_COUNT {
            random.next_gaussian();
            random.next_double();
            random.next_gaussian();
        }
        let dist = random.next_double() * 4.0;
        let angle = random.next_double() * std::f64::consts::TAU;
        let velocity = [
            angle.cos() * dist,
            0.01 + random.next_double() * 0.5,
            angle.sin() * dist,
        ];
        let random_brightness = 0.75 + random.next_float() * 0.25;
        let red = ((data >> 16) & 0xFF) as f32 / 255.0;
        let green = ((data >> 8) & 0xFF) as f32 / 255.0;
        let blue = (data & 0xFF) as f32 / 255.0;
        (
            [10.5 + velocity[0] * 0.1, 64.3, -2.5 + velocity[2] * 0.1],
            velocity,
            [
                red * random_brightness,
                green * random_brightness,
                blue * random_brightness,
                1.0,
            ],
            dist as f32,
        )
    }

    fn first_item_break_particle_velocity(seed: i64) -> [f64; 3] {
        let mut random = LevelEventSoundRandomState::with_seed(seed);
        [
            random.next_gaussian() * ITEM_BREAK_HORIZONTAL_VELOCITY_SCALE,
            random.next_double() * ITEM_BREAK_VERTICAL_VELOCITY_SCALE,
            random.next_gaussian() * ITEM_BREAK_HORIZONTAL_VELOCITY_SCALE,
        ]
    }

    fn first_vault_deactivation_particle() -> ([f64; 3], [f64; 3]) {
        let mut random = LevelEventSoundRandomState::with_seed(0);
        (
            [
                10.0 + expected_random_between(&mut random, 0.4, 0.6),
                64.0 + expected_random_between(&mut random, 0.4, 0.6),
                -3.0 + expected_random_between(&mut random, 0.4, 0.6),
            ],
            [
                random.next_gaussian() * 0.02,
                random.next_gaussian() * 0.02,
                random.next_gaussian() * 0.02,
            ],
        )
    }

    fn first_vault_activation_particle() -> [f64; 3] {
        let mut random = LevelEventSoundRandomState::with_seed(0);
        [
            10.0 + expected_random_between(&mut random, 0.1, 0.9),
            64.0 + expected_random_between(&mut random, 0.25, 0.75),
            -3.0 + expected_random_between(&mut random, 0.1, 0.9),
        ]
    }

    fn first_growth_wide_particle(pos: BlockPos) -> ([f64; 3], [f64; 3]) {
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let velocity = [
            random.next_gaussian() * 0.02,
            random.next_gaussian() * 0.02,
            random.next_gaussian() * 0.02,
        ];
        let position = [
            f64::from(pos.x)
                + GROWTH_PARTICLE_WIDE_START_OFFSET
                + random.next_double() * GROWTH_PARTICLE_WIDE_SPREAD * 2.0,
            f64::from(pos.y) + random.next_double() * GROWTH_PARTICLE_WIDE_HEIGHT,
            f64::from(pos.z)
                + GROWTH_PARTICLE_WIDE_START_OFFSET
                + random.next_double() * GROWTH_PARTICLE_WIDE_SPREAD * 2.0,
        ];
        (position, velocity)
    }

    fn expected_smash_attack_particles(count: i32) -> Vec<([f64; 3], [f64; 3])> {
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let center = [10.5, 65.0, -2.5];
        let mut particles = Vec::new();

        for _ in 0..smash_attack_particle_loop_count(count, 3.0) {
            particles.push((
                [
                    center[0] + random.next_gaussian() / 2.0,
                    center[1],
                    center[2] + random.next_gaussian() / 2.0,
                ],
                [
                    random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
                    random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
                    random.next_gaussian() * SMASH_ATTACK_CENTER_SPEED_SCALE,
                ],
            ));
        }

        for i in 0..smash_attack_particle_loop_count(count, 1.5) {
            let angle = i as f64;
            particles.push((
                [
                    center[0] + 3.5 * angle.cos() + random.next_gaussian() / 2.0,
                    center[1],
                    center[2] + 3.5 * angle.sin() + random.next_gaussian() / 2.0,
                ],
                [
                    random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
                    random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
                    random.next_gaussian() * SMASH_ATTACK_RING_SPEED_SCALE,
                ],
            ));
        }

        particles
    }

    #[derive(Debug)]
    struct ExpectedSculkChargeParticle {
        direction: (i32, i32, i32),
        position: [f64; 3],
        velocity: [f64; 3],
        roll: f32,
    }

    #[derive(Debug)]
    struct ExpectedSculkChargePopParticle {
        position: [f64; 3],
        velocity: [f64; 3],
    }

    fn expected_sculk_charge_particles(data: i32) -> Vec<ExpectedSculkChargeParticle> {
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let count = data >> 6;
        if count <= 0 {
            return Vec::new();
        }

        let mut particles = Vec::new();
        let particle_data = data & 63;
        if particle_data == 0 {
            for direction in BLOCK_FACE_DIRECTIONS {
                let roll = if *direction == BLOCK_FACE_DIRECTION_DOWN {
                    std::f32::consts::PI
                } else {
                    0.0
                };
                let step_factor = if direction.1 != 0 {
                    SCULK_CHARGE_FULL_BLOCK_Y_FACTOR
                } else {
                    SCULK_CHARGE_FULL_BLOCK_SIDE_FACTOR
                };
                append_expected_sculk_charge_face_particles(
                    &mut particles,
                    *direction,
                    step_factor,
                    roll,
                    count,
                    &mut random,
                );
            }
        } else {
            for (direction_index, direction) in BLOCK_FACE_DIRECTIONS.iter().enumerate() {
                if particle_data & (1 << direction_index) == 0 {
                    continue;
                }
                let roll = if *direction == BLOCK_FACE_DIRECTION_UP {
                    std::f32::consts::PI
                } else {
                    0.0
                };
                append_expected_sculk_charge_face_particles(
                    &mut particles,
                    *direction,
                    SCULK_CHARGE_MULTIFACE_FACTOR,
                    roll,
                    count,
                    &mut random,
                );
            }
        }
        particles
    }

    fn append_expected_sculk_charge_face_particles(
        particles: &mut Vec<ExpectedSculkChargeParticle>,
        direction: (i32, i32, i32),
        step_factor: f64,
        roll: f32,
        count: i32,
        random: &mut LevelEventSoundRandomState,
    ) {
        let particle_count = random.next_int_bound(count + 1);
        for _ in 0..particle_count {
            let speed = [
                expected_random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
                expected_random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
                expected_random_between(random, -SCULK_CHARGE_SPEED_VAR, SCULK_CHARGE_SPEED_VAR),
            ];
            let position = expected_block_face_position(direction, step_factor, random);
            let velocity = [
                if direction.0 == 0 { speed[0] } else { 0.0 },
                if direction.1 == 0 { speed[1] } else { 0.0 },
                if direction.2 == 0 { speed[2] } else { 0.0 },
            ];
            particles.push(ExpectedSculkChargeParticle {
                direction,
                position,
                velocity,
                roll,
            });
        }
    }

    fn expected_sculk_charge_pop_particles(
        is_full_block: bool,
    ) -> Vec<ExpectedSculkChargePopParticle> {
        let mut random = LevelEventSoundRandomState::with_seed(0);
        let particle_count = if is_full_block { 40 } else { 20 };
        let spread = if is_full_block {
            SCULK_CHARGE_POP_FULL_BLOCK_SPREAD
        } else {
            SCULK_CHARGE_POP_PARTIAL_BLOCK_SPREAD
        };
        (0..particle_count)
            .map(|_| {
                let velocity_x = 2.0 * f64::from(random.next_float()) - 1.0;
                let velocity_y = 2.0 * f64::from(random.next_float()) - 1.0;
                let velocity_z = 2.0 * f64::from(random.next_float()) - 1.0;
                ExpectedSculkChargePopParticle {
                    position: [
                        10.5 + velocity_x * spread,
                        64.5 + velocity_y * spread,
                        -2.5 + velocity_z * spread,
                    ],
                    velocity: [
                        velocity_x * SCULK_CHARGE_POP_SPEED,
                        velocity_y * SCULK_CHARGE_POP_SPEED,
                        velocity_z * SCULK_CHARGE_POP_SPEED,
                    ],
                }
            })
            .collect()
    }

    fn expected_block_face_position(
        (step_x, step_y, step_z): (i32, i32, i32),
        step_factor: f64,
        random: &mut LevelEventSoundRandomState,
    ) -> [f64; 3] {
        [
            10.5 + if step_x == 0 {
                expected_random_between(random, -0.5, 0.5)
            } else {
                f64::from(step_x) * step_factor
            },
            64.5 + if step_y == 0 {
                expected_random_between(random, -0.5, 0.5)
            } else {
                f64::from(step_y) * step_factor
            },
            -2.5 + if step_z == 0 {
                expected_random_between(random, -0.5, 0.5)
            } else {
                f64::from(step_z) * step_factor
            },
        ]
    }

    fn expected_random_between(random: &mut LevelEventSoundRandomState, min: f64, max: f64) -> f64 {
        min + random.next_double() * (max - min)
    }

    fn rgb_option(r: u8, g: u8, b: u8) -> [f32; 4] {
        [
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
            1.0,
        ]
    }

    fn rgb_option_06(r: u8, g: u8, b: u8) -> [f32; 4] {
        [
            f32::from(r) / 255.0 * 0.6,
            f32::from(g) / 255.0 * 0.6,
            f32::from(b) / 255.0 * 0.6,
            1.0,
        ]
    }

    fn assert_sculk_charge_command(
        command: &ParticleSpawnCommand,
        expected: &ExpectedSculkChargeParticle,
    ) {
        assert_particle_command(
            command,
            SCULK_CHARGE_PARTICLE_TYPE_ID,
            "minecraft:sculk_charge",
            expected.position,
            expected.velocity,
            true,
        );
        assert_eq!(command.option_roll, Some(expected.roll));
    }

    fn assert_sculk_charge_pop_command(
        command: &ParticleSpawnCommand,
        expected: &ExpectedSculkChargePopParticle,
    ) {
        assert_particle_command(
            command,
            SCULK_CHARGE_POP_PARTICLE_TYPE_ID,
            "minecraft:sculk_charge_pop",
            expected.position,
            expected.velocity,
            true,
        );
        assert_eq!(command.option_roll, None);
    }

    fn assert_item_break_particle_command(
        command: &ParticleSpawnCommand,
        item_id: i32,
        position: [f64; 3],
        velocity: [f64; 3],
    ) {
        assert_eq!(command.particle_type_id, ITEM_PARTICLE_TYPE_ID);
        assert_eq!(command.particle_id, "minecraft:item");
        assert!(command.sprite_ids.is_empty());
        for (actual, expected) in command.position.iter().zip(position) {
            assert_close(*actual, expected);
        }
        for (actual, expected) in command.velocity.iter().zip(velocity) {
            assert_close(*actual, expected);
        }
        assert_eq!(command.override_limiter, false);
        assert_eq!(command.always_show, false);
        assert_eq!(
            command.raw_options_len,
            item_particle_raw_options_len(item_id, 1)
        );
        assert_eq!(command.initial_delay_ticks, 0);
        assert_eq!(
            command.option_item,
            Some(ParticleItemOptionState {
                item_id,
                count: 1,
                component_patch_len: EMPTY_ITEM_COMPONENT_PATCH_OPTION_LEN,
            })
        );
    }

    fn assert_block_destroy_particle_command(
        command: &ParticleSpawnCommand,
        block_state_id: i32,
        position: [f64; 3],
        velocity: [f64; 3],
    ) {
        assert_eq!(command.particle_type_id, BLOCK_PARTICLE_TYPE_ID);
        assert_eq!(command.particle_id, "minecraft:block");
        assert!(command.sprite_ids.is_empty());
        for (actual, expected) in command.position.iter().zip(position) {
            assert_close(*actual, expected);
        }
        for (actual, expected) in command.velocity.iter().zip(velocity) {
            assert_close(*actual, expected);
        }
        assert_eq!(command.override_limiter, false);
        assert_eq!(command.always_show, false);
        assert_eq!(
            command.raw_options_len,
            block_particle_options(block_state_id).len()
        );
        assert_eq!(command.initial_delay_ticks, 0);
        assert_eq!(
            command.option_block,
            Some(ParticleBlockOptionState { block_state_id })
        );
        assert_eq!(command.option_item, None);
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
