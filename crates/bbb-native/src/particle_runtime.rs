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
    ParticleItemOptionState, ParticleItemPickupProjectileKind, ParticleItemPickupProjectileModel,
    ParticleScheduledSoundEvent, ParticleSoundEvent, ParticleSpawnBatch, ParticleSpawnCommand,
    ParticleSpriteUv, ParticleUvRect, Renderer,
};
use bbb_world::{
    block_name_has_invisible_render_shape, block_name_is_air,
    block_name_should_spawn_terrain_particles, AllayDuplicationParticleState,
    AnimalLoveParticleState, ArrowEffectParticleState, BlockPos as WorldBlockPos,
    DolphinHappyParticleState, EntityTamingParticleState, FireworkRocketExplosionParticleState,
    FireworkRocketTrailParticleState, FoxEatParticleState, HoneyBlockParticleState,
    LevelEventSoundRandomState, LivingEntityDrownParticleState, LivingEntityPoofParticleState,
    LivingEntityPortalParticleState, OminousItemSpawnerParticleState, RavagerRoarParticleState,
    SnowballHitParticleState, TakeItemEntityPickupParticleState,
    TakeItemEntityPickupProjectileModel, TerrainLight, ThrownEggHitParticleState,
    VaultConnectionParticleState, VillagerParticleKind, VillagerParticleState,
    WitchMagicParticleState,
};

use crate::{
    particle_registry::{vanilla_particle_type, ParticleTypeInfo},
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

/// Serializes the picked-up stack's `DataComponentPatchSummary` for carriage
/// through the renderer's item-pickup channel. The renderer never inspects the
/// payload (it cannot name protocol summary types), so this reuses the
/// component summary that `ClientboundTakeItemEntity` already decoded — no
/// second wire decode. `None` for empty/default patches, which the native bake
/// reconstructs as `DataComponentPatchSummary::default()`.
fn pickup_item_component_patch_bytes(stack: &ItemStackSummary) -> Option<Vec<u8>> {
    if stack.component_patch == Default::default() {
        return None;
    }
    serde_json::to_vec(&stack.component_patch).ok()
}

/// Projects the world pickup projectile descriptor into the renderer's
/// item-pickup carried-model field (the renderer cannot name world types).
/// The extracted yaw/pitch travel alongside the kind because vanilla
/// `ArrowRenderer.submit` / `ThrownTridentRenderer.submit` orient the model
/// with the picked entity's `yRot`/`xRot`.
fn particle_item_pickup_projectile_model(
    state: &TakeItemEntityPickupParticleState,
) -> Option<ParticleItemPickupProjectileModel> {
    let kind = match state.projectile_model? {
        TakeItemEntityPickupProjectileModel::Arrow { tipped: false } => {
            ParticleItemPickupProjectileKind::Arrow
        }
        TakeItemEntityPickupProjectileModel::Arrow { tipped: true } => {
            ParticleItemPickupProjectileKind::TippedArrow
        }
        TakeItemEntityPickupProjectileModel::SpectralArrow => {
            ParticleItemPickupProjectileKind::SpectralArrow
        }
        TakeItemEntityPickupProjectileModel::Trident { foil } => {
            ParticleItemPickupProjectileKind::Trident { foil }
        }
    };
    Some(ParticleItemPickupProjectileModel {
        kind,
        y_rot: state.item_y_rot,
        x_rot: state.item_x_rot,
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
#[cfg(test)]
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

mod atlas;
mod block_map_color;
mod ids;
mod resolver_block_events;
mod resolver_core;
mod resolver_entity_events;
mod resolver_firework;
mod resolver_trial_vault;

use atlas::*;
use block_map_color::*;
pub(crate) use ids::*;

#[cfg(test)]
mod tests;
