//! Client particle spawn sources (packets, level and entity events, animate
//! tick, weather, local actions) resolved into typed `ParticleEngine` spawns.

use bbb_client_clock::{Instant, UNIX_EPOCH};
use bbb_protocol::environment_particles::{
    ParticleOptionValue, ParticleOptions as EnvironmentParticleOptions,
};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
    time::Duration,
};

use anyhow::{Context, Result};
use bbb_client_assets::{
    AtlasLayout, AtlasPacker, AtlasSprite, ContentRegistryAssetProjection, PackRoots, SpriteImage,
};
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::{ParticleSpriteUv, ParticleUvRect, Renderer};
use bbb_client_world::particles::{
    ParticleEngine, ParticleEntityTargetSource, ParticleKind, ParticleParams, ParticleSound,
    ParticleSpawn, ParticleSpriteTable, ScheduledParticleSound, SpawnSprite,
    MISSING_PARTICLE_SPRITE,
};
use bbb_client_world::{
    block_name_has_invisible_render_shape, block_name_is_air,
    block_name_should_spawn_terrain_particles, vanilla_air_block_state_id,
    AllayDuplicationParticleState, AnimalLoveParticleState, ArrowEffectParticleState,
    BlazeAmbientParticleState, BlockPos as WorldBlockPos, DolphinHappyParticleState,
    EnderAmbientParticleState, EntityClientMovementParticleState,
    EntityClientMovementSimpleParticleKind, EntityClientMovementSimpleParticleState,
    EntityTamingParticleState, ExplosionEventState, EyeOfEnderTrailParticleState,
    FireworkRocketExplosionParticleState, FireworkRocketTrailParticleState, FoxEatParticleState,
    GuardianAmbientParticleState, GuardianAttackParticleState, HoneyBlockParticleState,
    LevelEventSoundRandomState, LivingEntityDrownParticleState, LivingEntityItemParticleState,
    LivingEntityPoofParticleState, LivingEntityPortalParticleState,
    LlamaSpitRecreationParticleState, LocalBlockAttackEffect, LocalInteractionParticleAction,
    LocalLeverAttachFace, OminousItemSpawnerParticleState, PowderSnowFlakeParticleState,
    RabbitSprintParticleState, RavagerRoarParticleState, ShulkerBulletTrailParticleState,
    SlimeLandingParticleKind, SlimeLandingParticleState, SnowballHitParticleState,
    ThrownEggHitParticleState, VaultConnectionParticleState, VillagerParticleKind,
    VillagerParticleState, WitchMagicParticleState, WorldStore,
};
use bbb_protocol::block_state_types::vanilla_falling_dust_static_facts;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::item_types::{VANILLA_ITEM_ENDER_EYE_ID, VANILLA_ITEM_SPLASH_POTION_ID};
use bbb_protocol::packets::{
    BlockPos, ClientParticleStatus, Direction as ProtocolDirection, FireworkExplosionShapeSummary,
    FireworkExplosionSummary, ItemStackSummary, ItemStackTemplateSummary, LevelEvent,
    LevelParticles, ParticleOptions, ParticlePayload, SoundSource, Vec3d,
};
use bbb_protocol::particle_types::VanillaParticleCodecFamily;
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

use bbb_client_render::scene::{
    biome_tint::{build_biome_blend, BiomeBlend},
    terrain_textures::{
        terrain_particle_biome_blend_center, TerrainParticleTintCatalog, TerrainTextureState,
    },
};

const PARTICLE_TEXTURE_ANIMATION_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LocalDestroyHitParticleState {
    pos: WorldBlockPos,
    face: ProtocolDirection,
    block_state_id: i32,
    shape_min: [f64; 3],
    shape_max: [f64; 3],
    biome_blend: Option<BiomeBlend>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LocalPlayerActionParticleState {
    BlockAttack(LocalBlockAttackEffect),
    DestroyHit(Box<LocalDestroyHitParticleState>),
    Interaction(LocalInteractionParticleAction),
}

/// A particle type with its typed options: what every spawn source hands in.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClientParticle {
    pub(crate) kind: ParticleKind,
    pub(crate) options: ParticleOptions,
}

impl ClientParticle {
    pub(crate) const fn simple(kind: ParticleKind) -> Self {
        Self {
            kind,
            options: ParticleOptions::Simple,
        }
    }

    pub(crate) const fn block(kind: ParticleKind, state_id: i32) -> Self {
        Self {
            kind,
            options: ParticleOptions::Block { state_id },
        }
    }

    pub(crate) const fn color(kind: ParticleKind, argb: u32) -> Self {
        Self {
            kind,
            options: ParticleOptions::Color { color: argb as i32 },
        }
    }

    pub(crate) const fn dust(rgb: u32, scale: f32) -> Self {
        Self {
            kind: ParticleKind::DUST,
            options: ParticleOptions::Dust {
                color: rgb as i32,
                scale,
            },
        }
    }

    pub(crate) const fn dust_transition(from_rgb: u32, to_rgb: u32, scale: f32) -> Self {
        Self {
            kind: ParticleKind::DUST_COLOR_TRANSITION,
            options: ParticleOptions::DustColorTransition {
                from_color: from_rgb as i32,
                to_color: to_rgb as i32,
                scale,
            },
        }
    }

    /// `ClientboundLevelParticlesPacket` options: `None` for an unknown type;
    /// undecodable options spawn the type without options. Resolves vanilla or
    /// session `ContentRegistry` particle ids.
    pub(crate) fn from_payload_with_registry(
        payload: &ParticlePayload,
        registry: &ContentRegistry,
    ) -> Option<Self> {
        let kind =
            ParticleKind::from_protocol_id_with_registry(payload.particle_type_id, registry)?;
        let options = ParticleOptions::decode_with_registry(
            payload.particle_type_id,
            &payload.raw_options,
            registry,
        )
        .unwrap_or(ParticleOptions::Simple);
        Some(Self { kind, options })
    }

    /// Data-driven options (biome ambient particles, dripstone attributes):
    /// `None` for an unknown type or options of another option class.
    pub(crate) fn from_environment_with_registry(
        particle: &EnvironmentParticleOptions,
        registry: &ContentRegistry,
    ) -> Option<Self> {
        let kind = ParticleKind::from_resource_id_with_registry(&particle.resource_id, registry)?;
        let options = match (&particle.option, kind.codec_family()?) {
            (ParticleOptionValue::Empty, VanillaParticleCodecFamily::Simple) => {
                ParticleOptions::Simple
            }
            (ParticleOptionValue::Block { block_state_id }, VanillaParticleCodecFamily::Block) => {
                ParticleOptions::Block {
                    state_id: *block_state_id,
                }
            }
            (ParticleOptionValue::Power { power_bits }, VanillaParticleCodecFamily::Power) => {
                ParticleOptions::Power {
                    power: f32::from_bits(*power_bits),
                }
            }
            (ParticleOptionValue::Dust { color, scale_bits }, VanillaParticleCodecFamily::Dust) => {
                ParticleOptions::Dust {
                    color: *color as i32,
                    scale: f32::from_bits(*scale_bits),
                }
            }
            (
                ParticleOptionValue::DustColorTransition {
                    from_color,
                    to_color,
                    scale_bits,
                },
                VanillaParticleCodecFamily::DustColorTransition,
            ) => ParticleOptions::DustColorTransition {
                from_color: *from_color as i32,
                to_color: *to_color as i32,
                scale: f32::from_bits(*scale_bits),
            },
            (
                ParticleOptionValue::Spell { color, power_bits },
                VanillaParticleCodecFamily::Spell,
            ) => ParticleOptions::Spell {
                color: *color as i32,
                power: f32::from_bits(*power_bits),
            },
            (ParticleOptionValue::Color { argb }, VanillaParticleCodecFamily::Color) => {
                ParticleOptions::Color {
                    color: *argb as i32,
                }
            }
            (
                ParticleOptionValue::SculkCharge { roll_bits },
                VanillaParticleCodecFamily::SculkCharge,
            ) => ParticleOptions::SculkCharge {
                roll: f32::from_bits(*roll_bits),
            },
            (ParticleOptionValue::Item { stack }, VanillaParticleCodecFamily::Item) => {
                let item_id = stack.item_id.filter(|&item_id| item_id >= 0)?;
                if stack.count <= 0 {
                    return None;
                }
                ParticleOptions::Item(Box::new(ItemStackTemplateSummary {
                    item_id,
                    count: stack.count,
                    component_patch: stack.component_patch.clone(),
                }))
            }
            (
                ParticleOptionValue::Vibration {
                    block_pos,
                    arrival_in_ticks,
                },
                VanillaParticleCodecFamily::Vibration,
            ) => ParticleOptions::Vibration {
                destination: BlockPos {
                    x: block_pos[0],
                    y: block_pos[1],
                    z: block_pos[2],
                },
                arrival_in_ticks: *arrival_in_ticks,
            },
            (
                ParticleOptionValue::Trail {
                    target_bits,
                    color,
                    duration,
                },
                VanillaParticleCodecFamily::Trail,
            ) => ParticleOptions::Trail {
                target: Vec3d {
                    x: f64::from_bits(target_bits[0]),
                    y: f64::from_bits(target_bits[1]),
                    z: f64::from_bits(target_bits[2]),
                },
                color: *color as i32,
                duration: *duration,
            },
            (ParticleOptionValue::Shriek { delay }, VanillaParticleCodecFamily::Shriek) => {
                ParticleOptions::Shriek { delay: *delay }
            }
            _ => return None,
        };
        Some(Self { kind, options })
    }

    /// The entity an entity-sourced vibration flies to.
    pub(crate) fn vibration_entity_id(&self) -> Option<i32> {
        match self.options {
            ParticleOptions::VibrationEntity { entity_id, .. } => Some(entity_id),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EnvironmentParticleSpawn {
    pub(crate) particle: ClientParticle,
    pub(crate) position: [f64; 3],
    pub(crate) velocity: [f64; 3],
    pub(crate) always_show: bool,
    /// Client-only late tint source; never part of packet/NBT particle options.
    pub(crate) color_source: Option<EnvironmentParticleColorSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EnvironmentParticleColorSource {
    ClientLeaf {
        block_pos: WorldBlockPos,
        block_resource_id: &'static str,
    },
}

pub(crate) trait ParticleBiomeSampler {
    fn biome_id_at(&self, pos: WorldBlockPos) -> Option<i32>;
}

/// The resolver plus the engine its spawns go to.
pub(crate) struct ParticleTarget<'a> {
    pub(crate) runtime: &'a mut NativeParticleRuntime,
    pub(crate) engine: &'a mut ParticleEngine,
}

/// How `ClientLevel.addParticle` callers spread one packet's particles
/// (`ClientPacketListener.handleParticleEvent`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LevelParticleEmission {
    pub(crate) override_limiter: bool,
    pub(crate) always_show: bool,
    pub(crate) position: Vec3d,
    pub(crate) offset: Vec3d,
    pub(crate) max_speed: f32,
    pub(crate) count: i32,
}

impl LevelParticleEmission {
    pub(crate) fn from_packet(packet: &LevelParticles) -> Self {
        Self {
            override_limiter: packet.override_limiter,
            always_show: packet.always_show,
            position: packet.position,
            offset: packet.offset,
            max_speed: packet.max_speed,
            count: packet.count,
        }
    }

    /// One particle at `position` moving at `velocity`.
    pub(crate) fn single(position: Vec3d, velocity: Vec3d) -> Self {
        Self {
            override_limiter: false,
            always_show: false,
            position,
            offset: velocity,
            max_speed: 1.0,
            count: 0,
        }
    }
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

/// Where an entity-sourced vibration in `particle` currently points.
pub(crate) fn vibration_entity_position(
    world: &WorldStore,
    particle: &ClientParticle,
) -> Option<LevelParticleEntityPosition> {
    let entity_id = particle.vibration_entity_id()?;
    let transform = world.probe_entity_transform(entity_id)?;
    Some(LevelParticleEntityPosition {
        entity_id,
        position: [
            transform.position.x,
            transform.position.y,
            transform.position.z,
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LevelEventParticleContext {
    pub(crate) sculk_charge_pop_full_block: Option<bool>,
    pub(crate) block_state_id_at_event_pos: Option<i32>,
    pub(crate) biome_id_at_event_pos: Option<i32>,
    pub(crate) vault_block_entity_at_event_pos: bool,
    pub(crate) vault_connection_particles: Option<VaultConnectionParticleState>,
    pub(crate) dripstone_drip_particle: Option<ClientParticle>,
    pub(crate) growth_particles: Option<LevelEventGrowthParticleContext>,
    pub(crate) in_block_particle_spread_height: Option<f64>,
    pub(crate) composter_fill_center_shape_max_y: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TrackingEmitterParticleState {
    pub(crate) kind: ParticleKind,
    pub(crate) position: [f64; 3],
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) lifetime_ticks: u32,
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

    pub(crate) fn insert(&mut self, dx: i32, dz: i32) {
        if let Some(bit) = growth_particle_support_bit(dx, dz) {
            self.bits |= bit;
        }
    }

    fn contains(self, dx: i32, dz: i32) -> bool {
        growth_particle_support_bit(dx, dz).is_some_and(|bit| self.bits & bit != 0)
    }
}

pub(crate) struct NativeParticleRenderRuntime {
    atlas: NativeParticleAtlas,
    texture_animation_tick: u64,
    last_texture_animation_at: Option<Instant>,
}

#[derive(Debug, Clone)]
struct CustomParticleSpriteEntry {
    sprites: Box<[u32]>,
    fallback_tint: Option<[f32; 4]>,
}

/// Resolves spawn sources into engine spawns, with the sprite and tint
/// tables resolved once per resource load.
pub(crate) struct NativeParticleRuntime {
    /// Per kind: a particle definition exists or the kind needs none.
    defined: Box<[bool]>,
    /// Particle-atlas sprites per kind, in definition texture order.
    kind_sprites: Vec<Box<[u32]>>,
    /// Particle-atlas sprite index lookup by texture resource id for session custom particles.
    particle_sprite_indices: HashMap<String, u32>,
    fallback_custom_particle_sprites: Box<[u32]>,
    content_registry: Arc<ContentRegistry>,
    custom_particles: HashMap<ParticleKind, CustomParticleSpriteEntry>,
    fixed_item_sprites: [Option<u32>; 3],
    /// Terrain-atlas sprite per vanilla block state id.
    terrain_sprites: Vec<Option<u32>>,
    /// Terrain-atlas sprite per custom `ContentRegistry` block state id.
    custom_terrain_sprites: HashMap<i32, u32>,
    terrain_particle_tint_colors: HashMap<i32, [f32; 4]>,
    falling_dust_block_tint_colors: HashMap<i32, [f32; 4]>,
    terrain_particle_tint_catalog: TerrainParticleTintCatalog,
    /// Empty-component item sprites per item protocol id.
    default_item_sprites: Vec<Arc<[u32]>>,
    random: LegacyRandom,
    particle_level_random: LegacyRandom,
    particle_status: ClientParticleStatus,
    biome_blend_radius: u32,
    explosion_tracker: ClientExplosionTracker,
}

pub(crate) struct NativeParticleResources {
    pub(crate) simulation: NativeParticleRuntime,
    pub(crate) render: NativeParticleRenderRuntime,
}

impl NativeParticleResources {
    pub(crate) fn load(roots: &PackRoots, particle_status: ClientParticleStatus) -> Result<Self> {
        let definitions = roots
            .load_particle_definition_catalog()
            .context("load particle definition catalog")?;
        let sprites = roots
            .load_particle_sprite_catalog()
            .context("load particle sprite catalog")?;
        let (atlas, sprite_indices) =
            particle_atlas_from_images(sprites.sprites().values().cloned().collect())
                .context("stitch particle atlas")?;
        let definition = |kind: ParticleKind| {
            kind.protocol_id()
                .and_then(bbb_protocol::particle_types::vanilla_particle_type)
                .and_then(|entry| definitions.definition(entry.resource_id))
        };
        let defined = ParticleKind::all()
            .map(|kind| definition(kind).is_some() || definitionless_particle_kind(kind))
            .collect();
        let kind_sprites: Vec<Box<[u32]>> = ParticleKind::all()
            .map(|kind| {
                definition(kind).map_or_else(Box::default, |definition| {
                    definition
                        .textures
                        .iter()
                        .map(|texture| {
                            sprite_indices
                                .get(texture.as_str())
                                .copied()
                                .unwrap_or(MISSING_PARTICLE_SPRITE)
                        })
                        .collect()
                })
            })
            .collect();
        let fallback_custom_particle_sprites = kind_sprites
            .get(ParticleKind::END_ROD.index())
            .filter(|list| !list.is_empty())
            .cloned()
            .or_else(|| {
                sprite_indices
                    .get("minecraft:generic_0")
                    .copied()
                    .map(|idx| Box::from([idx]))
            })
            .unwrap_or_default();
        let particle_sprite_indices = sprite_indices;
        Ok(Self {
            simulation: NativeParticleRuntime {
                defined,
                kind_sprites,
                particle_sprite_indices,
                fallback_custom_particle_sprites,
                content_registry: Arc::clone(ContentRegistry::vanilla()),
                custom_particles: HashMap::new(),
                fixed_item_sprites: [None; 3],
                terrain_sprites: Vec::new(),
                custom_terrain_sprites: HashMap::new(),
                terrain_particle_tint_colors: HashMap::new(),
                falling_dust_block_tint_colors: HashMap::new(),
                terrain_particle_tint_catalog: TerrainParticleTintCatalog::default(),
                default_item_sprites: Vec::new(),
                random: LegacyRandom::new(default_particle_seed()),
                particle_level_random: LegacyRandom::new(default_particle_seed()),
                particle_status,
                biome_blend_radius: 2,
                explosion_tracker: ClientExplosionTracker::default(),
            },
            render: NativeParticleRenderRuntime {
                atlas,
                texture_animation_tick: 0,
                last_texture_animation_at: None,
            },
        })
    }
}

impl NativeParticleRenderRuntime {
    pub(crate) fn upload_particle_atlas(&self, renderer: &mut Renderer) -> Result<()> {
        renderer.upload_particle_atlas(
            self.atlas.width,
            self.atlas.height,
            &self.atlas.rgba,
            self.atlas.sprite_uvs.clone(),
        )
    }

    pub(crate) fn maybe_upload_particle_atlas_animation(&mut self, renderer: &mut Renderer) {
        if !self.atlas.has_animation() {
            return;
        }
        let Some(tick) = advance_particle_texture_animation_tick(self, bbb_client_clock::now())
        else {
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

#[cfg(test)]
impl NativeParticleRuntime {
    /// Every particle type defined, without sprites or tints.
    pub(crate) fn for_tests() -> Self {
        Self {
            defined: ParticleKind::all().map(|_| true).collect(),
            kind_sprites: ParticleKind::all().map(|_| Box::default()).collect(),
            particle_sprite_indices: HashMap::new(),
            fallback_custom_particle_sprites: Box::default(),
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            custom_particles: HashMap::new(),
            fixed_item_sprites: [None; 3],
            terrain_sprites: Vec::new(),
            custom_terrain_sprites: HashMap::new(),
            terrain_particle_tint_colors: HashMap::new(),
            falling_dust_block_tint_colors: HashMap::new(),
            terrain_particle_tint_catalog: TerrainParticleTintCatalog::default(),
            default_item_sprites: Vec::new(),
            random: LegacyRandom::new(0),
            particle_level_random: LegacyRandom::new(0),
            particle_status: ClientParticleStatus::All,
            biome_blend_radius: 2,
            explosion_tracker: ClientExplosionTracker::default(),
        }
    }
}

impl NativeParticleRuntime {
    /// Synchronizes session `ContentRegistry` custom particles and their
    /// pack-projected sprites/tints into this runtime.
    pub(crate) fn sync_content_registry(
        &mut self,
        content_registry: Arc<ContentRegistry>,
        projection: Option<&ContentRegistryAssetProjection>,
    ) {
        self.content_registry = content_registry;
        self.custom_particles.clear();
        if self.content_registry.is_vanilla_only() {
            return;
        }
        for spec in self.content_registry.particles() {
            let Some(kind) =
                ParticleKind::from_protocol_id_with_registry(spec.id, &self.content_registry)
            else {
                continue;
            };
            let projected = projection.and_then(|proj| proj.particles.get(&spec.resource_id));
            let resolved_sprites: Vec<u32> = projected
                .map(|proj| {
                    proj.textures
                        .iter()
                        .filter_map(|texture| {
                            self.particle_sprite_indices.get(texture.as_str()).copied()
                        })
                        .collect()
                })
                .unwrap_or_default();
            let (sprites, fallback_tint) = if !resolved_sprites.is_empty() {
                let tint = projected.and_then(|proj| {
                    (!proj.resolved_from_pack).then(|| {
                        let rgb = proj.fallback_tint.rgb;
                        [rgb[0], rgb[1], rgb[2], 1.0]
                    })
                });
                (resolved_sprites.into_boxed_slice(), tint)
            } else {
                let tint = projected.map(|proj| {
                    let rgb = proj.fallback_tint.rgb;
                    [rgb[0], rgb[1], rgb[2], 1.0]
                });
                (self.fallback_custom_particle_sprites.clone(), tint)
            };
            self.custom_particles.insert(
                kind,
                CustomParticleSpriteEntry {
                    sprites,
                    fallback_tint,
                },
            );
        }
    }

    pub(crate) fn custom_particle_sprite(
        &self,
        kind: ParticleKind,
        age_ticks: u32,
        lifetime_ticks: u32,
    ) -> Option<u32> {
        let entry = self.custom_particles.get(&kind)?;
        if entry.sprites.is_empty() {
            return None;
        }
        let lifetime = u64::from(lifetime_ticks.max(1));
        let index = ((u64::from(age_ticks) * entry.sprites.len() as u64) / lifetime)
            .min((entry.sprites.len() - 1) as u64) as usize;
        Some(entry.sprites[index])
    }

    /// Re-resolves terrain sprites and tints after the terrain atlas changed.
    pub(crate) fn set_terrain_textures(&mut self, textures: &TerrainTextureState) {
        let block_states = bbb_client_world::BlockStateRegistry::vanilla_26_1();
        let state_count = block_states
            .iter()
            .map(|state| usize::try_from(state.id).map_or(0, |id| id + 1))
            .max()
            .unwrap_or(0);
        self.terrain_sprites = vec![None; state_count];
        for state in block_states.iter() {
            if let Ok(id) = usize::try_from(state.id) {
                self.terrain_sprites[id] = textures.particle_sprite_index_for_block_state(state.id);
            }
        }
        self.custom_terrain_sprites.clear();
        let content_registry = textures.content_registry();
        if !content_registry.is_vanilla_only() {
            for spec in content_registry.blocks() {
                for offset in 0..i32::from(spec.state_count) {
                    let Some(state_id) = spec.first_state_id.checked_add(offset) else {
                        break;
                    };
                    if let Some(sprite) = textures
                        .particle_sprite_index_for_block_state_with_registry(
                            state_id,
                            content_registry,
                        )
                    {
                        self.custom_terrain_sprites.insert(state_id, sprite);
                    }
                }
            }
        }
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
        self.set_terrain_biomes(textures);
        self.biome_blend_radius = textures.biome_blend_radius();
    }

    pub(crate) fn set_terrain_biomes(&mut self, textures: &TerrainTextureState) {
        self.terrain_particle_tint_catalog = textures.particle_tint_catalog();
    }

    /// Re-resolves item sprites after the item atlas changed.
    pub(crate) fn set_item_runtime(&mut self, items: &NativeItemRuntime) {
        self.default_item_sprites = items.default_item_particle_sprites();
        self.fixed_item_sprites =
            ParticleKind::FIXED_ITEM_SPRITE_IDS.map(|texture| items.atlas_sprite_index(texture));
    }

    /// The atlas sprites the engine draws from.
    pub(crate) fn sprite_table(&self) -> ParticleSpriteTable {
        ParticleSpriteTable::new(
            |kind| {
                self.kind_sprites
                    .get(kind.index())
                    .cloned()
                    .unwrap_or_default()
            },
            self.fixed_item_sprites,
        )
    }

    fn defined(&self, kind: ParticleKind) -> Option<ParticleKind> {
        if kind.is_custom() {
            return self.custom_particles.contains_key(&kind).then_some(kind);
        }
        self.defined
            .get(kind.index())
            .copied()
            .unwrap_or(false)
            .then_some(kind)
    }

    pub(crate) fn tick_explosion_particles(
        &mut self,
        out: &mut ParticleEngine,
        world: &WorldStore,
        random: &mut LevelEventSoundRandomState,
        camera_position: Option<[f64; 3]>,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        let spawns = self.explosion_tracker.tick(
            world,
            world.client_level_generation(),
            self.particle_status,
            random,
        );
        let biome_sampler = ExplosionWorldParticleSampler { world };
        for spawn in spawns {
            let Some(particle) = spawn.particle else {
                continue;
            };
            let context = LevelParticleSpawnContext {
                camera_position,
                vibration_entity_position: vibration_entity_position(world, &particle),
            };
            self.spawn_level_particles(
                out,
                &particle,
                LevelParticleEmission::single(spawn.position, spawn.velocity),
                context,
                Some(&biome_sampler),
                item_runtime,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spawn_explosion_particles(
        &mut self,
        out: &mut ParticleEngine,
        state: &ExplosionEventState,
        level_generation: u64,
        content_registry: &ContentRegistry,
        context: LevelParticleSpawnContext,
        biome_sampler: Option<&dyn ParticleBiomeSampler>,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        if let Some(particle) =
            ClientParticle::from_payload_with_registry(&state.explosion_particle, content_registry)
        {
            self.spawn_level_particles(
                out,
                &particle,
                LevelParticleEmission::single(
                    state.center,
                    Vec3d {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                ),
                context,
                biome_sampler,
                item_runtime,
            );
        }
        self.explosion_tracker
            .track(state, level_generation, content_registry);
    }
}

/// A spawn with every per-source option resolved; position, velocity and the
/// per-position terrain tint are filled in per particle.
#[derive(Debug, Clone)]
struct PreparedParticle {
    spawn: ParticleSpawn,
    /// Block state whose terrain tint colors the particle at its position.
    tint_block: Option<i32>,
    /// The provider accepts the options (`ParticleProvider.createParticle`
    /// returning non-null).
    accepts: bool,
}

impl PreparedParticle {
    fn with_color(mut self, color: Option<[f32; 4]>) -> Self {
        if let Some(color) = color {
            self.spawn.params = ParticleParams::Color(color);
            self.tint_block = None;
        }
        self
    }
}

fn particle_spawn(kind: ParticleKind, position: Vec3d, velocity: Vec3d) -> ParticleSpawn {
    ParticleSpawn::new(
        kind,
        [position.x, position.y, position.z],
        [velocity.x, velocity.y, velocity.z],
    )
}

fn definitionless_particle_kind(kind: ParticleKind) -> bool {
    matches!(
        kind,
        ParticleKind::BLOCK
            | ParticleKind::BLOCK_MARKER
            | ParticleKind::ITEM
            | ParticleKind::ITEM_SLIME
            | ParticleKind::ITEM_COBWEB
            | ParticleKind::ITEM_SNOWBALL
            | ParticleKind::ELDER_GUARDIAN
            | ParticleKind::EXPLOSION_EMITTER
            | ParticleKind::GUST_EMITTER_LARGE
            | ParticleKind::GUST_EMITTER_SMALL
            | ParticleKind::DUST_PILLAR
            | ParticleKind::BLOCK_CRUMBLE
    )
}

fn block_pos_containing(position: Vec3d) -> WorldBlockPos {
    WorldBlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

fn particle_biome_blend_at(
    block_pos: WorldBlockPos,
    biome_blend_radius: u32,
    biome_sampler: Option<&dyn ParticleBiomeSampler>,
) -> Option<BiomeBlend> {
    let sampler = biome_sampler?;
    Some(build_biome_blend(block_pos, biome_blend_radius, |sample| {
        sampler.biome_id_at(sample)
    }))
}

/// A positive duration option; zero and negative values keep the default.
fn positive_ticks(ticks: i32) -> Option<u32> {
    u32::try_from(ticks).ok().filter(|ticks| *ticks > 0)
}

fn particle_provider_accepts_block(kind: ParticleKind, block_state_id: i32) -> bool {
    match kind {
        ParticleKind::FALLING_DUST => vanilla_falling_dust_static_facts(block_state_id)
            .is_none_or(|facts| facts.provider_accepts()),
        ParticleKind::BLOCK | ParticleKind::DUST_PILLAR | ParticleKind::BLOCK_CRUMBLE => {
            terrain_particle_provider_accepts_block_state(block_state_id)
        }
        _ => true,
    }
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

fn argb_particle_color(color: i32) -> [f32; 4] {
    let color = color as u32;
    [
        ((color >> 16) & 0xff) as f32 / 255.0,
        ((color >> 8) & 0xff) as f32 / 255.0,
        (color & 0xff) as f32 / 255.0,
        ((color >> 24) & 0xff) as f32 / 255.0,
    ]
}

/// `minecraft:sound_event` registry id of a built-in sound.
fn sound_event(resource_id: &str) -> i32 {
    vanilla_sound_event_registry_id(resource_id).unwrap_or(-1)
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

fn destroy_block_shape_boxes(block_state_id: i32, pos: BlockPos) -> Vec<([f64; 3], [f64; 3])> {
    bbb_client_world::BlockStateRegistry::vanilla_26_1()
        .by_id(block_state_id)
        .and_then(|state| bbb_client_world::block_outline::block_state_shape_boxes_at(state, pos))
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

fn rotate_entity_local_vector(local: [f64; 3], x_rot: f32, y_rot: f32) -> [f64; 3] {
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
    let [x, y, z] = pointed_dripstone_particle_position(event.pos);
    Vec3d { x, y, z }
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
pub(crate) const ELECTRIC_SPARK_AXIS_RADIUS: f64 = 0.125;
const ELECTRIC_SPARK_AXIS_MIN: i32 = 10;
const ELECTRIC_SPARK_AXIS_MAX: i32 = 19;
const EGG_CRACK_PARTICLE_MAX: i32 = 6;
const ITEM_BREAK_PARTICLE_COUNT: i32 = 8;
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
const SMASH_ATTACK_CENTER_SPEED_SCALE: f64 = 0.2_f32 as f64;
const SMASH_ATTACK_RING_SPEED_SCALE: f64 = 0.05_f32 as f64;
const GROWTH_PARTICLE_WIDE_SPREAD: f64 = 3.0;
const GROWTH_PARTICLE_WIDE_HEIGHT: f64 = 1.0;
const GROWTH_PARTICLE_WIDE_START_OFFSET: f64 = 0.5 - GROWTH_PARTICLE_WIDE_SPREAD;
const GROWTH_PARTICLE_SUPPORT_RADIUS: i32 = 3;
const GROWTH_PARTICLE_SUPPORT_WIDTH: i32 = GROWTH_PARTICLE_SUPPORT_RADIUS * 2 + 1;
/// Matches `ParticleEngine`'s per-group particle budget (`16_384`), bounding
/// untrusted server packet particle counts before allocating/looping.
const MAX_PACKET_PARTICLE_COUNT: usize = 16_384;

fn smash_attack_particle_loop_count(count: i32, divisor: f32) -> usize {
    if count <= 0 || !divisor.is_finite() || divisor <= 0.0 {
        return 0;
    }
    let max_count = (MAX_PACKET_PARTICLE_COUNT as f64 * f64::from(divisor)).ceil() as i64;
    let clamped = i64::from(count).min(max_count) as f32;
    let limit = clamped / divisor;
    (limit.ceil() as usize).min(MAX_PACKET_PARTICLE_COUNT)
}

fn default_particle_seed() -> i64 {
    bbb_client_clock::system_now()
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
        if bound.isolate_lowest_one() == bound {
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

    fn next_boolean(&mut self) -> bool {
        self.next_bits(1) != 0
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
mod block_provider_acceptance;
mod dripstone;
pub(crate) use dripstone::{pointed_dripstone_drip_context, pointed_dripstone_particle_position};
mod environment;
pub(crate) use environment::particle_utils_axis_sample;
mod explosion;
pub(crate) mod extract;
mod ids;
mod redstone;
pub(crate) use redstone::{
    redstone_ore_particle_positions, redstone_ore_solid_render_faces,
    REDSTONE_EFFECT_DIRECTION_STEPS, REDSTONE_PARTICLE_COLOR, SCULK_PARTICLE_COLOR,
};
mod resolver_block_events;
mod resolver_core;
mod resolver_entity_events;
mod resolver_firework;
mod resolver_player_actions;
mod resolver_trial_vault;

use atlas::*;
use block_provider_acceptance::*;
use explosion::*;
use ids::*;
pub(crate) use resolver_player_actions::{
    candle_wick_offsets, local_destroy_hit_particle_state_with_biome_blend_radius,
};
