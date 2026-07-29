use std::collections::{HashMap, HashSet};

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type_deflects_projectiles, vanilla_entity_type_is_redirectable_projectile,
    VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    VANILLA_ENTITY_TYPE_END_CRYSTAL_ID, VANILLA_ENTITY_TYPE_TRIDENT_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_FIREWORK_ROCKET_ID;
use bbb_protocol::packets::EntityDataValueKind;
use bbb_protocol::packets::EquipmentSlot as ProtocolEquipmentSlot;
use hecs::Entity;

use super::animations::{
    java_random_next_float, java_random_next_int_bound, java_random_seed, spawn_random_bits,
};
use super::components::{
    EntityAbstractArrowRuntime, EntityAttributes, EntityClientAnimations,
    EntityClientMovementCollisionFacts, EntityEquipment, EntityExperienceOrbMovement,
    EntityExperienceOrbPlayerTargetFacts, EntityFishingHookMovement, EntityFishingHookState,
    EntityHurtingProjectile, EntityIdentity, EntityIncarnation, EntityInstanceReference,
    EntityMetadata, EntityProjectileLifecycle, EntitySupportingBlock, EntityTransform,
};
use super::dimensions::{vanilla_bounds_for_entity_data, vanilla_eye_height_for_entity_data};
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
    EntityClientMovementSoundKind, EntityClientMovementSoundState, EntityPickBoundsState,
    EntityStore, EntityVec3, EyeOfEnderTrailParticleState, FireworkRocketTrailParticleState,
    ShulkerBulletTrailParticleState, VANILLA_ENTITY_TYPE_PLAYER_ID,
};

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
    Entity {
        entity_id: i32,
        incarnation: EntityIncarnation,
        position: EntityVec3,
    },
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
    pub(crate) fn set_item_hurt_client(
        &mut self,
        entity_id: i32,
        arrow_hurt_client: bool,
        trident_hurt_client: bool,
    ) {
        if let Some(target) = self
            .targets
            .iter_mut()
            .find(|target| target.entity_id == entity_id)
        {
            target.hurt_client = arrow_hurt_client;
            target.trident_hurt_client = trident_hurt_client;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityFishingHookProjectileHit {
    pub(crate) scale: f64,
    pub(crate) reverse_deflection: Option<(i32, EntityIncarnation, EntityVec3)>,
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

fn fishing_hook_entity_hit(
    source_id: i32,
    from: EntityVec3,
    delta: EntityVec3,
    owner_root_vehicle_id: Option<i32>,
    left_owner: bool,
    tick_count: u32,
    last_deflected_by: Option<EntityIncarnation>,
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
                    .then_some((target.entity_id, target.incarnation, target.position))
                    .filter(|_| last_deflected_by != Some(target.incarnation)),
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
) -> HashMap<i32, EntityAbstractArrowCollisionFacts> {
    let mut facts_by_id = HashMap::new();
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

fn fishing_hook_segment_box_entry_scale(
    from: EntityVec3,
    to: EntityVec3,
    bounds: ProjectileAabb,
) -> Option<f64> {
    let delta = subtract(to, from);
    let mut nearest = 1.0;
    let mut hit = false;
    for (axis_delta, from_a, min_a, max_a, other) in [
        (
            delta.x,
            from.x,
            bounds.min.x,
            bounds.max.x,
            [
                (from.y, delta.y, bounds.min.y, bounds.max.y),
                (from.z, delta.z, bounds.min.z, bounds.max.z),
            ],
        ),
        (
            delta.y,
            from.y,
            bounds.min.y,
            bounds.max.y,
            [
                (from.z, delta.z, bounds.min.z, bounds.max.z),
                (from.x, delta.x, bounds.min.x, bounds.max.x),
            ],
        ),
        (
            delta.z,
            from.z,
            bounds.min.z,
            bounds.max.z,
            [
                (from.x, delta.x, bounds.min.x, bounds.max.x),
                (from.y, delta.y, bounds.min.y, bounds.max.y),
            ],
        ),
    ] {
        let plane = if axis_delta > 1.0e-7 {
            min_a
        } else if axis_delta < -1.0e-7 {
            max_a
        } else {
            continue;
        };
        let scale = (plane - from_a) / axis_delta;
        if 0.0 < scale
            && scale < nearest
            && other.iter().all(|(origin, movement, min, max)| {
                let point = origin + scale * movement;
                min - 1.0e-7 < point && point < max + 1.0e-7
            })
        {
            nearest = scale;
            hit = true;
        }
    }
    hit.then_some(nearest)
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
        hooked_entity_incarnation: None,
        biting: false,
        left_owner: false,
        last_deflected_by: None,
        random_seed: java_random_seed(spawn_random_bits(uuid, 0x6669_7368_5f72_6e67) as i64),
        synchronized_random_seed: java_random_seed(
            spawn_random_bits(uuid, 0x6669_7368_5f73_796e) as i64
        ),
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
        let Some(entity) = self.by_protocol_id.get(&entity_id).copied() else {
            return;
        };
        let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
            return;
        };
        let Some(accessors) = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            identity.entity_type_id,
        ) else {
            return;
        };
        drop(identity);
        let Ok(mut metadata) = self.ecs.get::<&mut EntityMetadata>(entity) else {
            return;
        };
        if entity_data_bool(
            &metadata.data_values,
            accessors.in_ground_data_id,
            accessors.in_ground_default,
        ) {
            set_entity_data_bool(&mut metadata, accessors.in_ground_data_id, false);
        }
    }

    pub(crate) fn client_movement_collision_inputs(
        &mut self,
        fluid_by_id: &HashMap<i32, crate::client::EntityFluidContactState>,
        world_fact_position_by_id: &HashMap<i32, EntityVec3>,
        collision_free_by_id: &HashMap<i32, bool>,
        spectator_player_uuids: &HashSet<uuid::Uuid>,
        lava_current_scale: f64,
        game_time: i64,
    ) -> Vec<EntityClientMovementCollisionInput> {
        let mut experience_orb_player_targets = None;
        let fishing_hook_needs_targets = {
            let mut hooks = self.ecs.query::<&EntityFishingHookMovement>();
            hooks.iter().any(|(_, movement)| {
                movement.owner_uuid.is_some_and(|owner_uuid| {
                    self.protocol_id_by_uuid
                        .get(&owner_uuid)
                        .and_then(|id| self.by_protocol_id.get(id))
                        .and_then(|entity| self.ecs.get::<&EntityIdentity>(*entity).ok())
                        .is_some_and(|owner| owner.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID)
                })
            })
        };
        let fishing_hook_targets = fishing_hook_needs_targets
            .then(|| self.projectile_targets(spectator_player_uuids))
            .unwrap_or_default();
        let mut inputs = Vec::new();
        let mut facts_query = self.ecs.query::<(
            &EntityClientMovementCollisionFacts,
            &EntityIdentity,
            &EntityMetadata,
            &EntityTransform,
            &EntitySupportingBlock,
            &EntityAttributes,
            &EntityClientAnimations,
            Option<&EntityExperienceOrbMovement>,
            Option<&EntityFishingHookMovement>,
        )>();
        for (
            _,
            (
                _,
                identity,
                metadata,
                transform,
                supporting_block,
                attributes,
                animations,
                orb_movement,
                fishing_hook_movement,
            ),
        ) in facts_query.iter()
        {
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
                && animations.animations.age_ticks == 0
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
                Some(animations.animations),
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
                    let next_tick = animations.animations.age_ticks.saturating_add(1);
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
                        animations.animations.age_ticks.saturating_add(1),
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
        drop(facts_query);
        for input in &mut inputs {
            if input.movement_skipped {
                continue;
            }
            let Some(&entity) = self.by_protocol_id.get(&input.entity_id) else {
                continue;
            };
            let Ok(mut supporting_block) = self.ecs.get::<&mut EntitySupportingBlock>(entity)
            else {
                continue;
            };
            let Some(multiplier) = supporting_block
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

    fn experience_orb_player_targets(
        &self,
        spectator_player_uuids: &HashSet<uuid::Uuid>,
    ) -> Vec<EntityExperienceOrbPlayerTarget> {
        let mut targets = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityExperienceOrbPlayerTargetFacts,
            &EntityIdentity,
            &EntityMetadata,
            &EntityAttributes,
            &EntityClientAnimations,
            &EntityTransform,
        )>();
        for (_, (_, identity, metadata, attributes, animations, transform)) in query.iter() {
            debug_assert_eq!(identity.entity_type_id, VANILLA_ENTITY_TYPE_PLAYER_ID);
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
                Some(animations.animations),
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
        drop(query);
        // `EntityGetter.getNearestPlayer` resolves equal distances by player
        // list order. Hecs archetype order may change after removal, so restore
        // the canonical spawn/re-add order before the strict-distance fold.
        targets.sort_by_key(|target| {
            self.order
                .iter()
                .position(|id| *id == target.entity_id)
                .unwrap_or(usize::MAX)
        });
        targets
    }

    fn projectile_targets(
        &self,
        spectator_player_uuids: &HashSet<uuid::Uuid>,
    ) -> Vec<EntityProjectileTarget> {
        let mut targets = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityAttributes,
            &EntityClientAnimations,
        )>();
        for (_, (identity, transform, metadata, attributes, animations)) in query.iter() {
            let Some(bounds) = vanilla_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            let alive = if vanilla_entity_metadata_type_is_living(identity.entity_type_id) {
                vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                    identity.entity_type_id,
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
            let pickable = vanilla_entity_metadata_type_is_pickable(identity.entity_type_id)
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
                let dragon_animation = animations.animations.ender_dragon;
                targets.extend(
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
        targets.sort_by_key(|target| {
            self.order
                .iter()
                .position(|id| *id == target.entity_id)
                .unwrap_or(usize::MAX)
        });
        targets
    }

    /// Immutable collision contract for the WorldStore terrain pass. Entity
    /// ownership, metadata, dimensions, and target identity stay in the ECS;
    /// the caller supplies collision-shape and scoreboard facts before the
    /// mutable arrow movement slot consumes the resolved results.
    pub(crate) fn abstract_arrow_collision_batch(
        &self,
        spectator_player_uuids: &HashSet<uuid::Uuid>,
    ) -> EntityAbstractArrowCollisionBatch {
        let outside_owner_by_id =
            self.projectile_outside_owner_collision_by_id(spectator_player_uuids);
        let mut sources = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityAttributes,
            &EntityClientAnimations,
            &EntityProjectileLifecycle,
            &EntityAbstractArrowRuntime,
        )>();
        for (_, (identity, transform, metadata, attributes, animations, lifecycle, runtime)) in
            query.iter()
        {
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
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            let owner_root_vehicle_id = lifecycle
                .owner_uuid
                .and_then(|owner_uuid| self.protocol_id_by_uuid.get(&owner_uuid).copied())
                .map(|owner_id| self.fishing_hook_root_vehicle_id(owner_id));
            let owner_entity_type_id = lifecycle.owner_uuid.and_then(|owner_uuid| {
                self.protocol_id_by_uuid
                    .get(&owner_uuid)
                    .and_then(|owner_id| self.by_protocol_id.get(owner_id))
                    .and_then(|owner| self.ecs.get::<&EntityIdentity>(*owner).ok())
                    .map(|owner| owner.entity_type_id)
            });
            sources.push(EntityAbstractArrowCollisionSource {
                entity_id: identity.id,
                entity_type_id: identity.entity_type_id,
                position: transform.position,
                movement: transform.delta_movement,
                bounds,
                on_ground: transform.on_ground.unwrap_or(false),
                tick_count: animations.animations.age_ticks.saturating_add(1),
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
        drop(query);
        let targets = (!sources.is_empty())
            .then(|| self.projectile_targets(spectator_player_uuids))
            .unwrap_or_default();
        EntityAbstractArrowCollisionBatch { sources, targets }
    }

    fn fishing_hook_root_vehicle_id(&self, entity_id: i32) -> i32 {
        let mut root = entity_id;
        let mut visited = Vec::new();
        while !visited.contains(&root) {
            visited.push(root);
            let Some(vehicle_id) = self
                .by_protocol_id
                .get(&root)
                .and_then(|entity| {
                    self.ecs
                        .get::<&super::components::EntityMount>(*entity)
                        .ok()
                })
                .and_then(|mount| mount.vehicle_id)
            else {
                break;
            };
            root = vehicle_id;
        }
        root
    }

    fn prepare_fishing_hook_movement(
        &self,
        identity: &EntityIdentity,
        position: EntityVec3,
        mut delta_movement: EntityVec3,
        on_ground: bool,
        fluid: crate::client::EntityFluidContactState,
        mut movement: EntityFishingHookMovement,
        game_time: i64,
        bounds: EntityPickBoundsState,
        tick_count: u32,
        targets: &[EntityProjectileTarget],
        spectator_player_uuids: &HashSet<uuid::Uuid>,
    ) -> (
        EntityVec3,
        bool,
        EntityFishingHookMovementUpdate,
        Option<EntityFishingHookProjectileHit>,
        Option<EntityVec3>,
    ) {
        movement.synchronized_random_seed =
            java_random_seed(((identity.uuid.as_u128() as u64) ^ game_time as u64) as i64);
        let mut update = EntityFishingHookMovementUpdate {
            movement,
            position_override: None,
            clear_hooked_entity_data: false,
        };
        let owner_is_current_player = movement.owner_uuid.is_some_and(|owner_uuid| {
            self.protocol_id_by_uuid
                .get(&owner_uuid)
                .and_then(|id| self.by_protocol_id.get(id))
                .and_then(|entity| self.ecs.get::<&EntityIdentity>(*entity).ok())
                .is_some_and(|owner| owner.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID)
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
            .zip(movement.hooked_entity_incarnation)
            .and_then(|(uuid, incarnation)| {
                self.fishing_hook_target(uuid, incarnation, spectator_player_uuids)
            });
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
                    update.movement.hooked_entity_incarnation = None;
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
                    let first =
                        java_random_next_float(&mut update.movement.synchronized_random_seed);
                    let second =
                        java_random_next_float(&mut update.movement.synchronized_random_seed);
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

    fn fishing_hook_target(
        &self,
        uuid: uuid::Uuid,
        incarnation: EntityIncarnation,
        spectator_player_uuids: &HashSet<uuid::Uuid>,
    ) -> Option<(EntityVec3, f64)> {
        let id = self.protocol_id_by_uuid.get(&uuid).copied()?;
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityTransform,
                &EntityMetadata,
                &EntityAttributes,
                &EntityClientAnimations,
            )>(entity)
            .ok()?;
        let (identity, transform, metadata, attributes, animations) = query.get()?;
        if identity.incarnation != incarnation {
            return None;
        }
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
            Some(animations.animations),
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
    pub(super) fn advance_direct_client_movement(
        &mut self,
        scheduled: &[Entity],
        fluid_facts_by_id: &HashMap<i32, super::store::EntityClientTickFluidFacts>,
        collisions_by_id: &HashMap<i32, EntityClientMovementCollisionResult>,
        abstract_arrow_collisions_by_id: &HashMap<i32, EntityAbstractArrowCollisionFacts>,
        block_effect_world: &mut crate::client::EntityBlockEffectWorld<'_>,
    ) -> super::store::EntityBlockEffectWork {
        let record_block_effect_movements = block_effect_world.has_loaded_chunks();
        let mut block_effect_work = super::store::EntityBlockEffectWork::default();
        let mut effects = Vec::new();
        for &entity in scheduled {
            let (entity_id, entity_type_id) = {
                let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                    continue;
                };
                (identity.id, identity.entity_type_id)
            };
            let Some(policy) = vanilla_entity_metadata_type_client_movement_policy(entity_type_id)
            else {
                continue;
            };
            match policy {
                VanillaEntityClientMovementPolicy::ThrowableProjectile
                | VanillaEntityClientMovementPolicy::ThrownExperienceBottle
                | VanillaEntityClientMovementPolicy::ThrownPotion => {
                    let Ok((metadata, transform)) = self
                        .ecs
                        .query_one_mut::<(&EntityMetadata, &mut EntityTransform)>(entity)
                    else {
                        continue;
                    };
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
                    let Ok((metadata, hurting_projectile, transform)) =
                        self.ecs.query_one_mut::<(
                            &EntityMetadata,
                            Option<&EntityHurtingProjectile>,
                            &mut EntityTransform,
                        )>(entity)
                    else {
                        continue;
                    };
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
                    let Ok(mut transform) = self.ecs.get::<&mut EntityTransform>(entity) else {
                        continue;
                    };
                    match policy {
                        VanillaEntityClientMovementPolicy::EyeOfEnder => {
                            // `EyeOfEnder.tick`: `super.tick()` has already run through
                            // the projectile lifecycle pass, then the client keeps the
                            // server-provided velocity and advances one position step.
                            advance_position(&mut transform);
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
                            advance_position(&mut transform);
                            rotate_towards_movement(&mut transform, 0.5);
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
                    let Ok((metadata, transform)) = self
                        .ecs
                        .query_one_mut::<(&EntityMetadata, &mut EntityTransform)>(entity)
                    else {
                        continue;
                    };
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
                        .then(|| self.trident_return_target(entity))
                        .flatten();
                    let in_water = fluid_facts_by_id
                        .get(&entity_id)
                        .is_some_and(|facts| facts.in_water);
                    // `ThrownTrident.tick` runs its loyalty return before
                    // `super.tick()`; the arrow head, movement, effects, and
                    // inertia all follow it within the same tick.
                    let affected_by_blocks = {
                        let Ok((metadata, mut projectile, transform)) =
                            self.ecs.query_one_mut::<(
                                &mut EntityMetadata,
                                Option<&mut EntityProjectileLifecycle>,
                                &mut EntityTransform,
                            )>(entity)
                        else {
                            continue;
                        };
                        if entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
                            if let (Some(projectile), Some(return_target)) =
                                (projectile.as_deref_mut(), return_target)
                            {
                                advance_returning_trident(
                                    entity_id,
                                    metadata,
                                    projectile,
                                    transform,
                                    accessors,
                                    return_target,
                                    &mut effects,
                                );
                            }
                        }
                        // `isAffectedByBlocks` reads the client `noPhysics`
                        // field that only the trident return above ever sets.
                        !projectile
                            .as_deref()
                            .is_some_and(|projectile| projectile.arrow_no_physics)
                    };
                    self.apply_arrow_water_or_rain_fire_clear(entity, in_water, block_effect_world);
                    let step = {
                        let Ok((metadata, animations, transform)) =
                            self.ecs.query_one_mut::<(
                                &mut EntityMetadata,
                                &mut EntityClientAnimations,
                                &mut EntityTransform,
                            )>(entity)
                        else {
                            continue;
                        };
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
                        if let Ok(mut projectile) =
                            self.ecs.get::<&mut EntityProjectileLifecycle>(entity)
                        {
                            projectile.trident_in_ground_time =
                                projectile.trident_in_ground_time.saturating_add(1);
                        }
                    }
                    let mut apply_segment =
                        |store: &mut EntityStore, segment: EntityBlockEffectMovement| {
                            let started_at = std::time::Instant::now();
                            let (effect_entities, effect_blocks, effect_callbacks) = store
                                .apply_arrow_block_effect_tick(
                                    entity,
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
                            .ecs
                            .get::<&EntityProjectileLifecycle>(entity)
                            .ok()
                            .and_then(|projectile| projectile.owner_uuid)
                            .and_then(|owner_uuid| self.protocol_id_by_uuid.get(&owner_uuid))
                            .and_then(|owner_id| self.by_protocol_id.get(owner_id))
                            .and_then(|owner| self.ecs.get::<&EntityTransform>(*owner).ok())
                            .map(|owner| calculate_view_vector(owner.x_rot, owner.y_rot));
                        let collision_step = {
                            let Ok((metadata, projectile, runtime, transform)) =
                                self.ecs.query_one_mut::<(
                                    &mut EntityMetadata,
                                    &mut EntityProjectileLifecycle,
                                    &mut EntityAbstractArrowRuntime,
                                    &mut EntityTransform,
                                )>(entity)
                            else {
                                continue;
                            };
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
                            let Some(target) =
                                self.by_protocol_id.get(&redirect.entity_id).copied()
                            else {
                                continue;
                            };
                            let Ok((identity, transform)) =
                                self.ecs
                                    .query_one_mut::<(&EntityIdentity, &mut EntityTransform)>(
                                        target,
                                    )
                            else {
                                continue;
                            };
                            if identity.incarnation == redirect.incarnation {
                                transform.delta_movement = redirect.movement;
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
                            if let Ok((metadata, projectile, runtime, animations, transform)) =
                                self.ecs.query_one_mut::<(
                                    &mut EntityMetadata,
                                    &mut EntityProjectileLifecycle,
                                    &mut EntityAbstractArrowRuntime,
                                    &mut EntityClientAnimations,
                                    &mut EntityTransform,
                                )>(entity)
                            {
                                apply_abstract_arrow_endpoint_hit(
                                    entity_type_id,
                                    metadata,
                                    projectile,
                                    runtime,
                                    animations,
                                    transform,
                                    accessors,
                                    collision,
                                );
                            }
                        }
                        if collision_step.discarded {
                            self.pending_local_discards.push(entity_id);
                        }
                    }
                    if let Some(finish) = finish {
                        if let Ok((metadata, transform)) = self
                            .ecs
                            .query_one_mut::<(&EntityMetadata, &mut EntityTransform)>(entity)
                        {
                            finish_abstract_arrow(metadata, transform, finish, accessors);
                        }
                        self.apply_arrow_base_tick_reset(entity);
                    }
                }
                VanillaEntityClientMovementPolicy::Item => {
                    let Some(collision) = collisions_by_id.get(&entity_id).copied() else {
                        continue;
                    };
                    let Ok((transform, supporting_block)) = self
                        .ecs
                        .query_one_mut::<(&mut EntityTransform, &mut EntitySupportingBlock)>(
                            entity,
                        )
                    else {
                        continue;
                    };
                    advance_item_entity(transform, supporting_block, collision);
                }
                VanillaEntityClientMovementPolicy::ExperienceOrb => {
                    let Some(collision) = collisions_by_id.get(&entity_id).copied() else {
                        continue;
                    };
                    let Ok((movement, transform, supporting_block)) =
                        self.ecs.query_one_mut::<(
                            &mut EntityExperienceOrbMovement,
                            &mut EntityTransform,
                            &mut EntitySupportingBlock,
                        )>(entity)
                    else {
                        continue;
                    };
                    advance_experience_orb(movement, transform, supporting_block, collision);
                }
                VanillaEntityClientMovementPolicy::FishingHook => {
                    let Some(collision) = collisions_by_id.get(&entity_id).copied() else {
                        continue;
                    };
                    let Ok((movement, metadata, transform, supporting_block)) =
                        self.ecs.query_one_mut::<(
                            &mut EntityFishingHookMovement,
                            &mut EntityMetadata,
                            &mut EntityTransform,
                            &mut EntitySupportingBlock,
                        )>(entity)
                    else {
                        continue;
                    };
                    advance_fishing_hook(
                        movement,
                        metadata,
                        transform,
                        supporting_block,
                        collision,
                        &mut effects,
                    );
                }
                VanillaEntityClientMovementPolicy::FallingBlock => {
                    let Ok((metadata, transform)) = self
                        .ecs
                        .query_one_mut::<(&EntityMetadata, &mut EntityTransform)>(entity)
                    else {
                        continue;
                    };
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
                    let Ok((metadata, transform)) = self
                        .ecs
                        .query_one_mut::<(&mut EntityMetadata, &mut EntityTransform)>(entity)
                    else {
                        continue;
                    };
                    advance_primed_tnt(
                        metadata,
                        transform,
                        accessors.fuse_data_id,
                        accessors.fuse_default,
                        collisions_by_id.get(&entity_id).copied(),
                    );
                }
                VanillaEntityClientMovementPolicy::FireworkRocket => {
                    self.advance_firework_rocket(entity, entity_id, entity_type_id, &mut effects);
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
                let Some(position) = self
                    .ecs
                    .get::<&EntityTransform>(entity)
                    .ok()
                    .map(|transform| transform.position)
                else {
                    continue;
                };
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
                    if let Ok(mut facts) = self
                        .ecs
                        .get::<&mut EntityClientMovementCollisionFacts>(entity)
                    {
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
        trident: Entity,
    ) -> Option<EntityClientMovementTridentReturnTarget> {
        let Ok(mut trident_query) = self
            .ecs
            .query_one::<(&EntityMetadata, &EntityProjectileLifecycle)>(trident)
        else {
            return None;
        };
        let (trident_metadata, projectile) = trident_query.get()?;
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
        drop(trident_query);
        let owner_id = self.protocol_id_by_uuid.get(&owner_uuid).copied()?;
        let owner = self.by_protocol_id.get(&owner_id).copied()?;
        let Ok(mut query) = self.ecs.query_one::<(
            &EntityIdentity,
            &EntityMetadata,
            &EntityAttributes,
            &EntityClientAnimations,
            &EntityTransform,
        )>(owner) else {
            return None;
        };
        let (identity, metadata, attributes, animations, transform) = query.get()?;
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
            Some(animations.animations),
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
        entity: Entity,
        entity_id: i32,
        entity_type_id: i32,
        effects: &mut Vec<EntityClientMovementEffectState>,
    ) {
        let Some(accessors) =
            vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(entity_type_id)
        else {
            return;
        };
        let (attached_to_target, shot_at_angle, play_launch_sound) = {
            let Ok((metadata, animations)) = self
                .ecs
                .query_one_mut::<(&EntityMetadata, &EntityClientAnimations)>(entity)
            else {
                return;
            };
            let silent = vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
                entity_data_bool(
                    &metadata.data_values,
                    accessors.silent_data_id,
                    accessors.silent_default,
                )
            });
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
                animations.animations.age_ticks == 0 && !silent,
            )
        };

        let attachment =
            attached_to_target.and_then(|target_id| self.firework_attachment_target(target_id));
        let Ok(mut transform) = self.ecs.get::<&mut EntityTransform>(entity) else {
            return;
        };
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
            advance_position(&mut transform);
        }
        update_projectile_rotation(&mut transform);
        if play_launch_sound {
            effects.push(EntityClientMovementEffectState::Sound(
                EntityClientMovementSoundState {
                    entity_id,
                    kind: EntityClientMovementSoundKind::FireworkRocketLaunch,
                    position: transform.position,
                },
            ));
        }
        effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::FireworkRocket(FireworkRocketTrailParticleState {
                entity_id,
                position: transform.position,
                delta_movement: transform.delta_movement,
            }),
        ));
    }

    fn firework_attachment_target(&mut self, target_id: i32) -> Option<(EntityVec3, EntityVec3)> {
        let target = self.by_protocol_id.get(&target_id).copied()?;
        let (target_type_id, is_fall_flying) = {
            let identity = self.ecs.get::<&EntityIdentity>(target).ok()?;
            if !vanilla_entity_metadata_type_is_living(identity.entity_type_id) {
                return None;
            }
            let metadata = self.ecs.get::<&EntityMetadata>(target).ok()?;
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
            let (main_firework, off_firework) = self
                .ecs
                .get::<&EntityEquipment>(target)
                .ok()
                .map(|equipment| {
                    let item_in = |slot| {
                        equipment
                            .equipment
                            .iter()
                            .find(|update| update.slot == slot)
                            .and_then(|update| update.item.item_id)
                            == Some(VANILLA_ITEM_FIREWORK_ROCKET_ID)
                    };
                    (
                        item_in(ProtocolEquipmentSlot::MainHand),
                        item_in(ProtocolEquipmentSlot::OffHand),
                    )
                })
                .unwrap_or_default();
            let main_arm_left = self.main_arm_left(target_id).unwrap_or(false);
            if off_firework && !main_firework {
                main_arm_left
            } else {
                !main_arm_left
            }
        } else {
            false
        };

        let mut target_transform = self.ecs.get::<&mut EntityTransform>(target).ok()?;
        let hand_angle = if is_fall_flying {
            let look = calculate_view_vector(target_transform.x_rot, target_transform.y_rot);
            let movement = target_transform.delta_movement;
            target_transform.delta_movement = EntityVec3 {
                x: movement.x + look.x * 0.1 + (look.x * 1.5 - movement.x) * 0.5,
                y: movement.y + look.y * 0.1 + (look.y * 1.5 - movement.y) * 0.5,
                z: movement.z + look.z * 0.1 + (look.z * 1.5 - movement.z) * 0.5,
            };
            if target_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                scale(
                    calculate_view_vector(
                        0.0,
                        target_transform.y_rot + if item_arm_right { 80.0 } else { -80.0 },
                    ),
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

fn advance_throwable_projectile(
    entity_id: i32,
    metadata: &EntityMetadata,
    transform: &mut EntityTransform,
    policy: VanillaEntityClientMovementPolicy,
    in_water: bool,
    effects: &mut Vec<EntityClientMovementEffectState>,
) {
    let no_gravity = vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
        entity_data_bool(
            &metadata.data_values,
            accessors.no_gravity_data_id,
            accessors.no_gravity_default,
        )
    });
    if !no_gravity {
        transform.delta_movement.y -= throwable_gravity(policy);
    }

    let movement_before_inertia = transform.delta_movement;
    if in_water {
        append_bubbles(
            entity_id,
            transform.position,
            movement_before_inertia,
            effects,
        );
    }
    let inertia = f64::from(if in_water { 0.8_f32 } else { 0.99_f32 });
    transform.delta_movement = scale(movement_before_inertia, inertia);
    advance_position(transform);
    update_projectile_rotation(transform);
}

fn advance_hurting_projectile(
    entity_id: i32,
    entity_type_id: i32,
    metadata: &EntityMetadata,
    hurting_projectile: Option<&EntityHurtingProjectile>,
    transform: &mut EntityTransform,
    policy: VanillaEntityClientMovementPolicy,
    in_water: bool,
    effects: &mut Vec<EntityClientMovementEffectState>,
) {
    let movement_before_inertia = transform.delta_movement;
    if in_water {
        append_bubbles(
            entity_id,
            transform.position,
            movement_before_inertia,
            effects,
        );
    }
    let acceleration_power = hurting_projectile
        .map(|projectile| projectile.acceleration_power)
        .unwrap_or_else(|| {
            if matches!(policy, VanillaEntityClientMovementPolicy::WindCharge) {
                0.0
            } else {
                0.1
            }
        });
    let acceleration = scale(normalize(movement_before_inertia), acceleration_power);
    let inertia = hurting_projectile_inertia(policy, entity_type_id, metadata, in_water);
    transform.delta_movement = scale(add(movement_before_inertia, acceleration), inertia);
    rotate_towards_movement(transform, 0.2);
    advance_position(transform);

    let trail = match policy {
        VanillaEntityClientMovementPolicy::WindCharge => None,
        VanillaEntityClientMovementPolicy::DragonFireball => {
            Some(EntityClientMovementSimpleParticleKind::DragonBreath)
        }
        _ => Some(EntityClientMovementSimpleParticleKind::Smoke),
    };
    if let Some(kind) = trail {
        effects.push(EntityClientMovementEffectState::Particle(
            EntityClientMovementParticleState::Simple(EntityClientMovementSimpleParticleState {
                entity_id,
                kind,
                position: EntityVec3 {
                    y: transform.position.y + 0.5,
                    ..transform.position
                },
                velocity: EntityVec3::default(),
            }),
        ));
    }
}

fn advance_llama_spit(metadata: &EntityMetadata, transform: &mut EntityTransform) {
    let movement = transform.delta_movement;
    advance_position(transform);
    update_projectile_rotation(transform);
    transform.delta_movement = scale(movement, f64::from(0.99_f32));
    let no_gravity = vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
        entity_data_bool(
            &metadata.data_values,
            accessors.no_gravity_data_id,
            accessors.no_gravity_default,
        )
    });
    if !no_gravity {
        transform.delta_movement.y -= 0.06;
    }
}

fn advance_returning_trident(
    entity_id: i32,
    metadata: &mut EntityMetadata,
    projectile: &mut EntityProjectileLifecycle,
    transform: &mut EntityTransform,
    arrow_accessors: super::schema::VanillaAbstractArrowMetadataAccessors,
    target: EntityClientMovementTridentReturnTarget,
    effects: &mut Vec<EntityClientMovementEffectState>,
) {
    let loyalty = target.loyalty;
    let flags = entity_data_byte(
        &metadata.data_values,
        arrow_accessors.flags_data_id,
        arrow_accessors.flags_default,
    );
    if loyalty <= 0
        || (!projectile.trident_dealt_damage && flags & arrow_accessors.no_physics_mask == 0)
    {
        return;
    }

    // Vanilla `setNoPhysics(true)` writes the synced flag and the private
    // `Entity.noPhysics` field together; `isAffectedByBlocks` reads the field.
    set_entity_data_byte(
        metadata,
        arrow_accessors.flags_data_id,
        flags | arrow_accessors.no_physics_mask,
    );
    projectile.arrow_no_physics = true;
    let to_owner = subtract(target.eye_position, transform.position);
    transform.position.y += to_owner.y * 0.015 * f64::from(loyalty);
    transform.delta_movement = add(
        scale(transform.delta_movement, 0.95),
        scale(normalize(to_owner), 0.05 * f64::from(loyalty)),
    );
    if projectile.trident_return_ticks == 0 {
        let silent = vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
            entity_data_bool(
                &metadata.data_values,
                accessors.silent_data_id,
                accessors.silent_default,
            )
        });
        if !silent {
            effects.push(EntityClientMovementEffectState::Sound(
                EntityClientMovementSoundState {
                    entity_id,
                    kind: EntityClientMovementSoundKind::TridentReturn,
                    position: transform.position,
                },
            ));
        }
    }
    projectile.trident_return_ticks = projectile.trident_return_ticks.saturating_add(1);
}

/// One arrow tick's movement outcome: the vanilla `applyEffectsFromBlocks`
/// slot with its `Entity.baseTick` reset placement, plus the deferred
/// inertia/gravity tail that runs after the effects callback.
struct ArrowMoveStep {
    first_effect: EntityBlockEffectMovement,
    collision: Option<EntityAbstractArrowCollisionFacts>,
    finish: Option<ArrowMoveFinish>,
    trident_initial_containment_grounded: bool,
}

struct ArrowMoveFinish {
    in_water: bool,
    physics_enabled: bool,
}

struct ArrowCollisionStep {
    second_effect: Option<EntityBlockEffectMovement>,
    reaches_endpoint: bool,
    discarded: bool,
    redirected_targets: Vec<EntityProjectileAimRedirect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EntityProjectileAimRedirect {
    entity_id: i32,
    incarnation: EntityIncarnation,
    movement: EntityVec3,
}

fn advance_abstract_arrow(
    entity_id: i32,
    entity_type_id: i32,
    metadata: &mut EntityMetadata,
    animations: &mut EntityClientAnimations,
    transform: &mut EntityTransform,
    accessors: super::schema::VanillaAbstractArrowMetadataAccessors,
    in_water: bool,
    collision: Option<&EntityAbstractArrowCollisionFacts>,
    effects: &mut Vec<EntityClientMovementEffectState>,
) -> ArrowMoveStep {
    let flags = entity_data_byte(
        &metadata.data_values,
        accessors.flags_data_id,
        accessors.flags_default,
    );
    let physics_enabled = flags & accessors.no_physics_mask == 0;
    let mut in_ground = entity_data_bool(
        &metadata.data_values,
        accessors.in_ground_data_id,
        accessors.in_ground_default,
    );
    let mut initial_containment_grounded = false;
    if physics_enabled {
        if collision.is_some_and(|collision| collision.initial_block_state_id.is_some()) {
            initial_containment_grounded = !in_ground;
            transform.delta_movement = EntityVec3::default();
            set_entity_data_bool(metadata, accessors.in_ground_data_id, true);
            in_ground = true;
            animations.animations.start_local_arrow_shake();
        }
    }
    // Vanilla decrements shake after the initial containment check and before
    // either the stuck branch or a later segment hit.
    animations.animations.advance_arrow_shake_tick();
    if in_ground && physics_enabled {
        // The stuck branch still calls the no-arg `applyEffectsFromBlocks`
        // (a degenerate old-to-current segment) but never `super.tick()`.
        return ArrowMoveStep {
            first_effect: EntityBlockEffectMovement::direct(transform.position, transform.position),
            collision: None,
            finish: None,
            trident_initial_containment_grounded: initial_containment_grounded,
        };
    }

    let original_position = transform.position;
    let movement = transform.delta_movement;
    if in_water {
        let water_inertia = if entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
            0.99_f32
        } else {
            0.6_f32
        };
        transform.delta_movement = scale(movement, f64::from(water_inertia));
        append_bubbles(
            entity_id,
            original_position,
            transform.delta_movement,
            effects,
        );
    }
    if flags & accessors.crit_mask != 0 {
        append_arrow_crit_particles(entity_id, original_position, movement, effects);
    }

    update_arrow_rotation(transform, movement, physics_enabled);
    let collision = physics_enabled.then(|| collision.cloned()).flatten();
    if physics_enabled {
        if let Some(collision) = collision.as_ref() {
            transform.position = collision
                .entity_hits
                .first()
                .map_or(collision.endpoint, |hit| hit.location);
        } else {
            transform.position = add(original_position, movement);
        }
    } else {
        transform.position = add(original_position, movement);
    }
    ArrowMoveStep {
        first_effect: EntityBlockEffectMovement::direct(original_position, transform.position),
        collision,
        finish: Some(ArrowMoveFinish {
            in_water,
            physics_enabled,
        }),
        trident_initial_containment_grounded: false,
    }
}

fn advance_abstract_arrow_collision_after_first_effect(
    entity_type_id: i32,
    metadata: &mut EntityMetadata,
    projectile: &mut EntityProjectileLifecycle,
    runtime: &mut EntityAbstractArrowRuntime,
    transform: &mut EntityTransform,
    accessors: super::schema::VanillaAbstractArrowMetadataAccessors,
    collision: &EntityAbstractArrowCollisionFacts,
    owner_look_angle: Option<EntityVec3>,
) -> ArrowCollisionStep {
    if collision.entity_hits.is_empty() {
        return ArrowCollisionStep {
            second_effect: None,
            reaches_endpoint: true,
            discarded: false,
            redirected_targets: Vec::new(),
        };
    }
    let mut redirected_targets = Vec::new();
    let outcome = apply_abstract_arrow_entity_hits(
        entity_type_id,
        metadata,
        projectile,
        runtime,
        transform,
        accessors,
        &collision.entity_hits,
        owner_look_angle,
        &mut redirected_targets,
    );
    if outcome.discarded || !outcome.continues_to_block {
        return ArrowCollisionStep {
            second_effect: None,
            reaches_endpoint: false,
            discarded: outcome.discarded,
            redirected_targets,
        };
    }
    let from = transform.position;
    transform.position = collision.endpoint;
    ArrowCollisionStep {
        second_effect: Some(EntityBlockEffectMovement::direct(from, collision.endpoint)),
        reaches_endpoint: true,
        discarded: false,
        redirected_targets,
    }
}

fn apply_abstract_arrow_endpoint_hit(
    entity_type_id: i32,
    metadata: &mut EntityMetadata,
    projectile: &mut EntityProjectileLifecycle,
    runtime: &mut EntityAbstractArrowRuntime,
    animations: &mut EntityClientAnimations,
    transform: &mut EntityTransform,
    accessors: super::schema::VanillaAbstractArrowMetadataAccessors,
    collision: &EntityAbstractArrowCollisionFacts,
) {
    if let Some(block_hit) = collision.block_hit {
        apply_abstract_arrow_block_hit(
            entity_type_id,
            metadata,
            projectile,
            runtime,
            animations,
            transform,
            accessors,
            block_hit.block_state_id,
        );
    } else if collision.world_border_hit {
        reverse_projectile(transform, runtime, None);
        transform.delta_movement = scale(transform.delta_movement, 0.2);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArrowEntityHitOutcome {
    continues_to_block: bool,
    discarded: bool,
}

fn apply_abstract_arrow_entity_hits(
    entity_type_id: i32,
    metadata: &mut EntityMetadata,
    projectile: &mut EntityProjectileLifecycle,
    runtime: &mut EntityAbstractArrowRuntime,
    transform: &mut EntityTransform,
    accessors: super::schema::VanillaAbstractArrowMetadataAccessors,
    hits: &[EntityAbstractArrowEntityHit],
    owner_look_angle: Option<EntityVec3>,
    redirected_targets: &mut Vec<EntityProjectileAimRedirect>,
) -> ArrowEntityHitOutcome {
    let pierce_level = entity_data_byte(
        &metadata.data_values,
        accessors.pierce_level_data_id,
        accessors.pierce_level_default,
    )
    .max(0);
    for hit in hits {
        if hit.deflects_projectiles {
            let deflector = EntityInstanceReference {
                entity_id: hit.entity_id,
                incarnation: hit.incarnation,
            };
            if runtime.last_deflected_by != Some(deflector) {
                reverse_projectile(transform, runtime, Some(deflector));
            }
            return ArrowEntityHitOutcome {
                continues_to_block: false,
                discarded: false,
            };
        }
        if hit.redirectable_projectile {
            if let Some(movement) = owner_look_angle {
                redirected_targets.push(EntityProjectileAimRedirect {
                    entity_id: hit.entity_id,
                    incarnation: hit.incarnation,
                    movement,
                });
            }
        }

        if entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
            projectile.trident_dealt_damage = true;
            reverse_projectile(transform, runtime, None);
            transform.delta_movement.x *= 0.02;
            transform.delta_movement.y *= 0.2;
            transform.delta_movement.z *= 0.02;
        } else {
            if pierce_level > 0 {
                if runtime.piercing_ignore_entity_ids.len()
                    >= usize::from(pierce_level as u8).saturating_add(1)
                {
                    return ArrowEntityHitOutcome {
                        continues_to_block: false,
                        discarded: true,
                    };
                }
                if !runtime.piercing_ignore_entity_ids.contains(&hit.entity_id) {
                    runtime.piercing_ignore_entity_ids.push(hit.entity_id);
                }
            }
            let flags = entity_data_byte(
                &metadata.data_values,
                accessors.flags_data_id,
                accessors.flags_default,
            );
            if flags & accessors.crit_mask != 0 {
                let speed = length_squared(transform.delta_movement).sqrt() as f32;
                let damage = (speed * 2.0).clamp(0.0, i32::MAX as f32).ceil() as i32;
                let _ = java_random_next_int_bound(&mut runtime.random_seed, damage / 2 + 2);
            }
            if hit.hurt_client {
                let _ = java_random_next_float(&mut runtime.random_seed);
                if pierce_level <= 0 {
                    return ArrowEntityHitOutcome {
                        continues_to_block: false,
                        discarded: true,
                    };
                }
            } else {
                reverse_projectile(transform, runtime, None);
                transform.delta_movement = scale(transform.delta_movement, 0.2);
            }
        }
    }
    ArrowEntityHitOutcome {
        continues_to_block: pierce_level > 0,
        discarded: false,
    }
}

fn reverse_projectile(
    transform: &mut EntityTransform,
    runtime: &mut EntityAbstractArrowRuntime,
    deflector: Option<EntityInstanceReference>,
) {
    let rotation = 170.0 + java_random_next_float(&mut runtime.random_seed) * 20.0;
    transform.delta_movement = scale(transform.delta_movement, -0.5);
    transform.y_rot += rotation;
    transform.old_y_rot += rotation;
    if let Some(deflector) = deflector {
        runtime.last_deflected_by = Some(deflector);
    }
}

fn apply_abstract_arrow_block_hit(
    entity_type_id: i32,
    metadata: &mut EntityMetadata,
    projectile: &mut EntityProjectileLifecycle,
    runtime: &mut EntityAbstractArrowRuntime,
    animations: &mut EntityClientAnimations,
    transform: &mut EntityTransform,
    accessors: super::schema::VanillaAbstractArrowMetadataAccessors,
    block_state_id: i32,
) {
    projectile.arrow_last_block_state_id = Some(block_state_id);
    let movement = transform.delta_movement;
    transform.position = EntityVec3 {
        x: transform.position.x - vanilla_signum(movement.x) * f64::from(0.05_f32),
        y: transform.position.y - vanilla_signum(movement.y) * f64::from(0.05_f32),
        z: transform.position.z - vanilla_signum(movement.z) * f64::from(0.05_f32),
    };
    transform.delta_movement = EntityVec3::default();
    let _ = java_random_next_float(&mut runtime.random_seed);
    set_entity_data_bool(metadata, accessors.in_ground_data_id, true);
    animations.animations.start_local_arrow_shake();
    let flags = entity_data_byte(
        &metadata.data_values,
        accessors.flags_data_id,
        accessors.flags_default,
    );
    set_entity_data_byte(
        metadata,
        accessors.flags_data_id,
        flags & !accessors.crit_mask,
    );
    set_entity_data_byte(metadata, accessors.pierce_level_data_id, 0);
    if entity_type_id == VANILLA_ENTITY_TYPE_TRIDENT_ID {
        projectile.arrow_hit_ground_sound_is_arrow = true;
    }
    runtime.piercing_ignore_entity_ids.clear();
}

fn vanilla_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

/// `AbstractArrow.tick` tail after the effects slot: `applyInertia(0.99F)`
/// multiplies the current delta (which block callbacks may have changed),
/// then `applyGravity`.
fn finish_abstract_arrow(
    metadata: &EntityMetadata,
    transform: &mut EntityTransform,
    finish: ArrowMoveFinish,
    accessors: super::schema::VanillaAbstractArrowMetadataAccessors,
) {
    if !finish.in_water {
        transform.delta_movement = scale(transform.delta_movement, f64::from(0.99_f32));
    }
    let no_gravity = vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
        entity_data_bool(
            &metadata.data_values,
            accessors.no_gravity_data_id,
            accessors.no_gravity_default,
        )
    });
    let in_ground = entity_data_bool(
        &metadata.data_values,
        accessors.in_ground_data_id,
        accessors.in_ground_default,
    );
    if finish.physics_enabled && !in_ground && !no_gravity {
        transform.delta_movement.y -= 0.05;
    }
}

fn advance_falling_block(
    metadata: &EntityMetadata,
    transform: &mut EntityTransform,
    collision: Option<EntityClientMovementCollisionResult>,
) {
    let requested = nonliving_gravity_movement(metadata, transform.delta_movement, 0.04);
    let clipped = collision
        .filter(|collision| collision.requested_movement == requested)
        .map_or(requested, |collision| collision.clipped_movement);
    transform.position = add(transform.position, clipped);
    transform.on_ground = Some(requested.y < 0.0 && clipped.y != requested.y);
    transform.delta_movement = scale(requested, f64::from(0.98_f32));
}

fn advance_item_entity(
    transform: &mut EntityTransform,
    supporting_block: &mut EntitySupportingBlock,
    collision: EntityClientMovementCollisionResult,
) {
    transform.delta_movement = collision.pre_move_delta;
    if collision.movement_skipped {
        return;
    }
    apply_client_collision_movement(transform, supporting_block, collision);
    let on_ground = transform.on_ground.unwrap_or(false);
    let friction = if on_ground {
        collision.ground_friction * 0.98_f32
    } else {
        0.98_f32
    };
    transform.delta_movement.x *= f64::from(friction);
    transform.delta_movement.y *= f64::from(0.98_f32);
    transform.delta_movement.z *= f64::from(friction);
    if on_ground && transform.delta_movement.y < 0.0 {
        transform.delta_movement.y *= -0.5;
    }
}

fn advance_experience_orb(
    movement: &mut EntityExperienceOrbMovement,
    transform: &mut EntityTransform,
    supporting_block: &mut EntitySupportingBlock,
    collision: EntityClientMovementCollisionResult,
) {
    if let Some(update) = collision.experience_orb_update {
        movement.following_player_uuid = update.following_player_uuid;
        movement.random_seed = update.random_seed;
    }
    let fall_speed = collision.pre_move_delta.y;
    transform.delta_movement = collision.pre_move_delta;
    apply_client_collision_movement(transform, supporting_block, collision);
    let on_ground = transform.on_ground.unwrap_or(false);
    let friction = if on_ground {
        collision.ground_friction * 0.98_f32
    } else {
        0.98_f32
    };
    transform.delta_movement = scale(transform.delta_movement, f64::from(friction));
    if on_ground && fall_speed < -0.03 {
        transform.delta_movement.y = -fall_speed * 0.4;
    }
}

fn advance_fishing_hook(
    movement: &mut EntityFishingHookMovement,
    metadata: &mut EntityMetadata,
    transform: &mut EntityTransform,
    supporting_block: &mut EntitySupportingBlock,
    collision: EntityClientMovementCollisionResult,
    effects: &mut Vec<EntityClientMovementEffectState>,
) {
    let Some(update) = collision.fishing_hook_update else {
        return;
    };
    *movement = update.movement;
    transform.delta_movement = collision.pre_move_delta;
    if let Some(deflection) = collision.fishing_hook_reverse_deflection {
        let rotation = 170.0 + java_random_next_float(&mut movement.random_seed) * 20.0;
        transform.y_rot += rotation;
        transform.old_y_rot += rotation;
        if let EntityFishingHookReverseDeflection::Entity {
            entity_id,
            incarnation,
            position,
        } = deflection
        {
            movement.last_deflected_by = Some(incarnation);
            effects.push(EntityClientMovementEffectState::Sound(
                EntityClientMovementSoundState {
                    entity_id,
                    kind: EntityClientMovementSoundKind::BreezeDeflect,
                    position,
                },
            ));
        }
    }
    if let Some(position) = update.position_override {
        transform.position = position;
    }
    if update.clear_hooked_entity_data {
        set_entity_data_int(
            metadata,
            super::schema::VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
            super::schema::VANILLA_FISHING_HOOK_DEFAULT_HOOKED_ENTITY,
        );
    }
    if collision.movement_skipped {
        return;
    }
    apply_client_collision_movement(transform, supporting_block, collision);
    update_projectile_rotation(transform);
    if movement.state == EntityFishingHookState::Flying
        && (transform.on_ground.unwrap_or(false)
            || collision.requested_movement.x != collision.clipped_movement.x
            || collision.requested_movement.z != collision.clipped_movement.z)
    {
        transform.delta_movement = EntityVec3::default();
    }
    transform.delta_movement = scale(transform.delta_movement, 0.92);
}

fn apply_client_collision_movement(
    transform: &mut EntityTransform,
    supporting_block: &mut EntitySupportingBlock,
    collision: EntityClientMovementCollisionResult,
) {
    transform.position = add(transform.position, collision.clipped_movement);
    // `Entity.move` uses `!Mth.equal` for horizontal collision, not exact
    // floating-point inequality. Vertical collision intentionally remains exact.
    let x_collision =
        (collision.requested_movement.x - collision.clipped_movement.x).abs() >= 1.0e-5;
    let z_collision =
        (collision.requested_movement.z - collision.clipped_movement.z).abs() >= 1.0e-5;
    if x_collision {
        transform.delta_movement.x = 0.0;
    }
    if z_collision {
        transform.delta_movement.z = 0.0;
    }
    let on_ground = collision.requested_movement.y < 0.0
        && collision.requested_movement.y != collision.clipped_movement.y;
    transform.on_ground = Some(on_ground);
    if on_ground {
        supporting_block.main = collision.supporting_block;
        supporting_block.on_ground_no_blocks = collision.supporting_block.is_none();
    } else {
        supporting_block.clear_physical();
    }
}

fn advance_primed_tnt(
    metadata: &mut EntityMetadata,
    transform: &mut EntityTransform,
    fuse_data_id: u8,
    fuse_default: i32,
    collision: Option<EntityClientMovementCollisionResult>,
) {
    let requested = nonliving_gravity_movement(metadata, transform.delta_movement, 0.04);
    let clipped = collision
        .filter(|collision| collision.requested_movement == requested)
        .map_or(requested, |collision| collision.clipped_movement);
    transform.position = add(transform.position, clipped);
    let on_ground = requested.y < 0.0 && clipped.y != requested.y;
    transform.on_ground = Some(on_ground);
    transform.delta_movement = scale(requested, f64::from(0.98_f32));
    if on_ground {
        transform.delta_movement.x *= 0.7;
        transform.delta_movement.y *= -0.5;
        transform.delta_movement.z *= 0.7;
    }

    let fuse = entity_data_int(&metadata.data_values, fuse_data_id, fuse_default);
    set_entity_data_int(metadata, fuse_data_id, fuse.wrapping_sub(1));
}

fn nonliving_gravity_movement(
    metadata: &EntityMetadata,
    mut movement: EntityVec3,
    gravity: f64,
) -> EntityVec3 {
    let no_gravity = vanilla_entity_metadata_entity_accessors().is_some_and(|accessors| {
        entity_data_bool(
            &metadata.data_values,
            accessors.no_gravity_data_id,
            accessors.no_gravity_default,
        )
    });
    if !no_gravity {
        movement.y -= gravity;
    }
    movement
}

fn append_arrow_crit_particles(
    entity_id: i32,
    position: EntityVec3,
    movement: EntityVec3,
    effects: &mut Vec<EntityClientMovementEffectState>,
) {
    effects.extend((0..4).map(|index| {
        let fraction = f64::from(index) / 4.0;
        EntityClientMovementEffectState::Particle(EntityClientMovementParticleState::Simple(
            EntityClientMovementSimpleParticleState {
                entity_id,
                kind: EntityClientMovementSimpleParticleKind::Crit,
                position: EntityVec3 {
                    x: position.x + movement.x * fraction,
                    y: position.y + movement.y * fraction,
                    z: position.z + movement.z * fraction,
                },
                velocity: EntityVec3 {
                    x: -movement.x,
                    y: -movement.y + 0.2,
                    z: -movement.z,
                },
            },
        ))
    }));
}

fn update_arrow_rotation(
    transform: &mut EntityTransform,
    movement: EntityVec3,
    physics_enabled: bool,
) {
    let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
    let target_y_rot = if physics_enabled {
        vanilla_degrees(movement.x.atan2(movement.z))
    } else {
        vanilla_degrees((-movement.x).atan2(-movement.z))
    };
    let target_x_rot = vanilla_degrees(movement.y.atan2(horizontal));
    transform.old_x_rot = unwrap_old_rotation(transform.old_x_rot, target_x_rot);
    transform.old_y_rot = unwrap_old_rotation(transform.old_y_rot, target_y_rot);
    transform.x_rot = lerp(0.2, transform.old_x_rot, target_x_rot);
    transform.y_rot = lerp(0.2, transform.old_y_rot, target_y_rot);
}

fn throwable_gravity(policy: VanillaEntityClientMovementPolicy) -> f64 {
    match policy {
        VanillaEntityClientMovementPolicy::ThrownExperienceBottle => 0.07,
        VanillaEntityClientMovementPolicy::ThrownPotion => 0.05,
        _ => 0.03,
    }
}

fn hurting_projectile_inertia(
    policy: VanillaEntityClientMovementPolicy,
    entity_type_id: i32,
    metadata: &EntityMetadata,
    in_water: bool,
) -> f64 {
    if matches!(policy, VanillaEntityClientMovementPolicy::WindCharge) {
        return 1.0;
    }
    if in_water {
        return f64::from(0.8_f32);
    }
    let dangerous = matches!(policy, VanillaEntityClientMovementPolicy::WitherSkull)
        && vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(entity_type_id)
            .is_some_and(|(data_id, default)| {
                entity_data_bool(&metadata.data_values, data_id, default)
            });
    f64::from(if dangerous { 0.73_f32 } else { 0.95_f32 })
}

fn append_bubbles(
    entity_id: i32,
    position: EntityVec3,
    movement: EntityVec3,
    effects: &mut Vec<EntityClientMovementEffectState>,
) {
    let position = EntityVec3 {
        x: position.x - movement.x * 0.25,
        y: position.y - movement.y * 0.25,
        z: position.z - movement.z * 0.25,
    };
    effects.extend((0..4).map(|_| {
        EntityClientMovementEffectState::Particle(EntityClientMovementParticleState::Simple(
            EntityClientMovementSimpleParticleState {
                entity_id,
                kind: EntityClientMovementSimpleParticleKind::Bubble,
                position,
                velocity: movement,
            },
        ))
    }));
}

fn entity_data_bool(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: bool,
) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match value.value {
            EntityDataValueKind::Boolean(value) => Some(value),
            _ => None,
        })
        .unwrap_or(default)
}

fn entity_data_byte(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: i8,
) -> i8 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match value.value {
            EntityDataValueKind::Byte(value) => Some(value),
            _ => None,
        })
        .unwrap_or(default)
}

fn entity_data_int(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: i32,
) -> i32 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match value.value {
            EntityDataValueKind::Int(value) => Some(value),
            _ => None,
        })
        .unwrap_or(default)
}

fn entity_data_float(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: f32,
) -> f32 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match value.value {
            EntityDataValueKind::Float(value) => Some(value),
            _ => None,
        })
        .unwrap_or(default)
}

fn item_entity_has_stack(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> bool {
    let Some(data_id) =
        vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(entity_type_id)
    else {
        return false;
    };
    data_values.iter().any(|value| {
        value.data_id == data_id
            && matches!(
                &value.value,
                EntityDataValueKind::ItemStack(stack)
                    if stack.item_id.is_some() && stack.count > 0
            )
    })
}

pub(super) fn apply_entity_fluid_currents(
    mut movement: EntityVec3,
    contact: crate::client::EntityFluidContactState,
    lava_current_scale: f64,
) -> EntityVec3 {
    for (current, count, current_scale) in [
        (contact.water_current, contact.water_current_count, 0.014),
        (
            contact.lava_current,
            contact.lava_current_count,
            lava_current_scale,
        ),
    ] {
        if count == 0 {
            continue;
        }
        let mut impulse = EntityVec3 {
            x: current.x,
            y: current.y,
            z: current.z,
        };
        if length_squared(impulse) < 1.0e-5 {
            continue;
        }
        impulse = scale(normalize(impulse), current_scale);
        if movement.x.abs() < 0.003
            && movement.z.abs() < 0.003
            && length_squared(impulse).sqrt() < 0.0045
        {
            impulse = scale(normalize(impulse), 0.0045);
        }
        movement = add(movement, impulse);
    }
    movement
}

fn select_experience_orb_player_target(
    orb_position: EntityVec3,
    following_player_uuid: Option<uuid::Uuid>,
    targets: &[EntityExperienceOrbPlayerTarget],
) -> Option<EntityExperienceOrbPlayerTarget> {
    if let Some(current) = following_player_uuid
        .and_then(|uuid| targets.iter().copied().find(|target| target.uuid == uuid))
    {
        if length_squared(subtract(current.position, orb_position)) <= 64.0 {
            return Some(current);
        }
    }
    let mut nearest = None;
    let mut nearest_distance = f64::INFINITY;
    for target in targets.iter().copied() {
        if target.is_dead_or_dying {
            continue;
        }
        let distance = length_squared(subtract(target.position, orb_position));
        // `EntityGetter.getNearestPlayer` uses strict comparisons, preserving
        // the first player at an equal distance and excluding exactly 8 blocks.
        if distance < 64.0 && distance < nearest_distance {
            nearest = Some(target);
            nearest_distance = distance;
        }
    }
    nearest
}

fn set_entity_data_byte(metadata: &mut EntityMetadata, data_id: u8, value: i8) {
    let value = bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        value: EntityDataValueKind::Byte(value),
    };
    if let Some(existing) = metadata
        .data_values
        .iter_mut()
        .find(|existing| existing.data_id == data_id)
    {
        *existing = value;
    } else {
        metadata.data_values.push(value);
        metadata.data_values.sort_by_key(|value| value.data_id);
    }
}

fn set_entity_data_bool(metadata: &mut EntityMetadata, data_id: u8, value: bool) {
    let value = bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
        value: EntityDataValueKind::Boolean(value),
    };
    if let Some(existing) = metadata
        .data_values
        .iter_mut()
        .find(|existing| existing.data_id == data_id)
    {
        *existing = value;
    } else {
        metadata.data_values.push(value);
        metadata.data_values.sort_by_key(|value| value.data_id);
    }
}

fn set_entity_data_int(metadata: &mut EntityMetadata, data_id: u8, value: i32) {
    let value = bbb_protocol::packets::EntityDataValue {
        data_id,
        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        value: EntityDataValueKind::Int(value),
    };
    if let Some(existing) = metadata
        .data_values
        .iter_mut()
        .find(|existing| existing.data_id == data_id)
    {
        *existing = value;
    } else {
        metadata.data_values.push(value);
        metadata.data_values.sort_by_key(|value| value.data_id);
    }
}

fn entity_data_optional_unsigned_int(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
    default: Option<i32>,
) -> Option<i32> {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match value.value {
            EntityDataValueKind::OptionalUnsignedInt(value) => Some(value),
            _ => None,
        })
        .unwrap_or(default)
}

fn add(left: EntityVec3, right: EntityVec3) -> EntityVec3 {
    EntityVec3 {
        x: left.x + right.x,
        y: left.y + right.y,
        z: left.z + right.z,
    }
}

fn horizontal_length_squared(value: EntityVec3) -> f64 {
    value.x * value.x + value.z * value.z
}

fn length_squared(value: EntityVec3) -> f64 {
    horizontal_length_squared(value) + value.y * value.y
}

fn subtract(left: EntityVec3, right: EntityVec3) -> EntityVec3 {
    EntityVec3 {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

fn scale(value: EntityVec3, scale: f64) -> EntityVec3 {
    EntityVec3 {
        x: value.x * scale,
        y: value.y * scale,
        z: value.z * scale,
    }
}

fn normalize(value: EntityVec3) -> EntityVec3 {
    let length = (value.x * value.x + value.y * value.y + value.z * value.z).sqrt();
    if length < f64::from(1.0e-5_f32) {
        EntityVec3::default()
    } else {
        scale(value, length.recip())
    }
}

pub(crate) fn calculate_view_vector(x_rot: f32, y_rot: f32) -> EntityVec3 {
    let radians_per_degree = (std::f64::consts::PI / 180.0) as f32;
    let real_x_rot = x_rot * radians_per_degree;
    let real_y_rot = -y_rot * radians_per_degree;
    let y_cos = vanilla_mth_cos(f64::from(real_y_rot));
    let y_sin = vanilla_mth_sin(f64::from(real_y_rot));
    let x_cos = vanilla_mth_cos(f64::from(real_x_rot));
    let x_sin = vanilla_mth_sin(f64::from(real_x_rot));
    EntityVec3 {
        x: f64::from(y_sin * x_cos),
        y: f64::from(-x_sin),
        z: f64::from(y_cos * x_cos),
    }
}

pub(super) fn vanilla_mth_sin(angle: f64) -> f32 {
    vanilla_mth_sin_table_value((angle * 10_430.378_350_470_453) as i64)
}

pub(super) fn vanilla_mth_cos(angle: f64) -> f32 {
    vanilla_mth_sin_table_value((angle * 10_430.378_350_470_453 + 16_384.0) as i64)
}

fn vanilla_mth_sin_table_value(raw_index: i64) -> f32 {
    let index = raw_index & 65_535;
    (index as f64 / 10_430.378_350_470_453).sin() as f32
}

fn advance_position(transform: &mut EntityTransform) {
    transform.position.x += transform.delta_movement.x;
    transform.position.y += transform.delta_movement.y;
    transform.position.z += transform.delta_movement.z;
}

fn rotate_towards_movement(transform: &mut EntityTransform, rotation_speed: f32) {
    let movement = transform.delta_movement;
    if movement == EntityVec3::default() {
        return;
    }
    let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
    let target_y_rot = vanilla_degrees(movement.z.atan2(movement.x)) + 90.0;
    let target_x_rot = vanilla_degrees(horizontal.atan2(movement.y)) - 90.0;
    transform.old_x_rot = unwrap_old_rotation(transform.old_x_rot, target_x_rot);
    transform.old_y_rot = unwrap_old_rotation(transform.old_y_rot, target_y_rot);
    transform.x_rot = lerp(rotation_speed, transform.old_x_rot, target_x_rot);
    transform.y_rot = lerp(rotation_speed, transform.old_y_rot, target_y_rot);
}

fn update_projectile_rotation(transform: &mut EntityTransform) {
    let movement = transform.delta_movement;
    let horizontal = (movement.x * movement.x + movement.z * movement.z).sqrt();
    let target_x_rot = vanilla_degrees(movement.y.atan2(horizontal));
    let target_y_rot = vanilla_degrees(movement.x.atan2(movement.z));
    transform.old_x_rot = unwrap_old_rotation(transform.old_x_rot, target_x_rot);
    transform.old_y_rot = unwrap_old_rotation(transform.old_y_rot, target_y_rot);
    transform.x_rot = lerp(0.2, transform.old_x_rot, target_x_rot);
    transform.y_rot = lerp(0.2, transform.old_y_rot, target_y_rot);
}

fn vanilla_degrees(radians: f64) -> f32 {
    (radians * f64::from(180.0_f32) / f64::from(std::f32::consts::PI)) as f32
}

fn unwrap_old_rotation(mut old: f32, target: f32) -> f32 {
    while target - old < -180.0 {
        old -= 360.0;
    }
    while target - old >= 180.0 {
        old += 360.0;
    }
    old
}

fn lerp(alpha: f32, start: f32, end: f32) -> f32 {
    start + alpha * (end - start)
}

#[cfg(test)]
mod tests {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_ARROW_ID, VANILLA_ENTITY_TYPE_BREEZE_ID,
        VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        VANILLA_ENTITY_TYPE_FIREBALL_ID, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        VANILLA_ENTITY_TYPE_SNOWBALL_ID, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID,
        VANILLA_ENTITY_TYPE_SPLASH_POTION_ID, VANILLA_ENTITY_TYPE_TNT_ID,
        VANILLA_ENTITY_TYPE_TRIDENT_ID, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID,
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    };
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, EntityDataValue, ItemStackSummary, RemoveEntities,
        SetBorderLerpSize as ProtocolSetBorderLerpSize, SetBorderSize as ProtocolSetBorderSize,
        SetEntityData as ProtocolSetEntityData, SetEntityMotion as ProtocolSetEntityMotion,
        Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    use crate::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension, WorldStore,
    };

    use super::*;

    #[test]
    fn direct_projectiles_integrate_synced_motion_and_shulker_rotation() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            10,
            VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID,
            ProtocolVec3d {
                x: 0.25,
                y: -0.5,
                z: 1.0,
            },
        ));
        store.apply_add_entity(projectile(
            20,
            VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
            ProtocolVec3d {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        ));
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        assert_eq!(
            store
                .drain_entity_client_movement_effect_states()
                .collect::<Vec<_>>(),
            vec![
                particle_effect(EntityClientMovementParticleState::EyeOfEnder(
                    EyeOfEnderTrailParticleState {
                        entity_id: 10,
                        origin: EntityVec3 {
                            x: 1.1875,
                            y: 63.625,
                            z: -1.25,
                        },
                        delta_movement: EntityVec3 {
                            x: 0.25,
                            y: -0.5,
                            z: 1.0,
                        },
                        in_water: false,
                    },
                )),
                particle_effect(EntityClientMovementParticleState::ShulkerBullet(
                    ShulkerBulletTrailParticleState {
                        entity_id: 20,
                        position: EntityVec3 {
                            x: 1.0,
                            y: 64.15,
                            z: -2.0,
                        },
                    },
                )),
            ]
        );

        assert_eq!(
            store.probe_entity_transform(10).unwrap().position,
            EntityVec3 {
                x: 1.25,
                y: 63.5,
                z: -1.0,
            }
        );
        let bullet = store.probe_entity_transform(20).unwrap();
        assert_eq!(
            bullet.position,
            EntityVec3 {
                x: 2.0,
                y: 64.0,
                z: -2.0,
            }
        );
        assert_eq!(bullet.y_rot, 45.0);
        assert_eq!(bullet.x_rot, 0.0);
        assert_eq!(
            generation_a.probe_entity_transform(20).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            "the retained generation remains isolated from client motion"
        );
    }

    #[test]
    fn throwable_projectiles_apply_generated_gravity_and_float_inertia() {
        let mut store = WorldStore::new();
        for (id, entity_type_id) in [
            (30, VANILLA_ENTITY_TYPE_SNOWBALL_ID),
            (31, VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID),
            (32, VANILLA_ENTITY_TYPE_SPLASH_POTION_ID),
            (33, VANILLA_ENTITY_TYPE_SNOWBALL_ID),
        ] {
            store.apply_add_entity(projectile(
                id,
                entity_type_id,
                ProtocolVec3d {
                    x: 0.25,
                    y: 0.5,
                    z: -0.75,
                },
            ));
        }
        let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 33,
            values: vec![EntityDataValue {
                data_id: entity_accessors.no_gravity_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));

        store.advance_entity_client_animations(1);

        let inertia = f64::from(0.99_f32);
        for (id, gravity) in [(30, 0.03), (31, 0.07), (32, 0.05)] {
            let transform = store.probe_entity_transform(id).unwrap();
            assert_vec3_near(
                transform.delta_movement,
                EntityVec3 {
                    x: 0.25 * inertia,
                    y: (0.5 - gravity) * inertia,
                    z: -0.75 * inertia,
                },
            );
            assert_vec3_near(
                transform.position,
                EntityVec3 {
                    x: 1.0 + 0.25 * inertia,
                    y: 64.0 + (0.5 - gravity) * inertia,
                    z: -2.0 - 0.75 * inertia,
                },
            );
        }
        let no_gravity = store.probe_entity_transform(33).unwrap();
        assert_vec3_near(
            no_gravity.delta_movement,
            EntityVec3 {
                x: 0.25 * inertia,
                y: 0.5 * inertia,
                z: -0.75 * inertia,
            },
        );
    }

    #[test]
    fn hurting_projectile_families_apply_acceleration_inertia_and_typed_trails() {
        let mut store = WorldStore::new();
        for (id, entity_type_id) in [
            (40, VANILLA_ENTITY_TYPE_FIREBALL_ID),
            (41, VANILLA_ENTITY_TYPE_WIND_CHARGE_ID),
            (42, VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID),
            (43, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID),
        ] {
            store.apply_add_entity(projectile(
                id,
                entity_type_id,
                ProtocolVec3d {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
            ));
        }
        let (dangerous_data_id, _) =
            vanilla_entity_metadata_wither_skull_dangerous_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
            )
            .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 43,
            values: vec![EntityDataValue {
                data_id: dangerous_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let fireball_delta = 1.1 * f64::from(0.95_f32);
        let wither_delta = 1.1 * f64::from(0.73_f32);
        assert_vec3_near(
            store.probe_entity_transform(40).unwrap().delta_movement,
            EntityVec3 {
                x: fireball_delta,
                y: 0.0,
                z: 0.0,
            },
        );
        assert_vec3_near(
            store.probe_entity_transform(41).unwrap().delta_movement,
            EntityVec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        );
        assert_vec3_near(
            store.probe_entity_transform(43).unwrap().delta_movement,
            EntityVec3 {
                x: wither_delta,
                y: 0.0,
                z: 0.0,
            },
        );
        assert_eq!(
            store
                .drain_entity_client_movement_effect_states()
                .collect::<Vec<_>>(),
            vec![
                simple_particle(
                    40,
                    EntityClientMovementSimpleParticleKind::Smoke,
                    fireball_delta
                ),
                simple_particle(
                    42,
                    EntityClientMovementSimpleParticleKind::DragonBreath,
                    fireball_delta,
                ),
                simple_particle(
                    43,
                    EntityClientMovementSimpleParticleKind::Smoke,
                    wither_delta
                ),
            ]
        );
        assert_vec3_near(
            generation_a.probe_entity_transform(43).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        );
    }

    #[test]
    fn throwable_water_path_emits_four_pre_inertia_bubbles() {
        let metadata = EntityMetadata::default();
        let mut transform = test_transform(EntityVec3 {
            x: 0.4,
            y: -0.2,
            z: 0.6,
        });
        let mut effects = Vec::new();

        advance_throwable_projectile(
            50,
            &metadata,
            &mut transform,
            VanillaEntityClientMovementPolicy::ThrowableProjectile,
            true,
            &mut effects,
        );

        let gravity_movement = EntityVec3 {
            x: 0.4,
            y: -0.23,
            z: 0.6,
        };
        let bubble_position = EntityVec3 {
            x: 1.0 - gravity_movement.x * 0.25,
            y: 64.0 - gravity_movement.y * 0.25,
            z: -2.0 - gravity_movement.z * 0.25,
        };
        assert_eq!(effects.len(), 4);
        assert!(effects.iter().all(|state| *state
            == particle_effect(EntityClientMovementParticleState::Simple(
                EntityClientMovementSimpleParticleState {
                    entity_id: 50,
                    kind: EntityClientMovementSimpleParticleKind::Bubble,
                    position: bubble_position,
                    velocity: gravity_movement,
                }
            ))));
        assert_vec3_near(
            transform.delta_movement,
            scale(gravity_movement, f64::from(0.8_f32)),
        );
    }

    #[test]
    fn abstract_arrows_honor_synced_ground_physics_and_crit_flags() {
        let mut store = WorldStore::new();
        for (id, entity_type_id) in [
            (54, VANILLA_ENTITY_TYPE_ARROW_ID),
            (55, VANILLA_ENTITY_TYPE_SPECTRAL_ARROW_ID),
            (56, VANILLA_ENTITY_TYPE_TRIDENT_ID),
        ] {
            store.apply_add_entity(projectile(
                id,
                entity_type_id,
                ProtocolVec3d {
                    x: 1.0,
                    y: 0.5,
                    z: 0.0,
                },
            ));
        }
        let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ARROW_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 54,
            values: vec![EntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.crit_mask),
            }],
        }));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 55,
            values: vec![EntityDataValue {
                data_id: accessors.in_ground_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 56,
            values: vec![EntityDataValue {
                data_id: accessors.flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.no_physics_mask),
            }],
        }));
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let inertia = f64::from(0.99_f32);
        let flying = store.probe_entity_transform(54).unwrap();
        assert_vec3_near(
            flying.position,
            EntityVec3 {
                x: 2.0,
                y: 64.5,
                z: -2.0,
            },
        );
        assert_vec3_near(
            flying.delta_movement,
            EntityVec3 {
                x: inertia,
                y: 0.5 * inertia - 0.05,
                z: 0.0,
            },
        );
        assert_eq!(flying.y_rot, 18.0);
        assert_eq!(
            store.probe_entity_transform(55).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            }
        );
        let no_physics = store.probe_entity_transform(56).unwrap();
        assert_vec3_near(
            no_physics.delta_movement,
            EntityVec3 {
                x: inertia,
                y: 0.5 * inertia,
                z: 0.0,
            },
        );
        assert_eq!(no_physics.y_rot, -18.0);

        let effects = store
            .drain_entity_client_movement_effect_states()
            .collect::<Vec<_>>();
        assert_eq!(effects.len(), 4);
        for (index, effect) in effects.into_iter().enumerate() {
            assert_eq!(
                effect,
                particle_effect(EntityClientMovementParticleState::Simple(
                    EntityClientMovementSimpleParticleState {
                        entity_id: 54,
                        kind: EntityClientMovementSimpleParticleKind::Crit,
                        position: EntityVec3 {
                            x: 1.0 + index as f64 / 4.0,
                            y: 64.0 + 0.5 * index as f64 / 4.0,
                            z: -2.0,
                        },
                        velocity: EntityVec3 {
                            x: -1.0,
                            y: -0.3,
                            z: 0.0,
                        },
                    },
                ))
            );
        }
        assert_eq!(
            generation_a.probe_entity_transform(54).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            "retained publication remains isolated from arrow movement"
        );
    }

    #[test]
    fn abstract_arrow_water_inertia_precedes_bubbles_and_free_flight() {
        for (entity_id, entity_type_id, inertia) in [
            (57, VANILLA_ENTITY_TYPE_ARROW_ID, 0.6_f32),
            (58, VANILLA_ENTITY_TYPE_TRIDENT_ID, 0.99_f32),
        ] {
            let mut metadata = EntityMetadata::default();
            let mut animations = EntityClientAnimations::default();
            let mut transform = test_transform(EntityVec3 {
                x: 0.5,
                y: -0.25,
                z: 1.0,
            });
            let mut effects = Vec::new();
            let accessors =
                vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(entity_type_id)
                    .unwrap();

            let step = advance_abstract_arrow(
                entity_id,
                entity_type_id,
                &mut metadata,
                &mut animations,
                &mut transform,
                accessors,
                true,
                None,
                &mut effects,
            );
            let segment = step.first_effect;
            assert_vec3_near(
                segment.from,
                EntityVec3 {
                    x: 1.0,
                    y: 64.0,
                    z: -2.0,
                },
            );
            assert_vec3_near(segment.to, transform.position);
            finish_abstract_arrow(
                &metadata,
                &mut transform,
                step.finish.expect("flight runs the inertia tail"),
                accessors,
            );

            let water_movement = scale(
                EntityVec3 {
                    x: 0.5,
                    y: -0.25,
                    z: 1.0,
                },
                f64::from(inertia),
            );
            assert_vec3_near(
                transform.position,
                EntityVec3 {
                    x: 1.5,
                    y: 63.75,
                    z: -1.0,
                },
            );
            assert_vec3_near(
                transform.delta_movement,
                EntityVec3 {
                    y: water_movement.y - 0.05,
                    ..water_movement
                },
            );
            assert_eq!(effects.len(), 4);
            assert!(effects.iter().all(|effect| {
                *effect
                    == particle_effect(EntityClientMovementParticleState::Simple(
                        EntityClientMovementSimpleParticleState {
                            entity_id,
                            kind: EntityClientMovementSimpleParticleKind::Bubble,
                            position: EntityVec3 {
                                x: 1.0 - water_movement.x * 0.25,
                                y: 64.0 - water_movement.y * 0.25,
                                z: -2.0 - water_movement.z * 0.25,
                            },
                            velocity: water_movement,
                        },
                    ))
            }));
        }
    }

    #[test]
    fn abstract_arrow_margin_refinement_honors_blocks_and_player_friendly_fire() {
        let owner_uuid = Uuid::from_u128(70);
        let target_uuid = Uuid::from_u128(71);
        let source = EntityAbstractArrowCollisionSource {
            entity_id: 70,
            entity_type_id: VANILLA_ENTITY_TYPE_ARROW_ID,
            position: EntityVec3 {
                x: 0.0,
                y: 0.9,
                z: 0.0,
            },
            movement: EntityVec3 {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
            bounds: EntityPickBoundsState::from_base_size(0.5, 0.5, 0.0),
            on_ground: false,
            tick_count: 22,
            owner_uuid: Some(owner_uuid),
            owner_entity_type_id: Some(VANILLA_ENTITY_TYPE_PLAYER_ID),
            owner_root_vehicle_id: None,
            left_owner: true,
            trident_dealt_damage: false,
            piercing_ignore_entity_ids: Vec::new(),
        };
        let target = EntityProjectileTarget {
            entity_id: 71,
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            uuid: target_uuid,
            root_vehicle_id: 71,
            position: EntityVec3 {
                x: 1.0,
                y: 0.0,
                z: 0.35,
            },
            bounds: EntityPickBoundsState::from_base_size(0.6, 1.8, 0.1),
            alive: true,
            pickable: true,
            deflects_projectiles: false,
            redirectable_projectile: false,
            hurt_client: false,
            trident_hurt_client: false,
            hurt_client_rejects_ender_dragon_owner: false,
            incarnation: EntityIncarnation(71),
        };
        let batch = || EntityAbstractArrowCollisionBatch {
            sources: vec![source.clone()],
            targets: vec![target],
        };
        let miss_clip = |_: i32, _: EntityVec3, movement: EntityVec3, _: bool, _: f64| {
            crate::client::ProjectileBlockMovementClip {
                movement: [movement.x, movement.y, movement.z],
                world_border_hit: false,
                block_state_id: None,
            }
        };
        let friendly_filtered = resolve_abstract_arrow_collision_facts(
            batch(),
            |_| None,
            miss_clip,
            |owner, target| owner != owner_uuid || target != target_uuid,
        );
        assert!(friendly_filtered[&70].entity_hits.is_empty());

        let unobstructed =
            resolve_abstract_arrow_collision_facts(batch(), |_| None, miss_clip, |_, _| true);
        assert_eq!(unobstructed[&70].entity_hits.len(), 1);
        assert!(
            unobstructed[&70].entity_hits[0].location.z >= 0.05 - 1.0e-12,
            "the inflated-margin entry must be refined back to the exact target surface"
        );

        let crystal = EntityProjectileTarget {
            entity_type_id: VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
            hurt_client: true,
            hurt_client_rejects_ender_dragon_owner: true,
            ..target
        };
        let mut dragon_owned_source = source.clone();
        dragon_owned_source.owner_entity_type_id = Some(VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID);
        let dragon_owned = resolve_abstract_arrow_collision_facts(
            EntityAbstractArrowCollisionBatch {
                sources: vec![dragon_owned_source],
                targets: vec![crystal],
            },
            |_| None,
            miss_clip,
            |_, _| true,
        );
        assert!(!dragon_owned[&70].entity_hits[0].hurt_client);

        let obstructed = resolve_abstract_arrow_collision_facts(
            batch(),
            |_| None,
            |_, _, movement, _, _| {
                let secondary = movement.z != 0.0;
                crate::client::ProjectileBlockMovementClip {
                    movement: if secondary {
                        [0.0; 3]
                    } else {
                        [movement.x, movement.y, movement.z]
                    },
                    world_border_hit: false,
                    block_state_id: secondary.then_some(1),
                }
            },
            |_, _| true,
        );
        assert!(obstructed[&70].entity_hits.is_empty());
    }

    #[test]
    fn abstract_arrow_block_hit_mutates_exact_saveable_client_state() {
        let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        )
        .unwrap();
        let mut metadata = EntityMetadata::default();
        set_entity_data_byte(&mut metadata, accessors.flags_data_id, accessors.crit_mask);
        set_entity_data_byte(&mut metadata, accessors.pierce_level_data_id, 4);
        let mut projectile = test_arrow_lifecycle();
        let mut runtime = test_arrow_runtime(72);
        runtime.piercing_ignore_entity_ids.extend([80, 81]);
        let mut animations = EntityClientAnimations::default();
        let mut transform = test_transform(EntityVec3 {
            x: 1.0,
            y: -0.25,
            z: 0.0,
        });
        let collision = EntityAbstractArrowCollisionFacts {
            initial_block_state_id: None,
            endpoint: EntityVec3 {
                x: 1.75,
                y: 63.75,
                z: -2.0,
            },
            world_border_hit: false,
            block_hit: Some(EntityAbstractArrowBlockHit {
                location: EntityVec3 {
                    x: 1.75,
                    y: 63.75,
                    z: -2.0,
                },
                block_state_id: 9,
            }),
            entity_hits: Vec::new(),
        };
        let step = advance_abstract_arrow(
            72,
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
            &mut metadata,
            &mut animations,
            &mut transform,
            accessors,
            false,
            Some(&collision),
            &mut Vec::new(),
        );
        assert_eq!(step.collision, Some(collision.clone()));
        apply_abstract_arrow_endpoint_hit(
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
            &mut metadata,
            &mut projectile,
            &mut runtime,
            &mut animations,
            &mut transform,
            accessors,
            &collision,
        );
        assert_eq!(projectile.arrow_last_block_state_id, Some(9));
        assert!(projectile.arrow_hit_ground_sound_is_arrow);
        assert!(runtime.piercing_ignore_entity_ids.is_empty());
        assert!(entity_data_bool(
            &metadata.data_values,
            accessors.in_ground_data_id,
            accessors.in_ground_default
        ));
        assert_eq!(
            entity_data_byte(
                &metadata.data_values,
                accessors.flags_data_id,
                accessors.flags_default
            ) & accessors.crit_mask,
            0
        );
        assert_eq!(
            entity_data_byte(
                &metadata.data_values,
                accessors.pierce_level_data_id,
                accessors.pierce_level_default
            ),
            0
        );
        assert_eq!(
            animations.animations.arrow_shake.unwrap().shake_time,
            7,
            "onHitBlock installs shake after this tick's decrement"
        );
        assert_vec3_near(
            transform.position,
            EntityVec3 {
                x: 1.75 - f64::from(0.05_f32),
                y: 63.75 + f64::from(0.05_f32),
                z: -2.0,
            },
        );
        assert_eq!(transform.delta_movement, EntityVec3::default());
    }

    #[test]
    fn trident_initial_block_containment_enters_ground_branch_on_the_same_tick() {
        let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        )
        .unwrap();
        let mut metadata = EntityMetadata::default();
        let mut animations = EntityClientAnimations::default();
        let mut transform = test_transform(EntityVec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let collision = EntityAbstractArrowCollisionFacts {
            initial_block_state_id: Some(9),
            endpoint: transform.position,
            world_border_hit: false,
            block_hit: None,
            entity_hits: Vec::new(),
        };
        let step = advance_abstract_arrow(
            720,
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
            &mut metadata,
            &mut animations,
            &mut transform,
            accessors,
            false,
            Some(&collision),
            &mut Vec::new(),
        );

        assert!(step.trident_initial_containment_grounded);
        assert!(step.finish.is_none());
        assert_eq!(transform.delta_movement, EntityVec3::default());
        assert!(entity_data_bool(
            &metadata.data_values,
            accessors.in_ground_data_id,
            accessors.in_ground_default,
        ));
        assert_eq!(animations.animations.arrow_shake.unwrap().shake_time, 6);
    }

    #[test]
    fn abstract_arrow_world_border_hit_reverses_before_inertia_and_gravity() {
        let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ARROW_ID,
        )
        .unwrap();
        let mut metadata = EntityMetadata::default();
        let mut projectile = test_arrow_lifecycle();
        let mut runtime = test_arrow_runtime(721);
        let mut animations = EntityClientAnimations::default();
        let mut transform = test_transform(EntityVec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let collision = EntityAbstractArrowCollisionFacts {
            initial_block_state_id: None,
            endpoint: EntityVec3 {
                x: 1.5,
                y: 64.0,
                z: -2.0,
            },
            world_border_hit: true,
            block_hit: None,
            entity_hits: Vec::new(),
        };
        let step = advance_abstract_arrow(
            721,
            VANILLA_ENTITY_TYPE_ARROW_ID,
            &mut metadata,
            &mut animations,
            &mut transform,
            accessors,
            false,
            Some(&collision),
            &mut Vec::new(),
        );
        apply_abstract_arrow_endpoint_hit(
            VANILLA_ENTITY_TYPE_ARROW_ID,
            &mut metadata,
            &mut projectile,
            &mut runtime,
            &mut animations,
            &mut transform,
            accessors,
            &collision,
        );
        finish_abstract_arrow(&metadata, &mut transform, step.finish.unwrap(), accessors);

        assert_vec3_near(
            transform.position,
            EntityVec3 {
                x: 1.5,
                y: 64.0,
                z: -2.0,
            },
        );
        assert_vec3_near(
            transform.delta_movement,
            EntityVec3 {
                x: -0.1 * f64::from(0.99_f32),
                y: -0.05,
                z: 0.0,
            },
        );
        assert_eq!(projectile.arrow_last_block_state_id, None);
    }

    #[test]
    fn abstract_arrow_piercing_limit_discards_after_level_plus_one_targets() {
        let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ARROW_ID,
        )
        .unwrap();
        let mut metadata = EntityMetadata::default();
        set_entity_data_byte(&mut metadata, accessors.pierce_level_data_id, 1);
        let mut projectile = test_arrow_lifecycle();
        let mut runtime = test_arrow_runtime(73);
        let mut animations = EntityClientAnimations::default();
        let mut transform = test_transform(EntityVec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        });
        let entity_hits = (80..83)
            .map(|entity_id| EntityAbstractArrowEntityHit {
                entity_id,
                incarnation: EntityIncarnation(entity_id as u64),
                location: EntityVec3 {
                    x: f64::from(entity_id - 79),
                    y: 64.0,
                    z: -2.0,
                },
                target_position: EntityVec3 {
                    x: f64::from(entity_id - 79),
                    y: 64.0,
                    z: -2.0,
                },
                deflects_projectiles: false,
                redirectable_projectile: false,
                hurt_client: true,
            })
            .collect();
        let collision = EntityAbstractArrowCollisionFacts {
            initial_block_state_id: None,
            endpoint: EntityVec3 {
                x: 2.0,
                y: 64.0,
                z: -2.0,
            },
            world_border_hit: false,
            block_hit: None,
            entity_hits,
        };
        let step = advance_abstract_arrow(
            73,
            VANILLA_ENTITY_TYPE_ARROW_ID,
            &mut metadata,
            &mut animations,
            &mut transform,
            accessors,
            false,
            Some(&collision),
            &mut Vec::new(),
        );
        let collision_step = advance_abstract_arrow_collision_after_first_effect(
            VANILLA_ENTITY_TYPE_ARROW_ID,
            &mut metadata,
            &mut projectile,
            &mut runtime,
            &mut transform,
            accessors,
            step.collision.as_ref().unwrap(),
            None,
        );
        assert!(collision_step.discarded);
        assert_eq!(runtime.piercing_ignore_entity_ids, vec![80, 81]);
        assert_vec3_near(
            transform.position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        );
    }

    #[test]
    fn abstract_arrow_client_hurt_discards_and_motion_correction_releases_ground_state() {
        let mut store = WorldStore::new();
        let mut arrow = projectile(
            74,
            VANILLA_ENTITY_TYPE_ARROW_ID,
            ProtocolVec3d {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
        );
        arrow.position = ProtocolVec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        };
        store.apply_add_entity(arrow.clone());
        let accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ARROW_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 74,
            values: vec![EntityDataValue {
                data_id: accessors.in_ground_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        assert!(store.apply_set_entity_motion(ProtocolSetEntityMotion {
            id: 74,
            delta_movement: ProtocolVec3d {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
        }));
        assert!(!store
            .entities
            .metadata_bool_for_entity(74, accessors.in_ground_data_id, accessors.in_ground_default,)
            .unwrap());

        let mut boat = projectile(
            75,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            ProtocolVec3d::default(),
        );
        boat.position = ProtocolVec3d {
            x: 1.5,
            y: 64.0,
            z: 0.0,
        };
        store.apply_add_entity(boat);
        store.advance_entity_client_animations(1);
        assert!(
            store.probe_entity(74).is_none(),
            "VehicleEntity.hurtClient accepts the simulated arrow hit, so a non-piercing arrow discards locally"
        );
        assert!(store.probe_entity(75).is_some());
    }

    #[test]
    fn item_entity_projectile_hurt_respects_effective_damage_resistant_component() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            76,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            ProtocolVec3d::default(),
        ));
        set_nonempty_item_stack(&mut store, 76);
        assert!(store.item_entity_accepts_projectile_hurt_client(76, "minecraft:arrow"));

        let arrow_damage_type_id = store
            .registry_holder_id_by_resource_id("minecraft:damage_type", "minecraft:arrow")
            .unwrap();
        let mut component_patch = bbb_protocol::packets::DataComponentPatchSummary::default();
        component_patch.added = 1;
        component_patch.added_type_ids = vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        ];
        component_patch.damage_resistant_type_registry_ids = vec![arrow_damage_type_id];
        let data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ITEM_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 76,
            values: vec![EntityDataValue {
                data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                value: EntityDataValueKind::ItemStack(ItemStackSummary {
                    item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                    count: 1,
                    component_patch,
                }),
            }],
        }));
        assert!(!store.item_entity_accepts_projectile_hurt_client(76, "minecraft:arrow"));
        assert!(store.item_entity_accepts_projectile_hurt_client(76, "minecraft:trident"));
    }

    #[test]
    fn abstract_arrow_hit_aim_deflects_redirectable_projectile_to_owner_look() {
        let mut store = WorldStore::new();
        let mut owner = projectile(77, VANILLA_ENTITY_TYPE_PLAYER_ID, ProtocolVec3d::default());
        owner.position = ProtocolVec3d {
            x: -5.0,
            y: 64.0,
            z: 0.0,
        };
        owner.x_rot = -10.0;
        owner.y_rot = 45.0;
        store.apply_add_entity(owner);

        let mut fireball = projectile(
            78,
            VANILLA_ENTITY_TYPE_FIREBALL_ID,
            ProtocolVec3d::default(),
        );
        fireball.position = ProtocolVec3d {
            x: 1.5,
            y: 64.0,
            z: 0.0,
        };
        store.apply_add_entity(fireball);

        let mut arrow = projectile(
            79,
            VANILLA_ENTITY_TYPE_ARROW_ID,
            ProtocolVec3d {
                x: 2.0,
                y: 0.0,
                z: 0.0,
            },
        );
        arrow.position = ProtocolVec3d {
            x: 0.0,
            y: 64.0,
            z: 0.0,
        };
        arrow.data = 77;
        store.apply_add_entity(arrow);

        store.advance_entity_client_animations(1);

        assert_vec3_near(
            store.probe_entity_transform(78).unwrap().delta_movement,
            calculate_view_vector(-10.0, 45.0),
        );
        assert!(
            store.probe_entity_transform(79).unwrap().delta_movement.x < 0.0,
            "the redirectable target still rejects client damage, so the source arrow reverses"
        );
    }

    #[test]
    fn abstract_arrow_runtime_round_trip_rebinds_deflector_and_preserves_private_state() {
        let mut store = WorldStore::new();
        let breeze = projectile(80, VANILLA_ENTITY_TYPE_BREEZE_ID, ProtocolVec3d::default());
        let breeze_uuid = breeze.uuid;
        store.apply_add_entity(breeze);
        store.apply_add_entity(projectile(
            81,
            VANILLA_ENTITY_TYPE_ARROW_ID,
            ProtocolVec3d::default(),
        ));

        let mut arrow = store.probe_entity(81).unwrap();
        let runtime = arrow.abstract_arrow_runtime.as_mut().unwrap();
        runtime.random_seed = 0x1234_5678_9abc;
        runtime.last_deflected_by = Some(crate::entities::EntityRuntimeReferenceState {
            entity_id: 80,
            uuid: breeze_uuid,
        });
        runtime.piercing_ignore_entity_ids = vec![82, 83];
        store.entities.insert_or_replace(arrow.clone());

        let breeze_incarnation = store
            .entities
            .projectile_targets(&HashSet::new())
            .into_iter()
            .find(|target| target.entity_id == 80)
            .unwrap()
            .incarnation;
        let runtime = store.entities.abstract_arrow_runtime(81).unwrap();
        assert_eq!(
            runtime.last_deflected_by,
            Some(EntityInstanceReference {
                entity_id: 80,
                incarnation: breeze_incarnation,
            })
        );
        assert_eq!(store.probe_entity(81).unwrap(), arrow);
        assert_eq!(
            store.publication_snapshot().probe_entity(81).unwrap(),
            arrow
        );

        let encoded = serde_json::to_value(&store).unwrap();
        let restored: WorldStore = serde_json::from_value(encoded).unwrap();
        assert_eq!(restored.probe_entity(81).unwrap(), arrow);
        let restored_breeze_incarnation = restored
            .entities
            .projectile_targets(&HashSet::new())
            .into_iter()
            .find(|target| target.entity_id == 80)
            .unwrap()
            .incarnation;
        let restored_runtime = restored.entities.abstract_arrow_runtime(81).unwrap();
        assert_eq!(
            restored_runtime.last_deflected_by,
            Some(EntityInstanceReference {
                entity_id: 80,
                incarnation: restored_breeze_incarnation,
            })
        );
        assert!(restored_runtime.pending_last_deflected_by.is_none());
        assert_eq!(restored_runtime.random_seed, 0x1234_5678_9abc);
        assert_eq!(restored_runtime.piercing_ignore_entity_ids, vec![82, 83]);
    }

    #[test]
    fn loyalty_trident_returns_to_live_owner_once_and_preserves_old_generation() {
        let mut store = WorldStore::new();
        let mut owner = projectile(90, VANILLA_ENTITY_TYPE_PLAYER_ID, ProtocolVec3d::default());
        owner.position = ProtocolVec3d {
            x: 10.0,
            y: 64.0,
            z: 0.0,
        };
        store.apply_add_entity(owner);
        let mut trident = projectile(
            91,
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
            ProtocolVec3d {
                x: 0.2,
                y: 0.1,
                z: -0.3,
            },
        );
        trident.data = 90;
        store.apply_add_entity(trident);
        let arrow_accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        )
        .unwrap();
        let trident_accessors = vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 91,
            values: vec![
                EntityDataValue {
                    data_id: arrow_accessors.flags_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(arrow_accessors.no_physics_mask),
                },
                EntityDataValue {
                    data_id: trident_accessors.loyalty_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(2),
                },
            ],
        }));
        let generation_a = store.publication_snapshot();
        let owner_eye_height = f64::from(store.probe_entity_camera_pose(90).unwrap().eye_height);
        let original_position = EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        };
        let to_owner = EntityVec3 {
            x: 9.0,
            y: owner_eye_height,
            z: 2.0,
        };
        let adjusted_position = EntityVec3 {
            y: original_position.y + to_owner.y * 0.03,
            ..original_position
        };
        let return_delta = add(
            scale(
                EntityVec3 {
                    x: 0.2,
                    y: 0.1,
                    z: -0.3,
                },
                0.95,
            ),
            scale(normalize(to_owner), 0.1),
        );

        store.advance_entity_client_animations(1);

        let transform = store.probe_entity_transform(91).unwrap();
        assert_vec3_near(transform.position, add(adjusted_position, return_delta));
        assert_vec3_near(
            transform.delta_movement,
            scale(return_delta, f64::from(0.99_f32)),
        );
        let lifecycle = store
            .probe_entity(91)
            .unwrap()
            .projectile_lifecycle
            .unwrap();
        assert_eq!(lifecycle.trident_return_ticks, 1);
        assert!(matches!(
            store
                .drain_entity_client_movement_effect_states()
                .collect::<Vec<_>>()
                .as_slice(),
            [EntityClientMovementEffectState::Sound(EntityClientMovementSoundState {
                entity_id: 91,
                kind: EntityClientMovementSoundKind::TridentReturn,
                position,
            })] if *position == adjusted_position
        ));
        assert_eq!(
            generation_a
                .probe_entity(91)
                .unwrap()
                .projectile_lifecycle
                .unwrap()
                .trident_return_ticks,
            0
        );
        assert_eq!(
            generation_a.probe_entity_transform(91).unwrap().position,
            original_position
        );

        store.advance_entity_client_animations(1);
        assert!(store
            .drain_entity_client_movement_effect_states()
            .next()
            .is_none());
        assert_eq!(
            store
                .probe_entity(91)
                .unwrap()
                .projectile_lifecycle
                .unwrap()
                .trident_return_ticks,
            2
        );
    }

    #[test]
    fn silent_returning_trident_suppresses_sound_but_advances_return_counter() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            92,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            ProtocolVec3d::default(),
        ));
        let mut trident = projectile(93, VANILLA_ENTITY_TYPE_TRIDENT_ID, ProtocolVec3d::default());
        trident.data = 92;
        store.apply_add_entity(trident);
        let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
        let arrow_accessors = vanilla_entity_metadata_abstract_arrow_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        )
        .unwrap();
        let trident_accessors = vanilla_entity_metadata_thrown_trident_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_TRIDENT_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 93,
            values: vec![
                EntityDataValue {
                    data_id: entity_accessors.silent_data_id,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: EntityDataValueKind::Boolean(true),
                },
                EntityDataValue {
                    data_id: arrow_accessors.flags_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(arrow_accessors.no_physics_mask),
                },
                EntityDataValue {
                    data_id: trident_accessors.loyalty_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(1),
                },
            ],
        }));

        store.advance_entity_client_animations(1);

        assert!(store
            .drain_entity_client_movement_effect_states()
            .next()
            .is_none());
        assert_eq!(
            store
                .probe_entity(93)
                .unwrap()
                .projectile_lifecycle
                .unwrap()
                .trident_return_ticks,
            1
        );
    }

    #[test]
    fn falling_block_and_tnt_apply_gravity_drag_and_client_fuse_tick() {
        let mut store = WorldStore::new();
        for (id, entity_type_id) in [
            (59, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID),
            (60, VANILLA_ENTITY_TYPE_TNT_ID),
        ] {
            store.apply_add_entity(projectile(
                id,
                entity_type_id,
                ProtocolVec3d {
                    x: 0.5,
                    y: 0.2,
                    z: -0.25,
                },
            ));
        }
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let requested = EntityVec3 {
            x: 0.5,
            y: 0.16,
            z: -0.25,
        };
        for id in [59, 60] {
            let transform = store.probe_entity_transform(id).unwrap();
            assert_vec3_near(
                transform.position,
                EntityVec3 {
                    x: 1.5,
                    y: 64.16,
                    z: -2.25,
                },
            );
            assert_vec3_near(
                transform.delta_movement,
                scale(requested, f64::from(0.98_f32)),
            );
            assert_eq!(transform.on_ground, Some(false));
        }
        assert_eq!(
            store.primed_tnt_fuse_remaining_in_ticks(60, 1.0),
            Some(79.0)
        );
        assert_eq!(
            store.primed_tnt_smoke_particle_states(),
            vec![super::super::PrimedTntSmokeParticleState {
                entity_id: 60,
                position: EntityVec3 {
                    x: 1.5,
                    y: 64.16,
                    z: -2.25,
                },
            }]
        );
        assert_eq!(
            generation_a.primed_tnt_fuse_remaining_in_ticks(60, 1.0),
            Some(80.0),
            "retained publication keeps the pre-tick fuse"
        );
    }

    #[test]
    fn item_entity_applies_client_gravity_drag_and_retained_generation_isolation() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            63,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            ProtocolVec3d {
                x: 0.5,
                y: 0.2,
                z: -0.25,
            },
        ));
        set_nonempty_item_stack(&mut store, 63);
        assert_eq!(
            store
                .entities
                .client_movement_fluid_inputs(&HashSet::new())
                .len(),
            1
        );
        assert_eq!(
            store
                .entities
                .client_movement_collision_inputs(
                    &HashMap::new(),
                    &HashMap::new(),
                    &HashMap::new(),
                    &HashSet::new(),
                    0.0023333333333333335,
                    0,
                )
                .len(),
            1
        );
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let requested = EntityVec3 {
            x: 0.5,
            y: 0.16,
            z: -0.25,
        };
        let transform = store.probe_entity_transform(63).unwrap();
        assert_vec3_near(
            transform.position,
            EntityVec3 {
                x: 1.5,
                y: 64.16,
                z: -2.25,
            },
        );
        assert_vec3_near(
            transform.delta_movement,
            EntityVec3 {
                x: requested.x * f64::from(0.98_f32),
                y: requested.y * f64::from(0.98_f32),
                z: requested.z * f64::from(0.98_f32),
            },
        );
        assert_eq!(
            generation_a.probe_entity_transform(63).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            "the retained item generation keeps its pre-tick transform"
        );
    }

    #[test]
    fn experience_orb_skips_first_client_tick_then_follows_nearby_player() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            64,
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            ProtocolVec3d::default(),
        ));
        let mut player = projectile(65, VANILLA_ENTITY_TYPE_PLAYER_ID, ProtocolVec3d::default());
        player.position.x = 5.0;
        store.apply_add_entity(player);

        store.advance_entity_client_animations(1);
        assert_eq!(
            store.probe_entity_transform(64).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            "Vanilla clears firstTick without running ExperienceOrb base motion"
        );
        assert_eq!(
            store
                .entities
                .client_movement_collision_inputs(
                    &HashMap::new(),
                    &HashMap::new(),
                    &HashMap::new(),
                    &HashSet::new(),
                    0.0023333333333333335,
                    0,
                )
                .len(),
            1,
            "the second orb tick must enter collision/world-fact extraction"
        );
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let attraction = EntityVec3 {
            x: 4.0,
            y: f64::from(1.62_f32) * 0.5,
            z: 0.0,
        };
        let power = 1.0 - length_squared(attraction).sqrt() / 8.0;
        let requested = add(
            EntityVec3 {
                x: 0.0,
                y: -0.03,
                z: 0.0,
            },
            scale(normalize(attraction), power * power * 0.1),
        );
        let transform = store.probe_entity_transform(64).unwrap();
        assert_vec3_near(
            transform.position,
            add(
                generation_a.probe_entity_transform(64).unwrap().position,
                requested,
            ),
        );
        assert_vec3_near(
            transform.delta_movement,
            scale(requested, f64::from(0.98_f32)),
        );
        assert_eq!(
            generation_a.probe_entity_transform(64).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            "the retained orb generation remains isolated from follow motion"
        );
        assert_eq!(
            generation_a
                .entities
                .experience_orb_movement(64)
                .unwrap()
                .following_player_uuid,
            None
        );
        assert_eq!(
            store
                .entities
                .experience_orb_movement(64)
                .unwrap()
                .following_player_uuid,
            Some(Uuid::from_u128(65)),
            "the live generation alone retains the selected Player instance"
        );
    }

    #[test]
    fn experience_orb_target_selection_preserves_vanilla_retention_and_range_rules() {
        let origin = EntityVec3::default();
        let dead_current = EntityExperienceOrbPlayerTarget {
            entity_id: 1,
            uuid: Uuid::from_u128(1),
            position: EntityVec3 {
                x: 8.0,
                y: 0.0,
                z: 0.0,
            },
            attraction_position: EntityVec3::default(),
            is_dead_or_dying: true,
        };
        let first_nearest = EntityExperienceOrbPlayerTarget {
            entity_id: 2,
            uuid: Uuid::from_u128(2),
            position: EntityVec3 {
                x: 3.0,
                y: 0.0,
                z: 0.0,
            },
            attraction_position: EntityVec3::default(),
            is_dead_or_dying: false,
        };
        let equal_nearest = EntityExperienceOrbPlayerTarget {
            entity_id: 3,
            uuid: Uuid::from_u128(3),
            ..first_nearest
        };

        assert_eq!(
            select_experience_orb_player_target(
                origin,
                Some(dead_current.uuid),
                &[dead_current, first_nearest],
            ),
            Some(dead_current),
            "an existing non-spectator target remains retained at exactly 8 blocks, even if dead"
        );
        assert_eq!(
            select_experience_orb_player_target(
                origin,
                None,
                &[dead_current, first_nearest, equal_nearest],
            ),
            Some(first_nearest),
            "a new target excludes dead/exact-range players and preserves first-player ties"
        );
    }

    #[test]
    fn fishing_hook_free_flight_uses_client_gravity_inertia_and_generation_isolation() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            80,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            ProtocolVec3d::default(),
        ));
        let mut hook = projectile(
            81,
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            ProtocolVec3d {
                x: 0.5,
                y: 0.2,
                z: -0.25,
            },
        );
        hook.data = 80;
        store.apply_add_entity(hook);
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let moved = EntityVec3 {
            x: 0.5,
            y: 0.17,
            z: -0.25,
        };
        assert_vec3_near(
            store.probe_entity_transform(81).unwrap().position,
            add(
                generation_a.probe_entity_transform(81).unwrap().position,
                moved,
            ),
        );
        assert_vec3_near(
            store.probe_entity_transform(81).unwrap().delta_movement,
            scale(moved, 0.92),
        );
        assert_eq!(
            generation_a
                .entities
                .fishing_hook_movement(81)
                .unwrap()
                .state,
            EntityFishingHookState::Flying
        );
        assert_eq!(
            generation_a.probe_entity_transform(81).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            }
        );
    }

    #[test]
    fn fishing_hook_batched_entity_ticks_use_each_pre_level_tick_game_time_for_synchronized_rng() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            85,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            ProtocolVec3d::default(),
        ));
        let mut hook = projectile(
            86,
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            ProtocolVec3d::default(),
        );
        hook.data = 85;
        store.apply_add_entity(hook);

        store.advance_entity_client_animations(3);

        assert_eq!(
            store
                .entities
                .fishing_hook_movement(86)
                .unwrap()
                .synchronized_random_seed,
            java_random_seed((86_u64 ^ 2_u64) as i64),
        );
    }

    #[test]
    fn client_level_catch_up_interleaves_entity_ticks_with_moving_world_border() {
        let mut store = WorldStore::new();
        store.apply_set_border_lerp_size(ProtocolSetBorderLerpSize {
            old_size: 10.0,
            new_size: 4.0,
            lerp_time: 4,
        });
        store.apply_add_entity(projectile(
            87,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            ProtocolVec3d::default(),
        ));
        let mut hook = projectile(
            88,
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            ProtocolVec3d {
                x: 0.82,
                y: 0.0,
                z: 0.0,
            },
        );
        hook.position.x = 2.0;
        hook.data = 87;
        store.apply_add_entity(hook);

        let (_, running_ticks) = store.advance_client_level_entity_ticks_with_observation(3);

        let transform = store.probe_entity_transform(88).unwrap();
        assert_eq!(running_ticks, 3);
        assert_eq!(store.world_border().current_size, 5.5);
        assert!(
            transform.position.x < 3.5744 && transform.delta_movement.x < 0.0,
            "the third entity tick must collide with the border advanced after the first two ticks: {transform:?}"
        );
    }

    #[test]
    fn fishing_hook_metadata_update_retains_exact_target_and_biting_rng_order() {
        let mut store = WorldStore::new();
        for id in [82, 83] {
            store.apply_add_entity(projectile(
                id,
                VANILLA_ENTITY_TYPE_PLAYER_ID,
                ProtocolVec3d::default(),
            ));
        }
        let mut hook = projectile(
            84,
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            ProtocolVec3d {
                x: 0.25,
                y: 0.5,
                z: -0.75,
            },
        );
        hook.data = 82;
        store.apply_add_entity(hook);
        let generation_a = store.publication_snapshot();
        let mut expected_seed = store
            .entities
            .fishing_hook_movement(84)
            .unwrap()
            .synchronized_random_seed;
        let random = java_random_next_float(&mut expected_seed);
        let expected_y = f64::from(-0.4_f32 * (0.6_f32 + random * 0.4_f32));

        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 84,
            values: vec![
                EntityDataValue {
                    data_id: crate::entities::VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    value: EntityDataValueKind::Int(84),
                },
                EntityDataValue {
                    data_id: crate::entities::VANILLA_FISHING_HOOK_BITING_DATA_ID,
                    serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: EntityDataValueKind::Boolean(true),
                },
            ],
        }));
        let movement = store.entities.fishing_hook_movement(84).unwrap();
        assert_eq!(movement.hooked_entity_uuid, Some(Uuid::from_u128(83)));
        assert!(movement.biting);
        assert_eq!(movement.synchronized_random_seed, expected_seed);
        assert_eq!(
            store.probe_entity_transform(84).unwrap().delta_movement.y,
            expected_y
        );
        assert_eq!(
            generation_a
                .entities
                .fishing_hook_movement(84)
                .unwrap()
                .hooked_entity_uuid,
            None,
            "the retained generation must not observe metadata-derived target identity"
        );

        store.advance_entity_client_animations(1);
        assert_eq!(
            store.entities.fishing_hook_movement(84).unwrap().state,
            EntityFishingHookState::HookedInEntity
        );
        let transition_position = store.probe_entity_transform(84).unwrap().position;
        store.advance_entity_client_animations(1);
        let hooked_position = store.probe_entity_transform(84).unwrap().position;
        assert_eq!(hooked_position.x, 1.0);
        assert_eq!(hooked_position.z, -2.0);
        assert!(hooked_position.y > transition_position.y);

        store.apply_remove_entities(RemoveEntities {
            entity_ids: vec![83],
        });
        let reused_id = projectile(83, VANILLA_ENTITY_TYPE_ZOMBIE_ID, ProtocolVec3d::default());
        store.apply_add_entity(reused_id);
        store.advance_entity_client_animations(1);
        assert_eq!(
            store.entities.fishing_hook_movement(84).unwrap().state,
            EntityFishingHookState::Flying
        );
        assert_eq!(
            store
                .entities
                .metadata_int_for_entity(
                    84,
                    crate::entities::VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID,
                    crate::entities::VANILLA_FISHING_HOOK_DEFAULT_HOOKED_ENTITY,
                )
                .unwrap(),
            0,
            "a same-id/same-UUID replacement cannot retarget the old hookedIn object"
        );
    }

    #[test]
    fn fishing_hook_entity_hit_uses_margin_and_owner_root_gate() {
        let target = EntityProjectileTarget {
            entity_id: 91,
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            uuid: uuid::Uuid::from_u128(91),
            root_vehicle_id: 91,
            position: EntityVec3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            bounds: EntityPickBoundsState::from_base_size(0.6, 1.8, 0.1),
            alive: true,
            pickable: true,
            deflects_projectiles: false,
            redirectable_projectile: false,
            hurt_client: false,
            trident_hurt_client: false,
            hurt_client_rejects_ender_dragon_owner: false,
            incarnation: EntityIncarnation(91),
        };
        let from = EntityVec3 {
            x: 0.0,
            y: 0.9,
            z: 0.0,
        };
        let delta = EntityVec3 {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        };
        assert_eq!(
            fishing_hook_entity_hit(90, from, delta, Some(91), false, 1, None, &[target]),
            None,
            "a hook that has not left its owner root cannot hit that graph"
        );
        let hit =
            fishing_hook_entity_hit(90, from, delta, Some(91), true, 22, None, &[target]).unwrap();
        assert!((hit.scale - 0.2).abs() <= 1.0e-6, "{}", hit.scale);

        let deflecting = EntityProjectileTarget {
            deflects_projectiles: true,
            ..target
        };
        assert!(
            fishing_hook_entity_hit(90, from, delta, Some(91), true, 22, None, &[deflecting],)
                .unwrap()
                .reverse_deflection
                .is_some()
        );
        assert_eq!(
            fishing_hook_entity_hit(
                90,
                from,
                delta,
                Some(91),
                true,
                22,
                Some(deflecting.incarnation),
                &[deflecting],
            )
            .unwrap()
            .reverse_deflection,
            None,
            "Projectile.lastDeflectedBy suppresses a repeated reverse from the same instance"
        );
        let replacement = EntityProjectileTarget {
            incarnation: EntityIncarnation(92),
            ..deflecting
        };
        assert!(
            fishing_hook_entity_hit(
                90,
                from,
                delta,
                Some(91),
                true,
                22,
                Some(deflecting.incarnation),
                &[replacement],
            )
            .unwrap()
            .reverse_deflection
            .is_some(),
            "a same-id/same-UUID replacement is a new Vanilla entity object"
        );
    }

    #[test]
    fn fishing_hook_projectile_targets_include_dragon_parts_but_not_parent() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            100,
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
            ProtocolVec3d::default(),
        ));

        let targets = store.entities.projectile_targets(&HashSet::new());
        let parent = targets
            .iter()
            .find(|target| target.entity_id == 100)
            .unwrap();
        assert!(!parent.pickable);
        for part_id in 101..=108 {
            let part = targets
                .iter()
                .find(|target| target.entity_id == part_id)
                .unwrap_or_else(|| panic!("missing EnderDragonPart {part_id}"));
            assert!(part.pickable);
            assert!(part.alive);
            assert_eq!(part.root_vehicle_id, part_id);
        }
    }

    #[test]
    fn entity_incarnation_changes_on_same_identity_readd_and_is_snapshot_isolated() {
        let mut store = WorldStore::new();
        let breeze = projectile(109, VANILLA_ENTITY_TYPE_BREEZE_ID, ProtocolVec3d::default());
        store.apply_add_entity(breeze.clone());
        let first = store
            .entities
            .projectile_targets(&HashSet::new())
            .into_iter()
            .find(|target| target.entity_id == 109)
            .unwrap()
            .incarnation;
        let generation_a = store.publication_snapshot();

        store.apply_add_entity(breeze);
        let replacement = store
            .entities
            .projectile_targets(&HashSet::new())
            .into_iter()
            .find(|target| target.entity_id == 109)
            .unwrap()
            .incarnation;
        let retained = generation_a
            .entities
            .projectile_targets(&HashSet::new())
            .into_iter()
            .find(|target| target.entity_id == 109)
            .unwrap()
            .incarnation;

        assert_ne!(replacement, first);
        assert_eq!(retained, first);

        store.apply_add_entity(projectile(
            110,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            ProtocolVec3d::default(),
        ));
        let player_before_respawn = store.entities.identity(110).unwrap().incarnation;
        store
            .entities
            .rebuild_local_player_entity_for_respawn(110, false, false)
            .unwrap();
        assert_ne!(
            store.entities.identity(110).unwrap().incarnation,
            player_before_respawn,
            "respawn creates a new Vanilla entity object even when id and UUID stay stable"
        );
    }

    #[test]
    fn fishing_hook_world_border_hit_reverses_and_damps_before_gravity() {
        let mut store = WorldStore::new();
        store.apply_set_border_size(ProtocolSetBorderSize { size: 4.0 });
        store.apply_add_entity(projectile(
            109,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            ProtocolVec3d::default(),
        ));
        let mut hook = projectile(
            110,
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            ProtocolVec3d {
                x: 10.0,
                y: 0.0,
                z: 0.0,
            },
        );
        hook.data = 109;
        store.apply_add_entity(hook);
        let mut expected_seed = store
            .entities
            .fishing_hook_movement(110)
            .unwrap()
            .random_seed;
        let _ = java_random_next_float(&mut expected_seed);

        store.advance_entity_client_animations(1);

        let transform = store.probe_entity_transform(110).unwrap();
        assert_vec3_near(
            transform.position,
            EntityVec3 {
                x: 0.0,
                y: 63.97,
                z: -2.0,
            },
        );
        assert_vec3_near(
            transform.delta_movement,
            EntityVec3 {
                x: -0.92,
                y: -0.0276,
                z: 0.0,
            },
        );
        assert_eq!(
            store
                .entities
                .fishing_hook_movement(110)
                .unwrap()
                .random_seed,
            expected_seed,
        );
        assert!(store
            .drain_entity_client_movement_effect_states()
            .next()
            .is_none());
    }

    #[test]
    fn fishing_hook_breeze_hit_reverses_and_emits_ordered_deflect_sound() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            111,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            ProtocolVec3d::default(),
        ));
        let mut hook = projectile(
            112,
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            ProtocolVec3d {
                x: 4.0,
                y: 0.0,
                z: 0.0,
            },
        );
        hook.data = 111;
        store.apply_add_entity(hook);
        let mut breeze = projectile(113, VANILLA_ENTITY_TYPE_BREEZE_ID, ProtocolVec3d::default());
        breeze.position.x = 2.0;
        store.apply_add_entity(breeze);
        let breeze_incarnation = store
            .entities
            .projectile_targets(&HashSet::new())
            .into_iter()
            .find(|target| target.entity_id == 113)
            .unwrap()
            .incarnation;
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let transform = store.probe_entity_transform(112).unwrap();
        assert_vec3_near(
            transform.position,
            EntityVec3 {
                x: -1.0,
                y: 63.97,
                z: -2.0,
            },
        );
        assert_vec3_near(
            transform.delta_movement,
            EntityVec3 {
                x: -1.84,
                y: -0.0276,
                z: 0.0,
            },
        );
        assert_eq!(
            generation_a
                .entities
                .fishing_hook_movement(112)
                .unwrap()
                .last_deflected_by,
            None,
        );
        assert_eq!(
            store
                .entities
                .fishing_hook_movement(112)
                .unwrap()
                .last_deflected_by,
            Some(breeze_incarnation),
        );
        assert_eq!(
            store
                .drain_entity_client_movement_effect_states()
                .collect::<Vec<_>>(),
            vec![EntityClientMovementEffectState::Sound(
                EntityClientMovementSoundState {
                    entity_id: 113,
                    kind: EntityClientMovementSoundKind::BreezeDeflect,
                    position: EntityVec3 {
                        x: 2.0,
                        y: 64.0,
                        z: -2.0,
                    },
                },
            )],
        );
    }

    #[test]
    fn fishing_hook_water_transition_and_bobbing_preserve_random_call_order() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            92,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            ProtocolVec3d::default(),
        ));
        let mut hook = projectile(
            93,
            VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID,
            ProtocolVec3d {
                x: 0.3,
                y: -0.2,
                z: 0.6,
            },
        );
        hook.data = 92;
        store.apply_add_entity(hook);
        let identity = store.entities.identity(93).unwrap();
        let bounds = store
            .entities
            .client_movement_fluid_inputs(&HashSet::new())
            .into_iter()
            .find(|input| input.entity_id == 93)
            .unwrap()
            .bounds;
        let movement = store.entities.fishing_hook_movement(93).unwrap();
        let water = crate::client::EntityFluidContactState {
            block_fluid_kind: Some(crate::TerrainFluidKind::Water),
            block_fluid_height: 0.8,
            ..crate::client::EntityFluidContactState::default()
        };

        let (transition_delta, skipped, mut update, entity_hit, collision_delta) =
            store.entities.prepare_fishing_hook_movement(
                &identity,
                EntityVec3 {
                    x: 1.0,
                    y: 64.5,
                    z: -2.0,
                },
                EntityVec3 {
                    x: 0.3,
                    y: -0.2,
                    z: 0.6,
                },
                false,
                water,
                movement,
                7,
                bounds,
                1,
                &[],
                &HashSet::new(),
            );
        assert!(skipped);
        assert_eq!(update.movement.state, EntityFishingHookState::Bobbing);
        assert_eq!(entity_hit, None);
        assert_eq!(collision_delta, None);
        assert_vec3_near(
            transition_delta,
            EntityVec3 {
                x: 0.09,
                y: -0.04,
                z: 0.18,
            },
        );

        update.movement.biting = true;
        let mut expected_regular = update.movement.random_seed;
        let regular = java_random_next_float(&mut expected_regular);
        let force = 64.5 - 0.04 - 64.0 - 0.8;
        let mut expected_sync = java_random_seed((93_u64 ^ 7_u64) as i64);
        let sync_first = java_random_next_float(&mut expected_sync);
        let sync_second = java_random_next_float(&mut expected_sync);
        let expected = EntityVec3 {
            x: transition_delta.x * 0.9,
            y: transition_delta.y
                - force * f64::from(regular) * 0.2
                - 0.1 * f64::from(sync_first) * f64::from(sync_second),
            z: transition_delta.z * 0.9,
        };
        let (bobbing_delta, skipped, update, _, collision_delta) =
            store.entities.prepare_fishing_hook_movement(
                &identity,
                EntityVec3 {
                    x: 1.0,
                    y: 64.5,
                    z: -2.0,
                },
                transition_delta,
                false,
                water,
                update.movement,
                7,
                bounds,
                2,
                &[],
                &HashSet::new(),
            );
        assert!(!skipped);
        assert_eq!(collision_delta, None);
        assert_vec3_near(bobbing_delta, expected);
        assert_eq!(update.movement.random_seed, expected_regular);
        assert_eq!(update.movement.synchronized_random_seed, expected_sync);
    }

    #[test]
    fn item_and_orb_collision_responses_use_block_friction_and_vertical_bounce() {
        let collision = EntityClientMovementCollisionResult {
            pre_move_delta: EntityVec3 {
                x: 0.25,
                y: -0.1,
                z: -0.5,
            },
            requested_movement: EntityVec3 {
                x: 0.25,
                y: -0.1,
                z: -0.5,
            },
            clipped_movement: EntityVec3 {
                x: 0.25,
                y: 0.0,
                z: -0.5,
            },
            supporting_block: Some(BlockPos { x: 1, y: 63, z: -2 }),
            ground_friction: 0.98,
            movement_skipped: false,
            experience_orb_update: None,
            fishing_hook_update: None,
            fishing_hook_reverse_deflection: None,
        };
        let mut support = EntitySupportingBlock::default();
        let mut item = test_transform(EntityVec3::default());
        advance_item_entity(&mut item, &mut support, collision);
        assert_eq!(item.on_ground, Some(true));
        assert_eq!(support.main, collision.supporting_block);
        assert_vec3_near(
            item.delta_movement,
            EntityVec3 {
                x: 0.25 * f64::from(0.98_f32 * 0.98_f32),
                y: -0.1 * f64::from(0.98_f32) * -0.5,
                z: -0.5 * f64::from(0.98_f32 * 0.98_f32),
            },
        );

        let mut orb_movement = experience_orb_movement_from_uuid(Uuid::from_u128(4));
        let mut orb_support = EntitySupportingBlock::default();
        let mut orb = test_transform(EntityVec3::default());
        advance_experience_orb(&mut orb_movement, &mut orb, &mut orb_support, collision);
        assert_eq!(orb.on_ground, Some(true));
        assert_vec3_near(
            orb.delta_movement,
            EntityVec3 {
                x: 0.25 * f64::from(0.98_f32 * 0.98_f32),
                y: 0.04,
                z: -0.5 * f64::from(0.98_f32 * 0.98_f32),
            },
        );
    }

    #[test]
    fn same_id_readd_regenerates_exact_world_fact_family_markers() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            66,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            ProtocolVec3d::default(),
        ));
        assert_eq!(
            store
                .entities
                .client_movement_fluid_inputs(&HashSet::new())
                .len(),
            1
        );

        store.apply_add_entity(projectile(
            66,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ProtocolVec3d::default(),
        ));
        assert!(store
            .entities
            .client_movement_fluid_inputs(&HashSet::new())
            .is_empty());
        assert!(store
            .entities
            .client_movement_collision_inputs(
                &HashMap::new(),
                &HashMap::new(),
                &HashMap::new(),
                &HashSet::new(),
                0.0023333333333333335,
                0,
            )
            .is_empty());

        store.apply_add_entity(projectile(
            66,
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            ProtocolVec3d::default(),
        ));
        assert_eq!(
            store
                .entities
                .experience_orb_player_targets(&HashSet::new())
                .len(),
            1
        );
    }

    #[test]
    fn item_water_and_experience_orb_lava_inputs_match_vanilla_float_paths() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            68,
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            ProtocolVec3d::default(),
        ));
        // Vanilla's first client orb tick only clears firstTick.
        store.advance_entity_client_animations(1);

        store.apply_add_entity(projectile(
            67,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            ProtocolVec3d {
                x: 0.5,
                y: 0.2,
                z: -0.25,
            },
        ));
        set_nonempty_item_stack(&mut store, 67);

        let mut fluid_by_id = HashMap::new();
        fluid_by_id.insert(
            67,
            crate::client::EntityFluidContactState {
                water_height: 0.2,
                ..crate::client::EntityFluidContactState::default()
            },
        );
        fluid_by_id.insert(
            68,
            crate::client::EntityFluidContactState {
                block_fluid_kind: Some(crate::TerrainFluidKind::Lava),
                ..crate::client::EntityFluidContactState::default()
            },
        );
        let inputs = store.entities.client_movement_collision_inputs(
            &fluid_by_id,
            &HashMap::new(),
            &HashMap::new(),
            &HashSet::new(),
            0.0023333333333333335,
            0,
        );
        let item = inputs.iter().find(|input| input.entity_id == 67).unwrap();
        assert_vec3_near(
            item.pre_move_delta,
            EntityVec3 {
                x: 0.5 * f64::from(0.99_f32),
                y: 0.2,
                z: -0.25 * f64::from(0.99_f32),
            },
        );

        let mut expected_seed = experience_orb_movement_from_uuid(Uuid::from_u128(68)).random_seed;
        let expected_orb = EntityVec3 {
            x: f64::from(
                (java_random_next_float(&mut expected_seed)
                    - java_random_next_float(&mut expected_seed))
                    * 0.2,
            ),
            y: f64::from(0.2_f32),
            z: f64::from(
                (java_random_next_float(&mut expected_seed)
                    - java_random_next_float(&mut expected_seed))
                    * 0.2,
            ),
        };
        let orb = inputs.iter().find(|input| input.entity_id == 68).unwrap();
        assert_eq!(orb.pre_move_delta, expected_orb);
        assert_eq!(
            orb.experience_orb_update.unwrap().random_seed,
            expected_seed
        );
    }

    #[test]
    fn item_world_collision_uses_generated_ice_friction_only_for_ground_motion() {
        let mut store = nonliving_collision_world();
        let ice = crate::registries::BlockStateRegistry::vanilla_26_1()
            .find_by_name_and_properties("minecraft:ice", &Default::default())
            .expect("vanilla ice block state")
            .id;
        assert!(
            store.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
                block_state_id: ice,
            })
        );
        let mut item = projectile(
            69,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            ProtocolVec3d {
                x: 0.25,
                y: -0.1,
                z: 0.0,
            },
        );
        item.position = ProtocolVec3d {
            x: 0.5,
            y: 1.05,
            z: 0.5,
        };
        store.apply_add_entity(item);
        set_nonempty_item_stack(&mut store, 69);

        store.advance_entity_client_animations(1);

        let transform = store.probe_entity_transform(69).unwrap();
        assert_eq!(transform.on_ground, Some(true));
        assert!((transform.position.y - 1.0).abs() <= 1.0e-9);
        assert_vec3_near(
            transform.delta_movement,
            EntityVec3 {
                x: 0.25 * f64::from(0.98_f32 * 0.98_f32),
                y: -0.14 * f64::from(0.98_f32) * -0.5,
                z: 0.0,
            },
        );
    }

    #[test]
    fn settled_item_preserves_vanilla_four_tick_movement_skip_schedule() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            70,
            VANILLA_ENTITY_TYPE_ITEM_ID,
            ProtocolVec3d::default(),
        ));
        set_nonempty_item_stack(&mut store, 70);
        store
            .entities
            .with_transform_mut(70, |transform| transform.on_ground = Some(true));

        store.advance_entity_client_animations(1);

        let first = store.probe_entity_transform(70).unwrap();
        assert_eq!(
            first.position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            }
        );
        assert_eq!(
            first.delta_movement,
            EntityVec3 {
                x: 0.0,
                y: -0.04,
                z: 0.0,
            },
            "the skipped tick retains gravity in deltaMovement without moving"
        );

        store.advance_entity_client_animations(1);
        assert!(store.probe_entity_transform(70).unwrap().position.y < 64.0);
    }

    #[test]
    fn experience_orb_no_collision_gate_includes_collidable_entities() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            71,
            VANILLA_ENTITY_TYPE_EXPERIENCE_ORB_ID,
            ProtocolVec3d::default(),
        ));
        store.apply_add_entity(projectile(
            72,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            ProtocolVec3d::default(),
        ));

        store.advance_entity_client_animations(1);
        store.advance_entity_client_animations(1);

        assert_eq!(
            store.probe_entity_transform(71).unwrap().position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            "an orb intersecting a collidable entity does not apply air gravity"
        );
    }

    #[test]
    fn nonliving_collision_response_matches_falling_and_tnt_ground_branches() {
        let metadata = EntityMetadata::default();
        let requested = EntityVec3 {
            x: 0.5,
            y: -0.04,
            z: -0.25,
        };
        let collision = Some(EntityClientMovementCollisionResult {
            pre_move_delta: requested,
            requested_movement: requested,
            clipped_movement: EntityVec3 {
                x: 0.5,
                y: 0.0,
                z: -0.25,
            },
            supporting_block: None,
            ground_friction: 0.6,
            movement_skipped: false,
            experience_orb_update: None,
            fishing_hook_update: None,
            fishing_hook_reverse_deflection: None,
        });

        let mut falling = test_transform(EntityVec3 {
            x: 0.5,
            y: 0.0,
            z: -0.25,
        });
        advance_falling_block(&metadata, &mut falling, collision);
        assert_eq!(falling.on_ground, Some(true));
        assert_vec3_near(
            falling.position,
            EntityVec3 {
                x: 1.5,
                y: 64.0,
                z: -2.25,
            },
        );
        assert_vec3_near(
            falling.delta_movement,
            scale(requested, f64::from(0.98_f32)),
        );

        let mut tnt_metadata = EntityMetadata::default();
        let mut tnt = test_transform(EntityVec3 {
            x: 0.5,
            y: 0.0,
            z: -0.25,
        });
        let accessors = vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_TNT_ID,
        )
        .unwrap();
        advance_primed_tnt(
            &mut tnt_metadata,
            &mut tnt,
            accessors.fuse_data_id,
            accessors.fuse_default,
            collision,
        );
        let drag = f64::from(0.98_f32);
        assert_eq!(tnt.on_ground, Some(true));
        assert_vec3_near(
            tnt.delta_movement,
            EntityVec3 {
                x: requested.x * drag * 0.7,
                y: requested.y * drag * -0.5,
                z: requested.z * drag * 0.7,
            },
        );
        assert_eq!(
            entity_data_int(
                &tnt_metadata.data_values,
                accessors.fuse_data_id,
                accessors.fuse_default,
            ),
            79
        );
    }

    #[test]
    fn nonliving_world_collision_inputs_clip_the_canonical_tick() {
        let mut store = nonliving_collision_world();
        for (id, entity_type_id) in [
            (61, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID),
            (62, VANILLA_ENTITY_TYPE_TNT_ID),
        ] {
            let mut entity = projectile(
                id,
                entity_type_id,
                ProtocolVec3d {
                    x: 0.0,
                    y: -0.1,
                    z: 0.0,
                },
            );
            entity.position = ProtocolVec3d {
                x: 0.5,
                y: 1.05,
                z: 0.5,
            };
            store.apply_add_entity(entity);
        }

        store.advance_entity_client_animations(1);

        let requested_y = -0.14;
        let falling = store.probe_entity_transform(61).unwrap();
        assert_eq!(falling.on_ground, Some(true));
        assert!((falling.position.y - 1.0).abs() <= 1.0e-9);
        assert!((falling.delta_movement.y - requested_y * f64::from(0.98_f32)).abs() <= 1.0e-12);
        let tnt = store.probe_entity_transform(62).unwrap();
        assert_eq!(tnt.on_ground, Some(true));
        assert!((tnt.position.y - 1.0).abs() <= 1.0e-9);
        assert!((tnt.delta_movement.y - requested_y * f64::from(0.98_f32) * -0.5).abs() <= 1.0e-12);
    }

    #[test]
    fn llama_spit_and_unattached_firework_apply_subclass_free_flight() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            60,
            VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID,
            ProtocolVec3d {
                x: 0.4,
                y: 0.2,
                z: -0.6,
            },
        ));
        store.apply_add_entity(projectile(
            61,
            VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
            ProtocolVec3d {
                x: 0.2,
                y: 0.8,
                z: -0.4,
            },
        ));
        store.apply_add_entity(projectile(
            62,
            VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
            ProtocolVec3d {
                x: 0.2,
                y: 0.8,
                z: -0.4,
            },
        ));
        let firework_accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 62,
            values: vec![EntityDataValue {
                data_id: firework_accessors.shot_at_angle_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));

        store.advance_entity_client_animations(1);

        let llama_inertia = f64::from(0.99_f32);
        assert_vec3_near(
            store.probe_entity_transform(60).unwrap().position,
            EntityVec3 {
                x: 1.4,
                y: 64.2,
                z: -2.6,
            },
        );
        assert_vec3_near(
            store.probe_entity_transform(60).unwrap().delta_movement,
            EntityVec3 {
                x: 0.4 * llama_inertia,
                y: 0.2 * llama_inertia - 0.06,
                z: -0.6 * llama_inertia,
            },
        );
        assert_vec3_near(
            store.probe_entity_transform(61).unwrap().position,
            EntityVec3 {
                x: 1.23,
                y: 64.84,
                z: -2.46,
            },
        );
        assert_vec3_near(
            store.probe_entity_transform(62).unwrap().position,
            EntityVec3 {
                x: 1.2,
                y: 64.8,
                z: -2.4,
            },
        );
        let effects = store
            .drain_entity_client_movement_effect_states()
            .collect::<Vec<_>>();
        assert!(matches!(
            effects.as_slice(),
            [
                EntityClientMovementEffectState::Sound(EntityClientMovementSoundState {
                    entity_id: 61,
                    kind: EntityClientMovementSoundKind::FireworkRocketLaunch,
                    ..
                }),
                EntityClientMovementEffectState::Particle(
                    EntityClientMovementParticleState::FireworkRocket(
                        FireworkRocketTrailParticleState { entity_id: 61, .. }
                    )
                ),
                EntityClientMovementEffectState::Sound(EntityClientMovementSoundState {
                    entity_id: 62,
                    kind: EntityClientMovementSoundKind::FireworkRocketLaunch,
                    ..
                }),
                EntityClientMovementEffectState::Particle(
                    EntityClientMovementParticleState::FireworkRocket(
                        FireworkRocketTrailParticleState { entity_id: 62, .. }
                    )
                )
            ]
        ));

        store.advance_entity_client_animations(1);
        let second_tick_effects = store
            .drain_entity_client_movement_effect_states()
            .collect::<Vec<_>>();
        assert_eq!(second_tick_effects.len(), 2);
        assert!(second_tick_effects
            .iter()
            .all(|effect| matches!(effect, EntityClientMovementEffectState::Particle(_))));
    }

    #[test]
    fn attached_firework_uses_stable_target_lookup_and_boosts_fall_flying_target() {
        let mut store = WorldStore::new();
        store.apply_add_entity(projectile(
            70,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ProtocolVec3d {
                x: 0.2,
                y: 0.1,
                z: -0.3,
            },
        ));
        store.apply_add_entity(projectile(
            71,
            VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
            ProtocolVec3d::default(),
        ));
        let entity_accessors = vanilla_entity_metadata_entity_accessors().unwrap();
        let firework_accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        )
        .unwrap();
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 70,
            values: vec![EntityDataValue {
                data_id: entity_accessors.shared_flags_data_id,
                serializer_id:
                    bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(entity_accessors.fall_flying_mask),
            }],
        }));
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id: 71,
            values: vec![EntityDataValue {
                data_id: firework_accessors.attached_to_target_data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_UNSIGNED_INT_ID,
                value: EntityDataValueKind::OptionalUnsignedInt(Some(70)),
            }],
        }));
        let generation_a = store.publication_snapshot();

        store.advance_entity_client_animations(1);

        let expected_movement = EntityVec3 {
            x: 0.1,
            y: 0.05,
            z: 0.7,
        };
        assert_vec3_near(
            store.probe_entity_transform(70).unwrap().delta_movement,
            expected_movement,
        );
        let rocket = store.probe_entity_transform(71).unwrap();
        assert_vec3_near(
            rocket.position,
            EntityVec3 {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
        );
        assert_vec3_near(rocket.delta_movement, expected_movement);
        assert_vec3_near(
            generation_a
                .probe_entity_transform(70)
                .unwrap()
                .delta_movement,
            EntityVec3 {
                x: 0.2,
                y: 0.1,
                z: -0.3,
            },
        );
        assert_eq!(
            store
                .drain_entity_client_movement_effect_states()
                .filter_map(|state| match state {
                    EntityClientMovementEffectState::Particle(
                        EntityClientMovementParticleState::FireworkRocket(state),
                    ) => Some(state.entity_id),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            vec![71]
        );
    }

    fn simple_particle(
        entity_id: i32,
        kind: EntityClientMovementSimpleParticleKind,
        moved_x: f64,
    ) -> EntityClientMovementEffectState {
        particle_effect(EntityClientMovementParticleState::Simple(
            EntityClientMovementSimpleParticleState {
                entity_id,
                kind,
                position: EntityVec3 {
                    x: 1.0 + moved_x,
                    y: 64.5,
                    z: -2.0,
                },
                velocity: EntityVec3::default(),
            },
        ))
    }

    fn particle_effect(
        state: EntityClientMovementParticleState,
    ) -> EntityClientMovementEffectState {
        EntityClientMovementEffectState::Particle(state)
    }

    fn assert_vec3_near(actual: EntityVec3, expected: EntityVec3) {
        for (actual, expected) in [
            (actual.x, expected.x),
            (actual.y, expected.y),
            (actual.z, expected.z),
        ] {
            assert!(
                (actual - expected).abs() <= 1.0e-12,
                "{actual} != {expected}"
            );
        }
    }

    fn test_transform(delta_movement: EntityVec3) -> EntityTransform {
        let position = EntityVec3 {
            x: 1.0,
            y: 64.0,
            z: -2.0,
        };
        EntityTransform {
            position,
            old_position: position,
            position_base: position,
            delta_movement,
            y_rot: 0.0,
            old_y_rot: 0.0,
            x_rot: 0.0,
            old_x_rot: 0.0,
            y_head_rot: 0.0,
            old_y_head_rot: 0.0,
            on_ground: None,
            interpolation: None,
        }
    }

    fn test_arrow_lifecycle() -> EntityProjectileLifecycle {
        EntityProjectileLifecycle {
            owner_uuid: None,
            left_owner: true,
            has_been_shot: true,
            trident_in_ground_time: 0,
            trident_dealt_damage: false,
            trident_return_ticks: 0,
            arrow_pickup_allowed: false,
            arrow_no_physics: false,
            arrow_last_block_state_id: None,
            arrow_hit_ground_sound_is_arrow: false,
        }
    }

    fn test_arrow_runtime(seed: i64) -> EntityAbstractArrowRuntime {
        EntityAbstractArrowRuntime {
            random_seed: java_random_seed(seed),
            last_deflected_by: None,
            pending_last_deflected_by: None,
            piercing_ignore_entity_ids: Vec::new(),
        }
    }

    fn projectile(
        id: i32,
        entity_type_id: i32,
        delta_movement: ProtocolVec3d,
    ) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement,
            y_rot: 0.0,
            x_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        }
    }

    fn set_nonempty_item_stack(store: &mut WorldStore, id: i32) {
        let data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ITEM_ID,
        )
        .expect("generated ItemEntity stack accessor");
        assert!(store.apply_set_entity_data(ProtocolSetEntityData {
            id,
            values: vec![EntityDataValue {
                data_id,
                serializer_id: bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                value: EntityDataValueKind::ItemStack(ItemStackSummary {
                    item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                    count: 1,
                    component_patch: Default::default(),
                }),
            }],
        }));
    }

    fn nonliving_collision_world() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: PalettedContainerData {
                    domain: PaletteDomain::BlockStates,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![0],
                    packed_data: Vec::new(),
                    entry_count: 4096,
                },
                biomes: PalettedContainerData {
                    domain: PaletteDomain::Biomes,
                    bits_per_entry: 0,
                    palette_kind: PaletteKind::SingleValue,
                    palette_global_ids: vec![0],
                    packed_data: Vec::new(),
                    entry_count: 64,
                },
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        let grass = crate::registries::BlockStateRegistry::vanilla_26_1()
            .find_by_name_and_properties(
                "minecraft:grass_block",
                &[("snowy".to_owned(), "false".to_owned())]
                    .into_iter()
                    .collect(),
            )
            .expect("vanilla grass block state")
            .id;
        assert!(
            world.apply_block_update(bbb_protocol::packets::BlockUpdate {
                pos: bbb_protocol::packets::BlockPos { x: 0, y: 0, z: 0 },
                block_state_id: grass,
            })
        );
        assert!(world.probe_block(BlockPos { x: 0, y: 0, z: 0 }).is_some());
        world
    }
}
