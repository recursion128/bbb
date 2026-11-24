use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;

use rustc_hash::FxHashMap;

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type_deflects_projectiles, vanilla_entity_type_is_redirectable_projectile,
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_ARROW_ID,
    VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
    VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
    VANILLA_ENTITY_TYPE_TRIDENT_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_FIREWORK_ROCKET_ID;
use bbb_protocol::packets::EntityDataValueKind;
use bbb_protocol::packets::EquipmentSlot as ProtocolEquipmentSlot;

use super::animations::{java_random_next_float, java_random_seed, spawn_random_bits};
use super::dimensions::{vanilla_bounds_for_entity_data, vanilla_eye_height_for_entity_data};
use super::record::{
    EntityAbstractArrowRuntime, EntityExperienceOrbMovement, EntityFishingHookMovement,
    EntityFishingHookState, EntityIdentity, EntityIncarnation, EntityInstanceReference,
    EntityMetadata, EntitySupportingBlock, EntityTransform,
};
use super::schema::{
    vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type,
    vanilla_entity_metadata_armor_stand_accessors_for_entity_type,
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_firework_rocket_accessors_for_entity_type,
    vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type,
    vanilla_entity_metadata_living_entity_accessors_for_entity_type,
    vanilla_entity_metadata_primed_tnt_accessors_for_entity_type,
    vanilla_entity_metadata_thrown_trident_accessors_for_entity_type,
    vanilla_entity_metadata_type_accepts_projectile_hurt_client,
    vanilla_entity_metadata_type_block_effect_tick_policy,
    vanilla_entity_metadata_type_client_movement_policy, vanilla_entity_metadata_type_is_living,
    vanilla_entity_metadata_type_is_pickable,
    vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type,
    VanillaEntityClientMovementPolicy,
};
use super::{
    EntityBlockEffectMovement, EntityClientMovementEffectState, EntityClientMovementParticleState,
    EntityClientMovementSimpleParticleKind, EntityClientMovementSimpleParticleState,
    EntityPickBoundsState, EntityStore, EntityVec3, EyeOfEnderTrailParticleState,
    FireworkRocketTrailParticleState, ShulkerBulletTrailParticleState,
    VANILLA_ENTITY_TYPE_PLAYER_ID,
};

mod math;
mod projectiles;
pub(crate) use math::*;
use projectiles::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityClientMovementCollisionInput {
    pub(crate) entity_id: i32,
    pub(crate) entity_type_id: i32,
    pub(crate) position: EntityVec3,
    pub(crate) pre_move_delta: EntityVec3,
    pub(crate) requested_movement: EntityVec3,
    pub(crate) bounds: EntityPickBoundsState,
    pub(crate) on_ground: bool,
    pub(crate) previous_supporting_block: EntitySupportingBlock,
    pub(crate) movement_skipped: bool,
    pub(crate) experience_orb_update: Option<EntityExperienceOrbMovementUpdate>,
    pub(crate) fishing_hook_update: Option<EntityFishingHookMovementUpdate>,
    pub(crate) fishing_hook_entity_hit: Option<EntityFishingHookProjectileHit>,
    pub(crate) fishing_hook_collision_delta: Option<EntityVec3>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityClientMovementCollisionResult {
    pub(crate) pre_move_delta: EntityVec3,
    pub(crate) requested_movement: EntityVec3,
    pub(crate) clipped_movement: EntityVec3,
    pub(crate) supporting_block: Option<crate::BlockPos>,
    pub(crate) ground_friction: f32,
    pub(crate) movement_skipped: bool,
    pub(crate) experience_orb_update: Option<EntityExperienceOrbMovementUpdate>,
    pub(crate) fishing_hook_update: Option<EntityFishingHookMovementUpdate>,
    pub(crate) fishing_hook_reverse_deflection: Option<EntityFishingHookReverseDeflection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EntityExperienceOrbMovementUpdate {
    pub(crate) following_player_uuid: Option<uuid::Uuid>,
    pub(crate) random_seed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityFishingHookMovementUpdate {
    pub(crate) movement: EntityFishingHookMovement,
    pub(crate) position_override: Option<EntityVec3>,
    pub(crate) clear_hooked_entity_data: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum EntityFishingHookReverseDeflection {
    WorldBorder,
    Entity { entity_id: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityClientMovementFluidInput {
    pub(crate) entity_id: i32,
    pub(crate) entity_type_id: i32,
    pub(crate) position: EntityVec3,
    pub(crate) bounds: EntityPickBoundsState,
    pub(crate) aabb_min: [f64; 3],
    pub(crate) aabb_max: [f64; 3],
    pub(crate) eye_position: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EntityClientMovementTridentReturnTarget {
    eye_position: EntityVec3,
    loyalty: i8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EntityExperienceOrbPlayerTarget {
    entity_id: i32,
    uuid: uuid::Uuid,
    position: EntityVec3,
    attraction_position: EntityVec3,
    is_dead_or_dying: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityProjectileTarget {
    pub(crate) entity_id: i32,
    pub(crate) entity_type_id: i32,
    uuid: uuid::Uuid,
    root_vehicle_id: i32,
    position: EntityVec3,
    bounds: EntityPickBoundsState,
    alive: bool,
    pickable: bool,
    deflects_projectiles: bool,
    redirectable_projectile: bool,
    hurt_client: bool,
    trident_hurt_client: bool,
    hurt_client_rejects_ender_dragon_owner: bool,
    incarnation: EntityIncarnation,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntityAbstractArrowCollisionSource {
    pub(crate) entity_id: i32,
    pub(crate) entity_type_id: i32,
    pub(crate) position: EntityVec3,
    pub(crate) movement: EntityVec3,
    pub(crate) bounds: EntityPickBoundsState,
    pub(crate) on_ground: bool,
    pub(crate) tick_count: u32,
    pub(crate) owner_uuid: Option<uuid::Uuid>,
    pub(crate) owner_entity_type_id: Option<i32>,
    pub(crate) owner_root_vehicle_id: Option<i32>,
    pub(crate) left_owner: bool,
    pub(crate) trident_dealt_damage: bool,
    pub(crate) piercing_ignore_entity_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntityAbstractArrowCollisionBatch {
    pub(crate) sources: Vec<EntityAbstractArrowCollisionSource>,
    pub(crate) targets: Vec<EntityProjectileTarget>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityAbstractArrowBlockHit {
    pub(crate) location: EntityVec3,
    pub(crate) block_state_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityAbstractArrowEntityHit {
    pub(crate) entity_id: i32,
    pub(crate) incarnation: EntityIncarnation,
    pub(crate) location: EntityVec3,
    pub(crate) target_position: EntityVec3,
    pub(crate) deflects_projectiles: bool,
    pub(crate) redirectable_projectile: bool,
    pub(crate) hurt_client: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntityAbstractArrowCollisionFacts {
    pub(crate) initial_block_state_id: Option<i32>,
    pub(crate) endpoint: EntityVec3,
    pub(crate) world_border_hit: bool,
    pub(crate) block_hit: Option<EntityAbstractArrowBlockHit>,
    pub(crate) entity_hits: Vec<EntityAbstractArrowEntityHit>,
}

impl EntityAbstractArrowCollisionBatch {
    pub(crate) fn update_item_hurt_clients(
        &mut self,
        mut accepts_hurt: impl FnMut(i32) -> [bool; 2],
    ) {
        for target in self
            .targets
            .iter_mut()
            .filter(|target| target.entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID)
        {
            let [arrow_hurt_client, trident_hurt_client] = accepts_hurt(target.entity_id);
            target.hurt_client = arrow_hurt_client;
            target.trident_hurt_client = trident_hurt_client;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityFishingHookProjectileHit {
    pub(crate) scale: f64,
    pub(crate) reverse_deflection: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ProjectileAabb {
    min: EntityVec3,
    max: EntityVec3,
}

impl ProjectileAabb {
    fn expand_towards(self, delta: EntityVec3) -> Self {
        Self {
            min: EntityVec3 {
                x: self.min.x + delta.x.min(0.0),
                y: self.min.y + delta.y.min(0.0),
                z: self.min.z + delta.z.min(0.0),
            },
            max: EntityVec3 {
                x: self.max.x + delta.x.max(0.0),
                y: self.max.y + delta.y.max(0.0),
                z: self.max.z + delta.z.max(0.0),
            },
        }
    }

    fn inflate(self, amount: f64) -> Self {
        Self {
            min: EntityVec3 {
                x: self.min.x - amount,
                y: self.min.y - amount,
                z: self.min.z - amount,
            },
            max: EntityVec3 {
                x: self.max.x + amount,
                y: self.max.y + amount,
                z: self.max.z + amount,
            },
        }
    }

    fn intersects(self, other: Self) -> bool {
        self.min.x < other.max.x
            && self.max.x > other.min.x
            && self.min.y < other.max.y
            && self.max.y > other.min.y
            && self.min.z < other.max.z
            && self.max.z > other.min.z
    }

    fn center(self) -> EntityVec3 {
        EntityVec3 {
            x: (self.min.x + self.max.x) * 0.5,
            y: (self.min.y + self.max.y) * 0.5,
            z: (self.min.z + self.max.z) * 0.5,
        }
    }
}

fn projectile_world_bounds(position: EntityVec3, bounds: EntityPickBoundsState) -> ProjectileAabb {
    ProjectileAabb {
        min: EntityVec3 {
            x: position.x + f64::from(bounds.min[0]),
            y: position.y + f64::from(bounds.min[1]),
            z: position.z + f64::from(bounds.min[2]),
        },
        max: EntityVec3 {
            x: position.x + f64::from(bounds.max[0]),
            y: position.y + f64::from(bounds.max[1]),
            z: position.z + f64::from(bounds.max[2]),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn fishing_hook_entity_hit(
    source_id: i32,
    from: EntityVec3,
    delta: EntityVec3,
    owner_root_vehicle_id: Option<i32>,
    left_owner: bool,
    tick_count: u32,
    last_deflected_by: Option<i32>,
    targets: &[EntityProjectileTarget],
) -> Option<EntityFishingHookProjectileHit> {
    let to = add(from, delta);
    let margin = (((tick_count as f32) - 2.0) / 20.0).clamp(0.0, 0.3);
    targets
        .iter()
        .filter(|target| target.entity_id != source_id && target.alive)
        .filter(|target| {
            let super_can_hit = target.pickable
                && (left_owner
                    || owner_root_vehicle_id
                        .is_none_or(|owner_root| target.root_vehicle_id != owner_root));
            super_can_hit || target.entity_type_id == super::VANILLA_ENTITY_TYPE_ITEM_ID
        })
        .filter_map(|target| {
            fishing_hook_segment_box_entry_scale(
                from,
                to,
                projectile_world_bounds(target.position, target.bounds).inflate(f64::from(margin)),
            )
            .map(|scale| EntityFishingHookProjectileHit {
                scale,
                reverse_deflection: target
                    .deflects_projectiles
                    .then_some(target.entity_id)
                    .filter(|_| last_deflected_by != Some(target.entity_id)),
            })
        })
        .min_by(|left, right| left.scale.total_cmp(&right.scale))
}

pub(crate) fn resolve_abstract_arrow_collision_facts(
    batch: EntityAbstractArrowCollisionBatch,
    mut initial_block_state: impl FnMut(&EntityAbstractArrowCollisionSource) -> Option<i32>,
    mut clip_blocks: impl FnMut(
        i32,
        EntityVec3,
        EntityVec3,
        bool,
        f64,
    ) -> crate::client::ProjectileBlockMovementClip,
    mut players_can_harm: impl FnMut(uuid::Uuid, uuid::Uuid) -> bool,
) -> FxHashMap<i32, EntityAbstractArrowCollisionFacts> {
    let mut facts_by_id = FxHashMap::default();
    for source in batch.sources {
        let initial_block_state_id = initial_block_state(&source);
        let clipped = clip_blocks(
            source.entity_type_id,
            source.position,
            source.movement,
            source.on_ground,
            source.position.y,
        );
        let block_location = add(
            source.position,
            EntityVec3 {
                x: clipped.movement[0],
                y: clipped.movement[1],
                z: clipped.movement[2],
            },
        );
        let block_hit = clipped
            .block_state_id
            .map(|block_state_id| EntityAbstractArrowBlockHit {
                location: block_location,
                block_state_id,
            });
        let endpoint = if block_hit.is_some() || clipped.world_border_hit {
            block_location
        } else {
            add(source.position, source.movement)
        };
        let to = endpoint;
        let search_bounds = projectile_world_bounds(source.position, source.bounds)
            .expand_towards(source.movement)
            .inflate(1.0);
        let margin = (((source.tick_count as f32) - 2.0) / 20.0).clamp(0.0, 0.3) as f64;
        let mut entity_hits = batch
            .targets
            .iter()
            .filter(|target| {
                target.entity_id != source.entity_id
                    && target.alive
                    && target.pickable
                    && search_bounds
                        .intersects(projectile_world_bounds(target.position, target.bounds))
                    && (source.left_owner
                        || source
                            .owner_root_vehicle_id
                            .is_none_or(|owner_root| target.root_vehicle_id != owner_root))
                    && !source
                        .piercing_ignore_entity_ids
                        .contains(&target.entity_id)
                    && !(target.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                        && source
                            .owner_uuid
                            .is_some_and(|owner_uuid| !players_can_harm(owner_uuid, target.uuid)))
            })
            .filter_map(|target| {
                let bounds = projectile_world_bounds(target.position, target.bounds);
                let location = if source.entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
                    fishing_hook_segment_box_entry_scale(
                        source.position,
                        to,
                        bounds.inflate(margin),
                    )
                    .map(|entry_scale| {
                        add(
                            source.position,
                            scale(subtract(to, source.position), entry_scale),
                        )
                    })
                } else if let Some(entry_scale) =
                    fishing_hook_segment_box_entry_scale(source.position, to, bounds)
                {
                    Some(add(
                        source.position,
                        scale(subtract(to, source.position), entry_scale),
                    ))
                } else if margin > 0.0 {
                    fishing_hook_segment_box_entry_scale(
                        source.position,
                        to,
                        bounds.inflate(margin),
                    )
                    .and_then(|outside_scale| {
                        let outside = add(
                            source.position,
                            scale(subtract(to, source.position), outside_scale),
                        );
                        let toward_center = bounds.center();
                        let center_delta = subtract(toward_center, outside);
                        let center_clip = clip_blocks(
                            source.entity_type_id,
                            outside,
                            center_delta,
                            source.on_ground,
                            source.position.y,
                        );
                        let toward_surface = if center_clip.block_state_id.is_some()
                            || center_clip.world_border_hit
                        {
                            add(
                                outside,
                                EntityVec3 {
                                    x: center_clip.movement[0],
                                    y: center_clip.movement[1],
                                    z: center_clip.movement[2],
                                },
                            )
                        } else {
                            toward_center
                        };
                        fishing_hook_segment_box_entry_scale(outside, toward_surface, bounds).map(
                            |surface_scale| {
                                add(
                                    outside,
                                    scale(subtract(toward_surface, outside), surface_scale),
                                )
                            },
                        )
                    })
                } else {
                    None
                }?;
                Some(EntityAbstractArrowEntityHit {
                    entity_id: target.entity_id,
                    incarnation: target.incarnation,
                    location,
                    target_position: target.position,
                    deflects_projectiles: target.deflects_projectiles,
                    redirectable_projectile: target.redirectable_projectile,
                    hurt_client: (if source.entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
                        target.trident_hurt_client
                    } else {
                        target.hurt_client
                    }) && !(target.hurt_client_rejects_ender_dragon_owner
                        && source.owner_entity_type_id
                            == Some(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID)),
                })
            })
            .collect::<Vec<_>>();
        if source.entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
            if source.trident_dealt_damage {
                entity_hits.clear();
            } else {
                entity_hits.sort_by(|left, right| {
                    length_squared(subtract(left.location, source.position))
                        .total_cmp(&length_squared(subtract(right.location, source.position)))
                });
                entity_hits.truncate(1);
            }
        } else {
            entity_hits.sort_by(|left, right| {
                length_squared(subtract(left.target_position, source.position)).total_cmp(
                    &length_squared(subtract(right.target_position, source.position)),
                )
            });
        }
        facts_by_id.insert(
            source.entity_id,
            EntityAbstractArrowCollisionFacts {
                initial_block_state_id,
                endpoint,
                world_border_hit: clipped.world_border_hit,
                block_hit,
                entity_hits,
            },
        );
    }
    facts_by_id
}

impl bbb_facts::collision::CollisionBounds for ProjectileAabb {
    fn min_x(self) -> f64 {
        self.min.x
    }
    fn min_y(self) -> f64 {
        self.min.y
    }
    fn min_z(self) -> f64 {
        self.min.z
    }
    fn max_x(self) -> f64 {
        self.max.x
    }
    fn max_y(self) -> f64 {
        self.max.y
    }
    fn max_z(self) -> f64 {
        self.max.z
    }
}

fn fishing_hook_segment_box_entry_scale(
    from: EntityVec3,
    to: EntityVec3,
    bounds: ProjectileAabb,
) -> Option<f64> {
    bbb_facts::collision::segment_box_entry_scale(
        bounds,
        [from.x, from.y, from.z],
        [to.x, to.y, to.z],
    )
}

pub(super) fn experience_orb_movement_from_uuid(uuid: uuid::Uuid) -> EntityExperienceOrbMovement {
    EntityExperienceOrbMovement {
        following_player_uuid: None,
        random_seed: java_random_seed(spawn_random_bits(uuid, 0x6578_7065_7269_656e) as i64),
    }
}

pub(super) fn fishing_hook_movement_from_uuid(
    uuid: uuid::Uuid,
    owner_uuid: Option<uuid::Uuid>,
) -> EntityFishingHookMovement {
    EntityFishingHookMovement {
        state: EntityFishingHookState::Flying,
        owner_uuid,
        hooked_entity_uuid: None,
        biting: false,
        left_owner: false,
        last_deflected_by: None,
        random_seed: java_random_seed(spawn_random_bits(uuid, 0x6669_7368_5f72_6e67) as i64),
    }
}

pub(super) fn client_movement_uses_in_water(entity_type_id: i32) -> bool {
    matches!(
        vanilla_entity_metadata_type_client_movement_policy(entity_type_id),
        Some(
            VanillaEntityClientMovementPolicy::EyeOfEnder
                | VanillaEntityClientMovementPolicy::AbstractArrow
                | VanillaEntityClientMovementPolicy::ThrowableProjectile
                | VanillaEntityClientMovementPolicy::ThrownExperienceBottle
                | VanillaEntityClientMovementPolicy::ThrownPotion
                | VanillaEntityClientMovementPolicy::HurtingProjectile
                | VanillaEntityClientMovementPolicy::WindCharge
                | VanillaEntityClientMovementPolicy::DragonFireball
                | VanillaEntityClientMovementPolicy::WitherSkull
        )
    )
}

pub(super) fn client_movement_uses_fluid_contact(entity_type_id: i32) -> bool {
    matches!(
        vanilla_entity_metadata_type_client_movement_policy(entity_type_id),
        Some(
            VanillaEntityClientMovementPolicy::Item
                | VanillaEntityClientMovementPolicy::ExperienceOrb
                | VanillaEntityClientMovementPolicy::FishingHook
        )
    )
}

pub(super) fn client_movement_uses_collision_facts(entity_type_id: i32) -> bool {
    matches!(
        vanilla_entity_metadata_type_client_movement_policy(entity_type_id),
        Some(
            VanillaEntityClientMovementPolicy::FallingBlock
                | VanillaEntityClientMovementPolicy::PrimedTnt
                | VanillaEntityClientMovementPolicy::Item
                | VanillaEntityClientMovementPolicy::ExperienceOrb
                | VanillaEntityClientMovementPolicy::FishingHook
        )
    )
}

impl EntityStore {
    /// Virtual `AbstractArrow.lerpMotion`: a non-zero server motion correction
    /// releases a locally grounded arrow before its next client tick.
    pub(crate) fn apply_abstract_arrow_lerp_motion(
        &mut self,
        entity_id: i32,
        movement: EntityVec3,
    ) {
        if length_squared(movement) <= 0.0 {
            return;
        }
        let Some(entity) = self.record_mut(entity_id) else {
            return;
        };
        let Some(accessors) = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            entity.identity.entity_type_id,
        ) else {
            return;
        };
        let metadata = &mut entity.metadata;
        if entity_data_bool(
            &metadata.data_values,
            accessors.in_ground_data_id,
            accessors.in_ground_default,
        ) {
            set_entity_data_bool(metadata, accessors.in_ground_data_id, false);
        }
    }

    pub(crate) fn client_movement_collision_inputs<
        S1: BuildHasher,
        S2: BuildHasher,
        S3: BuildHasher,
        S4: BuildHasher,
    >(
        &mut self,
        fluid_by_id: &HashMap<i32, crate::client::EntityFluidContactState, S1>,
        world_fact_position_by_id: &HashMap<i32, EntityVec3, S2>,
        collision_free_by_id: &HashMap<i32, bool, S3>,
        spectator_player_uuids: &HashSet<uuid::Uuid, S4>,
        lava_current_scale: f64,
        game_time: i64,
    ) -> Vec<EntityClientMovementCollisionInput> {
        let mut experience_orb_player_targets = None;
        let fishing_hook_needs_targets = {
            self.records()
                .filter_map(|entity| entity.family.fishing_hook())
                .any(|movement| {
                    movement.owner_uuid.is_some_and(|owner_uuid| {
                        self.protocol_id_for_uuid(owner_uuid)
                            .and_then(|id| self.entity_type_id(id))
                            == Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
                    })
                })
        };
        let fishing_hook_targets = if fishing_hook_needs_targets {
            self.projectile_targets(spectator_player_uuids)
        } else {
            Default::default()
        };
        let mut inputs = Vec::new();
        for entity in self.records() {
            if entity.family.collision_facts().is_none() {
                continue;
            }
            let identity = &entity.identity;
            let metadata = &entity.metadata;
            let transform = &entity.transform;
            let supporting_block = &entity.supporting_block;
            let attributes = &entity.attributes;
            let animations = &entity.animations;
            let orb_movement = entity.family.experience_orb();
            let fishing_hook_movement = entity.family.fishing_hook();
            let entity_type_id = identity.entity_type_id;
            let Some(policy) = vanilla_entity_metadata_type_client_movement_policy(entity_type_id)
            else {
                continue;
            };
            debug_assert!(matches!(
                policy,
                VanillaEntityClientMovementPolicy::FallingBlock
                    | VanillaEntityClientMovementPolicy::PrimedTnt
                    | VanillaEntityClientMovementPolicy::Item
                    | VanillaEntityClientMovementPolicy::ExperienceOrb
                    | VanillaEntityClientMovementPolicy::FishingHook
            ));
            let identity_data = identity.data;
            if policy == VanillaEntityClientMovementPolicy::ExperienceOrb
                && animations.age_ticks == 0
            {
                continue;
            }
            let position = world_fact_position_by_id
                .get(&identity.id)
                .copied()
                .unwrap_or(transform.position);
            let Some(bounds) = vanilla_bounds_for_entity_data(
                entity_type_id,
                identity_data,
                &metadata.data_values,
                &attributes.attributes,
                Some(*animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            let previous_supporting_block = *supporting_block;
            let fluid = fluid_by_id.get(&identity.id).copied().unwrap_or_default();
            let mut pre_move_delta = match policy {
                VanillaEntityClientMovementPolicy::FallingBlock
                | VanillaEntityClientMovementPolicy::PrimedTnt => {
                    nonliving_gravity_movement(metadata, transform.delta_movement, 0.04)
                }
                VanillaEntityClientMovementPolicy::Item
                | VanillaEntityClientMovementPolicy::ExperienceOrb => {
                    apply_entity_fluid_currents(transform.delta_movement, fluid, lava_current_scale)
                }
                VanillaEntityClientMovementPolicy::FishingHook => transform.delta_movement,
                _ => unreachable!("filtered client movement collision policy"),
            };
            let mut movement_skipped = false;
            let mut experience_orb_update = None;
            let mut fishing_hook_update = None;
            let mut fishing_hook_entity_hit = None;
            let mut fishing_hook_collision_delta = None;
            match policy {
                VanillaEntityClientMovementPolicy::Item => {
                    if !item_entity_has_stack(entity_type_id, &metadata.data_values) {
                        continue;
                    }
                    if fluid.water_height > 0.1 {
                        pre_move_delta.x *= f64::from(0.99_f32);
                        if pre_move_delta.y < f64::from(0.06_f32) {
                            pre_move_delta.y += f64::from(0.0005_f32);
                        }
                        pre_move_delta.z *= f64::from(0.99_f32);
                    } else if fluid.lava_height > 0.1 {
                        pre_move_delta.x *= f64::from(0.95_f32);
                        if pre_move_delta.y < f64::from(0.06_f32) {
                            pre_move_delta.y += f64::from(0.0005_f32);
                        }
                        pre_move_delta.z *= f64::from(0.95_f32);
                    } else {
                        pre_move_delta = nonliving_gravity_movement(metadata, pre_move_delta, 0.04);
                    }
                    let next_tick = animations.age_ticks.saturating_add(1);
                    movement_skipped = transform.on_ground.unwrap_or(false)
                        && horizontal_length_squared(pre_move_delta) <= f64::from(1.0e-5_f32)
                        && (i64::from(next_tick) + i64::from(identity.id)).rem_euclid(4) != 0;
                }
                VanillaEntityClientMovementPolicy::ExperienceOrb => {
                    let Some(orb_movement) = orb_movement.copied() else {
                        continue;
                    };
                    if fluid.eye_in_water {
                        pre_move_delta.x *= f64::from(0.99_f32);
                        pre_move_delta.y =
                            (pre_move_delta.y + f64::from(0.0005_f32)).min(f64::from(0.06_f32));
                        pre_move_delta.z *= f64::from(0.99_f32);
                    } else if collision_free_by_id
                        .get(&identity.id)
                        .copied()
                        .unwrap_or(true)
                    {
                        pre_move_delta.y -= 0.03;
                    }
                    let mut random_seed = orb_movement.random_seed;
                    if fluid.block_fluid_kind == Some(crate::TerrainFluidKind::Lava) {
                        pre_move_delta = EntityVec3 {
                            x: f64::from(
                                (java_random_next_float(&mut random_seed)
                                    - java_random_next_float(&mut random_seed))
                                    * 0.2,
                            ),
                            y: f64::from(0.2_f32),
                            z: f64::from(
                                (java_random_next_float(&mut random_seed)
                                    - java_random_next_float(&mut random_seed))
                                    * 0.2,
                            ),
                        };
                    }
                    let player_targets = experience_orb_player_targets.get_or_insert_with(|| {
                        self.experience_orb_player_targets(spectator_player_uuids)
                    });
                    let target = select_experience_orb_player_target(
                        position,
                        orb_movement.following_player_uuid,
                        player_targets,
                    );
                    if let Some(target) = target {
                        let delta = subtract(target.attraction_position, position);
                        let length_squared = length_squared(delta);
                        let power = 1.0 - length_squared.sqrt() / 8.0;
                        pre_move_delta =
                            add(pre_move_delta, scale(normalize(delta), power * power * 0.1));
                    }
                    experience_orb_update = Some(EntityExperienceOrbMovementUpdate {
                        following_player_uuid: target.map(|target| target.uuid),
                        random_seed,
                    });
                }
                VanillaEntityClientMovementPolicy::FishingHook => {
                    let Some(movement) = fishing_hook_movement.copied() else {
                        continue;
                    };
                    let update = self.prepare_fishing_hook_movement(
                        identity,
                        position,
                        pre_move_delta,
                        transform.on_ground.unwrap_or(false),
                        fluid,
                        movement,
                        game_time,
                        bounds,
                        animations.age_ticks.saturating_add(1),
                        &fishing_hook_targets,
                        spectator_player_uuids,
                    );
                    pre_move_delta = update.0;
                    movement_skipped = update.1;
                    fishing_hook_update = Some(update.2);
                    fishing_hook_entity_hit = update.3;
                    fishing_hook_collision_delta = update.4;
                }
                VanillaEntityClientMovementPolicy::FallingBlock
                | VanillaEntityClientMovementPolicy::PrimedTnt => {}
                _ => unreachable!("filtered client movement collision policy"),
            }
            let requested_movement = if movement_skipped {
                EntityVec3::default()
            } else {
                pre_move_delta
            };
            inputs.push(EntityClientMovementCollisionInput {
                entity_id: identity.id,
                entity_type_id,
                position,
                pre_move_delta,
                requested_movement,
                bounds,
                on_ground: transform.on_ground.unwrap_or(false),
                previous_supporting_block,
                movement_skipped,
                experience_orb_update,
                fishing_hook_update,
                fishing_hook_entity_hit,
                fishing_hook_collision_delta,
            });
        }
        for input in &mut inputs {
            if input.movement_skipped {
                continue;
            }
            let Some(entity) = self.record_mut(input.entity_id) else {
                continue;
            };
            let Some(multiplier) = entity
                .supporting_block
                .block_effects
                .consume_stuck_speed_multiplier()
            else {
                continue;
            };
            input.requested_movement.x *= multiplier.x;
            input.requested_movement.y *= multiplier.y;
            input.requested_movement.z *= multiplier.z;
            // `Entity.move` clears the canonical velocity when it consumes
            // `stuckSpeedMultiplier`; subclass post-move damping therefore
            // starts from zero.
            input.pre_move_delta = EntityVec3::default();
        }
        inputs
    }

    fn experience_orb_player_targets<S: BuildHasher>(
        &self,
        spectator_player_uuids: &HashSet<uuid::Uuid, S>,
    ) -> Vec<EntityExperienceOrbPlayerTarget> {
        let mut targets = Vec::new();
        // `EntityGetter.getNearestPlayer` resolves equal distances by player
        // list order, which insertion order matches.
        for entity in self.records() {
            let (identity, metadata, attributes, animations, transform) = (
                &entity.identity,
                &entity.metadata,
                &entity.attributes,
                &entity.animations,
                &entity.transform,
            );
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
                continue;
            }
            if spectator_player_uuids.contains(&identity.uuid) {
                continue;
            }
            let Some(living_accessors) =
                vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                    identity.entity_type_id,
                )
            else {
                continue;
            };
            let is_dead_or_dying = entity_data_float(
                &metadata.data_values,
                living_accessors.health_data_id,
                living_accessors.health_default,
            ) <= 0.0;
            let Some(eye_height) = vanilla_eye_height_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(*animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            targets.push(EntityExperienceOrbPlayerTarget {
                entity_id: identity.id,
                uuid: identity.uuid,
                position: transform.position,
                attraction_position: EntityVec3 {
                    x: transform.position.x,
                    y: transform.position.y + f64::from(eye_height) * 0.5,
                    z: transform.position.z,
                },
                is_dead_or_dying,
            });
        }
        targets
    }

    fn projectile_targets<S: BuildHasher>(
        &self,
        spectator_player_uuids: &HashSet<uuid::Uuid, S>,
    ) -> Vec<EntityProjectileTarget> {
        let mut targets = Vec::new();
        // Dragon parts follow every tracked entity, in their dragon's order.
        let mut dragon_parts = Vec::new();
        for entity in self.records() {
            let (identity, transform, metadata, attributes, animations) = (
                &entity.identity,
                &entity.transform,
                &entity.metadata,
                &entity.attributes,
                &entity.animations,
            );
            let Some(bounds) = super::content_facts::bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(*animations),
                metadata.painting_variant_projection.as_ref(),
                &self.content_registry,
            ) else {
                continue;
            };
            let alive = if super::content_facts::entity_metadata_is_living(
                identity.entity_type_id,
                &self.content_registry,
            ) {
                super::content_facts::entity_living_accessors(
                    identity.entity_type_id,
                    &self.content_registry,
                )
                .is_some_and(|accessors| {
                    entity_data_float(
                        &metadata.data_values,
                        accessors.health_data_id,
                        accessors.health_default,
                    ) > 0.0
                })
            } else {
                true
            };
            let pickable = (vanilla_entity_metadata_type_is_pickable(identity.entity_type_id)
                || super::content_facts::mod_entity_spec(
                    identity.entity_type_id,
                    &self.content_registry,
                )
                .is_some())
                // Interaction is intentionally pickable for cursor selection
                // but overrides `canBeHitByProjectile` to false.
                && identity.entity_type_id != super::VANILLA_ENTITY_TYPE_INTERACTION_ID
                && !(identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                    && spectator_player_uuids.contains(&identity.uuid))
                && (identity.entity_type_id != VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
                    || vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
                        identity.entity_type_id,
                    )
                    .is_some_and(|accessors| {
                        entity_data_byte(
                            &metadata.data_values,
                            accessors.client_flags_data_id,
                            accessors.client_flags_default,
                        ) & accessors.marker_mask
                            == 0
                    }));
            targets.push(EntityProjectileTarget {
                entity_id: identity.id,
                entity_type_id: identity.entity_type_id,
                uuid: identity.uuid,
                root_vehicle_id: self.fishing_hook_root_vehicle_id(identity.id),
                position: transform.position,
                bounds,
                alive,
                pickable,
                deflects_projectiles: vanilla_entity_type_deflects_projectiles(
                    identity.entity_type_id,
                ),
                redirectable_projectile: vanilla_entity_type_is_redirectable_projectile(
                    identity.entity_type_id,
                ),
                hurt_client: vanilla_entity_metadata_type_accepts_projectile_hurt_client(
                    identity.entity_type_id,
                ),
                trident_hurt_client: vanilla_entity_metadata_type_accepts_projectile_hurt_client(
                    identity.entity_type_id,
                ),
                hurt_client_rejects_ender_dragon_owner: identity.entity_type_id
                    == VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
                incarnation: identity.incarnation,
            });
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
                let dragon_animation = animations.ender_dragon_kind().and_then(|k| k.state);
                dragon_parts.extend(
                    super::dragon::ender_dragon_part_pick_targets_at_partial_tick(
                        identity.id,
                        *transform,
                        dragon_animation,
                        1.0,
                    )
                    .into_iter()
                    .map(|part| EntityProjectileTarget {
                        entity_id: part.entity_id,
                        entity_type_id: VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
                        uuid: identity.uuid,
                        root_vehicle_id: part.entity_id,
                        position: part.position,
                        bounds: part.bounds,
                        alive: true,
                        // `EnderDragonPart.isPickable` is unconditionally true;
                        // the parent EnderDragon itself remains non-pickable.
                        pickable: true,
                        deflects_projectiles: false,
                        redirectable_projectile: false,
                        hurt_client: false,
                        trident_hurt_client: false,
                        hurt_client_rejects_ender_dragon_owner: false,
                        incarnation: identity.incarnation,
                    }),
                );
            }
        }
        targets.append(&mut dragon_parts);
        targets
    }

    /// Immutable collision contract for the WorldStore terrain pass. Entity
    /// ownership, metadata, dimensions, and target identity stay in the ECS;
    /// the caller supplies collision-shape and scoreboard facts before the
    /// mutable arrow movement slot consumes the resolved results.
    pub(crate) fn abstract_arrow_collision_batch<S: BuildHasher>(
        &self,
        spectator_player_uuids: &HashSet<uuid::Uuid, S>,
    ) -> EntityAbstractArrowCollisionBatch {
        let outside_owner_by_id =
            self.projectile_outside_owner_collision_by_id(spectator_player_uuids);
        let mut sources = Vec::new();
        for entity in self.records() {
            let Some((lifecycle, runtime)) = entity.family.projectile_and_arrow() else {
                continue;
            };
            let (identity, transform, metadata, attributes, animations) = (
                &entity.identity,
                &entity.transform,
                &entity.metadata,
                &entity.attributes,
                &entity.animations,
            );
            let Some(accessors) = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
                identity.entity_type_id,
            ) else {
                continue;
            };
            let flags = entity_data_byte(
                &metadata.data_values,
                accessors.flags_data_id,
                accessors.flags_default,
            );
            if flags & accessors.no_physics_mask != 0 {
                continue;
            }
            let Some(bounds) = vanilla_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(*animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            let owner_root_vehicle_id = lifecycle
                .owner_uuid
                .and_then(|owner_uuid| self.protocol_id_by_uuid.get(&owner_uuid).copied())
                .map(|owner_id| self.fishing_hook_root_vehicle_id(owner_id));
            let owner_entity_type_id = lifecycle
                .owner_uuid
                .and_then(|owner_uuid| self.protocol_id_for_uuid(owner_uuid))
                .and_then(|owner_id| self.entity_type_id(owner_id));
            sources.push(EntityAbstractArrowCollisionSource {
                entity_id: identity.id,
                entity_type_id: identity.entity_type_id,
                position: transform.position,
                movement: transform.delta_movement,
                bounds,
                on_ground: transform.on_ground.unwrap_or(false),
                tick_count: animations.age_ticks.saturating_add(1),
                owner_uuid: lifecycle.owner_uuid,
                owner_entity_type_id,
                owner_root_vehicle_id,
                left_owner: lifecycle.left_owner
                    || outside_owner_by_id
                        .get(&identity.id)
                        .copied()
                        .unwrap_or(true),
                trident_dealt_damage: lifecycle.trident_dealt_damage,
                piercing_ignore_entity_ids: runtime.piercing_ignore_entity_ids.clone(),
            });
        }
        let targets = if !sources.is_empty() {
            self.projectile_targets(spectator_player_uuids)
        } else {
            Default::default()
        };
        EntityAbstractArrowCollisionBatch { sources, targets }
    }

    fn fishing_hook_root_vehicle_id(&self, entity_id: i32) -> i32 {
        let mut root = entity_id;
        let mut visited = Vec::new();
        while !visited.contains(&root) {
            visited.push(root);
            let Some(vehicle_id) = self.record(root).and_then(|entity| entity.mount.vehicle_id)
            else {
                break;
            };
            root = vehicle_id;
        }
        root
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_fishing_hook_movement<S: BuildHasher>(
        &self,
        identity: &EntityIdentity,
        position: EntityVec3,
        mut delta_movement: EntityVec3,
        on_ground: bool,
        fluid: crate::client::EntityFluidContactState,
        movement: EntityFishingHookMovement,
        _game_time: i64,
        bounds: EntityPickBoundsState,
        tick_count: u32,
        targets: &[EntityProjectileTarget],
        spectator_player_uuids: &HashSet<uuid::Uuid, S>,
    ) -> (
        EntityVec3,
        bool,
        EntityFishingHookMovementUpdate,
        Option<EntityFishingHookProjectileHit>,
        Option<EntityVec3>,
    ) {
        let mut update = EntityFishingHookMovementUpdate {
            movement,
            position_override: None,
            clear_hooked_entity_data: false,
        };
        let owner_is_current_player = movement.owner_uuid.is_some_and(|owner_uuid| {
            self.protocol_id_for_uuid(owner_uuid)
                .and_then(|id| self.entity_type_id(id))
                == Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
        });
        if !owner_is_current_player {
            // Vanilla discards here. Authoritative removal is outside this
            // slice, so freeze the removed instance rather than advancing it.
            return (delta_movement, true, update, None, None);
        }

        let owner_root_vehicle_id = movement
            .owner_uuid
            .and_then(|uuid| self.protocol_id_by_uuid.get(&uuid).copied())
            .map(|id| self.fishing_hook_root_vehicle_id(id));
        if !movement.left_owner {
            let source_bounds = projectile_world_bounds(position, bounds)
                .expand_towards(delta_movement)
                .inflate(1.0);
            update.movement.left_owner = owner_root_vehicle_id.is_none_or(|owner_root| {
                targets.iter().all(|target| {
                    !target.pickable
                        || target.root_vehicle_id != owner_root
                        || !source_bounds
                            .intersects(projectile_world_bounds(target.position, target.bounds))
                })
            });
        }

        let hooked_target = movement
            .hooked_entity_uuid
            .and_then(|uuid| self.fishing_hook_target(uuid, spectator_player_uuids));
        let has_hooked_entity_reference = movement.hooked_entity_uuid.is_some();
        let is_in_water = fluid.block_fluid_kind == Some(crate::TerrainFluidKind::Water)
            && fluid.block_fluid_height > 0.0;
        match movement.state {
            EntityFishingHookState::Flying => {
                if has_hooked_entity_reference {
                    delta_movement = EntityVec3::default();
                    update.movement.state = EntityFishingHookState::HookedInEntity;
                    return (delta_movement, true, update, None, None);
                }
                if is_in_water {
                    delta_movement.x *= 0.3;
                    delta_movement.y *= 0.2;
                    delta_movement.z *= 0.3;
                    update.movement.state = EntityFishingHookState::Bobbing;
                    return (delta_movement, true, update, None, None);
                }
            }
            EntityFishingHookState::HookedInEntity => {
                if let Some((target_position, target_height)) = hooked_target {
                    update.position_override = Some(EntityVec3 {
                        x: target_position.x,
                        y: target_position.y + target_height * 0.8,
                        z: target_position.z,
                    });
                } else {
                    update.movement.hooked_entity_uuid = None;
                    update.movement.state = EntityFishingHookState::Flying;
                    update.clear_hooked_entity_data = true;
                }
                return (delta_movement, true, update, None, None);
            }
            EntityFishingHookState::Bobbing => {
                let mut force =
                    position.y + delta_movement.y - position.y.floor() - fluid.block_fluid_height;
                if force.abs() < 0.01 {
                    force += force.signum() * 0.1;
                }
                let random = java_random_next_float(&mut update.movement.random_seed);
                delta_movement = EntityVec3 {
                    x: delta_movement.x * 0.9,
                    y: delta_movement.y - force * f64::from(random) * 0.2,
                    z: delta_movement.z * 0.9,
                };
                if is_in_water && movement.biting {
                    let first = java_random_next_float(&mut update.movement.random_seed);
                    let second = java_random_next_float(&mut update.movement.random_seed);
                    delta_movement.y -= 0.1 * f64::from(first) * f64::from(second);
                }
            }
        }
        let applies_gravity = !is_in_water && !on_ground && movement.hooked_entity_uuid.is_none();
        let collision_delta =
            (update.movement.state == EntityFishingHookState::Flying).then_some(delta_movement);
        let entity_hit = collision_delta.and_then(|collision_delta| {
            fishing_hook_entity_hit(
                identity.id,
                position,
                collision_delta,
                owner_root_vehicle_id,
                update.movement.left_owner,
                tick_count,
                movement.last_deflected_by,
                targets,
            )
        });
        if applies_gravity {
            delta_movement.y -= 0.03;
        }
        (delta_movement, false, update, entity_hit, collision_delta)
    }

    fn fishing_hook_target<S: BuildHasher>(
        &self,
        uuid: uuid::Uuid,
        spectator_player_uuids: &HashSet<uuid::Uuid, S>,
    ) -> Option<(EntityVec3, f64)> {
        let id = self.protocol_id_by_uuid.get(&uuid).copied()?;
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let transform = &entity.transform;
        let metadata = &entity.metadata;
        let attributes = &entity.attributes;
        let animations = &entity.animations;
        if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && spectator_player_uuids.contains(&identity.uuid)
        {
            return None;
        }
        if vanilla_entity_metadata_type_is_living(identity.entity_type_id) {
            let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                identity.entity_type_id,
            )?;
            if entity_data_float(
                &metadata.data_values,
                accessors.health_data_id,
                accessors.health_default,
            ) <= 0.0
            {
                return None;
            }
        }
        let bounds = vanilla_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(*animations),
            metadata.painting_variant_projection.as_ref(),
        )?;
        Some((transform.position, f64::from(bounds.max[1] - bounds.min[1])))
    }

    /// Advances client-owned projectile and nonliving movement in the frozen
    /// entity schedule. Incoming packets remain authoritative, while the
    /// external client's AbstractArrow hit simulation may locally discard its
    /// own projectile exactly as Vanilla does. Vehicle, item, orb, and
    /// fishing-hook movement stay with their dedicated collision/authority
    /// owners.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn advance_direct_client_movement(
        &mut self,
        scheduled: &[i32],
        fluid_facts_by_id: &FxHashMap<i32, super::store::EntityClientTickFluidFacts>,
        collisions_by_id: &FxHashMap<i32, EntityClientMovementCollisionResult>,
        abstract_arrow_collisions_by_id: &FxHashMap<i32, EntityAbstractArrowCollisionFacts>,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
        local_player_id: Option<i32>,
        mut local_player_pose: Option<&mut crate::LocalPlayerPoseState>,
    ) -> super::store::EntityBlockEffectWork {
        let record_block_effect_movements = block_effect_world.has_loaded_chunks();
        let mut block_effect_work = super::store::EntityBlockEffectWork::default();
        let mut effects = Vec::new();
        for &entity_id in scheduled {
            let Some(entity_type_id) = self.entity_type_id(entity_id) else {
                continue;
            };
            let Some(policy) = vanilla_entity_metadata_type_client_movement_policy(entity_type_id)
            else {
                continue;
            };
            match policy {
                VanillaEntityClientMovementPolicy::ThrowableProjectile
                | VanillaEntityClientMovementPolicy::ThrownExperienceBottle
                | VanillaEntityClientMovementPolicy::ThrownPotion => {
                    let Some(entity) = self.record_mut(entity_id) else {
                        continue;
                    };
                    let metadata = &entity.metadata;
                    let transform = &mut entity.transform;
                    advance_throwable_projectile(
                        entity_id,
                        metadata,
                        transform,
                        policy,
                        fluid_facts_by_id
                            .get(&entity_id)
                            .is_some_and(|facts| facts.in_water),
                        &mut effects,
                    );
                }
                VanillaEntityClientMovementPolicy::HurtingProjectile
                | VanillaEntityClientMovementPolicy::WindCharge
                | VanillaEntityClientMovementPolicy::DragonFireball
                | VanillaEntityClientMovementPolicy::WitherSkull => {
                    let Some(entity) = self.record_mut(entity_id) else {
                        continue;
                    };
                    let metadata = &entity.metadata;
                    let hurting_projectile = entity.family.hurting_projectile();
                    let transform = &mut entity.transform;
                    advance_hurting_projectile(
                        entity_id,
                        entity_type_id,
                        metadata,
                        hurting_projectile,
                        transform,
                        policy,
                        fluid_facts_by_id
                            .get(&entity_id)
                            .is_some_and(|facts| facts.in_water),
                        &mut effects,
                    );
                }
                VanillaEntityClientMovementPolicy::EyeOfEnder
                | VanillaEntityClientMovementPolicy::ShulkerBullet => {
                    let Some(entity) = self.record_mut(entity_id) else {
                        continue;
                    };
                    let transform = &mut entity.transform;
                    match policy {
                        VanillaEntityClientMovementPolicy::EyeOfEnder => {
                            // `EyeOfEnder.tick`: `super.tick()` has already run through
                            // the projectile lifecycle pass, then the client keeps the
                            // server-provided velocity and advances one position step.
                            advance_position(transform);
                            effects.push(EntityClientMovementEffectState::Particle(
                                EntityClientMovementParticleState::EyeOfEnder(
                                    EyeOfEnderTrailParticleState {
                                        entity_id,
                                        origin: EntityVec3 {
                                            x: transform.position.x
                                                - transform.delta_movement.x * 0.25,
                                            y: transform.position.y
                                                - transform.delta_movement.y * 0.25,
                                            z: transform.position.z
                                                - transform.delta_movement.z * 0.25,
                                        },
                                        delta_movement: transform.delta_movement,
                                        in_water: fluid_facts_by_id
                                            .get(&entity_id)
                                            .is_some_and(|facts| facts.in_water),
                                    },
                                ),
                            ));
                        }
                        VanillaEntityClientMovementPolicy::ShulkerBullet => {
                            // `ShulkerBullet.tick` changes its homing vector only on
                            // the server. The client integrates that synchronized
                            // vector and eases rotation toward it by one half.
                            advance_position(transform);
                            rotate_towards_movement(transform, 0.5);
                            effects.push(EntityClientMovementEffectState::Particle(
                                EntityClientMovementParticleState::ShulkerBullet(
                                    ShulkerBulletTrailParticleState {
                                        entity_id,
                                        position: EntityVec3 {
                                            x: transform.position.x - transform.delta_movement.x,
                                            y: transform.position.y - transform.delta_movement.y
                                                + 0.15,
                                            z: transform.position.z - transform.delta_movement.z,
                                        },
                                    },
                                ),
                            ));
                        }
                        _ => unreachable!("narrow direct movement policy match"),
                    }
                }
                VanillaEntityClientMovementPolicy::LlamaSpit => {
                    let Some(entity) = self.record_mut(entity_id) else {
                        continue;
                    };
                    let metadata = &entity.metadata;
                    let transform = &mut entity.transform;
                    advance_llama_spit(metadata, transform);
                }
                VanillaEntityClientMovementPolicy::AbstractArrow => {
                    let Some(accessors) =
                        vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
                            entity_type_id,
                        )
                    else {
                        continue;
                    };
                    let return_target = (entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID)
                        .then(|| self.trident_return_target(entity_id))
                        .flatten();
                    let in_water = fluid_facts_by_id
                        .get(&entity_id)
                        .is_some_and(|facts| facts.in_water);
                    // `ThrownTrident.tick` runs its loyalty return before
                    // `super.tick()`; the arrow head, movement, effects, and
                    // inertia all follow it within the same tick.
                    let affected_by_blocks = {
                        let Some(entity) = self.record_mut(entity_id) else {
                            continue;
                        };
                        let metadata = &mut entity.metadata;
                        let mut projectile = entity.family.projectile_lifecycle_mut();
                        let transform = &mut entity.transform;
                        if entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
                            if let (Some(projectile), Some(return_target)) =
                                (projectile.as_deref_mut(), return_target)
                            {
                                advance_returning_trident(
                                    metadata,
                                    projectile,
                                    transform,
                                    accessors,
                                    return_target,
                                );
                            }
                        }
                        // `isAffectedByBlocks` reads the client `noPhysics`
                        // field that only the trident return above ever sets.
                        !projectile
                            .as_deref()
                            .is_some_and(|projectile| projectile.arrow_no_physics)
                    };
                    self.apply_arrow_water_or_rain_fire_clear(
                        entity_id,
                        in_water,
                        block_effect_world,
                    );
                    let step = {
                        let Some(entity) = self.record_mut(entity_id) else {
                            continue;
                        };
                        let metadata = &mut entity.metadata;
                        let animations = &mut entity.animations;
                        let transform = &mut entity.transform;
                        advance_abstract_arrow(
                            entity_id,
                            entity_type_id,
                            metadata,
                            animations,
                            transform,
                            accessors,
                            in_water,
                            abstract_arrow_collisions_by_id.get(&entity_id),
                            &mut effects,
                        )
                    };
                    let ArrowMoveStep {
                        first_effect,
                        collision,
                        finish,
                        trident_initial_containment_grounded,
                    } = step;
                    if entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID
                        && trident_initial_containment_grounded
                    {
                        if let Some(projectile) = self
                            .record_mut(entity_id)
                            .and_then(|entity| entity.family.projectile_lifecycle_mut())
                        {
                            projectile.trident_in_ground_time =
                                projectile.trident_in_ground_time.saturating_add(1);
                        }
                    }
                    let mut apply_segment =
                        |store: &mut EntityStore, segment: EntityBlockEffectMovement| {
                            let started_at = bbb_client_clock::now();
                            let (effect_entities, effect_blocks, effect_callbacks) = store
                                .apply_arrow_block_effect_tick(
                                    entity_id,
                                    segment,
                                    super::EntityBlockEffectBaseTickReset::Skip,
                                    collisions_by_id,
                                    block_effect_world,
                                );
                            block_effect_work.duration += started_at.elapsed();
                            block_effect_work.entities += effect_entities;
                            block_effect_work.blocks =
                                block_effect_work.blocks.saturating_add(effect_blocks);
                            block_effect_work.callbacks =
                                block_effect_work.callbacks.saturating_add(effect_callbacks);
                        };
                    if affected_by_blocks {
                        apply_segment(self, first_effect);
                    }
                    if let Some(collision) = collision.as_ref() {
                        let owner_look_angle = self
                            .record(entity_id)
                            .and_then(|entity| entity.family.projectile_lifecycle())
                            .and_then(|projectile| projectile.owner_uuid)
                            .and_then(|owner_uuid| self.protocol_id_for_uuid(owner_uuid))
                            .and_then(|owner_id| self.record(owner_id))
                            .map(|owner| {
                                calculate_view_vector(owner.transform.x_rot, owner.transform.y_rot)
                            });
                        let collision_step = {
                            let Some(entity) = self.record_mut(entity_id) else {
                                continue;
                            };
                            let Some((projectile, runtime)) =
                                entity.family.projectile_and_arrow_mut()
                            else {
                                continue;
                            };
                            let metadata = &mut entity.metadata;
                            let transform = &mut entity.transform;
                            advance_abstract_arrow_collision_after_first_effect(
                                entity_type_id,
                                metadata,
                                projectile,
                                runtime,
                                transform,
                                accessors,
                                collision,
                                owner_look_angle,
                            )
                        };
                        for redirect in &collision_step.redirected_targets {
                            let Some(target) = self.record_mut(redirect.entity_id) else {
                                continue;
                            };
                            if target.identity.incarnation == redirect.incarnation {
                                target.transform.delta_movement = redirect.movement;
                            }
                        }
                        if affected_by_blocks {
                            if let Some(second_effect) = collision_step.second_effect {
                                apply_segment(self, second_effect);
                            }
                        }
                        if collision_step.reaches_endpoint
                            && (collision.block_hit.is_some() || collision.world_border_hit)
                        {
                            if let Some(entity) = self.record_mut(entity_id) {
                                if let Some((projectile, runtime)) =
                                    entity.family.projectile_and_arrow_mut()
                                {
                                    apply_abstract_arrow_endpoint_hit(
                                        entity_type_id,
                                        &mut entity.metadata,
                                        projectile,
                                        runtime,
                                        &mut entity.animations,
                                        &mut entity.transform,
                                        accessors,
                                        collision,
                                    );
                                }
                            }
                        }
                        if collision_step.discarded {
                            self.pending_local_discards.push(entity_id);
                        }
                    }
                    if let Some(finish) = finish {
                        if let Some(entity) = self.record_mut(entity_id) {
                            finish_abstract_arrow(
                                &entity.metadata,
                                &mut entity.transform,
                                finish,
                                accessors,
                            );
                        }
                        self.apply_arrow_base_tick_reset(entity_id);
                    }
                    // Vanilla `Arrow.tick` (`Arrow.java:74-106`) and `SpectralArrow.tick`
                    // (`SpectralArrow.java:35-40`): client-only tipped potion & spectral trail particles.
                    if matches!(
                        entity_type_id,
                        VANILLA_ENTITY_TYPE_ARROW_ID | VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID
                    ) {
                        let mut arrow_particles: [Option<crate::particles::ParticleSpawn>; 2] =
                            [None, None];
                        let mut arrow_particle_len = 0usize;
                        if let Some(entity) = self.record_mut(entity_id) {
                            let in_ground = entity_data_bool(
                                &entity.metadata.data_values,
                                accessors.in_ground_data_id,
                                accessors.in_ground_default,
                            );
                            let in_ground_time = if let Some((_, runtime)) =
                                entity.family.projectile_and_arrow_mut()
                            {
                                if in_ground {
                                    runtime.in_ground_time =
                                        runtime.in_ground_time.saturating_add(1);
                                } else {
                                    runtime.in_ground_time = 0;
                                }
                                runtime.in_ground_time
                            } else {
                                0
                            };
                            let pos = entity.transform.position;
                            if entity_type_id == VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID {
                                if !in_ground {
                                    arrow_particles[0] = Some(
                                        crate::particles::ParticleSpawn::new(
                                            crate::particles::ParticleKind::EFFECT,
                                            [pos.x, pos.y, pos.z],
                                            [0.0, 0.0, 0.0],
                                        )
                                        .with_params(crate::particles::ParticleParams::Spell {
                                            color: [1.0, 1.0, 1.0, 1.0],
                                            power: 1.0,
                                        }),
                                    );
                                    arrow_particle_len = 1;
                                }
                            } else if let Some(arrow_acc) =
                                super::schema::vanilla_entity_metadata_arrow_accessors_for_entity_type(
                                    entity_type_id,
                                )
                            {
                                let effect_color = entity_data_int(
                                    &entity.metadata.data_values,
                                    arrow_acc.effect_color_data_id,
                                    arrow_acc.effect_color_default,
                                );
                                if effect_color != -1 {
                                    let count = if in_ground {
                                        usize::from(in_ground_time % 5 == 0)
                                    } else {
                                        2
                                    };
                                    if count > 0 {
                                        let rgba = if ((effect_color as u32) >> 24) == 0 {
                                            crate::particles::rgb_color_f32(effect_color)
                                        } else {
                                            crate::particles::argb_color_f32(effect_color)
                                        };
                                        let width = 0.5_f64;
                                        let height = 0.5_f64;
                                        for slot in arrow_particles.iter_mut().take(count) {
                                            let rx = f64::from(
                                                entity
                                                    .animations
                                                    .next_local_interaction_random_float(entity_id),
                                            );
                                            let ry = f64::from(
                                                entity
                                                    .animations
                                                    .next_local_interaction_random_float(entity_id),
                                            );
                                            let rz = f64::from(
                                                entity
                                                    .animations
                                                    .next_local_interaction_random_float(entity_id),
                                            );
                                            *slot = Some(
                                                crate::particles::ParticleSpawn::new(
                                                    crate::particles::ParticleKind::ENTITY_EFFECT,
                                                    [
                                                        pos.x + width * (2.0 * rx - 1.0) * 0.5,
                                                        pos.y + height * ry,
                                                        pos.z + width * (2.0 * rz - 1.0) * 0.5,
                                                    ],
                                                    [0.0, 0.0, 0.0],
                                                )
                                                .with_params(
                                                    crate::particles::ParticleParams::Color(rgba),
                                                ),
                                            );
                                            arrow_particle_len += 1;
                                        }
                                    }
                                }
                            }
                        }
                        for spawn in arrow_particles
                            .into_iter()
                            .take(arrow_particle_len)
                            .flatten()
                        {
                            self.pending_client_particle_spawns.push(spawn);
                        }
                    }
                }
                VanillaEntityClientMovementPolicy::Item => {
                    let Some(collision) = collisions_by_id.get(&entity_id).copied() else {
                        continue;
                    };
                    let Some(entity) = self.record_mut(entity_id) else {
                        continue;
                    };
                    advance_item_entity(
                        &mut entity.transform,
                        &mut entity.supporting_block,
                        collision,
                    );
                }
                VanillaEntityClientMovementPolicy::ExperienceOrb => {
                    let Some(collision) = collisions_by_id.get(&entity_id).copied() else {
                        continue;
                    };
                    let Some(entity) = self.record_mut(entity_id) else {
                        continue;
                    };
                    let Some(movement) = entity.family.experience_orb_mut() else {
                        continue;
                    };
                    advance_experience_orb(
                        movement,
                        &mut entity.transform,
                        &mut entity.supporting_block,
                        collision,
                    );
                }
                VanillaEntityClientMovementPolicy::FishingHook => {
                    let Some(collision) = collisions_by_id.get(&entity_id).copied() else {
                        continue;
                    };
                    let Some(entity) = self.record_mut(entity_id) else {
                        continue;
                    };
                    let Some(movement) = entity.family.fishing_hook_mut() else {
                        continue;
                    };
                    advance_fishing_hook(
                        movement,
                        &mut entity.metadata,
                        &mut entity.transform,
                        &mut entity.supporting_block,
                        collision,
                    );
                }
                VanillaEntityClientMovementPolicy::FallingBlock => {
                    let Some(entity) = self.record_mut(entity_id) else {
                        continue;
                    };
                    let metadata = &entity.metadata;
                    let transform = &mut entity.transform;
                    advance_falling_block(
                        metadata,
                        transform,
                        collisions_by_id.get(&entity_id).copied(),
                    );
                }
                VanillaEntityClientMovementPolicy::PrimedTnt => {
                    let Some(accessors) =
                        vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
                            entity_type_id,
                        )
                    else {
                        continue;
                    };
                    let Some(entity) = self.record_mut(entity_id) else {
                        continue;
                    };
                    let metadata = &mut entity.metadata;
                    let transform = &mut entity.transform;
                    advance_primed_tnt(
                        metadata,
                        transform,
                        accessors.fuse_data_id,
                        accessors.fuse_default,
                        collisions_by_id.get(&entity_id).copied(),
                    );
                }
                VanillaEntityClientMovementPolicy::FireworkRocket => {
                    self.advance_firework_rocket(
                        entity_id,
                        entity_type_id,
                        local_player_id,
                        local_player_pose.as_deref_mut(),
                        &mut effects,
                    );
                }
                VanillaEntityClientMovementPolicy::ServerSynchronized
                | VanillaEntityClientMovementPolicy::Vehicle => {}
            }
            if record_block_effect_movements
                && vanilla_entity_metadata_type_block_effect_tick_policy(entity_type_id)
                    .is_some_and(|policy| policy != super::VanillaEntityBlockEffectTickPolicy::None)
            {
                let Some(collision) = collisions_by_id.get(&entity_id).copied() else {
                    continue;
                };
                if collision.movement_skipped {
                    continue;
                }
                let Some(entity) = self.record_mut(entity_id) else {
                    continue;
                };
                let position = entity.transform.position;
                let from = EntityVec3 {
                    x: position.x - collision.clipped_movement.x,
                    y: position.y - collision.clipped_movement.y,
                    z: position.z - collision.clipped_movement.z,
                };
                let moved = length_squared(collision.clipped_movement) > 1.0e-7
                    || length_squared(collision.requested_movement)
                        - length_squared(collision.clipped_movement)
                        < 1.0e-7;
                if moved {
                    if let Some(facts) = entity.family.collision_facts_mut() {
                        facts.block_effect_movements.record_movement(
                            EntityBlockEffectMovement::clipped(
                                from,
                                position,
                                collision.requested_movement,
                            ),
                        );
                    }
                }
            }
        }
        self.pending_entity_client_movement_effects.extend(effects);
        block_effect_work
    }

    fn trident_return_target(
        &self,
        trident_id: i32,
    ) -> Option<EntityClientMovementTridentReturnTarget> {
        let trident = self.record(trident_id)?;
        let trident_metadata = &trident.metadata;
        let projectile = trident.family.projectile_lifecycle()?;
        let trident_accessors = vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        )?;
        let loyalty = entity_data_byte(
            &trident_metadata.data_values,
            trident_accessors.loyalty_data_id,
            trident_accessors.loyalty_default,
        );
        let arrow_accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        )?;
        let no_physics = entity_data_byte(
            &trident_metadata.data_values,
            arrow_accessors.flags_data_id,
            arrow_accessors.flags_default,
        ) & arrow_accessors.no_physics_mask
            != 0;
        if loyalty <= 0 || (!projectile.trident_dealt_damage && !no_physics) {
            return None;
        }
        let owner_uuid = projectile.owner_uuid?;
        let owner_id = self.protocol_id_by_uuid.get(&owner_uuid).copied()?;
        let owner = self.record(owner_id)?;
        let identity = &owner.identity;
        let metadata = &owner.metadata;
        let attributes = &owner.attributes;
        let animations = &owner.animations;
        let transform = &owner.transform;
        if !vanilla_entity_metadata_type_is_living(identity.entity_type_id) {
            return None;
        }
        let living_accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            identity.entity_type_id,
        )?;
        if entity_data_float(
            &metadata.data_values,
            living_accessors.health_data_id,
            living_accessors.health_default,
        ) <= 0.0
        {
            return None;
        }
        let eye_height = vanilla_eye_height_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(*animations),
            metadata.painting_variant_projection.as_ref(),
        )?;
        Some(EntityClientMovementTridentReturnTarget {
            eye_position: EntityVec3 {
                x: transform.position.x,
                y: transform.position.y + f64::from(eye_height),
                z: transform.position.z,
            },
            loyalty,
        })
    }

    fn advance_firework_rocket(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        local_player_id: Option<i32>,
        local_player_pose: Option<&mut crate::LocalPlayerPoseState>,
        effects: &mut Vec<EntityClientMovementEffectState>,
    ) {
        let Some(accessors) =
            vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(entity_type_id)
        else {
            return;
        };
        let Some(entity) = self.record_mut(entity_id) else {
            return;
        };
        let (attached_to_target, shot_at_angle) = {
            let metadata = &entity.metadata;
            (
                entity_data_optional_unsigned_int(
                    &metadata.data_values,
                    accessors.attached_to_target_data_id,
                    accessors.attached_to_target_default,
                ),
                entity_data_bool(
                    &metadata.data_values,
                    accessors.shot_at_angle_data_id,
                    accessors.shot_at_angle_default,
                ),
            )
        };

        let attachment = attached_to_target.and_then(|target_id| {
            let pose = (local_player_id == Some(target_id))
                .then_some(local_player_pose)
                .flatten();
            self.firework_attachment_target(target_id, pose)
        });
        let Some(entity) = self.record_mut(entity_id) else {
            return;
        };
        let transform = &mut entity.transform;
        if let Some((position, delta_movement)) = attachment {
            transform.position = position;
            transform.delta_movement = delta_movement;
        } else if attached_to_target.is_none() {
            if !shot_at_angle {
                // The exact `horizontalCollision ? 1.0 : 1.15` branch remains
                // false in this free-flight owner; collision facts belong to
                // the dedicated movement/collision stage.
                transform.delta_movement.x *= 1.15;
                transform.delta_movement.y += 0.04;
                transform.delta_movement.z *= 1.15;
            }
            advance_position(transform);
        }
        update_projectile_rotation(transform);

        effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::FireworkRocket(FireworkRocketTrailParticleState {
                entity_id,
                position: transform.position,
                delta_movement: transform.delta_movement,
            }),
        ));
    }

    fn firework_attachment_target(
        &mut self,
        target_id: i32,
        local_player_pose: Option<&mut crate::LocalPlayerPoseState>,
    ) -> Option<(EntityVec3, EntityVec3)> {
        let target = self.record(target_id)?;
        let (target_type_id, is_fall_flying) = {
            let identity = &target.identity;
            if !vanilla_entity_metadata_type_is_living(identity.entity_type_id) {
                return None;
            }
            let metadata = &target.metadata;
            let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
            let shared_flags = metadata
                .data_values
                .iter()
                .find(|value| value.data_id == entity_accessors.shared_flags_data_id)
                .and_then(|value| match value.value {
                    EntityDataValueKind::Byte(value) => Some(value),
                    _ => None,
                })
                .unwrap_or(entity_accessors.shared_flags_default);
            (
                identity.entity_type_id,
                shared_flags as u8 & entity_accessors.fall_flying_mask as u8 != 0,
            )
        };
        let item_arm_right = if target_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            let item_in = |slot| {
                target
                    .equipment
                    .equipment
                    .iter()
                    .find(|update| update.slot == slot)
                    .and_then(|update| update.item.item_id)
                    == Some(VANILLA_ITEM_FIREWORK_ROCKET_ID)
            };
            let main_firework = item_in(ProtocolEquipmentSlot::MainHand);
            let off_firework = item_in(ProtocolEquipmentSlot::OffHand);
            let main_arm_left = self.main_arm_left(target_id).unwrap_or(false);
            if off_firework && !main_firework {
                main_arm_left
            } else {
                !main_arm_left
            }
        } else {
            false
        };

        let target_transform = &mut self.record_mut(target_id)?.transform;
        let hand_angle = if is_fall_flying {
            let (x_rot, y_rot, movement) = match local_player_pose.as_deref() {
                Some(pose) => (
                    pose.x_rot,
                    pose.y_rot,
                    EntityVec3 {
                        x: pose.delta_movement.x,
                        y: pose.delta_movement.y,
                        z: pose.delta_movement.z,
                    },
                ),
                None => (
                    target_transform.x_rot,
                    target_transform.y_rot,
                    target_transform.delta_movement,
                ),
            };
            let look = calculate_view_vector(x_rot, y_rot);
            let boosted = EntityVec3 {
                x: movement.x + look.x * 0.1 + (look.x * 1.5 - movement.x) * 0.5,
                y: movement.y + look.y * 0.1 + (look.y * 1.5 - movement.y) * 0.5,
                z: movement.z + look.z * 0.1 + (look.z * 1.5 - movement.z) * 0.5,
            };
            target_transform.delta_movement = boosted;
            // Vanilla `FireworkRocketEntity.tick` (`FireworkRocketEntity.java:163-172`):
            // write the elytra boost back to the authoritative local player pose
            // so `sync_local_player_entity` does not overwrite it on the next frame.
            if let Some(pose) = local_player_pose {
                pose.delta_movement = bbb_protocol::packets::Vec3d {
                    x: boosted.x,
                    y: boosted.y,
                    z: boosted.z,
                };
            }
            if target_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                scale(
                    calculate_view_vector(0.0, y_rot + if item_arm_right { 80.0 } else { -80.0 }),
                    0.5,
                )
            } else {
                EntityVec3::default()
            }
        } else {
            EntityVec3::default()
        };
        Some((
            add(target_transform.position, hand_angle),
            target_transform.delta_movement,
        ))
    }
}

#[cfg(test)]
mod tests;
