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
use bbb_protocol::packets::{
    decode_data_component_patch_summary, BlockPos, ClientParticleStatus,
    FireworkExplosionShapeSummary, FireworkExplosionSummary, ItemStackSummary, LevelEvent,
    LevelParticles, Vec3d,
};
use bbb_renderer::{
    ParticleBlockOptionState, ParticleChildSpawnTemplate, ParticleEntityTargetSource,
    ParticleItemOptionState, ParticleScheduledSoundEvent, ParticleSoundEvent, ParticleSpawnBatch,
    ParticleSpawnCommand, ParticleSpriteUv, ParticleUvRect, Renderer,
};
use bbb_world::{
    block_name_has_invisible_render_shape, block_name_is_air,
    block_name_should_spawn_terrain_particles, AllayDuplicationParticleState,
    AnimalLoveParticleState, ArrowEffectParticleState, BlockPos as WorldBlockPos,
    DolphinHappyParticleState, EntityTamingParticleState, FireworkRocketExplosionParticleState,
    FireworkRocketTrailParticleState, FoxEatParticleState, HoneyBlockParticleState,
    LevelEventSoundRandomState, LivingEntityDrownParticleState, LivingEntityPoofParticleState,
    LivingEntityPortalParticleState, OminousItemSpawnerParticleState, RavagerRoarParticleState,
    SnowballHitParticleState, TakeItemEntityPickupParticleState, TerrainLight,
    ThrownEggHitParticleState, VaultConnectionParticleState, VillagerParticleKind,
    VillagerParticleState, WitchMagicParticleState,
};

use crate::{
    particle_registry::{particle_type_ids, vanilla_particle_type, ParticleTypeInfo},
    terrain_runtime::{BlockRenderPosition, TerrainParticleTintCatalog, TerrainTextureState},
};

const PARTICLE_TEXTURE_ANIMATION_INTERVAL: Duration = Duration::from_millis(50);
const ITEM_PICKUP_PARTICLE_TYPE_ID: i32 = -1;
const ITEM_PICKUP_PARTICLE_ID: &str = "minecraft:item_pickup";
const FIREWORK_ROCKET_BLAST_SOUND_EVENT_ID: &str = "minecraft:entity.firework_rocket.blast";
const FIREWORK_ROCKET_BLAST_FAR_SOUND_EVENT_ID: &str = "minecraft:entity.firework_rocket.blast_far";
const FIREWORK_ROCKET_LARGE_BLAST_SOUND_EVENT_ID: &str =
    "minecraft:entity.firework_rocket.large_blast";
const FIREWORK_ROCKET_LARGE_BLAST_FAR_SOUND_EVENT_ID: &str =
    "minecraft:entity.firework_rocket.large_blast_far";
const FIREWORK_ROCKET_TWINKLE_SOUND_EVENT_ID: &str = "minecraft:entity.firework_rocket.twinkle";
const FIREWORK_ROCKET_TWINKLE_FAR_SOUND_EVENT_ID: &str =
    "minecraft:entity.firework_rocket.twinkle_far";

pub(crate) trait ParticleEventSink {
    fn maybe_upload_particle_atlas_animation(&mut self, _renderer: &mut Renderer) {}

    fn spawn_level_particles(
        &mut self,
        packet: &LevelParticles,
        context: LevelParticleSpawnContext,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch;
    fn spawn_level_event_particles(
        &mut self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
        random: &mut LevelEventSoundRandomState,
    ) -> ParticleSpawnBatch;
    fn spawn_firework_empty_explosion_particles(
        &mut self,
        position: [f64; 3],
        camera_position: Option<[f64; 3]>,
    ) -> ParticleSpawnBatch;
    fn spawn_firework_explosion_particles(
        &mut self,
        state: &FireworkRocketExplosionParticleState,
        camera_position: Option<[f64; 3]>,
    ) -> ParticleSpawnBatch;
    fn spawn_firework_rocket_trail_particles(
        &mut self,
        state: FireworkRocketTrailParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_ominous_item_spawner_particles(
        &mut self,
        state: OminousItemSpawnerParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_tracking_emitter_particles(
        &mut self,
        state: TrackingEmitterParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_take_item_entity_pickup_particles(
        &mut self,
        state: &TakeItemEntityPickupParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_ravager_roar_particles(
        &mut self,
        state: RavagerRoarParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_witch_magic_particles(&mut self, state: WitchMagicParticleState)
        -> ParticleSpawnBatch;
    fn spawn_living_entity_poof_particles(
        &mut self,
        state: LivingEntityPoofParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_living_entity_drown_particles(
        &mut self,
        state: LivingEntityDrownParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_living_entity_portal_particles(
        &mut self,
        state: LivingEntityPortalParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_arrow_effect_particles(
        &mut self,
        state: ArrowEffectParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_animal_love_particles(&mut self, state: AnimalLoveParticleState)
        -> ParticleSpawnBatch;
    fn spawn_allay_duplication_particles(
        &mut self,
        state: AllayDuplicationParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_entity_taming_particles(
        &mut self,
        state: EntityTamingParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_villager_particles(&mut self, state: VillagerParticleState) -> ParticleSpawnBatch;
    fn spawn_dolphin_happy_particles(
        &mut self,
        state: DolphinHappyParticleState,
    ) -> ParticleSpawnBatch;
    fn spawn_fox_eat_particles(
        &mut self,
        state: FoxEatParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch;
    fn spawn_snowball_hit_particles(
        &mut self,
        state: SnowballHitParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch;
    fn spawn_thrown_egg_hit_particles(
        &mut self,
        state: ThrownEggHitParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch;
    fn spawn_honey_block_particles(&mut self, state: HoneyBlockParticleState)
        -> ParticleSpawnBatch;
}

pub(crate) trait ParticleBiomeSampler {
    fn biome_id_at(&self, pos: WorldBlockPos) -> Option<i32>;
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct LevelParticleSpawnContext {
    pub(crate) camera_position: Option<[f64; 3]>,
    pub(crate) vibration_entity_position: Option<LevelParticleEntityPosition>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LevelParticleEntityPosition {
    pub(crate) entity_id: i32,
    pub(crate) position: [f64; 3],
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LevelEventParticleContext {
    pub(crate) sculk_charge_pop_full_block: Option<bool>,
    pub(crate) block_state_id_at_event_pos: Option<i32>,
    pub(crate) biome_id_at_event_pos: Option<i32>,
    pub(crate) vault_block_entity_at_event_pos: bool,
    pub(crate) vault_connection_particles: Option<VaultConnectionParticleState>,
    pub(crate) dripstone_drip_particle: Option<LevelEventDripstoneDripParticle>,
    pub(crate) growth_particles: Option<LevelEventGrowthParticleContext>,
    pub(crate) in_block_particle_spread_height: Option<f64>,
    pub(crate) composter_fill_center_shape_max_y: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TrackingEmitterParticleState {
    pub(crate) particle_type_id: i32,
    pub(crate) position: [f64; 3],
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) lifetime_ticks: u32,
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
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        self.resolver.resolve_level_particles_with_context(
            packet,
            context,
            biome_sampler,
            item_runtime,
        )
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

    fn spawn_firework_empty_explosion_particles(
        &mut self,
        position: [f64; 3],
        camera_position: Option<[f64; 3]>,
    ) -> ParticleSpawnBatch {
        self.resolver
            .firework_empty_explosion_particle_batch(position, camera_position)
    }

    fn spawn_firework_explosion_particles(
        &mut self,
        state: &FireworkRocketExplosionParticleState,
        camera_position: Option<[f64; 3]>,
    ) -> ParticleSpawnBatch {
        self.resolver
            .firework_explosion_particle_batch(state, camera_position)
    }

    fn spawn_firework_rocket_trail_particles(
        &mut self,
        state: FireworkRocketTrailParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.firework_rocket_trail_particle_batch(state)
    }

    fn spawn_ominous_item_spawner_particles(
        &mut self,
        state: OminousItemSpawnerParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.ominous_item_spawner_particle_batch(state)
    }

    fn spawn_tracking_emitter_particles(
        &mut self,
        state: TrackingEmitterParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.tracking_emitter_particle_batch(state)
    }

    fn spawn_take_item_entity_pickup_particles(
        &mut self,
        state: &TakeItemEntityPickupParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.take_item_entity_pickup_particle_batch(state)
    }

    fn spawn_ravager_roar_particles(
        &mut self,
        state: RavagerRoarParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.ravager_roar_particle_batch(state)
    }

    fn spawn_witch_magic_particles(
        &mut self,
        state: WitchMagicParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.witch_magic_particle_batch(state)
    }

    fn spawn_living_entity_poof_particles(
        &mut self,
        state: LivingEntityPoofParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.living_entity_poof_particle_batch(state)
    }

    fn spawn_living_entity_drown_particles(
        &mut self,
        state: LivingEntityDrownParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.living_entity_drown_particle_batch(state)
    }

    fn spawn_living_entity_portal_particles(
        &mut self,
        state: LivingEntityPortalParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.living_entity_portal_particle_batch(state)
    }

    fn spawn_arrow_effect_particles(
        &mut self,
        state: ArrowEffectParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.arrow_effect_particle_batch(state)
    }

    fn spawn_animal_love_particles(
        &mut self,
        state: AnimalLoveParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.animal_love_particle_batch(state)
    }

    fn spawn_allay_duplication_particles(
        &mut self,
        state: AllayDuplicationParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.allay_duplication_particle_batch(state)
    }

    fn spawn_entity_taming_particles(
        &mut self,
        state: EntityTamingParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.entity_taming_particle_batch(state)
    }

    fn spawn_villager_particles(&mut self, state: VillagerParticleState) -> ParticleSpawnBatch {
        self.resolver.villager_particle_batch(state)
    }

    fn spawn_dolphin_happy_particles(
        &mut self,
        state: DolphinHappyParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.dolphin_happy_particle_batch(state)
    }

    fn spawn_fox_eat_particles(
        &mut self,
        state: FoxEatParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        self.resolver.fox_eat_particle_batch(state, item_runtime)
    }

    fn spawn_snowball_hit_particles(
        &mut self,
        state: SnowballHitParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        self.resolver
            .snowball_hit_particle_batch(state, item_runtime)
    }

    fn spawn_thrown_egg_hit_particles(
        &mut self,
        state: ThrownEggHitParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        self.resolver
            .thrown_egg_hit_particle_batch(state, item_runtime)
    }

    fn spawn_honey_block_particles(
        &mut self,
        state: HoneyBlockParticleState,
    ) -> ParticleSpawnBatch {
        self.resolver.honey_block_particle_batch(state)
    }
}

#[derive(Debug, Clone)]
struct ParticleCommandResolver {
    definitions: ParticleDefinitionCatalog,
    sprites: ParticleSpriteCatalog,
    terrain_particle_sprite_ids: HashMap<i32, String>,
    terrain_particle_tint_colors: HashMap<i32, [f32; 4]>,
    falling_dust_block_tint_colors: HashMap<i32, [f32; 4]>,
    terrain_particle_tint_catalog: TerrainParticleTintCatalog,
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
            terrain_particle_tint_catalog: TerrainParticleTintCatalog::default(),
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
        self.terrain_particle_tint_catalog = textures.particle_tint_catalog();
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
            terrain_particle_tint_catalog: TerrainParticleTintCatalog::default(),
            default_item_particle_sprite_ids: HashMap::new(),
            random: LegacyRandom::new(seed),
            particle_level_random: LegacyRandom::new(seed),
            particle_status,
        }
    }

    fn resolve_level_particles(&mut self, packet: &LevelParticles) -> ParticleSpawnBatch {
        self.resolve_level_particles_with_context(
            packet,
            LevelParticleSpawnContext::default(),
            None,
            None,
        )
    }

    fn resolve_level_particles_with_context(
        &mut self,
        packet: &LevelParticles,
        context: LevelParticleSpawnContext,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
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
        let mut option_state =
            particle_option_render_state(particle_type.id, &packet.particle.raw_options);
        resolve_vibration_entity_target(&mut option_state, context);
        let provider_accepts_spawn =
            particle_provider_accepts_spawn(particle_type.id, &option_state);
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
                    option_state.clone(),
                    biome_sampler,
                    item_runtime,
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
                        option_state.clone(),
                        biome_sampler,
                        item_runtime,
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
                self.destroy_block_particle_batch(event, context)
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

    fn destroy_block_particle_batch(
        &self,
        event: &LevelEvent,
        context: LevelEventParticleContext,
    ) -> ParticleSpawnBatch {
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
        let event_pos = WorldBlockPos {
            x: event.pos.x,
            y: event.pos.y,
            z: event.pos.z,
        };
        let option_color = self.terrain_particle_tint_color_for_block_position(
            block_state_id,
            event_pos,
            context.biome_id_at_event_pos,
        );
        let option_state = ParticleOptionRenderState {
            block: Some(ParticleBlockOptionState { block_state_id }),
            color: option_color,
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
                option_state.clone(),
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
                        option_state.clone(),
                        None,
                        None,
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
                    option_state.clone(),
                    None,
                    None,
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
                    option_state.clone(),
                    None,
                    None,
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
                None,
                None,
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
                None,
                None,
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
                None,
                None,
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
                option_state.clone(),
                None,
                None,
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
        let vault_connection = context
            .vault_connection_particles
            .as_ref()
            .map(|_| self.simple_particle_template(VAULT_CONNECTION_PARTICLE_TYPE_ID));
        let smoke = self.append_template_result(&mut batch, smoke);
        let flame = self.append_template_result(&mut batch, flame);
        let vault_connection =
            vault_connection.and_then(|template| self.append_template_result(&mut batch, template));

        if let Some(connection) = context.vault_connection_particles.as_ref() {
            self.append_vault_connection_particles(
                &mut batch,
                connection,
                vault_connection.as_ref(),
                random,
            );
        }

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

    fn append_vault_connection_particles(
        &self,
        batch: &mut ParticleSpawnBatch,
        connection: &VaultConnectionParticleState,
        template: Option<&SimpleParticleTemplate>,
        random: &mut LevelEventSoundRandomState,
    ) {
        let position = Vec3d {
            x: connection.origin[0],
            y: connection.origin[1],
            z: connection.origin[2],
        };
        for target in &connection.targets {
            let direction = [
                target.target_position[0] - connection.origin[0],
                target.target_position[1] - connection.origin[1],
                target.target_position[2] - connection.origin[2],
            ];
            let particle_count = random.next_int_bound(4) + 2;
            for _ in 0..particle_count {
                let velocity = Vec3d {
                    x: direction[0] + f64::from(random.next_float() - 0.5),
                    y: direction[1] + f64::from(random.next_float() - 0.5),
                    z: direction[2] + f64::from(random.next_float() - 0.5),
                };
                if let Some(template) = template {
                    batch
                        .commands
                        .push(self.command_from_template(template, position, velocity, false));
                }
            }
        }
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

    fn firework_empty_explosion_particle_batch(
        &mut self,
        position: [f64; 3],
        camera_position: Option<[f64; 3]>,
    ) -> ParticleSpawnBatch {
        let count = self.random.next_i32(3) + 2;
        let template = match self.simple_particle_template(POOF_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..count {
            let velocity = Vec3d {
                x: self.random.next_gaussian() * 0.05,
                y: 0.005,
                z: self.random.next_gaussian() * 0.05,
            };
            let command_position = Vec3d {
                x: position[0],
                y: position[1],
                z: position[2],
            };
            if self.should_spawn_level_particle(
                template.particle_type.override_limiter,
                false,
                command_position,
                camera_position,
            ) {
                batch.commands.push(self.command_from_template(
                    &template,
                    command_position,
                    velocity,
                    false,
                ));
            }
        }

        batch
    }

    fn firework_explosion_particle_batch(
        &mut self,
        state: &FireworkRocketExplosionParticleState,
        camera_position: Option<[f64; 3]>,
    ) -> ParticleSpawnBatch {
        let firework = self.simple_particle_template(FIREWORK_PARTICLE_TYPE_ID);
        let flash = self.simple_particle_template(FLASH_PARTICLE_TYPE_ID);
        let mut batch = ParticleSpawnBatch::default();
        let firework = self.append_template_result(&mut batch, firework);
        let flash = self.append_template_result(&mut batch, flash);
        let position = Vec3d {
            x: state.position.x,
            y: state.position.y,
            z: state.position.z,
        };
        let movement = Vec3d {
            x: state.delta_movement.x,
            y: state.delta_movement.y,
            z: state.delta_movement.z,
        };

        if !state.explosions.is_empty() {
            batch
                .sound_events
                .push(self.firework_blast_sound_event(state, camera_position));
        }

        for explosion in &state.explosions {
            if let Some(firework) = firework.as_ref() {
                self.append_firework_explosion_sparks(
                    &mut batch, firework, position, movement, explosion,
                );
            }
            if let Some(flash) = flash.as_ref() {
                let colors = firework_explosion_colors(explosion);
                let mut command =
                    self.command_from_template(flash, position, Vec3d::default(), false);
                command.option_color = Some(firework_flash_color(colors[0]));
                batch.commands.push(command);
            }
        }

        if state
            .explosions
            .iter()
            .any(|explosion| explosion.has_twinkle)
        {
            batch
                .scheduled_sound_events
                .push(self.firework_twinkle_sound_event(state));
        }

        batch
    }

    fn firework_rocket_trail_particle_batch(
        &mut self,
        state: FireworkRocketTrailParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(FIREWORK_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let position = Vec3d {
            x: state.position.x,
            y: state.position.y,
            z: state.position.z,
        };
        let velocity = Vec3d {
            x: self.random.next_gaussian() * 0.05,
            y: -state.delta_movement.y * 0.5,
            z: self.random.next_gaussian() * 0.05,
        };
        ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            commands: vec![self.command_from_template(&template, position, velocity, false)],
            ..ParticleSpawnBatch::default()
        }
    }

    fn ominous_item_spawner_particle_batch(
        &mut self,
        state: OminousItemSpawnerParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(OMINOUS_SPAWNING_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let position = Vec3d {
            x: state.position.x,
            y: state.position.y,
            z: state.position.z,
        };
        let particle_count = self.random.next_i32(3) + 1;
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };

        for _ in 0..particle_count {
            let velocity = Vec3d {
                x: 0.4 * (self.random.next_gaussian() - self.random.next_gaussian()),
                y: 0.4 * (self.random.next_gaussian() - self.random.next_gaussian()),
                z: 0.4 * (self.random.next_gaussian() - self.random.next_gaussian()),
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }

        batch
    }

    fn firework_blast_sound_event(
        &mut self,
        state: &FireworkRocketExplosionParticleState,
        camera_position: Option<[f64; 3]>,
    ) -> ParticleSoundEvent {
        let far_effect = camera_position.is_some_and(|camera| {
            let dx = state.position.x - camera[0];
            let dy = state.position.y - camera[1];
            let dz = state.position.z - camera[2];
            dx * dx + dy * dy + dz * dz >= 256.0
        });
        let large_explosion = state.explosions.len() >= 3
            || state
                .explosions
                .iter()
                .any(|explosion| explosion.shape == FireworkExplosionShapeSummary::LargeBall);
        let sound_event_id = match (large_explosion, far_effect) {
            (true, true) => FIREWORK_ROCKET_LARGE_BLAST_FAR_SOUND_EVENT_ID,
            (true, false) => FIREWORK_ROCKET_LARGE_BLAST_SOUND_EVENT_ID,
            (false, true) => FIREWORK_ROCKET_BLAST_FAR_SOUND_EVENT_ID,
            (false, false) => FIREWORK_ROCKET_BLAST_SOUND_EVENT_ID,
        };
        ParticleSoundEvent {
            sound_event_id: sound_event_id.to_string(),
            source: "ambient".to_string(),
            position: [state.position.x, state.position.y, state.position.z],
            volume: 20.0,
            pitch: 0.95 + self.random.next_float() * 0.1,
            seed: self.particle_level_random.next_i64(),
            distance_delay: true,
        }
    }

    fn firework_twinkle_sound_event(
        &mut self,
        state: &FireworkRocketExplosionParticleState,
    ) -> ParticleScheduledSoundEvent {
        ParticleScheduledSoundEvent {
            event: ParticleSoundEvent {
                sound_event_id: FIREWORK_ROCKET_TWINKLE_SOUND_EVENT_ID.to_string(),
                source: "ambient".to_string(),
                position: [state.position.x, state.position.y, state.position.z],
                volume: 20.0,
                pitch: 0.9 + self.random.next_float() * 0.15,
                seed: self.particle_level_random.next_i64(),
                distance_delay: true,
            },
            delay_ticks: firework_twinkle_delay_ticks(state.explosions.len()),
            far_sound_event_id: Some(FIREWORK_ROCKET_TWINKLE_FAR_SOUND_EVENT_ID.to_string()),
            far_distance_squared: Some(256.0),
        }
    }

    fn append_firework_explosion_sparks(
        &mut self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        movement: Vec3d,
        explosion: &FireworkExplosionSummary,
    ) {
        let colors = firework_explosion_colors(explosion);
        match explosion.shape {
            FireworkExplosionShapeSummary::SmallBall => {
                self.append_firework_particle_ball(
                    batch, template, position, 0.25, 2, &colors, explosion,
                );
            }
            FireworkExplosionShapeSummary::LargeBall => {
                self.append_firework_particle_ball(
                    batch, template, position, 0.5, 4, &colors, explosion,
                );
            }
            FireworkExplosionShapeSummary::Star => {
                self.append_firework_particle_shape(
                    batch,
                    template,
                    position,
                    0.5,
                    FIREWORK_STAR_PARTICLE_COORDS,
                    &colors,
                    explosion,
                    false,
                );
            }
            FireworkExplosionShapeSummary::Creeper => {
                self.append_firework_particle_shape(
                    batch,
                    template,
                    position,
                    0.5,
                    FIREWORK_CREEPER_PARTICLE_COORDS,
                    &colors,
                    explosion,
                    true,
                );
            }
            FireworkExplosionShapeSummary::Burst => {
                self.append_firework_particle_burst(
                    batch, template, position, movement, &colors, explosion,
                );
            }
        }
    }

    fn append_firework_particle_ball(
        &mut self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        base_speed: f64,
        steps: i32,
        colors: &[i32],
        explosion: &FireworkExplosionSummary,
    ) {
        for y_step in -steps..=steps {
            for x_step in -steps..=steps {
                let mut z_step = -steps;
                while z_step <= steps {
                    let xa =
                        f64::from(x_step) + (self.random.next_f64() - self.random.next_f64()) * 0.5;
                    let ya =
                        f64::from(y_step) + (self.random.next_f64() - self.random.next_f64()) * 0.5;
                    let za =
                        f64::from(z_step) + (self.random.next_f64() - self.random.next_f64()) * 0.5;
                    let len = (xa * xa + ya * ya + za * za).sqrt() / base_speed
                        + self.random.next_gaussian() * 0.05;
                    let velocity = Vec3d {
                        x: xa / len,
                        y: ya / len,
                        z: za / len,
                    };
                    self.append_firework_spark(
                        batch, template, position, velocity, colors, explosion,
                    );
                    if y_step != -steps && y_step != steps && x_step != -steps && x_step != steps {
                        z_step += steps * 2 - 1;
                    }
                    z_step += 1;
                }
            }
        }
    }

    fn append_firework_particle_shape(
        &mut self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        base_speed: f64,
        coords: &[[f64; 2]],
        colors: &[i32],
        explosion: &FireworkExplosionSummary,
        flat: bool,
    ) {
        let sx = coords[0][0];
        let sy = coords[0][1];
        self.append_firework_spark(
            batch,
            template,
            position,
            Vec3d {
                x: sx * base_speed,
                y: sy * base_speed,
                z: 0.0,
            },
            colors,
            explosion,
        );
        let base_angle = f64::from(self.random.next_float()) * std::f64::consts::PI;
        let angle_mod = if flat { 0.034 } else { 0.34 };

        for angle_step in 0..3 {
            let angle = base_angle + f64::from(angle_step) * std::f64::consts::PI * angle_mod;
            let mut ox = sx;
            let mut oy = sy;
            for coord in coords.iter().skip(1) {
                let tx = coord[0];
                let ty = coord[1];
                for sub_step_index in 1..=4 {
                    let sub_step = f64::from(sub_step_index) * 0.25;
                    let mut xa = lerp_f64(sub_step, ox, tx) * base_speed;
                    let ya = lerp_f64(sub_step, oy, ty) * base_speed;
                    let za = xa * angle.sin();
                    xa *= angle.cos();
                    for flip in [-1.0, 1.0] {
                        self.append_firework_spark(
                            batch,
                            template,
                            position,
                            Vec3d {
                                x: xa * flip,
                                y: ya,
                                z: za * flip,
                            },
                            colors,
                            explosion,
                        );
                    }
                }
                ox = tx;
                oy = ty;
            }
        }
    }

    fn append_firework_particle_burst(
        &mut self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        movement: Vec3d,
        colors: &[i32],
        explosion: &FireworkExplosionSummary,
    ) {
        let base_off_x = self.random.next_gaussian() * 0.05;
        let base_off_z = self.random.next_gaussian() * 0.05;
        for _ in 0..70 {
            let velocity = Vec3d {
                x: movement.x * 0.5 + self.random.next_gaussian() * 0.15 + base_off_x,
                y: movement.y * 0.5 + self.random.next_f64() * 0.5,
                z: movement.z * 0.5 + self.random.next_gaussian() * 0.15 + base_off_z,
            };
            self.append_firework_spark(batch, template, position, velocity, colors, explosion);
        }
    }

    fn append_firework_spark(
        &mut self,
        batch: &mut ParticleSpawnBatch,
        template: &SimpleParticleTemplate,
        position: Vec3d,
        velocity: Vec3d,
        colors: &[i32],
        explosion: &FireworkExplosionSummary,
    ) {
        let mut command = self.command_from_template(template, position, velocity, false);
        command.option_color = Some(firework_spark_color(random_firework_color(
            colors,
            &mut self.random,
        )));
        command.option_firework_trail = explosion.has_trail;
        command.option_firework_twinkle = explosion.has_twinkle;
        if !explosion.fade_colors.is_empty() {
            command.option_color_to = Some(firework_spark_fade_color(random_firework_color(
                &explosion.fade_colors,
                &mut self.random,
            )));
        }
        batch.commands.push(command);
    }

    fn tracking_emitter_particle_batch(
        &mut self,
        state: TrackingEmitterParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(state.particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let width = f64::from(state.width.max(0.0));
        let height = f64::from(state.height.max(0.0));
        for initial_delay_ticks in 0..state.lifetime_ticks {
            for _ in 0..16 {
                let xa = f64::from(self.random.next_float()) * 2.0 - 1.0;
                let ya = f64::from(self.random.next_float()) * 2.0 - 1.0;
                let za = f64::from(self.random.next_float()) * 2.0 - 1.0;
                if xa * xa + ya * ya + za * za > 1.0 {
                    continue;
                }
                let position = Vec3d {
                    x: state.position[0] + width * (xa / 4.0),
                    y: state.position[1] + height * (0.5 + ya / 4.0),
                    z: state.position[2] + width * (za / 4.0),
                };
                let velocity = Vec3d {
                    x: xa,
                    y: ya + 0.2,
                    z: za,
                };
                let mut command = self.command_from_template(&template, position, velocity, false);
                command.initial_delay_ticks = initial_delay_ticks;
                batch.commands.push(command);
            }
        }
        batch
    }

    fn take_item_entity_pickup_particle_batch(
        &mut self,
        state: &TakeItemEntityPickupParticleState,
    ) -> ParticleSpawnBatch {
        let target_y_offset = state.target_eye_height * 0.5;
        ParticleSpawnBatch {
            commands: vec![ParticleSpawnCommand {
                particle_type_id: ITEM_PICKUP_PARTICLE_TYPE_ID,
                particle_id: ITEM_PICKUP_PARTICLE_ID.to_string(),
                sprite_ids: Vec::new(),
                position: [
                    state.item_position.x,
                    state.item_position.y,
                    state.item_position.z,
                ],
                velocity: [
                    state.item_delta_movement.x,
                    state.item_delta_movement.y,
                    state.item_delta_movement.z,
                ],
                override_limiter: true,
                always_show: false,
                raw_options_len: 0,
                initial_delay_ticks: 0,
                child_spawn_templates: Vec::new(),
                option_color: None,
                option_color_to: None,
                option_scale: None,
                option_power: None,
                option_target: Some([
                    state.target_position.x,
                    state.target_position.y + f64::from(target_y_offset),
                    state.target_position.z,
                ]),
                option_entity_target_source: Some(ParticleEntityTargetSource {
                    entity_id: state.target_entity_id,
                    y_offset: target_y_offset,
                }),
                option_duration_ticks: None,
                option_roll: None,
                option_block: None,
                option_item: state
                    .item_stack
                    .as_ref()
                    .and_then(particle_item_option_state_for_stack),
                option_item_pickup_source_entity_id: Some(state.item_entity_id),
                option_item_pickup_age_ticks: Some(state.item_age_ticks),
                option_item_pickup_light: Some(particle_shader_light(state.item_light)),
                option_item_pickup_experience_orb_icon: state.experience_orb_icon,
                option_firework_trail: false,
                option_firework_twinkle: false,
                option_firework_half_lifetime_age: false,
            }],
            ..ParticleSpawnBatch::default()
        }
    }

    fn ravager_roar_particle_batch(
        &mut self,
        state: RavagerRoarParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(POOF_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let position = Vec3d {
            x: state.center.x,
            y: state.center.y,
            z: state.center.z,
        };
        for _ in 0..40 {
            let velocity = Vec3d {
                x: self.random.next_gaussian() * 0.2,
                y: self.random.next_gaussian() * 0.2,
                z: self.random.next_gaussian() * 0.2,
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }
        batch
    }

    fn witch_magic_particle_batch(&mut self, state: WitchMagicParticleState) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(WITCH_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let count = self.random.next_i32(35) + 10;
        for _ in 0..count {
            let position = Vec3d {
                x: state.position.x + self.random.next_gaussian() * 0.13_f32 as f64,
                y: state.bounding_box_max_y + 0.5 + self.random.next_gaussian() * 0.13_f32 as f64,
                z: state.position.z + self.random.next_gaussian() * 0.13_f32 as f64,
            };
            batch.commands.push(self.command_from_template(
                &template,
                position,
                Vec3d::default(),
                false,
            ));
        }
        batch
    }

    fn living_entity_poof_particle_batch(
        &mut self,
        state: LivingEntityPoofParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(POOF_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        for _ in 0..20 {
            let velocity = Vec3d {
                x: self.random.next_gaussian() * 0.02,
                y: self.random.next_gaussian() * 0.02,
                z: self.random.next_gaussian() * 0.02,
            };
            let position = Vec3d {
                x: state.position.x + f64::from(state.width) * (2.0 * self.random.next_f64() - 1.0)
                    - velocity.x * 10.0,
                y: state.position.y + f64::from(state.height) * self.random.next_f64()
                    - velocity.y * 10.0,
                z: state.position.z + f64::from(state.width) * (2.0 * self.random.next_f64() - 1.0)
                    - velocity.z * 10.0,
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }
        batch
    }

    fn living_entity_drown_particle_batch(
        &mut self,
        state: LivingEntityDrownParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(BUBBLE_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let velocity = Vec3d {
            x: state.delta_movement.x,
            y: state.delta_movement.y,
            z: state.delta_movement.z,
        };
        for _ in 0..8 {
            let position = Vec3d {
                x: state.position.x + self.random.next_f64() - self.random.next_f64(),
                y: state.position.y + self.random.next_f64() - self.random.next_f64(),
                z: state.position.z + self.random.next_f64() - self.random.next_f64(),
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }
        batch
    }

    fn living_entity_portal_particle_batch(
        &mut self,
        state: LivingEntityPortalParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(PORTAL_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let width = f64::from(state.width.max(0.0));
        let height = f64::from(state.height.max(0.0));
        for i in 0..128 {
            let alpha = i as f64 / 127.0;
            let velocity = Vec3d {
                x: f64::from((self.random.next_float() - 0.5) * 0.2),
                y: f64::from((self.random.next_float() - 0.5) * 0.2),
                z: f64::from((self.random.next_float() - 0.5) * 0.2),
            };
            let position = Vec3d {
                x: lerp_f64(alpha, state.previous_position.x, state.position.x)
                    + (self.random.next_f64() - 0.5) * width * 2.0,
                y: lerp_f64(alpha, state.previous_position.y, state.position.y)
                    + self.random.next_f64() * height,
                z: lerp_f64(alpha, state.previous_position.z, state.position.z)
                    + (self.random.next_f64() - 0.5) * width * 2.0,
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }
        batch
    }

    fn arrow_effect_particle_batch(
        &mut self,
        state: ArrowEffectParticleState,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(ENTITY_EFFECT_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let width = f64::from(state.width.max(0.0));
        let height = f64::from(state.height.max(0.0));
        let color = rgb_particle_color_u32(state.color_rgb);
        for _ in 0..ARROW_EFFECT_PARTICLE_COUNT {
            let position = Vec3d {
                x: state.position.x + width * ((2.0 * self.random.next_f64() - 1.0) * 0.5),
                y: state.position.y + height * self.random.next_f64(),
                z: state.position.z + width * ((2.0 * self.random.next_f64() - 1.0) * 0.5),
            };
            let mut command =
                self.command_from_template(&template, position, Vec3d::default(), false);
            command.option_color = Some(color);
            batch.commands.push(command);
        }
        batch
    }

    fn animal_love_particle_batch(&mut self, state: AnimalLoveParticleState) -> ParticleSpawnBatch {
        self.entity_event_aabb_particle_batch(
            HEART_PARTICLE_TYPE_ID,
            state.position,
            state.width,
            state.height,
            ANIMAL_LOVE_PARTICLE_COUNT,
            ENTITY_EVENT_DEFAULT_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    fn allay_duplication_particle_batch(
        &mut self,
        state: AllayDuplicationParticleState,
    ) -> ParticleSpawnBatch {
        self.entity_event_aabb_particle_batch(
            HEART_PARTICLE_TYPE_ID,
            state.position,
            state.width,
            state.height,
            ALLAY_DUPLICATION_PARTICLE_COUNT,
            ENTITY_EVENT_DEFAULT_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    fn entity_taming_particle_batch(
        &mut self,
        state: EntityTamingParticleState,
    ) -> ParticleSpawnBatch {
        let particle_type_id = if state.success {
            HEART_PARTICLE_TYPE_ID
        } else {
            SMOKE_PARTICLE_TYPE_ID
        };
        self.entity_event_aabb_particle_batch(
            particle_type_id,
            state.position,
            state.width,
            state.height,
            ENTITY_TAMING_PARTICLE_COUNT,
            ENTITY_EVENT_DEFAULT_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    fn villager_particle_batch(&mut self, state: VillagerParticleState) -> ParticleSpawnBatch {
        let particle_type_id = match state.kind {
            VillagerParticleKind::Heart => HEART_PARTICLE_TYPE_ID,
            VillagerParticleKind::Angry => ANGRY_VILLAGER_PARTICLE_TYPE_ID,
            VillagerParticleKind::Happy => HAPPY_VILLAGER_PARTICLE_TYPE_ID,
            VillagerParticleKind::Splash => SPLASH_PARTICLE_TYPE_ID,
        };
        self.entity_event_aabb_particle_batch(
            particle_type_id,
            state.position,
            state.width,
            state.height,
            VILLAGER_PARTICLE_COUNT,
            VILLAGER_PARTICLE_Y_OFFSET,
            ENTITY_EVENT_PARTICLE_VELOCITY_SCALE,
        )
    }

    fn dolphin_happy_particle_batch(
        &mut self,
        state: DolphinHappyParticleState,
    ) -> ParticleSpawnBatch {
        self.entity_event_aabb_particle_batch(
            HAPPY_VILLAGER_PARTICLE_TYPE_ID,
            state.position,
            state.width,
            state.height,
            DOLPHIN_HAPPY_PARTICLE_COUNT,
            DOLPHIN_HAPPY_PARTICLE_Y_OFFSET,
            DOLPHIN_HAPPY_PARTICLE_VELOCITY_SCALE,
        )
    }

    fn fox_eat_particle_batch(
        &mut self,
        state: FoxEatParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        let Some(item) = particle_item_option_state_for_stack(&state.item_stack) else {
            return ParticleSpawnBatch::default();
        };
        let template = match self.simple_particle_template(ITEM_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let option_state = ParticleOptionRenderState {
            item: Some(item),
            item_stack: Some(state.item_stack.clone()),
            item_component_patch_empty: state.item_stack.component_patch == Default::default(),
            ..ParticleOptionRenderState::default()
        };
        let look = fox_look_vector(state.x_rot, state.y_rot);
        let position = Vec3d {
            x: state.position.x + look[0] * 0.5,
            y: state.position.y,
            z: state.position.z + look[2] * 0.5,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        for _ in 0..FOX_EAT_PARTICLE_COUNT {
            let local_velocity = [
                f64::from((self.random.next_float() - 0.5) * FOX_EAT_HORIZONTAL_VELOCITY_RANGE),
                f64::from(
                    self.random.next_float() * FOX_EAT_VERTICAL_VELOCITY_RANGE
                        + FOX_EAT_VERTICAL_VELOCITY_BASE,
                ),
                0.0,
            ];
            let direction = fox_rotate_velocity(local_velocity, state.x_rot, state.y_rot);
            let velocity = Vec3d {
                x: direction[0],
                y: direction[1] + FOX_EAT_VERTICAL_VELOCITY_OFFSET,
                z: direction[2],
            };
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                position,
                velocity,
                template.particle_type.override_limiter,
                false,
                0,
                0,
                option_state.clone(),
                None,
                item_runtime,
            ));
        }
        batch
    }

    fn entity_event_aabb_particle_batch(
        &mut self,
        particle_type_id: i32,
        entity_position: bbb_world::EntityVec3,
        entity_width: f32,
        entity_height: f32,
        count: usize,
        y_offset: f64,
        velocity_scale: f64,
    ) -> ParticleSpawnBatch {
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        let width = f64::from(entity_width.max(0.0));
        let height = f64::from(entity_height.max(0.0));
        for _ in 0..count {
            let velocity = Vec3d {
                x: self.random.next_gaussian() * velocity_scale,
                y: self.random.next_gaussian() * velocity_scale,
                z: self.random.next_gaussian() * velocity_scale,
            };
            let position = Vec3d {
                x: entity_position.x + width * (2.0 * self.random.next_f64() - 1.0),
                y: entity_position.y + height * self.random.next_f64() + y_offset,
                z: entity_position.z + width * (2.0 * self.random.next_f64() - 1.0),
            };
            batch
                .commands
                .push(self.command_from_template(&template, position, velocity, false));
        }
        batch
    }

    fn snowball_hit_particle_batch(
        &self,
        state: SnowballHitParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        let (particle_type_id, option_state) = match state
            .item_stack
            .as_ref()
            .and_then(particle_item_option_state_for_stack)
        {
            Some(item) => (
                ITEM_PARTICLE_TYPE_ID,
                ParticleOptionRenderState {
                    item: Some(item),
                    item_stack: state.item_stack.clone(),
                    item_component_patch_empty: state
                        .item_stack
                        .as_ref()
                        .is_some_and(|stack| stack.component_patch == Default::default()),
                    ..ParticleOptionRenderState::default()
                },
            ),
            None => (
                ITEM_SNOWBALL_PARTICLE_TYPE_ID,
                ParticleOptionRenderState::default(),
            ),
        };
        let template = match self.simple_particle_template(particle_type_id) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        for _ in 0..8 {
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                Vec3d {
                    x: state.position.x,
                    y: state.position.y,
                    z: state.position.z,
                },
                Vec3d::default(),
                template.particle_type.override_limiter,
                false,
                0,
                0,
                option_state.clone(),
                None,
                item_runtime,
            ));
        }
        batch
    }

    fn thrown_egg_hit_particle_batch(
        &mut self,
        state: ThrownEggHitParticleState,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnBatch {
        let Some(item) = particle_item_option_state_for_stack(&state.item_stack) else {
            return ParticleSpawnBatch::default();
        };
        let template = match self.simple_particle_template(ITEM_PARTICLE_TYPE_ID) {
            Ok(template) => template,
            Err(batch) => return batch,
        };
        let option_state = ParticleOptionRenderState {
            item: Some(item),
            item_stack: Some(state.item_stack.clone()),
            item_component_patch_empty: state.item_stack.component_patch == Default::default(),
            ..ParticleOptionRenderState::default()
        };
        let mut batch = ParticleSpawnBatch {
            missing_sprite_count: template.missing_sprite_count,
            ..ParticleSpawnBatch::default()
        };
        for _ in 0..8 {
            let velocity = Vec3d {
                x: f64::from((self.random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
                y: f64::from((self.random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
                z: f64::from((self.random.next_float() - 0.5) * THROWN_EGG_HIT_VELOCITY_SCALE),
            };
            batch.commands.push(self.command_for_type(
                template.particle_type,
                &template.sprite_ids,
                Vec3d {
                    x: state.position.x,
                    y: state.position.y,
                    z: state.position.z,
                },
                velocity,
                template.particle_type.override_limiter,
                false,
                0,
                0,
                option_state.clone(),
                None,
                item_runtime,
            ));
        }
        batch
    }

    fn honey_block_particle_batch(&self, state: HoneyBlockParticleState) -> ParticleSpawnBatch {
        let Some(particle_type) = vanilla_particle_type(BLOCK_PARTICLE_TYPE_ID) else {
            return ParticleSpawnBatch {
                unknown_particle_type_count: 1,
                ..ParticleSpawnBatch::default()
            };
        };
        let mut batch = ParticleSpawnBatch::default();
        let position = Vec3d {
            x: state.position.x,
            y: state.position.y,
            z: state.position.z,
        };
        let option_state = ParticleOptionRenderState {
            block: Some(ParticleBlockOptionState {
                block_state_id: state.block_state_id,
            }),
            ..ParticleOptionRenderState::default()
        };
        for _ in 0..state.count {
            batch.commands.push(self.command_for_type(
                particle_type,
                &[],
                position,
                Vec3d::default(),
                particle_type.override_limiter,
                false,
                positive_var_i32_len(state.block_state_id),
                0,
                option_state.clone(),
                None,
                None,
            ));
        }
        batch
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
            None,
            None,
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
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
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
            biome_sampler,
            item_runtime,
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
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) -> ParticleSpawnCommand {
        let child_spawn_templates = self.child_spawn_templates_for_type(particle_type);
        let sprite_ids =
            self.sprite_ids_for_command(particle_type.id, sprite_ids, &option_state, item_runtime);
        let option_color = option_state.color.or_else(|| {
            self.tint_color_for_command(particle_type.id, &option_state, position, biome_sampler)
        });
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
            option_entity_target_source: option_state.vibration_entity_source.map(|source| {
                ParticleEntityTargetSource {
                    entity_id: source.entity_id,
                    y_offset: source.y_offset,
                }
            }),
            option_duration_ticks: option_state.duration_ticks,
            option_roll: option_state.roll,
            option_block: option_state.block,
            option_item: option_state.item,
            option_item_pickup_source_entity_id: None,
            option_item_pickup_age_ticks: None,
            option_item_pickup_light: None,
            option_item_pickup_experience_orb_icon: None,
            option_firework_trail: false,
            option_firework_twinkle: false,
            option_firework_half_lifetime_age: false,
        }
    }

    fn sprite_ids_for_command(
        &self,
        particle_type_id: i32,
        sprite_ids: &[String],
        option_state: &ParticleOptionRenderState,
        item_runtime: Option<&NativeItemRuntime>,
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
            if let (Some(items), Some(stack)) = (item_runtime, option_state.item_stack.as_ref()) {
                if let Some(sprite_ids) = items.item_particle_sprite_ids_for_stack(stack) {
                    if !sprite_ids.is_empty() {
                        return sprite_ids;
                    }
                }
            }
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
        option_state: &ParticleOptionRenderState,
        position: Vec3d,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
    ) -> Option<[f32; 4]> {
        let block_state_id = option_state.block?.block_state_id;
        let block_pos = block_pos_containing(position);
        let biome_id = biome_sampler.and_then(|sampler| sampler.biome_id_at(block_pos));
        match particle_type_id {
            BLOCK_PARTICLE_TYPE_ID
            | DUST_PILLAR_PARTICLE_TYPE_ID
            | BLOCK_CRUMBLE_PARTICLE_TYPE_ID => self
                .terrain_particle_tint_color_for_block_position(
                    block_state_id,
                    block_pos,
                    biome_id,
                ),
            FALLING_DUST_PARTICLE_TYPE_ID => {
                let render_position = BlockRenderPosition {
                    x: block_pos.x,
                    y: block_pos.y,
                    z: block_pos.z,
                };
                let block_tint = self
                    .falling_dust_block_tint_colors
                    .contains_key(&block_state_id)
                    .then(|| {
                        self.terrain_particle_tint_catalog
                            .falling_dust_block_tint_color_for_block_state(
                                block_state_id,
                                biome_id,
                                Some(render_position),
                            )
                    })
                    .flatten()
                    .or_else(|| {
                        self.falling_dust_block_tint_colors
                            .get(&block_state_id)
                            .copied()
                    });
                block_tint.or_else(|| falling_dust_map_color_for_block_state_id(block_state_id))
            }
            _ => None,
        }
    }

    fn terrain_particle_tint_color_for_block_position(
        &self,
        block_state_id: i32,
        block_pos: WorldBlockPos,
        biome_id: Option<i32>,
    ) -> Option<[f32; 4]> {
        if !self
            .terrain_particle_tint_colors
            .contains_key(&block_state_id)
        {
            return None;
        }
        let render_position = BlockRenderPosition {
            x: block_pos.x,
            y: block_pos.y,
            z: block_pos.z,
        };
        self.terrain_particle_tint_catalog
            .terrain_particle_tint_color_for_block_state(
                block_state_id,
                biome_id,
                Some(render_position),
            )
            .or_else(|| {
                self.terrain_particle_tint_colors
                    .get(&block_state_id)
                    .copied()
            })
    }

    fn child_spawn_templates_for_type(
        &self,
        particle_type: ParticleTypeInfo,
    ) -> Vec<ParticleChildSpawnTemplate> {
        let child_particle_type_ids = match particle_type.id {
            EXPLOSION_EMITTER_PARTICLE_TYPE_ID => vec![EXPLOSION_PARTICLE_TYPE_ID],
            LAVA_PARTICLE_TYPE_ID => vec![SMOKE_PARTICLE_TYPE_ID],
            DRIPPING_LAVA_PARTICLE_TYPE_ID => {
                vec![FALLING_LAVA_PARTICLE_TYPE_ID, LANDING_LAVA_PARTICLE_TYPE_ID]
            }
            FALLING_LAVA_PARTICLE_TYPE_ID => vec![LANDING_LAVA_PARTICLE_TYPE_ID],
            DRIPPING_WATER_PARTICLE_TYPE_ID => {
                vec![FALLING_WATER_PARTICLE_TYPE_ID, SPLASH_PARTICLE_TYPE_ID]
            }
            FALLING_WATER_PARTICLE_TYPE_ID => vec![SPLASH_PARTICLE_TYPE_ID],
            DRIPPING_HONEY_PARTICLE_TYPE_ID => {
                vec![
                    FALLING_HONEY_PARTICLE_TYPE_ID,
                    LANDING_HONEY_PARTICLE_TYPE_ID,
                ]
            }
            FALLING_HONEY_PARTICLE_TYPE_ID => vec![LANDING_HONEY_PARTICLE_TYPE_ID],
            DRIPPING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID => {
                vec![
                    FALLING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID,
                    LANDING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID,
                ]
            }
            FALLING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID => {
                vec![LANDING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID]
            }
            DRIPPING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID => {
                vec![
                    FALLING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID,
                    LANDING_LAVA_PARTICLE_TYPE_ID,
                ]
            }
            FALLING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID => vec![LANDING_LAVA_PARTICLE_TYPE_ID],
            DRIPPING_DRIPSTONE_WATER_PARTICLE_TYPE_ID => {
                vec![
                    FALLING_DRIPSTONE_WATER_PARTICLE_TYPE_ID,
                    SPLASH_PARTICLE_TYPE_ID,
                ]
            }
            FALLING_DRIPSTONE_WATER_PARTICLE_TYPE_ID => vec![SPLASH_PARTICLE_TYPE_ID],
            GUST_EMITTER_LARGE_PARTICLE_TYPE_ID | GUST_EMITTER_SMALL_PARTICLE_TYPE_ID => {
                vec![GUST_PARTICLE_TYPE_ID]
            }
            _ => return Vec::new(),
        };
        child_particle_type_ids
            .into_iter()
            .filter_map(|child_particle_type_id| {
                self.simple_particle_template(child_particle_type_id).ok()
            })
            .map(|template| ParticleChildSpawnTemplate {
                particle_type_id: template.particle_type.id,
                particle_id: template.particle_type.name.to_string(),
                sprite_ids: template.sprite_ids,
            })
            .collect()
    }
}

fn block_pos_containing(position: Vec3d) -> WorldBlockPos {
    WorldBlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
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

#[derive(Debug, Clone, Default, PartialEq)]
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
    item_stack: Option<ItemStackSummary>,
    item_component_patch_empty: bool,
    vibration_entity_source: Option<VibrationEntityPositionSource>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VibrationEntityPositionSource {
    pub(crate) entity_id: i32,
    pub(crate) y_offset: f32,
}

fn particle_item_option_state_for_stack(
    stack: &ItemStackSummary,
) -> Option<ParticleItemOptionState> {
    let item_id = stack.item_id?;
    if item_id < 0 || stack.count <= 0 {
        return None;
    }
    Some(ParticleItemOptionState {
        item_id,
        count: stack.count,
        component_patch_len: usize::from(stack.component_patch != Default::default()),
    })
}

fn particle_shader_light(light: TerrainLight) -> [f32; 2] {
    [
        light.block.min(15) as f32 / 15.0,
        light.sky.min(15) as f32 / 15.0,
    ]
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
            let component_patch_len = decoder.remaining_len();
            let component_patch_empty = decoder.remaining() == [0, 0];
            let Ok(component_patch) = decode_data_component_patch_summary(&mut decoder) else {
                return ParticleOptionRenderState::default();
            };
            if !decoder.is_empty() {
                return ParticleOptionRenderState::default();
            }
            ParticleOptionRenderState {
                item: Some(ParticleItemOptionState {
                    item_id,
                    count,
                    component_patch_len,
                }),
                item_stack: Some(ItemStackSummary {
                    item_id: Some(item_id),
                    count,
                    component_patch,
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
            let Ok(source) = decode_vibration_position_source(&mut decoder) else {
                return ParticleOptionRenderState::default();
            };
            let Ok(arrival_ticks) = decoder.read_var_i32() else {
                return ParticleOptionRenderState::default();
            };
            if !decoder.is_empty() {
                return ParticleOptionRenderState::default();
            }
            let (target, vibration_entity_source) = match source {
                VibrationPositionSource::Block(target) => (Some(target), None),
                VibrationPositionSource::Entity(source) => (None, Some(source)),
            };
            ParticleOptionRenderState {
                target,
                vibration_entity_source,
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
    option_state: &ParticleOptionRenderState,
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

fn copper_lantern_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    let name = name.strip_prefix("waxed_").unwrap_or(name);
    let name = name
        .strip_prefix("exposed_")
        .or_else(|| name.strip_prefix("weathered_"))
        .or_else(|| name.strip_prefix("oxidized_"))
        .unwrap_or(name);
    (name == "copper_lantern").then_some(MAP_COLOR_METAL)
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

fn default_none_static_map_color(name: &str) -> Option<u32> {
    let name = name.strip_prefix("minecraft:")?;
    matches!(
        name,
        "air" | "cave_air" | "void_air" | "nether_portal" | "test_instance_block"
    )
    .then_some(MAP_COLOR_NONE)
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
    if let Some(color) = copper_lantern_static_map_color(name) {
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
    if let Some(color) = default_none_static_map_color(name) {
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
        | "minecraft:dirt_path"
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
        | "minecraft:end_stone_brick_wall"
        | "minecraft:bone_block" => Some(MAP_COLOR_SAND),
        "minecraft:sponge" | "minecraft:wet_sponge" => Some(MAP_COLOR_YELLOW),
        "minecraft:snow" | "minecraft:snow_block" => Some(MAP_COLOR_SNOW),
        "minecraft:ice"
        | "minecraft:packed_ice"
        | "minecraft:blue_ice"
        | "minecraft:frosted_ice" => Some(MAP_COLOR_ICE),
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
        "minecraft:redstone_block" => Some(MAP_COLOR_FIRE),
        "minecraft:slime_block" => Some(MAP_COLOR_GRASS),
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
        "minecraft:oak_wood" | "minecraft:stripped_oak_wood" | "minecraft:petrified_oak_slab" => {
            Some(MAP_COLOR_WOOD)
        }
        "minecraft:birch_wood" | "minecraft:stripped_birch_wood" => Some(MAP_COLOR_SAND),
        "minecraft:jungle_wood" | "minecraft:stripped_jungle_wood" => Some(MAP_COLOR_DIRT),
        "minecraft:acacia_wood" => Some(MAP_COLOR_GRAY),
        "minecraft:stripped_acacia_wood" => Some(MAP_COLOR_ORANGE),
        "minecraft:cherry_wood" => Some(MAP_COLOR_TERRACOTTA_GRAY),
        "minecraft:stripped_cherry_wood" => Some(MAP_COLOR_TERRACOTTA_PINK),
        "minecraft:dark_oak_wood" | "minecraft:stripped_dark_oak_wood" => Some(MAP_COLOR_BROWN),
        "minecraft:stripped_pale_oak_wood" => Some(MAP_COLOR_QUARTZ),
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
        "packed_mud" => MAP_COLOR_DIRT,
        "bricks" | "brick_stairs" | "brick_slab" | "brick_wall" => MAP_COLOR_RED,
        "mud_bricks" | "mud_brick_stairs" | "mud_brick_slab" | "mud_brick_wall" => {
            MAP_COLOR_TERRACOTTA_LIGHT_GRAY
        }
        "nether_brick_stairs"
        | "nether_brick_slab"
        | "nether_brick_wall"
        | "nether_brick_fence"
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
        "mycelium" => MAP_COLOR_PURPLE,
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

pub(crate) fn vibration_entity_position_source_from_options(
    particle_type_id: i32,
    raw_options: &[u8],
) -> Option<VibrationEntityPositionSource> {
    if particle_type_id != VIBRATION_PARTICLE_TYPE_ID {
        return None;
    }
    let mut decoder = Decoder::new(raw_options);
    let source = decode_vibration_position_source(&mut decoder).ok()?;
    decoder.read_var_i32().ok()?;
    if !decoder.is_empty() {
        return None;
    }
    match source {
        VibrationPositionSource::Entity(source) => Some(source),
        VibrationPositionSource::Block(_) => None,
    }
}

fn resolve_vibration_entity_target(
    option_state: &mut ParticleOptionRenderState,
    context: LevelParticleSpawnContext,
) {
    if option_state.target.is_some() {
        return;
    }
    let Some(source) = option_state.vibration_entity_source else {
        return;
    };
    let Some(position) = context.vibration_entity_position else {
        return;
    };
    if position.entity_id != source.entity_id {
        return;
    }
    option_state.target = Some([
        position.position[0],
        position.position[1] + f64::from(source.y_offset),
        position.position[2],
    ]);
}

enum VibrationPositionSource {
    Block([f64; 3]),
    Entity(VibrationEntityPositionSource),
}

fn decode_vibration_position_source(
    decoder: &mut Decoder<'_>,
) -> bbb_protocol::codec::Result<VibrationPositionSource> {
    match decoder.read_var_i32()? {
        0 => {
            let packed = decoder.read_i64()?;
            let x = (packed >> 38) as i32;
            let y = ((packed << 52) >> 52) as i32;
            let z = ((packed << 26) >> 38) as i32;
            Ok(VibrationPositionSource::Block([
                f64::from(x) + 0.5,
                f64::from(y) + 0.5,
                f64::from(z) + 0.5,
            ]))
        }
        1 => {
            let entity_id = decoder.read_var_i32()?;
            let y_offset = decoder.read_f32()?;
            Ok(VibrationPositionSource::Entity(
                VibrationEntityPositionSource {
                    entity_id,
                    y_offset,
                },
            ))
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
    batch.sound_events.append(&mut other.sound_events);
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

fn fox_look_vector(x_rot: f32, y_rot: f32) -> [f64; 3] {
    let real_x_rot = x_rot.to_radians();
    let real_y_rot = -y_rot.to_radians();
    let y_cos = real_y_rot.cos();
    let y_sin = real_y_rot.sin();
    let x_cos = real_x_rot.cos();
    let x_sin = real_x_rot.sin();
    [
        f64::from(y_sin * x_cos),
        f64::from(-x_sin),
        f64::from(y_cos * x_cos),
    ]
}

fn fox_rotate_velocity(local: [f64; 3], x_rot: f32, y_rot: f32) -> [f64; 3] {
    let x_rad = -x_rot.to_radians();
    let x_cos = f64::from(x_rad.cos());
    let x_sin = f64::from(x_rad.sin());
    let after_x = [
        local[0],
        local[1] * x_cos + local[2] * x_sin,
        local[2] * x_cos - local[1] * x_sin,
    ];
    let y_rad = -y_rot.to_radians();
    let y_cos = f64::from(y_rad.cos());
    let y_sin = f64::from(y_rad.sin());
    [
        after_x[0] * y_cos + after_x[2] * y_sin,
        after_x[1],
        after_x[2] * y_cos - after_x[0] * y_sin,
    ]
}

fn firework_explosion_colors(explosion: &FireworkExplosionSummary) -> Vec<i32> {
    if explosion.colors.is_empty() {
        vec![FIREWORK_BLACK_COLOR]
    } else {
        explosion.colors.clone()
    }
}

fn firework_twinkle_delay_ticks(explosion_count: usize) -> u32 {
    if explosion_count == 0 {
        0
    } else {
        (explosion_count as u32).saturating_mul(2) - 1 + 15
    }
}

fn random_firework_color(colors: &[i32], random: &mut LegacyRandom) -> i32 {
    let index = random.next_i32(colors.len() as i32) as usize;
    colors[index]
}

fn firework_spark_color(rgb: i32) -> [f32; 4] {
    let [red, green, blue, _] = firework_argb_color(rgb);
    [red, green, blue, 0.99]
}

fn firework_spark_fade_color(rgb: i32) -> [f32; 4] {
    let [red, green, blue, _] = firework_argb_color(rgb);
    [red, green, blue, 1.0]
}

fn firework_flash_color(argb: i32) -> [f32; 4] {
    firework_argb_color(argb)
}

fn firework_argb_color(argb: i32) -> [f32; 4] {
    [
        ((argb >> 16) & 0xff) as f32 / 255.0,
        ((argb >> 8) & 0xff) as f32 / 255.0,
        (argb & 0xff) as f32 / 255.0,
        ((argb >> 24) & 0xff) as f32 / 255.0,
    ]
}

fn lerp_f64(alpha: f64, from: f64, to: f64) -> f64 {
    from + (to - from) * alpha
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
particle_type_ids! {
    // Every id below is derived from `PARTICLE_TYPES_26_1` (index == id); the
    // generated test reasserts each pairing and guards the registry length so a
    // reorder or truncation cannot drift these constants silently.
    expect_registry_len = 117;

    const ANGRY_VILLAGER_PARTICLE_TYPE_ID = "minecraft:angry_villager";
    const BLOCK_PARTICLE_TYPE_ID = "minecraft:block";
    const BLOCK_MARKER_PARTICLE_TYPE_ID = "minecraft:block_marker";
    const BUBBLE_PARTICLE_TYPE_ID = "minecraft:bubble";
    const CLOUD_PARTICLE_TYPE_ID = "minecraft:cloud";
    pub(crate) const CRIT_PARTICLE_TYPE_ID = "minecraft:crit";
    const DRAGON_BREATH_PARTICLE_TYPE_ID = "minecraft:dragon_breath";
    const DRIPPING_LAVA_PARTICLE_TYPE_ID = "minecraft:dripping_lava";
    const FALLING_LAVA_PARTICLE_TYPE_ID = "minecraft:falling_lava";
    const LANDING_LAVA_PARTICLE_TYPE_ID = "minecraft:landing_lava";
    const DRIPPING_WATER_PARTICLE_TYPE_ID = "minecraft:dripping_water";
    const FALLING_WATER_PARTICLE_TYPE_ID = "minecraft:falling_water";
    const DUST_PARTICLE_TYPE_ID = "minecraft:dust";
    const DUST_COLOR_TRANSITION_PARTICLE_TYPE_ID = "minecraft:dust_color_transition";
    const EFFECT_PARTICLE_TYPE_ID = "minecraft:effect";
    pub(crate) const ELDER_GUARDIAN_PARTICLE_TYPE_ID = "minecraft:elder_guardian";
    pub(crate) const ENCHANTED_HIT_PARTICLE_TYPE_ID = "minecraft:enchanted_hit";
    pub(crate) const ENTITY_EFFECT_PARTICLE_TYPE_ID = "minecraft:entity_effect";
    const EXPLOSION_EMITTER_PARTICLE_TYPE_ID = "minecraft:explosion_emitter";
    const EXPLOSION_PARTICLE_TYPE_ID = "minecraft:explosion";
    const GUST_PARTICLE_TYPE_ID = "minecraft:gust";
    const GUST_EMITTER_LARGE_PARTICLE_TYPE_ID = "minecraft:gust_emitter_large";
    const GUST_EMITTER_SMALL_PARTICLE_TYPE_ID = "minecraft:gust_emitter_small";
    const FALLING_DUST_PARTICLE_TYPE_ID = "minecraft:falling_dust";
    const FIREWORK_PARTICLE_TYPE_ID = "minecraft:firework";
    const FLAME_PARTICLE_TYPE_ID = "minecraft:flame";
    const TINTED_LEAVES_PARTICLE_TYPE_ID = "minecraft:tinted_leaves";
    const SCULK_CHARGE_PARTICLE_TYPE_ID = "minecraft:sculk_charge";
    const SCULK_CHARGE_POP_PARTICLE_TYPE_ID = "minecraft:sculk_charge_pop";
    const SOUL_FIRE_FLAME_PARTICLE_TYPE_ID = "minecraft:soul_fire_flame";
    const FLASH_PARTICLE_TYPE_ID = "minecraft:flash";
    const HAPPY_VILLAGER_PARTICLE_TYPE_ID = "minecraft:happy_villager";
    const COMPOSTER_PARTICLE_TYPE_ID = "minecraft:composter";
    const HEART_PARTICLE_TYPE_ID = "minecraft:heart";
    const INSTANT_EFFECT_PARTICLE_TYPE_ID = "minecraft:instant_effect";
    const ITEM_PARTICLE_TYPE_ID = "minecraft:item";
    const VIBRATION_PARTICLE_TYPE_ID = "minecraft:vibration";
    const TRAIL_PARTICLE_TYPE_ID = "minecraft:trail";
    const ITEM_SLIME_PARTICLE_TYPE_ID = "minecraft:item_slime";
    const ITEM_COBWEB_PARTICLE_TYPE_ID = "minecraft:item_cobweb";
    const ITEM_SNOWBALL_PARTICLE_TYPE_ID = "minecraft:item_snowball";
    const LARGE_SMOKE_PARTICLE_TYPE_ID = "minecraft:large_smoke";
    const LAVA_PARTICLE_TYPE_ID = "minecraft:lava";
    const POOF_PARTICLE_TYPE_ID = "minecraft:poof";
    const PORTAL_PARTICLE_TYPE_ID = "minecraft:portal";
    pub(crate) const SMOKE_PARTICLE_TYPE_ID = "minecraft:smoke";
    const WHITE_SMOKE_PARTICLE_TYPE_ID = "minecraft:white_smoke";
    pub(crate) const TOTEM_OF_UNDYING_PARTICLE_TYPE_ID = "minecraft:totem_of_undying";
    const SPLASH_PARTICLE_TYPE_ID = "minecraft:splash";
    const WITCH_PARTICLE_TYPE_ID = "minecraft:witch";
    const DRIPPING_HONEY_PARTICLE_TYPE_ID = "minecraft:dripping_honey";
    const FALLING_HONEY_PARTICLE_TYPE_ID = "minecraft:falling_honey";
    const LANDING_HONEY_PARTICLE_TYPE_ID = "minecraft:landing_honey";
    const DRIPPING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID = "minecraft:dripping_obsidian_tear";
    const FALLING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID = "minecraft:falling_obsidian_tear";
    const LANDING_OBSIDIAN_TEAR_PARTICLE_TYPE_ID = "minecraft:landing_obsidian_tear";
    const SMALL_FLAME_PARTICLE_TYPE_ID = "minecraft:small_flame";
    const DRIPPING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID = "minecraft:dripping_dripstone_lava";
    const FALLING_DRIPSTONE_LAVA_PARTICLE_TYPE_ID = "minecraft:falling_dripstone_lava";
    const DRIPPING_DRIPSTONE_WATER_PARTICLE_TYPE_ID = "minecraft:dripping_dripstone_water";
    const FALLING_DRIPSTONE_WATER_PARTICLE_TYPE_ID = "minecraft:falling_dripstone_water";
    const ELECTRIC_SPARK_PARTICLE_TYPE_ID = "minecraft:electric_spark";
    const WAX_ON_PARTICLE_TYPE_ID = "minecraft:wax_on";
    const WAX_OFF_PARTICLE_TYPE_ID = "minecraft:wax_off";
    const SCRAPE_PARTICLE_TYPE_ID = "minecraft:scrape";
    const SHRIEK_PARTICLE_TYPE_ID = "minecraft:shriek";
    const EGG_CRACK_PARTICLE_TYPE_ID = "minecraft:egg_crack";
    const TRIAL_SPAWNER_DETECTED_PLAYER_PARTICLE_TYPE_ID = "minecraft:trial_spawner_detection";
    const TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS_PARTICLE_TYPE_ID =
        "minecraft:trial_spawner_detection_ominous";
    const VAULT_CONNECTION_PARTICLE_TYPE_ID = "minecraft:vault_connection";
    const OMINOUS_SPAWNING_PARTICLE_TYPE_ID = "minecraft:ominous_spawning";
    const DUST_PILLAR_PARTICLE_TYPE_ID = "minecraft:dust_pillar";
    const TRIAL_OMEN_PARTICLE_TYPE_ID = "minecraft:trial_omen";
    const BLOCK_CRUMBLE_PARTICLE_TYPE_ID = "minecraft:block_crumble";
}
const FIREWORK_BLACK_COLOR: i32 = 1_973_019;
const FIREWORK_CREEPER_PARTICLE_COORDS: &[[f64; 2]] = &[
    [0.0, 0.2],
    [0.2, 0.2],
    [0.2, 0.6],
    [0.6, 0.6],
    [0.6, 0.2],
    [0.2, 0.2],
    [0.2, 0.0],
    [0.4, 0.0],
    [0.4, -0.6],
    [0.2, -0.6],
    [0.2, -0.4],
    [0.0, -0.4],
];
const FIREWORK_STAR_PARTICLE_COORDS: &[[f64; 2]] = &[
    [0.0, 1.0],
    [0.3455, 0.309],
    [0.9511, 0.309],
    [0.3795918367346939, -0.12653061224489795],
    [0.6122448979591837, -0.8040816326530612],
    [0.0, -0.35918367346938773],
];
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
const ARROW_EFFECT_PARTICLE_COUNT: usize = 20;
const ANIMAL_LOVE_PARTICLE_COUNT: usize = 7;
const ALLAY_DUPLICATION_PARTICLE_COUNT: usize = 3;
const ENTITY_TAMING_PARTICLE_COUNT: usize = 7;
const VILLAGER_PARTICLE_COUNT: usize = 5;
const DOLPHIN_HAPPY_PARTICLE_COUNT: usize = 7;
const ENTITY_EVENT_DEFAULT_Y_OFFSET: f64 = 0.5;
const ENTITY_EVENT_PARTICLE_VELOCITY_SCALE: f64 = 0.02;
const VILLAGER_PARTICLE_Y_OFFSET: f64 = 1.0;
const DOLPHIN_HAPPY_PARTICLE_Y_OFFSET: f64 = 0.2;
const DOLPHIN_HAPPY_PARTICLE_VELOCITY_SCALE: f64 = 0.01;
const THROWN_EGG_HIT_VELOCITY_SCALE: f32 = 0.08;
const FOX_EAT_PARTICLE_COUNT: usize = 8;
const FOX_EAT_HORIZONTAL_VELOCITY_RANGE: f32 = 0.1;
const FOX_EAT_VERTICAL_VELOCITY_RANGE: f32 = 0.1;
const FOX_EAT_VERTICAL_VELOCITY_BASE: f32 = 0.1;
const FOX_EAT_VERTICAL_VELOCITY_OFFSET: f64 = 0.05;
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
const MAP_COLOR_GRASS: u32 = 8_368_696;
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

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 / (1_u32 << 24) as f32
    }

    fn next_i64(&mut self) -> i64 {
        let high = self.next_bits(32) as i32 as i64;
        let low = self.next_bits(32) as i32 as i64;
        (high << 32).wrapping_add(low)
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
mod tests;
