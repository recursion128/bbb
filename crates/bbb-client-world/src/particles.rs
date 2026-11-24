//! Client particle simulation (Vanilla `ParticleEngine`): typed spawns, the
//! per-kind provider table and the particle instances ticked at 20 TPS.

use std::{collections::VecDeque, sync::Arc};

use bbb_protocol::packets::SoundSource;

use crate::{ParticleBlockFluidSurfaceSample, TakeItemEntityPickupParticleState, WorldStore};

mod descriptors;
mod instance;
mod kinds;

pub use descriptors::ParticleFacingCameraMode;
use descriptors::{
    ParticleChildEmissionDescriptor, ParticleLimitDescriptor, ParticleRandom, Provider,
    DEFAULT_PARTICLE_RANDOM_SEED,
};
pub use instance::{ItemPickupParticle, ParticleAtlasUvSubRect, ParticleInstance};
pub use kinds::{
    ParticleKind, ParticleQuadShape, ParticleRenderGroup, ParticleRenderLayer,
    ParticleTextureAtlasKind,
};

const MAX_PARTICLES_PER_GROUP: usize = 16_384;
/// Sprite slot that names no atlas sprite (an unresolved texture id); keeps the
/// slot count, and so the sprite RNG draws, of the definition it came from.
pub const MISSING_PARTICLE_SPRITE: u32 = u32::MAX;

/// One particle to add: the type, its typed options and where it starts.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleSpawn {
    pub kind: ParticleKind,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    /// Ticks before the particle first ticks or renders (shriek delay,
    /// tracking-emitter steps).
    pub delay_ticks: u32,
    pub sprite: SpawnSprite,
    pub params: ParticleParams,
}

impl ParticleSpawn {
    pub fn new(kind: ParticleKind, position: [f64; 3], velocity: [f64; 3]) -> Self {
        Self {
            kind,
            position,
            velocity,
            delay_ticks: 0,
            sprite: SpawnSprite::Kind,
            params: ParticleParams::None,
        }
    }

    pub fn with_params(mut self, params: ParticleParams) -> Self {
        self.params = params;
        self
    }

    pub fn with_sprite(mut self, sprite: SpawnSprite) -> Self {
        self.sprite = sprite;
        self
    }

    pub fn with_delay(mut self, delay_ticks: u32) -> Self {
        self.delay_ticks = delay_ticks;
        self
    }

    /// Converts a protocol `ParticlePayload` (e.g. from `DATA_EFFECT_PARTICLES`
    /// or `AreaEffectCloud.DATA_PARTICLE`) into a `ParticleSpawn`.
    pub fn from_protocol_payload_with_registry(
        payload: &bbb_protocol::packets::ParticlePayload,
        position: [f64; 3],
        velocity: [f64; 3],
        registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Option<Self> {
        use bbb_protocol::packets::{ItemStackSummary, ParticleOptions};
        let kind =
            ParticleKind::from_protocol_id_with_registry(payload.particle_type_id, registry)?;
        let options = ParticleOptions::decode_with_registry(
            payload.particle_type_id,
            &payload.raw_options,
            registry,
        )
        .unwrap_or(ParticleOptions::Simple);
        let mut spawn = Self::new(kind, position, velocity);
        match options {
            ParticleOptions::Simple | ParticleOptions::Power { .. } => {}
            ParticleOptions::Block { state_id } => {
                spawn.sprite = SpawnSprite::BlockState(state_id);
            }
            ParticleOptions::Dust { color, scale } => {
                spawn.params = ParticleParams::Dust {
                    color: rgb_color_f32(color),
                    scale,
                };
            }
            ParticleOptions::DustColorTransition {
                from_color,
                to_color,
                scale,
            } => {
                spawn.params = ParticleParams::DustTransition {
                    from: rgb_color_f32(from_color),
                    to: rgb_color_f32(to_color),
                    scale,
                };
            }
            ParticleOptions::Spell { color, power } => {
                spawn.params = ParticleParams::Spell {
                    color: rgb_color_f32(color),
                    power,
                };
            }
            ParticleOptions::Color { color } => {
                spawn.params = ParticleParams::Color(argb_color_f32(color));
            }
            ParticleOptions::SculkCharge { roll } => {
                spawn.params = ParticleParams::SculkCharge { roll };
            }
            ParticleOptions::Item(template) => {
                spawn.sprite = SpawnSprite::ItemStack(Box::new(ItemStackSummary {
                    count: template.count,
                    item_id: Some(template.item_id),
                    component_patch: template.component_patch,
                }));
            }
            ParticleOptions::Vibration {
                destination,
                arrival_in_ticks,
            } => {
                spawn.params = ParticleParams::Vibration {
                    target: Some([
                        f64::from(destination.x) + 0.5,
                        f64::from(destination.y) + 0.5,
                        f64::from(destination.z) + 0.5,
                    ]),
                    source: None,
                    arrival_ticks: u32::try_from(arrival_in_ticks).ok().filter(|&t| t > 0),
                };
            }
            ParticleOptions::VibrationEntity {
                entity_id,
                y_offset,
                arrival_in_ticks,
            } => {
                spawn.params = ParticleParams::Vibration {
                    target: None,
                    source: Some(ParticleEntityTargetSource {
                        entity_id,
                        y_offset,
                    }),
                    arrival_ticks: u32::try_from(arrival_in_ticks).ok().filter(|&t| t > 0),
                };
            }
            ParticleOptions::Trail {
                target,
                color,
                duration,
            } => {
                spawn.params = ParticleParams::Trail {
                    target: [target.x, target.y, target.z],
                    color: rgb_color_f32(color),
                    duration_ticks: u32::try_from(duration).ok().filter(|&t| t > 0),
                };
            }
            ParticleOptions::Shriek { delay } => {
                spawn.delay_ticks = delay.max(0) as u32;
            }
        }
        Some(spawn)
    }

    fn render_group(&self) -> ParticleRenderGroup {
        let info = self.kind.info();
        if (info.descriptor.provider == Provider::VibrationSignal && self.params.target().is_none())
            || matches!(
                info.child_emission,
                Some(
                    ParticleChildEmissionDescriptor::HugeExplosionSeed
                        | ParticleChildEmissionDescriptor::GustSeed { .. }
                )
            )
        {
            ParticleRenderGroup::NoRender
        } else {
            info.render_group
        }
    }
}

/// Where a spawn takes its sprites from.
#[derive(Debug, Clone, PartialEq)]
pub enum SpawnSprite {
    /// The kind's particle-atlas sprites (its particle definition).
    Kind,
    /// One terrain-atlas sprite (`TerrainParticle` from a block state).
    Terrain(u32),
    /// Item-atlas sprites of a resolved item stack, one picked at random.
    Items(Arc<[u32]>),
    /// Unresolved block-state id (`BLOCK` / `FALLING_DUST` / `DUST_PILLAR` / `BLOCK_CRUMBLE`)
    /// resolved into `Terrain` by the client atlas before rendering.
    BlockState(i32),
    /// Unresolved item stack (`ITEM`) resolved into `Items` by the client atlas.
    ItemStack(Box<bbb_protocol::packets::ItemStackSummary>),
}

/// Typed option values a spawn carries into its instance.
#[derive(Debug, Clone, PartialEq)]
pub enum ParticleParams {
    None,
    /// Color options, terrain/falling-dust tints and firework flashes.
    Color([f32; 4]),
    Dust {
        color: [f32; 4],
        scale: f32,
    },
    DustTransition {
        from: [f32; 4],
        to: [f32; 4],
        scale: f32,
    },
    Spell {
        color: [f32; 4],
        power: f32,
    },
    SculkCharge {
        roll: f32,
    },
    Vibration {
        target: Option<[f64; 3]>,
        source: Option<ParticleEntityTargetSource>,
        arrival_ticks: Option<u32>,
    },
    Trail {
        target: [f64; 3],
        color: [f32; 4],
        duration_ticks: Option<u32>,
    },
    FireworkSpark {
        color: [f32; 4],
        fade_to: Option<[f32; 3]>,
        trail: bool,
        twinkle: bool,
        half_lifetime: bool,
    },
    ItemPickup(Box<ItemPickupParticle>),
}

impl ParticleParams {
    fn color(&self) -> Option<[f32; 4]> {
        match self {
            Self::Color(color)
            | Self::Dust { color, .. }
            | Self::DustTransition { from: color, .. }
            | Self::Spell { color, .. }
            | Self::Trail { color, .. }
            | Self::FireworkSpark { color, .. } => Some(*color),
            _ => None,
        }
    }

    fn scale(&self) -> Option<f32> {
        match self {
            Self::Dust { scale, .. } | Self::DustTransition { scale, .. } => Some(*scale),
            _ => None,
        }
    }

    fn target(&self) -> Option<[f64; 3]> {
        match self {
            Self::Vibration { target, .. } => *target,
            Self::Trail { target, .. } => Some(*target),
            Self::ItemPickup(pickup) => Some(pickup.target),
            _ => None,
        }
    }

    fn duration_ticks(&self) -> Option<u32> {
        match self {
            Self::Vibration { arrival_ticks, .. } => *arrival_ticks,
            Self::Trail { duration_ticks, .. } => *duration_ticks,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleEntityTargetSource {
    pub entity_id: i32,
    pub y_offset: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleLocalPlayerScopeContext {
    pub eye_position: [f64; 3],
    pub first_person: bool,
    pub scoping: bool,
}

/// One candidate for the per-particle nearest-player selection performed by
/// the PlayerCloud / Sneeze tick (vanilla
/// `level.getNearestPlayer(x, y, z, 2.0, false)`,
/// `PlayerCloudParticle.java:51`). The native pump projects the local player
/// plus every remote player entity, already filtered through vanilla
/// `EntitySelector.NO_SPECTATORS` (`EntityGetter.java:95-98`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticlePlayerMotionContext {
    pub position: [f64; 3],
    pub delta_movement: [f64; 3],
}

/// What a particle tick reads from outside the particle engine.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParticleTickContext<'a> {
    pub scope: Option<ParticleLocalPlayerScopeContext>,
    pub players: &'a [ParticlePlayerMotionContext],
    pub sound_camera_position: Option<[f64; 3]>,
}

/// World queries used by particle movement.
pub trait ParticleLevel {
    fn entity_position(&self, id: i32) -> Option<[f64; 3]>;
    fn clip_movement(
        &self,
        position: [f64; 3],
        movement: [f64; 3],
        half_width: f64,
        height: f64,
    ) -> [f64; 3];
    fn block_fluid_surface(&self, position: [f64; 3]) -> ParticleBlockFluidSurfaceSample;
}

impl ParticleLevel for WorldStore {
    fn entity_position(&self, id: i32) -> Option<[f64; 3]> {
        let pos = self.probe_entity_transform(id)?.position;
        Some([pos.x, pos.y, pos.z])
    }

    fn clip_movement(
        &self,
        position: [f64; 3],
        movement: [f64; 3],
        half_width: f64,
        height: f64,
    ) -> [f64; 3] {
        self.clip_particle_collision_movement(position, movement, half_width, height)
    }

    fn block_fluid_surface(&self, position: [f64; 3]) -> ParticleBlockFluidSurfaceSample {
        self.particle_block_fluid_surface_sample(position)
    }
}

/// A sound a particle plays; `sound` is the `minecraft:sound_event` registry id.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleSound {
    pub sound: i32,
    pub source: SoundSource,
    pub position: [f64; 3],
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
    pub distance_delay: bool,
}

/// A sound released after `delay_ticks`; `far_sound` replaces `sound` when the
/// camera is at least `far_distance_squared` away at release.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScheduledParticleSound {
    pub sound: ParticleSound,
    pub delay_ticks: u32,
    pub far_sound: Option<(i32, f64)>,
}

impl ScheduledParticleSound {
    fn release(mut self, camera_position: Option<[f64; 3]>) -> ParticleSound {
        if let (Some((far_sound, far_distance_squared)), Some(camera)) =
            (self.far_sound, camera_position)
        {
            let dx = self.sound.position[0] - camera[0];
            let dy = self.sound.position[1] - camera[1];
            let dz = self.sound.position[2] - camera[2];
            if dx * dx + dy * dy + dz * dz >= far_distance_squared {
                self.sound.sound = far_sound;
            }
        }
        self.sound
    }
}

/// Atlas sprite indices resolved once per resource load.
#[derive(Debug, Clone, Default)]
pub struct ParticleSpriteTable {
    /// Particle-atlas sprites per kind, in particle-definition order.
    kinds: Vec<Box<[u32]>>,
    /// Item-atlas sprites of the fixed-item kinds, indexed like
    /// [`ParticleKind::fixed_item_sprite_slot`].
    fixed_items: [Option<u32>; 3],
}

impl ParticleSpriteTable {
    pub fn new(
        mut kind_sprites: impl FnMut(ParticleKind) -> Box<[u32]>,
        fixed_items: [Option<u32>; 3],
    ) -> Self {
        Self {
            kinds: ParticleKind::all().map(&mut kind_sprites).collect(),
            fixed_items,
        }
    }

    fn kind(&self, kind: ParticleKind) -> &[u32] {
        self.kinds.get(kind.index()).map_or(&[], |sprites| sprites)
    }
}

/// The particle engine: queued spawns, live instances and the sounds they emit.
#[derive(Debug, Clone)]
pub struct ParticleEngine {
    pending: PendingParticles,
    active: [VecDeque<ParticleInstance>; ParticleRenderGroup::ALL.len()],
    sounds: VecDeque<ParticleSound>,
    scheduled_sounds: Vec<ScheduledParticleSound>,
    sprites: ParticleSpriteTable,
    random: ParticleRandom,
    sound_random: ParticleRandom,
}

impl Default for ParticleEngine {
    fn default() -> Self {
        Self::with_seed(DEFAULT_PARTICLE_RANDOM_SEED)
    }
}

impl ParticleEngine {
    fn with_seed(seed: i64) -> Self {
        Self {
            pending: PendingParticles::default(),
            active: Default::default(),
            sounds: VecDeque::new(),
            scheduled_sounds: Vec::new(),
            sprites: ParticleSpriteTable::default(),
            random: ParticleRandom::new(seed),
            sound_random: ParticleRandom::new(seed.wrapping_add(1)),
        }
    }

    /// A new sprite table also drops live and queued particles, whose sprite
    /// indices point into the old one (`Minecraft.java:608`:
    /// `particleResources.onReload(particleEngine::clearParticles)`).
    pub fn set_sprites(&mut self, sprites: ParticleSpriteTable) {
        self.sprites = sprites;
        self.pending = PendingParticles::default();
        for group in &mut self.active {
            group.clear();
        }
    }

    /// Drops all live particles, queued spawns, emitters, and pending/delayed
    /// particle sounds on level replacement or disconnect (`ParticleEngine.clearParticles`).
    pub fn clear(&mut self) {
        self.pending = PendingParticles::default();
        for group in &mut self.active {
            group.clear();
        }
        self.sounds.clear();
        self.scheduled_sounds.clear();
    }

    pub fn spawn(&mut self, spawn: ParticleSpawn) {
        self.pending.push(spawn);
    }

    /// Vanilla `ItemPickupParticle`, created directly by
    /// `ClientPacketListener.handleTakeItemEntity`.
    pub fn spawn_item_pickup(&mut self, state: &TakeItemEntityPickupParticleState) {
        let pickup = ItemPickupParticle::from_state(state);
        let spawn = ParticleSpawn::new(
            ParticleKind::ITEM_PICKUP,
            [
                state.item_position.x,
                state.item_position.y,
                state.item_position.z,
            ],
            [
                state.item_delta_movement.x,
                state.item_delta_movement.y,
                state.item_delta_movement.z,
            ],
        );
        self.spawn(spawn.with_params(ParticleParams::ItemPickup(Box::new(pickup))));
    }

    pub fn play_sound(&mut self, sound: ParticleSound) {
        self.sounds.push_back(sound);
    }

    pub fn schedule_sound(&mut self, sound: ScheduledParticleSound) {
        self.scheduled_sounds.push(sound);
    }

    /// `ParticleEngine.tick` (`ParticleEngine.java:79-104`), `ticks` times:
    /// live particles tick first, then the spawns queued since the last tick
    /// join them, so a new particle first moves on the following tick.
    pub fn advance(&mut self, ticks: u32, level: &dyn ParticleLevel, context: ParticleTickContext) {
        for _ in 0..ticks {
            self.tick_active(level, context);
            self.update_scheduled_sounds(context.sound_camera_position);
            self.drain_pending(context.scope);
        }
    }

    fn update_scheduled_sounds(&mut self, camera_position: Option<[f64; 3]>) {
        let sounds = &mut self.sounds;
        self.scheduled_sounds.retain_mut(|scheduled| {
            scheduled.delay_ticks = scheduled.delay_ticks.saturating_sub(1);
            if scheduled.delay_ticks != 0 {
                return true;
            }
            sounds.push_back(scheduled.release(camera_position));
            false
        });
    }

    fn tick_active(&mut self, level: &dyn ParticleLevel, context: ParticleTickContext) {
        let Self {
            pending,
            active,
            sounds,
            sprites,
            random,
            sound_random,
            ..
        } = self;
        for group in active {
            group.retain_mut(|instance| {
                if instance.delay_ticks > 0 {
                    instance.delay_ticks -= 1;
                    return true;
                }
                if instance.age_ticks >= instance.lifetime_ticks {
                    instance.spawn_removal_children(false, pending);
                    release_limit(&mut pending.limited_counts, instance.kind);
                    return false;
                }
                instance.tick_motion(random, level, sprites);
                if instance.removed {
                    instance.spawn_removal_children(true, pending);
                    if let Some(sound) = instance.removal_sound(sound_random) {
                        sounds.push_back(sound);
                    }
                    release_limit(&mut pending.limited_counts, instance.kind);
                    return false;
                }
                instance.update_player_cloud_motion(context.players);
                instance.age_ticks = instance.age_ticks.saturating_add(1);
                instance.update_sprite_from_age(sprites);
                instance.update_alpha_from_age();
                instance.update_spell_scope_alpha(context.scope);
                instance.update_color_fade_from_age();
                instance.spawn_children(random, pending);
                true
            });
        }
    }

    fn drain_pending(&mut self, scope: Option<ParticleLocalPlayerScopeContext>) {
        for group in ParticleRenderGroup::ALL {
            let active = &mut self.active[group as usize];
            for spawn in self.pending.groups[group as usize].drain(..) {
                if active.len() == MAX_PARTICLES_PER_GROUP {
                    let evicted = active.pop_front().unwrap();
                    release_limit(&mut self.pending.limited_counts, evicted.kind);
                }
                active.push_back(ParticleInstance::from_spawn(
                    spawn,
                    group,
                    &self.sprites,
                    &mut self.random,
                    scope,
                ));
            }
        }
    }

    pub fn drain_sounds(&mut self) -> impl Iterator<Item = ParticleSound> + '_ {
        self.sounds.drain(..)
    }

    pub fn instances(&self) -> impl Iterator<Item = &ParticleInstance> {
        self.active.iter().flatten()
    }

    pub fn single_quads(&self) -> &VecDeque<ParticleInstance> {
        &self.active[ParticleRenderGroup::SingleQuads as usize]
    }

    pub fn instances_in_group(
        &self,
        group: ParticleRenderGroup,
    ) -> impl Iterator<Item = &ParticleInstance> {
        self.active[group as usize].iter()
    }

    pub fn len(&self) -> usize {
        self.active.iter().map(VecDeque::len).sum()
    }

    /// Spawns queued for the next tick.
    pub fn pending_len(&self) -> usize {
        self.pending.groups.iter().map(VecDeque::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.active.iter().all(VecDeque::is_empty)
    }
}

#[derive(Debug, Clone, Default)]
struct PendingParticles {
    groups: [VecDeque<ParticleSpawn>; ParticleRenderGroup::ALL.len()],
    /// Includes admitted spawns until they die or are evicted, whether queued or live.
    limited_counts: [usize; ParticleLimitDescriptor::COUNT],
}

impl PendingParticles {
    fn push(&mut self, spawn: ParticleSpawn) {
        if let Some(limit) = spawn.kind.info().particle_limit {
            let count = &mut self.limited_counts[limit as usize];
            if *count == limit.limit() {
                return;
            }
            *count += 1;
        }
        let group = &mut self.groups[spawn.render_group() as usize];
        if group.len() == MAX_PARTICLES_PER_GROUP {
            let evicted = group.pop_front().unwrap();
            release_limit(&mut self.limited_counts, evicted.kind);
        }
        group.push_back(spawn);
    }
}

fn release_limit(counts: &mut [usize; ParticleLimitDescriptor::COUNT], kind: ParticleKind) {
    if let Some(limit) = kind.info().particle_limit {
        counts[limit as usize] -= 1;
    }
}

pub(crate) fn rgb_color_f32(color: i32) -> [f32; 4] {
    let color = color as u32;
    [
        ((color >> 16) & 0xff) as f32 / 255.0,
        ((color >> 8) & 0xff) as f32 / 255.0,
        (color & 0xff) as f32 / 255.0,
        1.0,
    ]
}

pub(crate) fn argb_color_f32(color: i32) -> [f32; 4] {
    let color = color as u32;
    [
        ((color >> 16) & 0xff) as f32 / 255.0,
        ((color >> 8) & 0xff) as f32 / 255.0,
        (color & 0xff) as f32 / 255.0,
        ((color >> 24) & 0xff) as f32 / 255.0,
    ]
}

#[cfg(test)]
mod tests;
