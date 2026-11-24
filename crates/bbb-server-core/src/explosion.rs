//! Explosion presentation, exposure and damage arithmetic.

#[cfg(test)]
use crate::block_getter::BlockGetter;
use crate::random::GameRandom;

use bbb_protocol::packets::{
    BlockPos, ParticlePayload, SoundEventHolder, Vec3d, WeightedExplosionParticle,
};
use rustc_hash::FxHashSet;
use std::sync::{Arc, OnceLock};
use uuid::Uuid;

use bbb_server_types::fluid::{block_state_fluid_state, block_state_is_air, FluidState};
#[cfg(test)]
use bbb_server_types::geometry::empty_context_block_collision_boxes;
use bbb_server_types::geometry::Aabb;

/// `ServerExplosion.LARGE_EXPLOSION_RADIUS` (`ServerExplosion.java:42`).
pub const LARGE_EXPLOSION_RADIUS: f32 = bbb_facts::explosion::LARGE_EXPLOSION_RADIUS;

/// `player.distanceToSqr(center) < 4096.0` (`ServerLevel.java:1225`).
pub const EXPLOSION_PACKET_RANGE_SQR: f64 = 4096.0;

/// `WaterFluid.getExplosionResistance` (`WaterFluid.java:122-124`) and
/// `LavaFluid.getExplosionResistance` (`LavaFluid.java:226-228`); the empty
/// fluid answers zero (`EmptyFluid.java:42-44`).
pub const FLUID_EXPLOSION_RESISTANCE: f32 = bbb_facts::explosion::FLUID_EXPLOSION_RESISTANCE;

/// `Explosion.BlockInteraction` (`Explosion.java:42-57`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockInteraction {
    Keep,
    Destroy,
    DestroyWithDecay,
    TriggerBlock,
}

impl BlockInteraction {
    /// `BlockInteraction.shouldAffectBlocklikeEntities()` (`Explosion.java:54-56`).
    pub const fn should_affect_blocklike_entities(self) -> bool {
        matches!(self, Self::Destroy | Self::DestroyWithDecay)
    }
}

/// `Level.ExplosionInteraction` (`Level.java:1110-1128`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplosionInteraction {
    None,
    Block,
    Mob,
    Tnt,
    Trigger,
}

/// The game rules `ServerLevel.explode` reads when it turns an
/// [`ExplosionInteraction`] into a [`BlockInteraction`]
/// (`ServerLevel.java:1210-1218`, `:1232-1234`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplosionGameRules {
    pub mob_griefing: bool,
    pub block_explosion_drop_decay: bool,
    pub mob_explosion_drop_decay: bool,
    pub tnt_explosion_drop_decay: bool,
}

impl Default for ExplosionGameRules {
    fn default() -> Self {
        Self {
            mob_griefing: true,
            block_explosion_drop_decay: true,
            mob_explosion_drop_decay: true,
            tnt_explosion_drop_decay: true,
        }
    }
}

impl ExplosionInteraction {
    /// The `switch (interactionType)` of `ServerLevel.explode`
    /// (`ServerLevel.java:1210-1218`) over `getDestroyType` (`:1232-1234`).
    pub const fn block_interaction(self, rules: ExplosionGameRules) -> BlockInteraction {
        const fn destroy_type(drop_decay: bool) -> BlockInteraction {
            if drop_decay {
                BlockInteraction::DestroyWithDecay
            } else {
                BlockInteraction::Destroy
            }
        }
        match self {
            Self::None => BlockInteraction::Keep,
            Self::Block => destroy_type(rules.block_explosion_drop_decay),
            Self::Mob => {
                if rules.mob_griefing {
                    destroy_type(rules.mob_explosion_drop_decay)
                } else {
                    BlockInteraction::Keep
                }
            }
            Self::Tnt => destroy_type(rules.tnt_explosion_drop_decay),
            Self::Trigger => BlockInteraction::TriggerBlock,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplosionSourceEntityKind {
    Player {
        instabuild: bool,
        may_build: bool,
        spectator: bool,
    },
    Mob,
    LivingOther,
    Other,
}

/// One entity identity frozen into an explosion before its source can leave
/// the level.
#[derive(Debug, Clone, PartialEq)]
pub struct ExplosionSourceEntity {
    pub custom_name: Option<bbb_protocol::RichTextComponent>,
    pub entity_id: i32,
    pub entity_resource_id: String,
    pub uuid: Uuid,
    pub position: Vec3d,
    pub kind: ExplosionSourceEntityKind,
}

impl ExplosionSourceEntity {
    pub const fn is_mob(&self) -> bool {
        matches!(self.kind, ExplosionSourceEntityKind::Mob)
    }

    pub const fn is_living_non_player(&self) -> bool {
        matches!(
            self.kind,
            ExplosionSourceEntityKind::Mob | ExplosionSourceEntityKind::LivingOther
        )
    }

    pub const fn player_facts(&self) -> Option<(bool, bool)> {
        match self.kind {
            ExplosionSourceEntityKind::Player {
                instabuild,
                may_build,
                ..
            } => Some((instabuild, may_build)),
            ExplosionSourceEntityKind::Mob
            | ExplosionSourceEntityKind::LivingOther
            | ExplosionSourceEntityKind::Other => None,
        }
    }
}

/// `ServerExplosion.source`, `Explosion.getIndirectSourceEntity`, and the
/// entities carried by its `DamageSource`.
#[derive(Debug, Clone, PartialEq)]
pub struct ExplosionSource {
    /// `Explosion.getDirectSourceEntity()` / `DamageSource.getDirectEntity()`.
    pub direct: ExplosionSourceEntity,
    /// `Explosion.getIndirectSourceEntity()` (`Explosion.java:12-24`).
    pub indirect: Option<ExplosionSourceEntity>,
    /// `DamageSource.getEntity()`. Custom End Crystal explosions deliberately
    /// differ from [`Self::indirect`].
    pub causing: Option<ExplosionSourceEntity>,
    /// `Entity.isInWater()` of the direct source, read by
    /// `BlockAttachedEntity.ignoreExplosion` (`BlockAttachedEntity.java:93-100`).
    /// `Entity.updateFluidInteraction` has no owner in this crate (see
    /// `crate::entity`'s module documentation), so a producer passes what it
    /// can know and names the gap where it cannot.
    pub in_water: bool,
    /// `source.is(EntityType.BREEZE_WIND_CHARGE) || source.is(EntityType.WIND_CHARGE)`
    /// (`ServerExplosion.java:308`).
    pub wind_charge: bool,
}

/// What `PrimedTnt.explode` hands `Level.explode` (`PrimedTnt.java:121-136`)
/// before the level supplies the particles and the sound
/// (`Level.java:646-672`).
#[derive(Debug, Clone, PartialEq)]
pub struct PendingExplosion {
    pub source: Option<ExplosionSource>,
    pub center: Vec3d,
    pub radius: f32,
    pub fire: bool,
    pub interaction: ExplosionInteraction,
    pub damage_source: Option<ExplosionDamageSource>,
    pub damage_profile: ExplosionDamageProfile,
    pub presentation: ExplosionPresentation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExplosionDamageSource {
    pub damage_type: String,
    pub explicit_position: Option<Vec3d>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExplosionDamageProfile {
    Default,
    /// `PrimedTnt.USED_PORTAL_DAMAGE_CALCULATOR` excludes Nether portal
    /// blocks after `PrimedTnt.teleport` (`PrimedTnt.java:40-50`, `:190-197`).
    PrimedTntUsedPortal,
    /// `MinecartTNT` supplies an entity-based calculator whose rail behavior
    /// depends on the source state frozen at explosion time
    /// (`MinecartTNT.java:102-121`, `:170-187`).
    TntMinecart {
        primed: bool,
    },
    /// A dangerous `WitherSkull` caps the resistance of blocks the wither can
    /// destroy at 0.8 (`WitherSkull.java:51-55`).
    DangerousWitherSkull,
    /// `SimpleExplosionDamageCalculator.java:28-51`.
    Simple {
        explodes_blocks: bool,
        damages_entities: bool,
        knockback_multiplier: f32,
        immune_blocks: Option<Arc<FxHashSet<i32>>>,
    },
}

impl ExplosionDamageProfile {
    pub const fn damages_entities(&self) -> bool {
        match self {
            Self::Simple {
                damages_entities, ..
            } => *damages_entities,
            Self::Default
            | Self::PrimedTntUsedPortal
            | Self::TntMinecart { .. }
            | Self::DangerousWitherSkull => true,
        }
    }

    pub const fn knockback_multiplier(&self) -> f32 {
        match self {
            Self::Simple {
                knockback_multiplier,
                ..
            } => *knockback_multiplier,
            Self::Default
            | Self::PrimedTntUsedPortal
            | Self::TntMinecart { .. }
            | Self::DangerousWitherSkull => 1.0,
        }
    }

    pub fn wind_charge(damages_entities: bool, knockback_multiplier: f32) -> Self {
        static IMMUNE: OnceLock<Arc<FxHashSet<i32>>> = OnceLock::new();
        let immune = IMMUNE.get_or_init(|| {
            Arc::new(
                bbb_server_types::fluid::block_type_ids_in_tag(
                    "minecraft:blocks_wind_charge_explosions",
                )
                .into_iter()
                .collect(),
            )
        });
        Self::Simple {
            explodes_blocks: true,
            damages_entities,
            knockback_multiplier,
            immune_blocks: Some(Arc::clone(immune)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExplosionPresentation {
    Default,
    Custom {
        small_particle: ParticlePayload,
        large_particle: ParticlePayload,
        block_particles: Vec<WeightedExplosionParticle>,
        sound: SoundEventHolder,
    },
}

impl ExplosionPresentation {
    /// Wind-charge particles and sounds (`WindCharge.java:58-74`, `BreezeWindCharge.java:23-39`).
    pub fn wind_charge(sound: &str) -> Self {
        Self::Custom {
            small_particle: ParticlePayload {
                particle_type_id:
                    bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_GUST_EMITTER_SMALL_ID,
                raw_options: Vec::new(),
            },
            large_particle: ParticlePayload {
                particle_type_id:
                    bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_GUST_EMITTER_LARGE_ID,
                raw_options: Vec::new(),
            },
            block_particles: Vec::new(),
            sound: SoundEventHolder::Reference {
                registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                    sound,
                )
                .expect("builtin wind charge sound"),
            },
        }
    }
}

/// `Block.getExplosionResistance()` (`Block.java:456-458`) for a built-in
/// block state, or `None` for an unknown state.
pub fn vanilla_block_explosion_resistance(block_state_id: i32) -> Option<f32> {
    let block_type_id =
        bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id)?;
    bbb_protocol::block_types::vanilla_block_explosion_resistance(block_type_id)
}

/// `FluidState.getExplosionResistance()` (`FluidState.java:112-114`) over the
/// fluid type's answer.
pub fn fluid_explosion_resistance(block_state_id: i32) -> f32 {
    match block_state_fluid_state(block_state_id) {
        FluidState::Empty => 0.0,
        FluidState::Source { .. } | FluidState::Flowing { .. } => FLUID_EXPLOSION_RESISTANCE,
    }
}

/// `ExplosionDamageCalculator.getBlockExplosionResistance`
/// (`ExplosionDamageCalculator.java:11-17`).
///
/// `Err` is the block half of the `max` being unavailable; the fluid half
/// alone cannot stand in for it.
pub fn block_explosion_resistance_with_registry(
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<Option<f32>, ()> {
    let fluid = block_state_fluid_state(block_state_id);
    if block_state_is_air(block_state_id) && fluid == FluidState::Empty {
        return Ok(None);
    }
    let block = vanilla_block_explosion_resistance(block_state_id)
        .or_else(|| {
            content_registry
                .block_by_state_id(block_state_id)
                .map(|(spec, _)| {
                    if spec.destroy_speed < 0.0 {
                        3_600_000.0
                    } else {
                        spec.destroy_speed.max(0.0)
                    }
                })
        })
        .ok_or(())?;
    Ok(Some(block.max(fluid_explosion_resistance(block_state_id))))
}

pub fn shuffle<T>(list: &mut [T], random: &GameRandom) {
    let mut i = list.len();
    while i > 1 {
        let swap_to = random.next_int(i32::try_from(i).expect("a list of block positions"));
        list.swap(i - 1, swap_to as usize);
        i -= 1;
    }
}

#[cfg(test)]
pub fn get_seen_percent<L: BlockGetter + ?Sized>(level: &L, center: Vec3d, bb: Aabb) -> f32 {
    bbb_facts::explosion::get_seen_percent([center.x, center.y, center.z], bb, |[x, y, z]| {
        let pos = BlockPos { x, y, z };
        empty_context_block_collision_boxes(level.block_state(pos), pos)
            .unwrap_or_else(|| vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })])
    })
}

#[cfg(test)]
pub fn clip_misses<L: BlockGetter + ?Sized>(level: &L, from: Vec3d, to: Vec3d) -> bool {
    bbb_facts::explosion::clip_misses_with(
        [from.x, from.y, from.z],
        [to.x, to.y, to.z],
        |[x, y, z]| {
            let pos = BlockPos { x, y, z };
            empty_context_block_collision_boxes(level.block_state(pos), pos)
                .unwrap_or_else(|| vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })])
        },
    )
}

/// One entity's share of `ServerExplosion.hurtEntities`
/// (`ServerExplosion.java:184-199`): the values the hurt, the push and the
/// player's packet are built from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityHit {
    /// `Math.sqrt(entity.distanceToSqr(center)) / doubleRadius` (`:184`).
    pub dist: f64,
    /// `getSeenPercent(center, entity)` (`:190`).
    pub exposure: f32,
    /// `getEntityDamageAmount(this, entity, exposure)` (`:192`).
    pub damage: f32,
    /// `direction.scale(knockbackPower)` (`:198-199`).
    pub knockback: Vec3d,
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub fn entity_hit_with_profile<L: BlockGetter + ?Sized>(
    level: &L,
    center: Vec3d,
    radius: f32,
    entity_position: Vec3d,
    entity_origin: Vec3d,
    bounding_box: Aabb,
    knockback_resistance: f64,
    profile: &ExplosionDamageProfile,
) -> Option<EntityHit> {
    entity_hit_with_profile_and_shapes(
        center,
        radius,
        entity_position,
        entity_origin,
        bounding_box,
        knockback_resistance,
        profile,
        |pos| {
            empty_context_block_collision_boxes(level.block_state(pos), pos)
                .unwrap_or_else(|| vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })])
        },
    )
}

#[allow(clippy::too_many_arguments)]
pub fn entity_hit_with_profile_and_shapes(
    center: Vec3d,
    radius: f32,
    entity_position: Vec3d,
    entity_origin: Vec3d,
    bounding_box: Aabb,
    knockback_resistance: f64,
    profile: &ExplosionDamageProfile,
    mut boxes_at: impl FnMut(BlockPos) -> Vec<Aabb>,
) -> Option<EntityHit> {
    let hit = bbb_facts::explosion::compute_explosion_entity_hit(
        bbb_facts::explosion::ExplosionHitInput {
            center: [center.x, center.y, center.z],
            radius,
            entity_position: [entity_position.x, entity_position.y, entity_position.z],
            entity_origin: [entity_origin.x, entity_origin.y, entity_origin.z],
            bounding_box,
            knockback_resistance,
            damages_entities: profile.damages_entities(),
            knockback_multiplier: profile.knockback_multiplier(),
        },
        |[x, y, z]| boxes_at(BlockPos { x, y, z }),
    )?;
    Some(EntityHit {
        dist: hit.dist,
        exposure: hit.exposure,
        damage: hit.damage,
        knockback: Vec3d {
            x: hit.knockback[0],
            y: hit.knockback[1],
            z: hit.knockback[2],
        },
    })
}

/// `ServerExplosion.isSmall()` (`ServerExplosion.java:312-314`) over
/// `interactsWithBlocks` (`:264-266`).
pub const fn is_small(radius: f32, block_interaction: BlockInteraction) -> bool {
    radius < LARGE_EXPLOSION_RADIUS || matches!(block_interaction, BlockInteraction::Keep)
}

/// The `AABB` `hurtEntities` queries (`ServerExplosion.java:174-182`), or
/// `None` for the `radius < 1.0E-5F` skip (`:173`).
pub fn hurt_entities_box(center: Vec3d, radius: f32) -> Option<Aabb> {
    bbb_facts::explosion::hurt_entities_box([center.x, center.y, center.z], radius).map(Aabb::from)
}

/// `ServerExplosion.shouldAffectBlocklikeEntities()` (`ServerExplosion.java:306-310`).
pub fn should_affect_blocklike_entities(
    mob_griefing: bool,
    source: Option<&ExplosionSource>,
    block_interaction: BlockInteraction,
) -> bool {
    let is_not_wind_charge = source.is_none_or(|source| !source.wind_charge);
    if mob_griefing {
        is_not_wind_charge
    } else {
        block_interaction.should_affect_blocklike_entities() && is_not_wind_charge
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod dynamic_shape_tests {
    use std::collections::BTreeMap;

    use bbb_protocol::block_state_types::{
        vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
    };
    use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
    use rustc_hash::FxHashMap;

    use super::*;

    struct SparseBlocks {
        air: i32,
        cells: FxHashMap<BlockPos, i32>,
    }

    impl SparseBlocks {
        fn single(pos: BlockPos, state: i32) -> Self {
            let air = vanilla_block_default_state_id(
                vanilla_block_type_for_resource_id("minecraft:air")
                    .unwrap()
                    .id,
            )
            .unwrap();
            let mut cells = FxHashMap::default();
            cells.insert(pos, state);
            Self { air, cells }
        }
    }

    impl BlockGetter for SparseBlocks {
        fn block_state(&self, pos: BlockPos) -> i32 {
            self.cells.get(&pos).copied().unwrap_or(self.air)
        }
    }

    fn default_state(name: &str) -> i32 {
        vanilla_block_default_state_id(vanilla_block_type_for_resource_id(name).unwrap().id)
            .unwrap()
    }

    #[test]
    fn explosion_rays_respect_dynamic_block_collision_shapes() {
        let obstacle = BlockPos { x: 1, y: 64, z: 0 };
        let from = Vec3d {
            x: 0.5,
            y: 64.5,
            z: 0.5,
        };
        let to = Vec3d {
            x: 2.5,
            y: 64.5,
            z: 0.5,
        };
        let target_bb = Aabb {
            min_x: 2.2,
            min_y: 64.2,
            min_z: 0.2,
            max_x: 2.8,
            max_y: 64.8,
            max_z: 0.8,
        };

        // Powder snow and moving piston (without BE) have empty CollisionContext.empty() shapes.
        for empty_dynamic in ["minecraft:powder_snow", "minecraft:moving_piston"] {
            let level = SparseBlocks::single(obstacle, default_state(empty_dynamic));
            assert!(
                clip_misses(&level, from, to),
                "{empty_dynamic} should not block ray"
            );
            assert_eq!(get_seen_percent(&level, from, target_bb), 1.0);
        }

        // Closed shulker box defaults to a full block when no open BE is present.
        let shulker = SparseBlocks::single(obstacle, default_state("minecraft:shulker_box"));
        assert!(!clip_misses(&shulker, from, to));
        assert_eq!(get_seen_percent(&shulker, from, target_bb), 0.0);

        // Scaffolding (`SHAPE_STABLE` in `CollisionContext.empty()`) is hollow through its mid-height
        // interior (`y = 64.5, z = 0.5`) but solid across its top slab (`y = 64.95`).
        let scaffolding = SparseBlocks::single(obstacle, default_state("minecraft:scaffolding"));
        assert!(clip_misses(&scaffolding, from, to));
        assert!(!clip_misses(
            &scaffolding,
            Vec3d {
                x: 0.5,
                y: 64.95,
                z: 0.5
            },
            Vec3d {
                x: 2.5,
                y: 64.95,
                z: 0.5
            },
        ));

        // Bamboo (`3/16` column shifted by `getOffset(pos)`) blocks a ray through its offset column
        // center while letting a ray past the corner of the same block cell miss.
        let bamboo_state = default_state("minecraft:bamboo");
        let bamboo_box = empty_context_block_collision_boxes(bamboo_state, obstacle).unwrap()[0];
        let bamboo_level = SparseBlocks::single(obstacle, bamboo_state);
        let bamboo_mid_z =
            f64::from(obstacle.z) + f64::midpoint(bamboo_box.min_z, bamboo_box.max_z);
        assert!(!clip_misses(
            &bamboo_level,
            Vec3d {
                x: 0.5,
                y: 64.5,
                z: bamboo_mid_z
            },
            Vec3d {
                x: 2.5,
                y: 64.5,
                z: bamboo_mid_z
            },
        ));
        let corner_z = if bamboo_box.min_z > 0.1 { 0.05 } else { 0.95 };
        assert!(clip_misses(
            &bamboo_level,
            Vec3d {
                x: 0.5,
                y: 64.5,
                z: corner_z
            },
            Vec3d {
                x: 2.5,
                y: 64.5,
                z: corner_z
            },
        ));

        // Pointed dripstone (`tip_merge` `6/16` column shifted by `getOffset(pos)`).
        let dripstone_props: BTreeMap<String, String> = [
            ("thickness".to_string(), "tip_merge".to_string()),
            ("vertical_direction".to_string(), "up".to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ]
        .into_iter()
        .collect();
        let dripstone_state = vanilla_block_state_id_by_name_and_properties(
            "minecraft:pointed_dripstone",
            &dripstone_props,
        )
        .unwrap();
        let dripstone_box =
            empty_context_block_collision_boxes(dripstone_state, obstacle).unwrap()[0];
        let dripstone_level = SparseBlocks::single(obstacle, dripstone_state);
        let dripstone_corner_z = if dripstone_box.min_z > 0.1 {
            0.05
        } else {
            0.95
        };
        assert!(clip_misses(
            &dripstone_level,
            Vec3d {
                x: 0.5,
                y: 64.5,
                z: dripstone_corner_z
            },
            Vec3d {
                x: 2.5,
                y: 64.5,
                z: dripstone_corner_z
            },
        ));
    }
}
