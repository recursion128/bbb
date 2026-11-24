use std::{collections::HashMap, sync::LazyLock};

use bbb_facts::particle_types::{self, VANILLA_PARTICLE_TYPES};

use super::descriptors::particle_limit_for_kind;
use super::descriptors::{
    FallingLeavesDescriptor, ParticleAlphaCurve, ParticleChildEmissionDescriptor,
    ParticleDescriptor, ParticleFacingCameraMode, ParticleLightEmissionDescriptor,
    ParticleLimitDescriptor, ParticleTickMotionDescriptor, Provider,
};
use crate::TerrainFluidKind;

/// A particle type: the `minecraft:particle_type` registry id, plus the
/// client-only [`ParticleKind::ITEM_PICKUP`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ParticleKind(u16);

impl ParticleKind {
    pub const ANGRY_VILLAGER: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_ANGRY_VILLAGER_ID as u16);
    pub const BLOCK: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID as u16);
    pub const BLOCK_MARKER: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_BLOCK_MARKER_ID as u16);
    pub const BUBBLE: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_BUBBLE_ID as u16);
    pub const CLOUD: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_CLOUD_ID as u16);
    pub const COPPER_FIRE_FLAME: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_COPPER_FIRE_FLAME_ID as u16);
    pub const CRIT: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_CRIT_ID as u16);
    pub const DAMAGE_INDICATOR: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_DAMAGE_INDICATOR_ID as u16);
    pub const DRAGON_BREATH: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_DRAGON_BREATH_ID as u16);
    pub const DRIPPING_LAVA: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_DRIPPING_LAVA_ID as u16);
    pub const FALLING_LAVA: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_FALLING_LAVA_ID as u16);
    pub const LANDING_LAVA: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_LANDING_LAVA_ID as u16);
    pub const DRIPPING_WATER: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_DRIPPING_WATER_ID as u16);
    pub const FALLING_WATER: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_FALLING_WATER_ID as u16);
    pub const DUST: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_DUST_ID as u16);
    pub const DUST_COLOR_TRANSITION: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_DUST_COLOR_TRANSITION_ID as u16);
    pub const EFFECT: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_EFFECT_ID as u16);
    pub const ELDER_GUARDIAN: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_ELDER_GUARDIAN_ID as u16);
    pub const ENCHANTED_HIT: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_ENCHANTED_HIT_ID as u16);
    pub const ENCHANT: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_ENCHANT_ID as u16);
    pub const END_ROD: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_END_ROD_ID as u16);
    pub const ENTITY_EFFECT: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID as u16);
    pub const EXPLOSION_EMITTER: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_EXPLOSION_EMITTER_ID as u16);
    pub const EXPLOSION: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_EXPLOSION_ID as u16);
    pub const GUST: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_GUST_ID as u16);
    pub const SMALL_GUST: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SMALL_GUST_ID as u16);
    pub const GUST_EMITTER_LARGE: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_GUST_EMITTER_LARGE_ID as u16);
    pub const GUST_EMITTER_SMALL: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_GUST_EMITTER_SMALL_ID as u16);
    pub const SONIC_BOOM: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SONIC_BOOM_ID as u16);
    pub const FALLING_DUST: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_FALLING_DUST_ID as u16);
    pub const FIREWORK: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_FIREWORK_ID as u16);
    pub const FISHING: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_FISHING_ID as u16);
    pub const FLAME: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_FLAME_ID as u16);
    pub const INFESTED: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_INFESTED_ID as u16);
    pub const CHERRY_LEAVES: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_CHERRY_LEAVES_ID as u16);
    pub const PALE_OAK_LEAVES: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_PALE_OAK_LEAVES_ID as u16);
    pub const TINTED_LEAVES: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_TINTED_LEAVES_ID as u16);
    pub const SCULK_SOUL: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SCULK_SOUL_ID as u16);
    pub const SCULK_CHARGE: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_SCULK_CHARGE_ID as u16);
    pub const SCULK_CHARGE_POP: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_SCULK_CHARGE_POP_ID as u16);
    pub const SOUL_FIRE_FLAME: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_SOUL_FIRE_FLAME_ID as u16);
    pub const SOUL: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SOUL_ID as u16);
    pub const FLASH: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_FLASH_ID as u16);
    pub const HAPPY_VILLAGER: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_HAPPY_VILLAGER_ID as u16);
    pub const COMPOSTER: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_COMPOSTER_ID as u16);
    pub const HEART: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_HEART_ID as u16);
    pub const INSTANT_EFFECT: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_INSTANT_EFFECT_ID as u16);
    pub const ITEM: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_ITEM_ID as u16);
    pub const VIBRATION: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_VIBRATION_ID as u16);
    pub const TRAIL: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_TRAIL_ID as u16);
    pub const PAUSE_MOB_GROWTH: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_PAUSE_MOB_GROWTH_ID as u16);
    pub const RESET_MOB_GROWTH: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_RESET_MOB_GROWTH_ID as u16);
    pub const ITEM_SLIME: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_ITEM_SLIME_ID as u16);
    pub const ITEM_COBWEB: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_ITEM_COBWEB_ID as u16);
    pub const ITEM_SNOWBALL: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_ITEM_SNOWBALL_ID as u16);
    pub const LARGE_SMOKE: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_LARGE_SMOKE_ID as u16);
    pub const LAVA: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_LAVA_ID as u16);
    pub const MYCELIUM: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_MYCELIUM_ID as u16);
    pub const NOTE: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_NOTE_ID as u16);
    pub const POOF: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_POOF_ID as u16);
    pub const PORTAL: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_PORTAL_ID as u16);
    pub const RAIN: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_RAIN_ID as u16);
    pub const SMOKE: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SMOKE_ID as u16);
    pub const WHITE_SMOKE: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_WHITE_SMOKE_ID as u16);
    pub const SNEEZE: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SNEEZE_ID as u16);
    pub const SPIT: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SPIT_ID as u16);
    pub const SQUID_INK: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SQUID_INK_ID as u16);
    pub const SWEEP_ATTACK: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_SWEEP_ATTACK_ID as u16);
    pub const TOTEM_OF_UNDYING: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_TOTEM_OF_UNDYING_ID as u16);
    pub const UNDERWATER: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_UNDERWATER_ID as u16);
    pub const SPLASH: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SPLASH_ID as u16);
    pub const WITCH: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_WITCH_ID as u16);
    pub const BUBBLE_POP: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_BUBBLE_POP_ID as u16);
    pub const CURRENT_DOWN: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_CURRENT_DOWN_ID as u16);
    pub const BUBBLE_COLUMN_UP: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_BUBBLE_COLUMN_UP_ID as u16);
    pub const NAUTILUS: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_NAUTILUS_ID as u16);
    pub const DOLPHIN: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_DOLPHIN_ID as u16);
    pub const CAMPFIRE_COSY_SMOKE: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_CAMPFIRE_COSY_SMOKE_ID as u16);
    pub const CAMPFIRE_SIGNAL_SMOKE: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_CAMPFIRE_SIGNAL_SMOKE_ID as u16);
    pub const DRIPPING_HONEY: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_DRIPPING_HONEY_ID as u16);
    pub const FALLING_HONEY: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_FALLING_HONEY_ID as u16);
    pub const LANDING_HONEY: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_LANDING_HONEY_ID as u16);
    pub const FALLING_NECTAR: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_FALLING_NECTAR_ID as u16);
    pub const FALLING_SPORE_BLOSSOM: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_FALLING_SPORE_BLOSSOM_ID as u16);
    pub const ASH: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_ASH_ID as u16);
    pub const CRIMSON_SPORE: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_CRIMSON_SPORE_ID as u16);
    pub const WARPED_SPORE: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_WARPED_SPORE_ID as u16);
    pub const SPORE_BLOSSOM_AIR: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_SPORE_BLOSSOM_AIR_ID as u16);
    pub const DRIPPING_OBSIDIAN_TEAR: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_DRIPPING_OBSIDIAN_TEAR_ID as u16);
    pub const FALLING_OBSIDIAN_TEAR: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_FALLING_OBSIDIAN_TEAR_ID as u16);
    pub const LANDING_OBSIDIAN_TEAR: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_LANDING_OBSIDIAN_TEAR_ID as u16);
    pub const REVERSE_PORTAL: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_REVERSE_PORTAL_ID as u16);
    pub const WHITE_ASH: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_WHITE_ASH_ID as u16);
    pub const SMALL_FLAME: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SMALL_FLAME_ID as u16);
    pub const SNOWFLAKE: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SNOWFLAKE_ID as u16);
    pub const DRIPPING_DRIPSTONE_LAVA: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_DRIPPING_DRIPSTONE_LAVA_ID as u16);
    pub const FALLING_DRIPSTONE_LAVA: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_FALLING_DRIPSTONE_LAVA_ID as u16);
    pub const DRIPPING_DRIPSTONE_WATER: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_DRIPPING_DRIPSTONE_WATER_ID as u16);
    pub const FALLING_DRIPSTONE_WATER: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_FALLING_DRIPSTONE_WATER_ID as u16);
    pub const GLOW_SQUID_INK: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_GLOW_SQUID_INK_ID as u16);
    pub const GLOW: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_GLOW_ID as u16);
    pub const WAX_ON: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_WAX_ON_ID as u16);
    pub const WAX_OFF: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_WAX_OFF_ID as u16);
    pub const ELECTRIC_SPARK: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_ELECTRIC_SPARK_ID as u16);
    pub const SCRAPE: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SCRAPE_ID as u16);
    pub const SHRIEK: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_SHRIEK_ID as u16);
    pub const EGG_CRACK: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_EGG_CRACK_ID as u16);
    pub const DUST_PLUME: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_DUST_PLUME_ID as u16);
    pub const TRIAL_SPAWNER_DETECTED_PLAYER: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_TRIAL_SPAWNER_DETECTED_PLAYER_ID as u16);
    pub const TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_TRIAL_SPAWNER_DETECTED_PLAYER_OMINOUS_ID as u16);
    pub const VAULT_CONNECTION: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_VAULT_CONNECTION_ID as u16);
    pub const DUST_PILLAR: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_DUST_PILLAR_ID as u16);
    pub const OMINOUS_SPAWNING: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_OMINOUS_SPAWNING_ID as u16);
    pub const RAID_OMEN: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_RAID_OMEN_ID as u16);
    pub const TRIAL_OMEN: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_TRIAL_OMEN_ID as u16);
    pub const BLOCK_CRUMBLE: Self =
        Self(particle_types::VANILLA_PARTICLE_TYPE_BLOCK_CRUMBLE_ID as u16);
    pub const FIREFLY: Self = Self(particle_types::VANILLA_PARTICLE_TYPE_FIREFLY_ID as u16);
    /// Vanilla creates `ItemPickupParticle` directly; it has no registered type.
    pub const ITEM_PICKUP: Self = Self(VANILLA_PARTICLE_TYPES.len() as u16);
    const COUNT: usize = VANILLA_PARTICLE_TYPES.len() + 1;

    pub fn from_protocol_id(id: i32) -> Option<Self> {
        let index = u16::try_from(id).ok()?;
        (usize::from(index) < VANILLA_PARTICLE_TYPES.len()).then_some(Self(index))
    }

    /// Resolves a built-in vanilla particle id or a session-registered custom
    /// `ContentRegistry` particle id.
    pub fn from_protocol_id_with_registry(
        id: i32,
        registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Option<Self> {
        if let Some(kind) = Self::from_protocol_id(id) {
            return Some(kind);
        }
        if !registry.is_vanilla_only() && registry.particle_by_id(id).is_some() {
            // Custom ids live above `ITEM_PICKUP` (`VANILLA_PARTICLE_TYPES.len()`) so
            // `ITEM_PICKUP` never collides with any custom protocol id.
            let encoded = u16::try_from(id.checked_add(1)?).ok()?;
            return (usize::from(encoded) > VANILLA_PARTICLE_TYPES.len()).then_some(Self(encoded));
        }
        None
    }

    pub fn from_resource_id(resource_id: &str) -> Option<Self> {
        static BY_RESOURCE_ID: LazyLock<HashMap<&'static str, ParticleKind>> =
            LazyLock::new(|| {
                VANILLA_PARTICLE_TYPES
                    .iter()
                    .map(|entry| (entry.resource_id, ParticleKind(entry.id as u16)))
                    .collect()
            });
        BY_RESOURCE_ID.get(resource_id).copied()
    }

    pub fn from_resource_id_with_registry(
        resource_id: &str,
        registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Option<Self> {
        if let Some(kind) = Self::from_resource_id(resource_id) {
            return Some(kind);
        }
        if !registry.is_vanilla_only() {
            if let Some(spec) = registry.particle_by_resource_id(resource_id) {
                return Self::from_protocol_id_with_registry(spec.id, registry);
            }
        }
        None
    }

    pub fn is_custom(self) -> bool {
        usize::from(self.0) > VANILLA_PARTICLE_TYPES.len()
    }

    pub fn protocol_id(self) -> Option<i32> {
        if self == Self::ITEM_PICKUP {
            None
        } else if self.is_custom() {
            Some(i32::from(self.0) - 1)
        } else {
            Some(i32::from(self.0))
        }
    }

    pub fn index(self) -> usize {
        usize::from(self.0)
    }

    /// `ParticleType.getOverrideLimiter`.
    pub fn override_limiter(self) -> bool {
        VANILLA_PARTICLE_TYPES
            .get(self.index())
            .is_some_and(|entry| entry.override_limiter)
    }

    pub fn override_limiter_with_registry(
        self,
        registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> bool {
        if let Some(entry) = VANILLA_PARTICLE_TYPES.get(self.index()) {
            return entry.override_limiter;
        }
        if self.is_custom() {
            return self
                .protocol_id()
                .and_then(|id| registry.particle_by_id(id))
                .is_some_and(|spec| spec.override_limiter);
        }
        false
    }

    /// The option class `ParticleTypes.STREAM_CODEC` reads for this type.
    pub fn codec_family(self) -> Option<particle_types::VanillaParticleCodecFamily> {
        if self.is_custom() {
            return Some(particle_types::VanillaParticleCodecFamily::Simple);
        }
        VANILLA_PARTICLE_TYPES
            .get(self.index())
            .map(|entry| entry.codec_family)
    }

    pub fn all() -> impl Iterator<Item = Self> {
        (0..Self::COUNT as u16).map(Self)
    }

    pub fn render_layer(self) -> ParticleRenderLayer {
        self.info().render_layer
    }

    pub fn quad_shape(self) -> ParticleQuadShape {
        self.info().quad_shape
    }

    pub fn facing_camera_mode(self) -> ParticleFacingCameraMode {
        self.info().facing_camera_mode
    }

    /// Slot in the fixed-item sprite table: `BreakingItemParticle` providers
    /// that always draw one item texture (`ParticleResources.registerProviders`).
    pub fn fixed_item_sprite_slot(self) -> Option<usize> {
        match self {
            Self::ITEM_SLIME => Some(0),
            Self::ITEM_COBWEB => Some(1),
            Self::ITEM_SNOWBALL => Some(2),
            _ => None,
        }
    }

    /// Item texture ids of the fixed-item slots.
    pub const FIXED_ITEM_SPRITE_IDS: [&'static str; 3] = [
        "minecraft:item/slime_ball",
        "minecraft:block/cobweb",
        "minecraft:item/snowball",
    ];

    pub(crate) fn info(self) -> &'static KindInfo {
        KIND_INFO
            .get(self.index())
            .unwrap_or_else(|| &KIND_INFO[Self::END_ROD.index()])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleRenderGroup {
    SingleQuads,
    ItemPickup,
    ElderGuardians,
    NoRender,
}

impl ParticleRenderGroup {
    pub(super) const ALL: [Self; 4] = [
        Self::SingleQuads,
        Self::ItemPickup,
        Self::ElderGuardians,
        Self::NoRender,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleRenderLayer {
    OpaqueTerrain,
    TranslucentTerrain,
    OpaqueItems,
    TranslucentItems,
    Opaque,
    Translucent,
}

impl ParticleRenderLayer {
    pub fn texture_atlas(self) -> ParticleTextureAtlasKind {
        match self {
            Self::OpaqueTerrain | Self::TranslucentTerrain => ParticleTextureAtlasKind::Terrain,
            Self::OpaqueItems | Self::TranslucentItems => ParticleTextureAtlasKind::Items,
            Self::Opaque | Self::Translucent => ParticleTextureAtlasKind::Particles,
        }
    }
}

pub use bbb_render_types::ParticleTextureAtlasKind;

/// How a single-quad particle is meshed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticleQuadShape {
    Billboard,
    /// `ShriekParticle`: two fixed quads.
    Shriek,
    /// `VibrationSignalParticle`: two quads facing along the flight direction.
    Vibration,
}

/// Everything about a kind that does not change per instance.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KindInfo {
    pub(crate) descriptor: ParticleDescriptor,
    pub(crate) tick_motion: ParticleTickMotionDescriptor,
    pub(crate) collision_size: Option<[f32; 2]>,
    pub(crate) drip_fluid: Option<TerrainFluidKind>,
    pub(crate) required_fluid: Option<TerrainFluidKind>,
    pub(crate) moves_without_collision: bool,
    pub(crate) air_downward_acceleration: f64,
    pub(crate) falling_leaves: Option<FallingLeavesDescriptor>,
    pub(crate) light_emission: ParticleLightEmissionDescriptor,
    pub(crate) facing_camera_mode: ParticleFacingCameraMode,
    pub(crate) alpha_curve: ParticleAlphaCurve,
    pub(crate) color_fade_target: Option<[f32; 3]>,
    pub(crate) child_emission: Option<ParticleChildEmissionDescriptor>,
    pub(crate) particle_limit: Option<ParticleLimitDescriptor>,
    pub(crate) render_group: ParticleRenderGroup,
    pub(crate) render_layer: ParticleRenderLayer,
    /// `TerrainParticle`/`BreakingItemParticle` sample a random quarter-texture
    /// (`uo`/`vo`) at construction.
    pub(crate) uv_sub_rect: bool,
    pub(crate) quad_shape: ParticleQuadShape,
}

static KIND_INFO: LazyLock<Vec<KindInfo>> = LazyLock::new(|| {
    ParticleKind::all()
        .map(|kind| {
            let descriptor = ParticleDescriptor::for_kind(kind);
            KindInfo {
                descriptor,
                tick_motion: descriptor.tick_motion(),
                collision_size: descriptor.collision_size(),
                drip_fluid: descriptor.drip_fluid(),
                required_fluid: descriptor.required_fluid(),
                moves_without_collision: descriptor.moves_without_collision(),
                air_downward_acceleration: descriptor.air_downward_acceleration(),
                falling_leaves: descriptor.falling_leaves(),
                light_emission: descriptor.light_emission(),
                facing_camera_mode: descriptor.facing_camera_mode(),
                alpha_curve: descriptor.alpha_curve(),
                color_fade_target: descriptor.color_fade_target(),
                child_emission: descriptor.child_emission(),
                particle_limit: particle_limit_for_kind(kind),
                render_group: render_group_for_kind(kind),
                render_layer: render_layer_for_kind(kind),
                uv_sub_rect: matches!(
                    kind,
                    ParticleKind::BLOCK
                        | ParticleKind::DUST_PILLAR
                        | ParticleKind::BLOCK_CRUMBLE
                        | ParticleKind::ITEM
                        | ParticleKind::ITEM_SLIME
                        | ParticleKind::ITEM_COBWEB
                        | ParticleKind::ITEM_SNOWBALL
                ),
                quad_shape: match descriptor.provider {
                    Provider::Shriek => ParticleQuadShape::Shriek,
                    Provider::VibrationSignal => ParticleQuadShape::Vibration,
                    _ => ParticleQuadShape::Billboard,
                },
            }
        })
        .collect()
});

fn render_group_for_kind(kind: ParticleKind) -> ParticleRenderGroup {
    match kind {
        ParticleKind::ITEM_PICKUP => ParticleRenderGroup::ItemPickup,
        ParticleKind::ELDER_GUARDIAN => ParticleRenderGroup::ElderGuardians,
        _ => ParticleRenderGroup::SingleQuads,
    }
}

fn render_layer_for_kind(kind: ParticleKind) -> ParticleRenderLayer {
    match kind {
        ParticleKind::BLOCK
        | ParticleKind::BLOCK_MARKER
        | ParticleKind::DUST_PILLAR
        | ParticleKind::BLOCK_CRUMBLE => ParticleRenderLayer::OpaqueTerrain,
        ParticleKind::ITEM
        | ParticleKind::ITEM_SLIME
        | ParticleKind::ITEM_COBWEB
        | ParticleKind::ITEM_SNOWBALL => ParticleRenderLayer::OpaqueItems,
        ParticleKind::CLOUD
        | ParticleKind::CAMPFIRE_COSY_SMOKE
        | ParticleKind::CAMPFIRE_SIGNAL_SMOKE
        | ParticleKind::SNEEZE
        | ParticleKind::TOTEM_OF_UNDYING
        | ParticleKind::SQUID_INK
        | ParticleKind::GLOW_SQUID_INK
        | ParticleKind::END_ROD
        | ParticleKind::SOUL
        | ParticleKind::SCULK_SOUL
        | ParticleKind::SCULK_CHARGE
        | ParticleKind::SCULK_CHARGE_POP
        | ParticleKind::SHRIEK
        | ParticleKind::VIBRATION
        | ParticleKind::VAULT_CONNECTION
        | ParticleKind::EFFECT
        | ParticleKind::INSTANT_EFFECT
        | ParticleKind::ENTITY_EFFECT
        | ParticleKind::FLASH
        | ParticleKind::FIREWORK
        | ParticleKind::FIREFLY
        | ParticleKind::ELDER_GUARDIAN
        | ParticleKind::INFESTED
        | ParticleKind::RAID_OMEN
        | ParticleKind::TRIAL_OMEN
        | ParticleKind::WITCH => ParticleRenderLayer::Translucent,
        _ => ParticleRenderLayer::Opaque,
    }
}
