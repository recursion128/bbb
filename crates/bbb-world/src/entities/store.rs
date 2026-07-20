use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt,
    time::{Duration, Instant},
};

use bbb_protocol::attribute_modifier_types::{
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
    VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
};
use bbb_protocol::attribute_types::vanilla_attribute_type;
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_SLEEPING_ID,
    VANILLA_ENTITY_POSE_STANDING_ID, VANILLA_ENTITY_POSE_SWIMMING_ID,
};
use bbb_protocol::packets::AttributeSnapshot as ProtocolAttributeSnapshot;
use hecs::{Entity, World};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_DRAGON_HEAD_ID;
use bbb_protocol::packets::EquipmentSlot as ProtocolEquipmentSlot;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::packets::{
    EntityDataRegistryHolder, EntityDataRegistryReference,
    EntityDataValue as ProtocolEntityDataValue, EntityDataValueKind,
};

use super::renderer_catalog::vanilla_living_entity_renderer_name_is_upside_down;
use super::schema::{
    vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type,
    vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type,
    vanilla_entity_metadata_bat_flags_accessor_for_entity_type,
    vanilla_entity_metadata_bee_accessors_for_entity_type,
    vanilla_entity_metadata_end_crystal_accessors_for_entity_type,
    vanilla_entity_metadata_enderman_render_accessors_for_entity_type,
    vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type,
    vanilla_entity_metadata_firework_rocket_accessors_for_entity_type,
    vanilla_entity_metadata_fox_accessors_for_entity_type,
    vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type,
    vanilla_entity_metadata_hanging_direction_accessor_for_entity_type,
    vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type,
    vanilla_entity_metadata_item_frame_accessors_for_entity_type,
    vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type,
    vanilla_entity_metadata_player_absorption_accessor_for_entity_type,
    vanilla_entity_metadata_primed_tnt_accessors_for_entity_type,
    vanilla_entity_metadata_shulker_accessors_for_entity_type,
    vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type,
    vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type,
    vanilla_entity_metadata_tamable_animal_accessors_for_entity_type,
    vanilla_entity_metadata_type_is_camel, vanilla_entity_metadata_type_is_guardian,
    vanilla_entity_metadata_type_uses_player_entity_event_handler,
    vanilla_entity_metadata_vex_flags_accessor_for_entity_type,
    vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type,
    vanilla_entity_type_default_attribute, VanillaFireworkRocketMetadataAccessors,
};

use super::{
    is_vanilla_abstract_nautilus_type, is_vanilla_boat_type, is_vanilla_can_wear_horse_armor_type,
    is_vanilla_llama_type, is_vanilla_minecart_type, is_vanilla_vehicle_entity_type,
    vanilla_entity_event_kind, vanilla_entity_type_is_squid, ArmorMaterialKind,
    EntityAttachmentFace, EntityAttributes, EntityBlockModelState, EntityCameraHurtState,
    EntityCameraPoseState, EntityClientAnimationState, EntityClientAnimations,
    EntityClientMovementParticleState, EntityDamage, EntityEquipment, EntityHurtingProjectile,
    EntityIdentity, EntityInterpolationCollisionInput, EntityLeash, EntityMetadata,
    EntityMinecartLerp, EntityMobEffects, EntityModelSourceState, EntityMount,
    EntityProjectileLifecycle, EntityState, EntitySupportingBlock, EntityTransform,
    EntityTransformState, EntityTransientEvents, EntityVec3, EvokerFangsCritParticleState,
    FallingBlockModelState, FireworkRocketExplosionParticleState, FireworkRocketItemState,
    FoxEatParticleState, HappyGhastHarnessColor, ItemEntityStackState, ItemFrameRenderState,
    LlamaBodyDecorColor, LocalPlayerAttackSwingState, MinecartDisplayBlockState,
    OminousItemSpawnerItemState, OminousItemSpawnerParticleState, RavagerRoarParticleState,
    RavagerStunParticleState, WolfArmorCrackiness, VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
    VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
    VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
    VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
    VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID, VANILLA_ENTITY_TYPE_PANDA_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID,
    VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_TNT_MINECART_ID,
    VANILLA_MAX_AIR_SUPPLY_TICKS,
};
use crate::entities::animations::{
    allay_is_dancing, axolotl_is_playing_dead, bat_is_resting, boat_bubble_time,
    boat_paddle_states, camel_is_dashing, cat_is_lying, cat_relax_state_one,
    copper_golem_animation_flags, creaking_can_move, creaking_is_tearing_down,
    entity_animation_uses_in_water, entity_chunk_coordinate, entity_is_fall_flying,
    guardian_attack_duration, guardian_attack_target_id, guardian_is_moving,
    piglin_is_charging_crossbow, pillager_is_charging_crossbow, player_is_using_item,
    sniffer_can_play_digging_sound, warden_heartbeat_delay, wither_side_head_target_ids,
    wither_side_head_target_rotation, wolf_is_interested, EntityClientTickParticleTriggers,
    WitherHeadTargetRotations,
};
use crate::entities::dimensions::{
    entity_data_pose, item_frame_facing, item_frame_item, item_frame_map_id, item_frame_rotation,
    vanilla_age_scale, vanilla_body_anchor_y_offset_for_entity_data,
    vanilla_bounds_for_entity_data, vanilla_client_position_for_entity_data,
    vanilla_eye_height_for_entity_data, vanilla_is_baby, vanilla_is_cat,
    vanilla_living_entity_type, vanilla_local_player_passenger_position_for_entity_data,
    vanilla_model_source_bounds_for_entity_data, vanilla_passenger_position_for_entity_data,
    vanilla_passenger_vehicle_debug_target_for_entity_data, vanilla_pick_bounds_for_entity_data,
    vanilla_render_scale, vanilla_shulker_attached_position,
};
use crate::entities::dragon::{
    ender_dragon_parent_pick_target, ender_dragon_part_parent_id,
    ender_dragon_part_pick_targets_at_partial_tick,
};
use crate::entities::projectiles::{
    entity_hurting_projectile_from_state, entity_projectile_lifecycle_from_state,
};
use crate::entities::renderer_catalog::{
    vanilla_entity_renderer_extracts_casting_spell,
    vanilla_entity_renderer_extracts_mob_aggressive,
    vanilla_entity_renderer_extracts_unhappy_counter, vanilla_entity_renderer_minimum_block_light,
    vanilla_entity_renderer_uses_abstract_feline_model,
    vanilla_entity_renderer_uses_abstract_horse_renderer,
    vanilla_entity_renderer_uses_mob_item_in_hand_layer,
};
use crate::entities::schema::{
    vanilla_entity_metadata_entity_accessors,
    vanilla_entity_metadata_living_entity_accessors_for_entity_type,
    vanilla_entity_metadata_mob_accessors_for_entity_type,
    vanilla_entity_metadata_type_interpolation_steps,
    vanilla_entity_metadata_type_is_abstract_piglin, vanilla_entity_metadata_type_is_living,
    vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer,
    VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION, VANILLA_AVATAR_MAIN_HAND_DATA_ID,
    VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
    VANILLA_AVATAR_MODEL_PART_CAPE as VANILLA_AVATAR_CAPE_PART_MASK, VANILLA_HUMANOID_ARM_LEFT_ID,
    VANILLA_HUMANOID_ARM_RIGHT_ID, VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
    VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
};
use crate::registries::{armor_trim_sprite, ArmorTrimLayer, RegistrySet};
use crate::ItemEquipmentSlot;
use bbb_protocol::entity_types::*;

fn within_death_tick_distance(
    position: super::EntityVec3,
    player_chunk: Option<(i32, i32)>,
    simulation_distance: Option<i32>,
) -> bool {
    let (Some((player_chunk_x, player_chunk_z)), Some(simulation_distance)) =
        (player_chunk, simulation_distance)
    else {
        return true;
    };
    let entity_chunk_x = entity_chunk_coordinate(position.x);
    let entity_chunk_z = entity_chunk_coordinate(position.z);
    let x_distance = (i64::from(entity_chunk_x) - i64::from(player_chunk_x)).abs();
    let z_distance = (i64::from(entity_chunk_z) - i64::from(player_chunk_z)).abs();
    x_distance.max(z_distance) <= i64::from(simulation_distance)
}

/// Vanilla `Entity.getTicksRequiredToFreeze()`: the powder-snow freeze threshold
/// at which `isFullyFrozen()` becomes true and the body starts shaking.
const VANILLA_TICKS_REQUIRED_TO_FREEZE: i32 = 140;

/// Vanilla 26.1 `EntityType.PIG` registry id, used to gate `PigRenderState.saddle`.
/// Vanilla 26.1 `EntityType.WOLF` registry id, used to gate `WolfArmorLayer`.
/// Vanilla 26.1 skeleton-family ids whose renderers extend `HumanoidMobRenderer`
/// and therefore feed `ItemInHandLayer` through `getItemHeldByArm`.
/// Vanilla 26.1 `EvokerRenderer` installs an `ItemInHandLayer` over the illager model.
/// Vanilla 26.1 `ZombifiedPiglinRenderer` extends `HumanoidMobRenderer`.
fn wolf_armor_crackiness(
    item: &ItemStackSummary,
    default_item_max_damage: &BTreeMap<i32, i32>,
) -> WolfArmorCrackiness {
    if item.component_patch.unbreakable {
        return WolfArmorCrackiness::None;
    }
    let Some(item_id) = item.item_id else {
        return WolfArmorCrackiness::None;
    };
    let Some(max_damage) = item
        .component_patch
        .max_damage
        .or_else(|| default_item_max_damage.get(&item_id).copied())
        .filter(|max_damage| *max_damage > 0)
    else {
        return WolfArmorCrackiness::None;
    };
    let damage = item
        .component_patch
        .damage
        .unwrap_or(0)
        .clamp(0, max_damage);
    let fraction = (max_damage - damage) as f32 / max_damage as f32;
    if fraction < 0.32 {
        WolfArmorCrackiness::High
    } else if fraction < 0.69 {
        WolfArmorCrackiness::Medium
    } else if fraction < 0.95 {
        WolfArmorCrackiness::Low
    } else {
        WolfArmorCrackiness::None
    }
}

fn end_crystal_renderer_y(time_in_ticks: f32) -> f32 {
    let hh = (time_in_ticks * 0.2).sin() / 2.0 + 0.5;
    (hh * hh + hh) * 0.4 - 1.4
}

fn end_crystal_intersects_ender_dragon_search_box(
    dragon_position: super::EntityVec3,
    crystal_position: super::EntityVec3,
) -> bool {
    // Vanilla 26.1 `EntityType.ENDER_DRAGON.sized(16, 8)` and
    // `EntityType.END_CRYSTAL.sized(2, 2)`, with `EnderDragon.checkCrystals` searching
    // `getBoundingBox().inflate(32)`.
    const DRAGON_HALF_WIDTH: f64 = 8.0;
    const DRAGON_HEIGHT: f64 = 8.0;
    const CRYSTAL_HALF_WIDTH: f64 = 1.0;
    const CRYSTAL_HEIGHT: f64 = 2.0;
    const SEARCH_INFLATE: f64 = 32.0;

    let dragon_min = [
        dragon_position.x - DRAGON_HALF_WIDTH - SEARCH_INFLATE,
        dragon_position.y - SEARCH_INFLATE,
        dragon_position.z - DRAGON_HALF_WIDTH - SEARCH_INFLATE,
    ];
    let dragon_max = [
        dragon_position.x + DRAGON_HALF_WIDTH + SEARCH_INFLATE,
        dragon_position.y + DRAGON_HEIGHT + SEARCH_INFLATE,
        dragon_position.z + DRAGON_HALF_WIDTH + SEARCH_INFLATE,
    ];
    let crystal_min = [
        crystal_position.x - CRYSTAL_HALF_WIDTH,
        crystal_position.y,
        crystal_position.z - CRYSTAL_HALF_WIDTH,
    ];
    let crystal_max = [
        crystal_position.x + CRYSTAL_HALF_WIDTH,
        crystal_position.y + CRYSTAL_HEIGHT,
        crystal_position.z + CRYSTAL_HALF_WIDTH,
    ];

    crystal_max[0] >= dragon_min[0]
        && crystal_min[0] <= dragon_max[0]
        && crystal_max[1] >= dragon_min[1]
        && crystal_min[1] <= dragon_max[1]
        && crystal_max[2] >= dragon_min[2]
        && crystal_min[2] <= dragon_max[2]
}

fn vanilla_camel_saddle_type(entity_type_id: i32) -> bool {
    vanilla_entity_metadata_type_is_camel(entity_type_id)
}

fn equipment_saddle_item<'a>(
    equipment: Option<&'a EntityEquipment>,
    equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
) -> Option<&'a ItemStackSummary> {
    let item = equipment?
        .equipment
        .iter()
        .find(|update| update.slot == ProtocolEquipmentSlot::Saddle)
        .map(|update| &update.item)?;
    if item.count <= 0 {
        return None;
    }
    let item_id = item.item_id?;
    (equipment_slots.get(&item_id).copied() == Some(ItemEquipmentSlot::Saddle)).then_some(item)
}

fn equipment_contains_saddle_item(
    equipment: Option<&EntityEquipment>,
    equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
) -> bool {
    equipment_saddle_item(equipment, equipment_slots).is_some()
}

/// Vanilla `WitherBoss.DATA_ID_INV` data id (19): the int spawn-invulnerability countdown
/// (`getInvulnerableTicks`), the fourth `WitherBoss` accessor after `Mob.DATA_MOB_FLAGS_ID` (15) and
/// the three `DATA_TARGET_A/B/C` ints (16..=18).
pub(crate) struct EntityStore {
    pub(super) ecs: World,
    pub(super) by_protocol_id: BTreeMap<i32, Entity>,
    pub(super) order: Vec<i32>,
    pending_ravager_stun_particles: Vec<RavagerStunParticleState>,
    pending_evoker_fangs_crit_particles: Vec<EvokerFangsCritParticleState>,
    pub(super) pending_entity_client_movement_particles: Vec<EntityClientMovementParticleState>,
    pending_ominous_item_spawner_particles: Vec<OminousItemSpawnerParticleState>,
    client_tick_scratch: EntityClientTickScratch,
    publication_snapshot_observation: EntityPublicationSnapshotObservation,
}

#[derive(Default)]
struct EntityClientTickScratch {
    scheduled_set: HashSet<i32>,
    scheduled_entities: Vec<Entity>,
    player_passenger_ancestors: HashSet<i32>,
}

impl EntityClientTickScratch {
    fn clear(&mut self) {
        self.scheduled_set.clear();
        self.scheduled_entities.clear();
        self.player_passenger_ancestors.clear();
    }
}

pub(crate) struct BeeSoundState {
    pub(crate) transform: EntityTransformState,
    pub(crate) is_baby: bool,
    pub(crate) angry: bool,
}

/// Logical work performed by one batched entity client-tick advance.
///
/// `scanned_entities` counts tracked rows considered across all ticks in the
/// batch. `mutated_entities` counts the ordered root/passenger schedule whose
/// animation state advances, without comparing the large component afterward.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EntityClientTickWork {
    pub scanned_entities: usize,
    pub mutated_entities: usize,
    pub preparation_duration: Duration,
    pub schedule_duration: Duration,
    pub projectile_duration: Duration,
    pub interpolation_duration: Duration,
    pub animation_duration: Duration,
}

impl EntityClientTickWork {
    pub(crate) fn accumulate(&mut self, other: Self) {
        self.scanned_entities = self.scanned_entities.saturating_add(other.scanned_entities);
        self.mutated_entities = self.mutated_entities.saturating_add(other.mutated_entities);
        self.preparation_duration += other.preparation_duration;
        self.schedule_duration += other.schedule_duration;
        self.projectile_duration += other.projectile_duration;
        self.interpolation_duration += other.interpolation_duration;
        self.animation_duration += other.animation_duration;
    }
}

/// Cost and logical work of rebuilding the entity shell for one immutable
/// world generation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EntityPublicationSnapshotObservation {
    pub duration: Duration,
    pub rebuilt_entities: usize,
}

#[cfg(test)]
pub(crate) struct EntityPublicationPayloadSharing {
    pub(crate) data_values: bool,
    pub(crate) painting_variant: bool,
    pub(crate) equipment: bool,
    pub(crate) attributes: bool,
    pub(crate) passengers: bool,
    pub(crate) effects: bool,
    pub(crate) damage: bool,
}

impl EntityStore {
    #[cfg(test)]
    pub(crate) fn publication_payload_sharing(
        &self,
        other: &Self,
        id: i32,
    ) -> Option<EntityPublicationPayloadSharing> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let other_entity = other.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let other_metadata = other.ecs.get::<&EntityMetadata>(other_entity).ok()?;
        let data_values = metadata.data_values.ptr_eq(&other_metadata.data_values);
        let painting_variant = metadata
            .painting_variant_projection
            .ptr_eq(&other_metadata.painting_variant_projection);
        drop(metadata);
        drop(other_metadata);
        let equipment = self.ecs.get::<&EntityEquipment>(entity).ok()?;
        let other_equipment = other.ecs.get::<&EntityEquipment>(other_entity).ok()?;
        let equipment = equipment.equipment.ptr_eq(&other_equipment.equipment);
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let other_attributes = other.ecs.get::<&EntityAttributes>(other_entity).ok()?;
        let attributes = attributes.attributes.ptr_eq(&other_attributes.attributes);
        let mount = self.ecs.get::<&EntityMount>(entity).ok()?;
        let other_mount = other.ecs.get::<&EntityMount>(other_entity).ok()?;
        let passengers = mount.passengers.ptr_eq(&other_mount.passengers);
        let effects = self.ecs.get::<&EntityMobEffects>(entity).ok()?;
        let other_effects = other.ecs.get::<&EntityMobEffects>(other_entity).ok()?;
        let effects = effects.effects.ptr_eq(&other_effects.effects);
        let damage = self.ecs.get::<&EntityDamage>(entity).ok()?;
        let other_damage = other.ecs.get::<&EntityDamage>(other_entity).ok()?;
        let damage = damage.last_damage.ptr_eq(&other_damage.last_damage);
        Some(EntityPublicationPayloadSharing {
            data_values,
            painting_variant,
            equipment,
            attributes,
            passengers,
            effects,
            damage,
        })
    }

    pub(crate) fn insert_or_replace(&mut self, state: EntityState) {
        if let Some(entity) = self.by_protocol_id.get(&state.id).copied() {
            self.replace_existing_components(entity, state);
            return;
        }

        if !self.order.contains(&state.id) {
            self.order.push(state.id);
        }

        self.spawn_components(state);
    }

    /// Spawns (or re-add-resets) an entity directly from `AddEntity` packet
    /// components without projecting through a full `EntityState`. A vanilla
    /// re-add of an existing protocol id resets every component to its
    /// spawn-fresh value.
    pub(crate) fn apply_add_entity_components(
        &mut self,
        identity: EntityIdentity,
        transform: EntityTransform,
    ) {
        let id = identity.id;
        let entity_type_id = identity.entity_type_id;
        let fresh_projectile = entity_hurting_projectile_from_state(entity_type_id, None);
        let fresh_projectile_lifecycle = entity_projectile_lifecycle_from_state(
            entity_type_id,
            None,
            self.uuid_for_protocol_id(identity.data),
        );
        let fresh_client_animations = EntityClientAnimations {
            animations: {
                let mut animations = EntityClientAnimationState {
                    spawn_body_y_rot: vanilla_living_entity_type(entity_type_id).then_some(
                        if entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_ID {
                            0.0
                        } else {
                            transform.y_head_rot
                        },
                    ),
                    ..EntityClientAnimationState::default()
                };
                animations.initialize_spawn_defaults(entity_type_id, identity.uuid);
                animations
            },
        };

        if let Some(entity) = self.by_protocol_id.get(&id).copied() {
            if let Ok(mut existing) = self.ecs.get::<&mut EntityIdentity>(entity) {
                *existing = identity;
            }
            if let Ok(mut existing) = self.ecs.get::<&mut EntityTransform>(entity) {
                *existing = transform;
            }
            fn reset<T: hecs::Component + Default>(ecs: &mut World, entity: Entity) {
                if let Ok(mut existing) = ecs.get::<&mut T>(entity) {
                    *existing = T::default();
                }
            }
            reset::<EntityMetadata>(&mut self.ecs, entity);
            reset::<EntityEquipment>(&mut self.ecs, entity);
            reset::<EntityAttributes>(&mut self.ecs, entity);
            reset::<EntityTransientEvents>(&mut self.ecs, entity);
            reset::<EntityMount>(&mut self.ecs, entity);
            reset::<EntityLeash>(&mut self.ecs, entity);
            reset::<EntityMobEffects>(&mut self.ecs, entity);
            if let Ok(mut existing) = self.ecs.get::<&mut EntityClientAnimations>(entity) {
                *existing = fresh_client_animations;
            }
            reset::<EntityDamage>(&mut self.ecs, entity);
            reset::<EntitySupportingBlock>(&mut self.ecs, entity);
            if is_vanilla_minecart_type(entity_type_id) {
                let updated = {
                    if let Ok(mut lerp) = self.ecs.get::<&mut EntityMinecartLerp>(entity) {
                        *lerp = EntityMinecartLerp::default();
                        true
                    } else {
                        false
                    }
                };
                if !updated {
                    let _ = self.ecs.insert_one(entity, EntityMinecartLerp::default());
                }
            } else {
                let _ = self.ecs.remove_one::<EntityMinecartLerp>(entity);
            }
            if let Some(projectile) = fresh_projectile {
                let updated = {
                    if let Ok(mut existing) = self.ecs.get::<&mut EntityHurtingProjectile>(entity) {
                        *existing = projectile;
                        true
                    } else {
                        false
                    }
                };
                if !updated {
                    let _ = self.ecs.insert_one(entity, projectile);
                }
            } else {
                let _ = self.ecs.remove_one::<EntityHurtingProjectile>(entity);
            }
            if let Some(projectile) = fresh_projectile_lifecycle {
                let updated = {
                    if let Ok(mut existing) = self.ecs.get::<&mut EntityProjectileLifecycle>(entity)
                    {
                        *existing = projectile;
                        true
                    } else {
                        false
                    }
                };
                if !updated {
                    let _ = self.ecs.insert_one(entity, projectile);
                }
            } else {
                let _ = self.ecs.remove_one::<EntityProjectileLifecycle>(entity);
            }
            return;
        }

        if !self.order.contains(&id) {
            self.order.push(id);
        }
        let entity = self.ecs.spawn((
            identity,
            transform,
            EntityMetadata::default(),
            EntityEquipment::default(),
            EntityAttributes::default(),
            EntityTransientEvents::default(),
            EntityMount::default(),
            EntityLeash::default(),
            EntityMobEffects::default(),
            fresh_client_animations,
            EntityDamage::default(),
            EntitySupportingBlock::default(),
        ));
        if is_vanilla_minecart_type(entity_type_id) {
            let _ = self.ecs.insert_one(entity, EntityMinecartLerp::default());
        }
        if let Some(projectile) = fresh_projectile {
            let _ = self.ecs.insert_one(entity, projectile);
        }
        if let Some(projectile) = fresh_projectile_lifecycle {
            let _ = self.ecs.insert_one(entity, projectile);
        }
        self.by_protocol_id.insert(id, entity);
    }

    fn spawn_components(&mut self, state: EntityState) {
        let id = state.id;
        let spawn_owner_uuid = self.uuid_for_protocol_id(state.data);
        let entity = self.ecs.spawn((
            EntityIdentity::from(&state),
            EntityTransform::from(&state),
            EntityMetadata::from(&state),
            EntityEquipment::from(&state),
            EntityAttributes::from(&state),
            EntityTransientEvents::from(&state),
            EntityMount::from(&state),
            EntityLeash::from(&state),
            EntityMobEffects::from(&state),
            EntityClientAnimations::from(&state),
            EntityDamage::from(&state),
            EntitySupportingBlock::from(&state),
        ));
        // Only vanilla minecart types run the new-behavior lerp, so attach `EntityMinecartLerp`
        // conditionally. This keeps the per-client-tick `query_mut::<&mut EntityMinecartLerp>` scan
        // and the lerp-step accounting limited to actual minecarts instead of every entity.
        if is_vanilla_minecart_type(state.entity_type_id) {
            let _ = self
                .ecs
                .insert_one(entity, EntityMinecartLerp::from(&state));
        }
        if let Some(projectile) =
            entity_hurting_projectile_from_state(state.entity_type_id, state.hurting_projectile)
        {
            let _ = self.ecs.insert_one(entity, projectile);
        }
        if let Some(projectile) = entity_projectile_lifecycle_from_state(
            state.entity_type_id,
            state.projectile_lifecycle,
            spawn_owner_uuid,
        ) {
            let _ = self.ecs.insert_one(entity, projectile);
        }
        self.by_protocol_id.insert(id, entity);
    }

    fn replace_existing_components(&mut self, entity: Entity, state: EntityState) {
        self.sync_components_from_state(entity, &state);
    }

    pub(crate) fn get(&self, id: i32) -> Option<EntityState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.project_entity(entity)
    }

    pub(crate) fn contains(&self, id: i32) -> bool {
        self.by_protocol_id.contains_key(&id)
    }

    pub(crate) fn entity_type_id(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityIdentity>(entity)
            .ok()
            .map(|identity| identity.entity_type_id)
    }

    pub(crate) fn uuid_for_protocol_id(&self, id: i32) -> Option<Uuid> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityIdentity>(entity)
            .ok()
            .map(|identity| identity.uuid)
    }

    pub(crate) fn player_rideable_jumping_cooldown(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if !vanilla_entity_metadata_type_is_camel(identity.entity_type_id) {
            return Some(0.0);
        }
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(
            client_animations
                .animations
                .camel_jump_cooldown(partial_ticks),
        )
    }

    pub(crate) fn saddle_slot_contains_saddle_item(
        &self,
        id: i32,
        equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
    ) -> bool {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return false;
        };
        let Ok(equipment) = self.ecs.get::<&EntityEquipment>(entity) else {
            return false;
        };
        equipment_contains_saddle_item(Some(&*equipment), equipment_slots)
    }

    pub(crate) fn equipment_slot_contains_nonempty_item(
        &self,
        id: i32,
        slot: ProtocolEquipmentSlot,
    ) -> bool {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return false;
        };
        let Ok(equipment) = self.ecs.get::<&EntityEquipment>(entity) else {
            return false;
        };
        equipment.equipment.iter().any(|update| {
            update.slot == slot && update.item.count > 0 && update.item.item_id.is_some()
        })
    }

    pub(crate) fn first_entity_id_with_type_and_data(
        &self,
        entity_type_id: i32,
        data: i32,
    ) -> Option<i32> {
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if identity.entity_type_id == entity_type_id && identity.data == data {
                return Some(identity.id);
            }
        }
        None
    }

    pub(crate) fn identity(&self, id: i32) -> Option<EntityIdentity> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityIdentity>(entity)
            .ok()
            .map(|identity| (*identity).clone())
    }

    pub(crate) fn main_arm_left(&self, id: i32) -> Option<bool> {
        let entity_type_id = self.entity_type_id(id)?;
        Some(self.entity_main_arm_left(id, entity_type_id))
    }

    pub(crate) fn is_silent(&self, id: i32) -> Option<bool> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        self.metadata_bool(id, accessors.silent_data_id, accessors.silent_default)
    }

    pub(crate) fn guardian_attack_sound_state(&self, id: i32) -> Option<(super::EntityVec3, f32)> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if !vanilla_entity_metadata_type_is_guardian(identity.entity_type_id) {
            return None;
        }
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let attack = animations.animations.guardian_attack.unwrap_or_default();
        let duration = guardian_attack_duration(identity.entity_type_id);
        Some((transform.position, attack.attack_scale(0.0, duration)))
    }

    pub(crate) fn sniffer_can_play_digging_sound(&self, id: i32) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(sniffer_can_play_digging_sound(
            identity.entity_type_id,
            &metadata.data_values,
        ))
    }

    pub(crate) fn no_gravity(&self, id: i32) -> Option<bool> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        self.metadata_bool(
            id,
            accessors.no_gravity_data_id,
            accessors.no_gravity_default,
        )
    }

    pub(crate) fn ticks_frozen(&self, id: i32) -> Option<i32> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        self.metadata_int(
            id,
            accessors.ticks_frozen_data_id,
            accessors.ticks_frozen_default,
        )
    }

    /// Vanilla `Entity.isFullyFrozen()` = `getTicksFrozen() >=
    /// getTicksRequiredToFreeze()` (140): the powder-snow freeze is complete
    /// (drives the shaking body and, for the local player, the frozen heart
    /// sprites in `Gui.HeartType.forPlayer`).
    pub(crate) fn is_fully_frozen(&self, id: i32) -> bool {
        self.ticks_frozen(id).unwrap_or(0) >= VANILLA_TICKS_REQUIRED_TO_FREEZE
    }

    /// Vanilla `Entity.getAirSupply()`: the synced `DATA_AIR_SUPPLY_ID` int,
    /// defaulting to the define-time `getMaxAirSupply()` (300, Entity.java:312)
    /// while the server has not synced a non-default value.
    pub(crate) fn air_supply(&self, id: i32) -> Option<i32> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        self.metadata_int(
            id,
            accessors.air_supply_data_id,
            VANILLA_MAX_AIR_SUPPLY_TICKS,
        )
    }

    /// Vanilla `LivingEntity.getHealth()`: the synced `DATA_HEALTH_ID` float,
    /// defaulting to the define-time `1.0F` (LivingEntity.java:314) while the
    /// server has not synced the spawn-time health.
    pub(crate) fn living_entity_health(&self, id: i32) -> Option<f32> {
        let entity_type_id = self.entity_type_id(id)?;
        let accessors =
            vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)?;
        self.metadata_float(id, accessors.health_data_id, accessors.health_default)
    }

    pub(crate) fn is_baby(&self, id: i32) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(vanilla_is_baby(
            identity.entity_type_id,
            &metadata.data_values,
        ))
    }

    pub(crate) fn is_fall_flying(&self, id: i32) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(entity_is_fall_flying(&metadata.data_values))
    }

    pub(crate) fn is_descending(&self, id: i32) -> Option<bool> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        Some(
            self.metadata_byte(
                id,
                accessors.shared_flags_data_id,
                accessors.shared_flags_default,
            )? & accessors.shift_key_down_mask
                != 0,
        )
    }

    /// Vanilla `Player.getAbsorptionAmount()`: the synced
    /// `DATA_PLAYER_ABSORPTION_ID` float, defaulting to the define-time `0.0F`
    /// (Player.java:224).
    pub(crate) fn player_absorption(&self, id: i32) -> Option<f32> {
        let entity_type_id = self.entity_type_id(id)?;
        let (data_id, default) =
            vanilla_entity_metadata_player_absorption_accessor_for_entity_type(entity_type_id)?;
        self.metadata_float(id, data_id, default)
    }

    pub(crate) fn metadata_int_for_entity(
        &self,
        id: i32,
        data_id: u8,
        default: i32,
    ) -> Option<i32> {
        self.metadata_int(id, data_id, default)
    }

    pub(crate) fn metadata_bool_for_entity(
        &self,
        id: i32,
        data_id: u8,
        default: bool,
    ) -> Option<bool> {
        self.metadata_bool(id, data_id, default)
    }

    pub(crate) fn metadata_registry_reference_for_entity(
        &self,
        id: i32,
        data_id: u8,
        serializer: EntityDataRegistryHolder,
    ) -> Option<Option<EntityDataRegistryReference>> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::RegistryId {
                        serializer: actual,
                        value,
                    } if *actual == serializer => Some(value.clone()),
                    _ => None,
                }),
        )
    }

    pub(super) fn metadata_byte(&self, id: i32, data_id: u8, default: i8) -> Option<i8> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::Byte(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    fn metadata_optional_component(&self, id: i32, data_id: u8) -> Option<String> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        metadata
            .data_values
            .iter()
            .find(|value| value.data_id == data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalComponent(component) => component
                    .as_ref()
                    .map(|component| component.text().to_string()),
                _ => None,
            })
    }

    /// Vanilla `LivingEntity.getSleepingPos`: the synced bed position the entity is
    /// sleeping in (`SLEEPING_POS_ID`), or `None` when it is not in a bed.
    pub(crate) fn sleeping_pos(&self, id: i32) -> Option<crate::BlockPos> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            identity.entity_type_id,
        )?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        metadata
            .data_values
            .iter()
            .find(|value| value.data_id == accessors.sleeping_pos_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalBlockPos(Some(pos)) => Some(crate::BlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                }),
                _ => None,
            })
    }

    /// Vanilla `AvatarRenderer.isEntityUpsideDown` inputs for a player entity: its
    /// profile UUID (to resolve the GameProfile name from the player-info list) and
    /// whether the `CAPE` model part is shown (`DATA_PLAYER_MODE_CUSTOMISATION & 1`).
    /// `None` for non-player entities, which never use the avatar upside-down path.
    pub(crate) fn avatar_upside_down_inputs(&self, id: i32) -> Option<(Uuid, bool)> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
            return None;
        }
        let cape_shown = self
            .metadata_byte(
                id,
                VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
                VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION,
            )
            .unwrap_or(VANILLA_AVATAR_DEFAULT_MODEL_CUSTOMIZATION)
            & VANILLA_AVATAR_CAPE_PART_MASK
            != 0;
        Some((identity.uuid, cape_shown))
    }

    fn entity_main_arm_left(&self, id: i32, entity_type_id: i32) -> bool {
        if entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            return self
                .metadata_humanoid_arm(
                    id,
                    VANILLA_AVATAR_MAIN_HAND_DATA_ID,
                    VANILLA_HUMANOID_ARM_RIGHT_ID,
                )
                .is_some_and(|arm| arm == VANILLA_HUMANOID_ARM_LEFT_ID);
        }
        if vanilla_entity_renderer_uses_mob_item_in_hand_layer(entity_type_id) {
            let Some(accessors) =
                vanilla_entity_metadata_mob_accessors_for_entity_type(entity_type_id)
            else {
                return false;
            };
            return self
                .metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                .is_some_and(|flags| flags & accessors.left_handed_mask != 0);
        }
        false
    }

    /// Vanilla `Entity.getEyeHeight(Pose.STANDING)` used by the sleeping bed
    /// head-offset translate: the eye height resolved with the synced pose stripped
    /// so the dimensions fall back to standing rather than `SLEEPING_DIMENSIONS`.
    pub(crate) fn standing_eye_height(&self, id: i32) -> Option<f32> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let standing_data: Vec<_> = metadata
            .data_values
            .iter()
            .filter(|value| value.data_id != accessors.pose_data_id)
            .cloned()
            .collect();
        vanilla_eye_height_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &standing_data,
            &attributes.attributes,
            Some(client_animations.animations),
            metadata.painting_variant_projection.as_ref(),
        )
    }

    fn metadata_bool(&self, id: i32, data_id: u8, default: bool) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::Boolean(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    fn metadata_int(&self, id: i32, data_id: u8, default: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::Int(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    fn metadata_long(&self, id: i32, data_id: u8, default: i64) -> Option<i64> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::Long(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    pub(crate) fn bee_sound_state(&self, id: i32, game_time: i64) -> Option<BeeSoundState> {
        let transform = self.transform_state(id)?;
        let accessors =
            vanilla_entity_metadata_bee_accessors_for_entity_type(transform.entity_type_id)?;
        let anger_end_time = self.metadata_long(
            id,
            accessors.anger_end_time_data_id,
            accessors.anger_end_time_default,
        )?;
        Some(BeeSoundState {
            transform,
            is_baby: self.is_baby(id)?,
            angry: anger_end_time > 0 && anger_end_time.wrapping_sub(game_time) > 0,
        })
    }

    fn metadata_float(&self, id: i32, data_id: u8, default: f32) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::Float(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    fn metadata_humanoid_arm(&self, id: i32, data_id: u8, default: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::HumanoidArm(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    fn metadata_optional_unsigned_int(&self, id: i32, data_id: u8) -> Option<Option<i32>> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::OptionalUnsignedInt(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(None),
        )
    }

    /// The block-state id carried by an `OPTIONAL_BLOCK_STATE` accessor at `data_id`. The
    /// protocol decodes wire id `0` to `None`, mirroring vanilla's empty optional; an absent or
    /// wrong-typed metadata entry also defaults to `None`.
    fn metadata_optional_block_state(&self, id: i32, data_id: u8) -> Option<Option<i32>> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::OptionalBlockState(state) => Some(*state),
                    _ => None,
                })
                .unwrap_or(None),
        )
    }

    fn metadata_block_state(&self, id: i32, data_id: u8) -> Option<Option<i32>> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(metadata.data_values.iter().find_map(|value| {
            if value.data_id != data_id {
                return None;
            }
            match &value.value {
                EntityDataValueKind::BlockState(state) => Some(*state),
                _ => None,
            }
        }))
    }

    pub(crate) fn enderman_carried_block_state_id(&self, id: i32) -> Option<Option<i32>> {
        let identity = self.identity(id)?;
        let Some(accessors) = vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
            identity.entity_type_id,
        ) else {
            return Some(None);
        };
        self.metadata_optional_block_state(id, accessors.carried_block_state_data_id)
    }

    pub(crate) fn minecart_display_block_state(
        &self,
        id: i32,
        registries: &RegistrySet,
    ) -> Option<MinecartDisplayBlockState> {
        let identity = self.identity(id)?;
        if !is_vanilla_minecart_type(identity.entity_type_id) {
            return None;
        }
        let accessors = vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(
            identity.entity_type_id,
        )?;

        let custom_state_id = self
            .metadata_optional_block_state(id, accessors.custom_display_block_data_id)
            .unwrap_or(None);
        let block = if let Some(state_id) = custom_state_id {
            let state = registries.block_state(state_id)?;
            if state.name == "minecraft:air" {
                return None;
            }
            EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            }
        } else {
            let furnace_has_fuel =
                vanilla_entity_metadata_furnace_minecart_fuel_accessor_for_entity_type(
                    identity.entity_type_id,
                )
                .is_some_and(|(data_id, default)| {
                    self.metadata_bool(id, data_id, default).unwrap_or(default)
                });
            default_minecart_display_block_state(
                identity.entity_type_id,
                furnace_has_fuel,
                registries,
            )?
        };
        let display_offset = self
            .metadata_int(
                id,
                accessors.display_offset_data_id,
                accessors.display_offset_default,
            )
            .unwrap_or(accessors.display_offset_default);
        Some(MinecartDisplayBlockState {
            block,
            display_offset,
        })
    }

    pub(crate) fn falling_block_state(
        &self,
        id: i32,
        registries: &RegistrySet,
    ) -> Option<FallingBlockModelState> {
        let identity = self.identity(id)?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID {
            return None;
        }
        let state = registries.block_state(identity.data)?;
        if state.name == "minecraft:air" {
            return None;
        }
        Some(FallingBlockModelState {
            block_state_id: identity.data,
            block: EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            },
        })
    }

    pub(crate) fn primed_tnt_block_state(
        &self,
        id: i32,
        registries: &RegistrySet,
    ) -> Option<EntityBlockModelState> {
        let identity = self.identity(id)?;
        let accessors =
            vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(identity.entity_type_id)?;

        let state_id = self
            .metadata_block_state(id, accessors.block_state_data_id)
            .unwrap_or(None);
        if let Some(state_id) = state_id {
            let state = registries.block_state(state_id)?;
            if state.name == "minecraft:air" {
                return None;
            }
            return Some(EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            });
        }

        default_primed_tnt_block_state(registries, accessors.block_state_default)
    }

    pub(crate) fn primed_tnt_fuse_remaining_in_ticks(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        let identity = self.identity(id)?;
        let accessors =
            vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(identity.entity_type_id)?;
        let fuse = self
            .metadata_int(id, accessors.fuse_data_id, accessors.fuse_default)
            .unwrap_or(accessors.fuse_default);
        Some(fuse as f32 - partial_ticks + 1.0)
    }

    pub(crate) fn primed_tnt_smoke_particle_states(
        &self,
    ) -> Vec<super::PrimedTntSmokeParticleState> {
        let mut states = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(mut query) = self
                .ecs
                .query_one::<(&EntityIdentity, &EntityTransform)>(entity)
            else {
                continue;
            };
            let Some((identity, transform)) = query.get() else {
                continue;
            };
            let Some(accessors) = vanilla_entity_metadata_primed_tnt_accessors_for_entity_type(
                identity.entity_type_id,
            ) else {
                continue;
            };
            let fuse = self
                .metadata_int(identity.id, accessors.fuse_data_id, accessors.fuse_default)
                .unwrap_or(accessors.fuse_default);
            // Vanilla `PrimedTnt.tick` decrements fuse first and only emits smoke when the
            // resulting fuse remains positive.
            if fuse <= 1 {
                continue;
            }
            states.push(super::PrimedTntSmokeParticleState {
                entity_id: identity.id,
                position: transform.position,
            });
        }
        states
    }

    pub(crate) fn take_ravager_stun_particle_states(&mut self) -> Vec<RavagerStunParticleState> {
        std::mem::take(&mut self.pending_ravager_stun_particles)
    }

    pub(crate) fn take_evoker_fangs_crit_particle_states(
        &mut self,
    ) -> Vec<EvokerFangsCritParticleState> {
        std::mem::take(&mut self.pending_evoker_fangs_crit_particles)
    }

    pub(crate) fn drain_entity_client_movement_particle_states(
        &mut self,
    ) -> std::vec::Drain<'_, EntityClientMovementParticleState> {
        self.pending_entity_client_movement_particles.drain(..)
    }

    pub(crate) fn take_ominous_item_spawner_particle_states(
        &mut self,
    ) -> Vec<OminousItemSpawnerParticleState> {
        std::mem::take(&mut self.pending_ominous_item_spawner_particles)
    }

    pub(crate) fn ravager_roar_particle_state(&self, id: i32) -> Option<RavagerRoarParticleState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_RAVAGER_ID {
            return None;
        }
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let bounds = self.pick_bounds(id)?;
        let center_offset = [
            f64::from((bounds.min[0] + bounds.max[0]) * 0.5),
            f64::from((bounds.min[1] + bounds.max[1]) * 0.5),
            f64::from((bounds.min[2] + bounds.max[2]) * 0.5),
        ];
        Some(RavagerRoarParticleState {
            entity_id: id,
            center: super::EntityVec3 {
                x: transform.position.x + center_offset[0],
                y: transform.position.y + center_offset[1],
                z: transform.position.z + center_offset[2],
            },
        })
    }

    pub(crate) fn vault_connection_player_target(
        &self,
        uuid: Uuid,
        vault_pos: crate::BlockPos,
        range: f64,
    ) -> Option<(i32, [f64; 3])> {
        if !range.is_finite() || range < 0.0 {
            return None;
        }
        let range_sqr = range * range;
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let (entity_id, position) = {
                let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                    continue;
                };
                if identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID || identity.uuid != uuid
                {
                    continue;
                }
                let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                    return None;
                };
                (identity.id, transform.position)
            };
            if block_pos_dist_sqr(entity_block_pos(position), vault_pos) > range_sqr {
                return None;
            }
            let bounds = self.pick_bounds(entity_id)?;
            let height = f64::from(bounds.max[1] - bounds.min[1]);
            return Some((
                entity_id,
                [position.x, position.y + height / 2.0, position.z],
            ));
        }
        None
    }

    pub(crate) fn pose(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        Some(entity_data_pose(&metadata.data_values))
    }

    pub(crate) fn pick_bounds(&self, id: i32) -> Option<super::EntityPickBoundsState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        vanilla_pick_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(client_animations.animations),
            metadata.painting_variant_projection.as_ref(),
        )
    }

    pub(crate) fn body_anchor_y_offset(
        &self,
        id: i32,
        game_time: i64,
        is_front: bool,
        partial_ticks: f32,
    ) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        vanilla_body_anchor_y_offset_for_entity_data(
            identity.entity_type_id,
            &metadata.data_values,
            &attributes.attributes,
            game_time,
            is_front,
            partial_ticks,
        )
    }

    pub(crate) fn model_source_bounds(
        &self,
        id: i32,
        registries: &RegistrySet,
    ) -> Option<super::EntityPickBoundsState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let mut bounds = vanilla_model_source_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(client_animations.animations),
            metadata.painting_variant_projection.as_ref(),
        )?;
        if let Some(display) = self.minecart_display_block_state(identity.id, registries) {
            bounds = bounds.expand_towards_y(display.display_offset as f32 * 0.75 / 16.0);
        }
        Some(bounds)
    }

    pub(crate) fn pick_targets_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<super::EntityPickTargetState> {
        let mut targets = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(mut query) = self.ecs.query_one::<(
                &EntityIdentity,
                &EntityTransform,
                Option<&EntityClientAnimations>,
            )>(entity) else {
                continue;
            };
            let Some((identity, transform, client_animations)) = query.get() else {
                continue;
            };
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
                let dragon_animation =
                    client_animations.and_then(|animations| animations.animations.ender_dragon);
                targets.extend(ender_dragon_part_pick_targets_at_partial_tick(
                    identity.id,
                    *transform,
                    dragon_animation,
                    partial_ticks,
                ));
            } else if let Some(bounds) = self.pick_bounds(identity.id) {
                targets.push(super::EntityPickTargetState {
                    entity_id: identity.id,
                    position: transform.render_position(partial_ticks),
                    bounds,
                });
            }
        }
        targets
    }

    pub(crate) fn debug_hitbox_targets_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<super::EntityPickTargetState> {
        let mut targets = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(mut query) = self.ecs.query_one::<(
                &EntityIdentity,
                &EntityTransform,
                Option<&EntityClientAnimations>,
            )>(entity) else {
                continue;
            };
            let Some((identity, transform, client_animations)) = query.get() else {
                continue;
            };
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
                targets.push(ender_dragon_parent_pick_target(
                    identity.id,
                    *transform,
                    partial_ticks,
                ));
                let dragon_animation =
                    client_animations.and_then(|animations| animations.animations.ender_dragon);
                targets.extend(ender_dragon_part_pick_targets_at_partial_tick(
                    identity.id,
                    *transform,
                    dragon_animation,
                    partial_ticks,
                ));
            } else if let Some(bounds) = self.pick_bounds(identity.id) {
                targets.push(super::EntityPickTargetState {
                    entity_id: identity.id,
                    position: transform.render_position(partial_ticks),
                    bounds,
                });
            }
        }
        targets
    }

    pub(crate) fn fill_model_candidates_at_partial_tick(
        &self,
        partial_ticks: f32,
        registries: &RegistrySet,
        candidates: &mut Vec<super::EntityRenderCandidateState>,
    ) {
        candidates.clear();
        candidates.reserve(self.ecs.len() as usize);
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityEquipment,
            &EntityAttributes,
            &EntityClientAnimations,
            Option<&EntityMount>,
            Option<&EntityLeash>,
        )>();
        for (_, (identity, transform, metadata, equipment, attributes, animations, mount, leash)) in
            query.iter()
        {
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
                continue;
            }
            let Some(mut bounds) = vanilla_model_source_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            if let Some(display_offset) = minecart_display_offset_for_components(
                identity.entity_type_id,
                &metadata.data_values,
                registries,
            ) {
                bounds = bounds.expand_towards_y(display_offset as f32 * 0.75 / 16.0);
            }
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_ID {
                let scale = vanilla_render_scale(identity.entity_type_id, &attributes.attributes);
                if let Some(offset) =
                    animations
                        .animations
                        .shulker_render_offset(transform.position, 0.0, scale)
                {
                    // `ShulkerRenderer.shouldRender` tests the complete old-to-
                    // current transition at partial tick zero, independently
                    // of the render state's current interpolation fraction.
                    bounds = expand_bounds_towards_offset(bounds, offset);
                }
            }
            candidates.push(super::EntityRenderCandidateState {
                entity_id: identity.id,
                uuid: identity.uuid,
                entity_type_id: identity.entity_type_id,
                position: transform.render_position(partial_ticks),
                bounds,
                vehicle_id: mount.and_then(|mount| mount.vehicle_id),
                leash_holder_id: leash.and_then(|leash| leash.holder_id),
                fishing_hook_owner_id: (identity.entity_type_id
                    == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID)
                    .then_some(identity.data),
                is_spectator: false,
                is_sleeping: vanilla_living_entity_type(identity.entity_type_id)
                    && entity_data_pose(&metadata.data_values) == VANILLA_ENTITY_POSE_SLEEPING_ID,
                wears_dragon_head: equipment.equipment.iter().any(|update| {
                    update.slot == ProtocolEquipmentSlot::Head
                        && update.item.item_id == Some(VANILLA_ITEM_DRAGON_HEAD_ID)
                }),
            });
        }
    }

    pub(crate) fn restore_protocol_order_for_model_candidates(
        &self,
        candidates: &mut Vec<super::EntityRenderCandidateState>,
    ) {
        let mut by_id = candidates
            .drain(..)
            .map(|candidate| (candidate.entity_id, candidate))
            .collect::<HashMap<_, _>>();
        candidates.reserve(by_id.len());
        candidates.extend(
            self.order
                .iter()
                .filter_map(|entity_id| by_id.remove(entity_id)),
        );
    }

    #[cfg(test)]
    pub(crate) fn model_candidates_at_partial_tick(
        &self,
        partial_ticks: f32,
        registries: &RegistrySet,
    ) -> Vec<super::EntityRenderCandidateState> {
        let mut candidates = Vec::new();
        self.fill_model_candidates_at_partial_tick(partial_ticks, registries, &mut candidates);
        candidates
    }

    pub(crate) fn refresh_client_position_from_entity_data(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let old_position = {
            let (identity, metadata, transform) = self
                .ecs
                .query_one_mut::<(&EntityIdentity, &EntityMetadata, &mut EntityTransform)>(entity)
                .ok()?;
            let position = vanilla_client_position_for_entity_data(
                identity.entity_type_id,
                transform.position_base,
                identity.data,
                &metadata.data_values,
                metadata.painting_variant_projection.as_ref(),
            )?;
            let old_position = transform.position;
            transform.position = position;
            old_position
        };
        self.finish_shulker_position_change(id, old_position);
        Some(())
    }

    pub(crate) fn finish_shulker_position_change(
        &mut self,
        id: i32,
        old_position: super::EntityVec3,
    ) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityMount,
                &mut EntityTransform,
                &mut EntityClientAnimations,
            )>(entity)
            .ok()?;
        let (identity, mount, transform, animations) = query.get()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_SHULKER_ID {
            return Some(false);
        }
        if mount.vehicle_id.is_none() {
            // Vanilla `Shulker.setPos` keeps the packet codec base untouched,
            // but snaps a non-passenger's canonical position to its attachment
            // block center before starting the client render offset.
            transform.position = vanilla_shulker_attached_position(transform.position);
        }
        let started = animations.animations.record_shulker_position_change(
            old_position,
            transform.position,
            mount.vehicle_id.is_some(),
        );
        if started {
            // Vanilla Shulker.setPos makes the generic position endpoints equal
            // and delegates the visible transition to getRenderPosition.
            transform.old_position = transform.position;
        }
        Some(started)
    }

    pub(crate) fn shulker_started_riding(&mut self, id: i32) {
        if self.entity_type_id(id) != Some(VANILLA_ENTITY_TYPE_SHULKER_ID) {
            return;
        }
        let _ = self.with_client_animations_mut(id, |animations| {
            animations.animations.shulker_started_riding();
        });
    }

    pub(crate) fn shulker_stopped_riding(&mut self, id: i32) {
        if self.entity_type_id(id) != Some(VANILLA_ENTITY_TYPE_SHULKER_ID) {
            return;
        }
        let Some(position) = self.transform(id).map(|transform| transform.position) else {
            return;
        };
        let _ = self.with_client_animations_mut(id, |animations| {
            animations.animations.shulker_stopped_riding(position);
        });
    }

    pub(crate) fn transform(&self, id: i32) -> Option<EntityTransform> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityTransform>(entity)
            .ok()
            .map(|transform| *transform)
    }

    pub(crate) fn attribute_value(&self, id: i32, attribute_id: i32) -> Option<f64> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        attributes
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == attribute_id)
            .map(vanilla_attribute_value)
            .or_else(|| vanilla_default_attribute_base(identity.entity_type_id, attribute_id))
    }

    pub(crate) fn attribute_base_value(&self, id: i32, attribute_id: i32) -> Option<f64> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        attributes
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == attribute_id)
            .map(|attribute| attribute.base)
            .or_else(|| vanilla_default_attribute_base(identity.entity_type_id, attribute_id))
    }

    pub(crate) fn attribute_has_modifier(
        &self,
        id: i32,
        attribute_id: i32,
        modifier_id: &str,
    ) -> bool {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return false;
        };
        let Ok(attributes) = self.ecs.get::<&EntityAttributes>(entity) else {
            return false;
        };
        attributes
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == attribute_id)
            .is_some_and(|attribute| {
                attribute
                    .modifiers
                    .iter()
                    .any(|modifier| modifier.id == modifier_id)
            })
    }

    pub(crate) fn transform_state(&self, id: i32) -> Option<EntityTransformState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.transform_state_for_entity(entity)
    }

    fn new_minecart_passenger_render_offset(
        &self,
        vehicle_id: i32,
        partial_ticks: f32,
    ) -> Option<super::EntityVec3> {
        let vehicle = self.by_protocol_id.get(&vehicle_id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(vehicle).ok()?;
        if !is_vanilla_minecart_type(identity.entity_type_id) {
            return None;
        }
        let transform = self.ecs.get::<&EntityTransform>(vehicle).ok()?;
        let minecart_lerp = self.ecs.get::<&EntityMinecartLerp>(vehicle).ok()?;
        let old_step = minecart_lerp.old_step?;
        let render_step = minecart_lerp.render_step(partial_ticks)?;
        let cart_lerp_position = lerp_entity_vec3(
            partial_ticks,
            super::movement::entity_vec3(old_step.position),
            transform.position,
        );
        Some(sub_entity_vec3(
            super::movement::entity_vec3(render_step.position),
            cart_lerp_position,
        ))
    }

    fn turtle_passenger_render_position(
        &self,
        passenger_id: i32,
        vehicle_id: i32,
        partial_ticks: f32,
    ) -> Option<super::EntityVec3> {
        let passenger_bounds = self.pick_bounds(passenger_id)?;
        let passenger_entity = self.by_protocol_id.get(&passenger_id).copied()?;
        let mut passenger_query = self
            .ecs
            .query_one::<(&EntityIdentity, &EntityTransform, &EntityMetadata)>(passenger_entity)
            .ok()?;
        let (passenger_identity, passenger_transform, passenger_metadata) =
            passenger_query.get()?;

        let vehicle_bounds = self.pick_bounds(vehicle_id)?;
        let vehicle_entity = self.by_protocol_id.get(&vehicle_id).copied()?;
        let mut vehicle_query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityTransform,
                &EntityMount,
                &EntityMetadata,
                &EntityAttributes,
                Option<&EntityClientAnimations>,
            )>(vehicle_entity)
            .ok()?;
        let (
            vehicle_identity,
            vehicle_transform,
            vehicle_mount,
            vehicle_metadata,
            vehicle_attributes,
            vehicle_client_animations,
        ) = vehicle_query.get()?;
        if vehicle_identity.entity_type_id != VANILLA_ENTITY_TYPE_TURTLE_ID {
            return None;
        }

        vanilla_passenger_position_for_entity_data(
            vehicle_identity.entity_type_id,
            vehicle_transform.render_position(partial_ticks),
            vehicle_transform.render_y_rot(partial_ticks),
            vehicle_bounds,
            &vehicle_metadata.data_values,
            &vehicle_attributes.attributes,
            vehicle_client_animations.map(|animations| animations.animations),
            &vehicle_mount.passengers,
            passenger_id,
            passenger_identity.entity_type_id,
            passenger_transform.render_y_rot(partial_ticks),
            passenger_bounds,
            &passenger_metadata.data_values,
            0,
        )
    }

    fn living_body_render_y_rot(&self, id: i32, partial_ticks: f32) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut query = self
            .ecs
            .query_one::<(&EntityIdentity, &EntityTransform, &EntityClientAnimations)>(entity)
            .ok()?;
        let (identity, transform, animations) = query.get()?;
        if !vanilla_living_entity_type(identity.entity_type_id) {
            return None;
        }
        Some(if animations.animations.age_ticks == 0 {
            animations
                .animations
                .spawn_body_y_rot
                .unwrap_or_else(|| transform.render_body_y_rot(partial_ticks))
        } else {
            transform.render_body_y_rot(partial_ticks)
        })
    }

    pub(crate) fn passenger_position(
        &self,
        passenger_id: i32,
        vehicle_id: i32,
        game_time: i64,
    ) -> Option<super::EntityVec3> {
        let passenger_bounds = self.pick_bounds(passenger_id)?;
        let passenger_entity = self.by_protocol_id.get(&passenger_id).copied()?;
        let mut passenger_query = self
            .ecs
            .query_one::<(&EntityIdentity, &EntityTransform, &EntityMetadata)>(passenger_entity)
            .ok()?;
        let (passenger_identity, passenger_transform, passenger_metadata) =
            passenger_query.get()?;

        let vehicle_bounds = self.pick_bounds(vehicle_id)?;
        let vehicle_entity = self.by_protocol_id.get(&vehicle_id).copied()?;
        let mut vehicle_query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityTransform,
                &EntityMount,
                &EntityMetadata,
                &EntityAttributes,
                Option<&EntityClientAnimations>,
            )>(vehicle_entity)
            .ok()?;
        let (
            vehicle_identity,
            vehicle_transform,
            vehicle_mount,
            vehicle_metadata,
            vehicle_attributes,
            vehicle_client_animations,
        ) = vehicle_query.get()?;

        vanilla_passenger_position_for_entity_data(
            vehicle_identity.entity_type_id,
            vehicle_transform.position,
            vehicle_transform.y_rot,
            vehicle_bounds,
            &vehicle_metadata.data_values,
            &vehicle_attributes.attributes,
            vehicle_client_animations.map(|animations| animations.animations),
            &vehicle_mount.passengers,
            passenger_id,
            passenger_identity.entity_type_id,
            passenger_transform.y_rot,
            passenger_bounds,
            &passenger_metadata.data_values,
            game_time,
        )
    }

    pub(crate) fn local_player_passenger_position(
        &self,
        vehicle_id: i32,
        passenger_id: i32,
        passenger_is_swimming: bool,
        game_time: i64,
    ) -> Option<super::EntityVec3> {
        let vehicle_bounds = self.pick_bounds(vehicle_id)?;
        let vehicle_entity = self.by_protocol_id.get(&vehicle_id).copied()?;
        let mut vehicle_query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityTransform,
                &EntityMount,
                &EntityMetadata,
                &EntityAttributes,
                Option<&EntityClientAnimations>,
            )>(vehicle_entity)
            .ok()?;
        let (
            vehicle_identity,
            vehicle_transform,
            vehicle_mount,
            vehicle_metadata,
            vehicle_attributes,
            vehicle_client_animations,
        ) = vehicle_query.get()?;
        vanilla_local_player_passenger_position_for_entity_data(
            vehicle_identity.entity_type_id,
            vehicle_transform.position,
            vehicle_transform.y_rot,
            vehicle_bounds,
            &vehicle_metadata.data_values,
            &vehicle_attributes.attributes,
            vehicle_client_animations.map(|animations| animations.animations),
            &vehicle_mount.passengers,
            passenger_id,
            passenger_is_swimming,
            game_time,
        )
    }

    pub(crate) fn model_source(
        &self,
        id: i32,
        position: super::EntityVec3,
        partial_ticks: f32,
        minecart_improvements_enabled: bool,
        registries: &RegistrySet,
        default_item_max_damage: &BTreeMap<i32, i32>,
        armor_materials: &BTreeMap<i32, ArmorMaterialKind>,
        wings_equipment_assets: &BTreeMap<i32, String>,
        equipment_slots: &BTreeMap<i32, ItemEquipmentSlot>,
        llama_body_decor_colors: &BTreeMap<i32, LlamaBodyDecorColor>,
        happy_ghast_harness_colors: &BTreeMap<i32, HappyGhastHarnessColor>,
        nautilus_body_armor_materials: &BTreeMap<i32, ArmorMaterialKind>,
        horse_body_armor_materials: &BTreeMap<i32, ArmorMaterialKind>,
        wolf_body_armor_materials: &BTreeMap<i32, ArmorMaterialKind>,
    ) -> Option<EntityModelSourceState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        // Resolve the entity's storage location once and fetch every component this projection reads
        // in a single `query_one`. The required components fail the query (yielding `None`) if any is
        // missing; optional ones stay `Option<&_>` so a bare entity resolves them to `None`.
        //
        // Vanilla `HumanoidArmorLayer` worn armor: resolve the item worn in each armor slot to its
        // equipment-asset material. The `EntityEquipment` component holds the synced `SetEquipment`
        // items; a bare entity (no equipment component / empty slot / non-armor item) resolves to None.
        let mut query = self
            .ecs
            .query_one::<(
                &EntityIdentity,
                &EntityTransform,
                &EntityMetadata,
                &EntityAttributes,
                &EntityClientAnimations,
                Option<&EntityEquipment>,
                Option<&EntityMount>,
                Option<&EntityMinecartLerp>,
            )>(entity)
            .ok()?;
        let (
            identity,
            transform,
            metadata,
            attributes,
            client_animations,
            equipment,
            mount,
            minecart_lerp,
        ) = query.get()?;
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let armor_item = |slot: ProtocolEquipmentSlot| -> Option<&ItemStackSummary> {
            equipment
                .as_ref()?
                .equipment
                .iter()
                .find(|update| update.slot == slot)
                .map(|update| &update.item)
        };
        let armor_material = |slot: ProtocolEquipmentSlot| -> Option<ArmorMaterialKind> {
            let item_id = armor_item(slot)?.item_id?;
            armor_materials.get(&item_id).copied()
        };
        // Vanilla `DyedItemColor.getOrDefault`: the per-slot worn item's `dyed_color` component (a
        // packed RGB), carried alongside the material. Only leather consumes it client-side; a slot
        // with no equipment / no dye component resolves to None.
        let armor_dye = |slot: ProtocolEquipmentSlot| -> Option<i32> {
            armor_item(slot)?.component_patch.dyed_color
        };
        let armor_foil = |slot: ProtocolEquipmentSlot| -> bool {
            armor_material(slot)
                .and_then(|_| armor_item(slot))
                .is_some_and(ItemStackSummary::has_foil)
        };
        let armor_trim = |slot: ProtocolEquipmentSlot| {
            let material = armor_material(slot)?;
            armor_trim_sprite(
                registries,
                armor_item(slot)?,
                material.equipment_asset_id(),
                if slot == ProtocolEquipmentSlot::Legs {
                    ArmorTrimLayer::HumanoidLeggings
                } else {
                    ArmorTrimLayer::Humanoid
                },
            )
        };
        let chest_wings_equipment_asset = armor_item(ProtocolEquipmentSlot::Chest)
            .filter(|item| item.count > 0)
            .and_then(|item| item.item_id)
            .and_then(|item_id| wings_equipment_assets.get(&item_id));
        let chest_wings_trim = chest_wings_equipment_asset.and_then(|equipment_asset| {
            armor_trim_sprite(
                registries,
                armor_item(ProtocolEquipmentSlot::Chest)?,
                equipment_asset,
                ArmorTrimLayer::Wings,
            )
        });
        let chest_wings_foil = chest_wings_equipment_asset
            .and_then(|_| armor_item(ProtocolEquipmentSlot::Chest))
            .is_some_and(ItemStackSummary::has_foil);
        // Vanilla `SimpleEquipmentLayer` saddle users copy `EquipmentSlot.SADDLE` into render state,
        // then render only a non-empty equippable saddle item. bbb resolves the default saddle item
        // from the item equipment-slot map.
        let saddle_slot_item = || equipment_saddle_item(equipment, equipment_slots);
        let saddle_slot_contains_saddle_item = || saddle_slot_item().is_some();
        let saddle_trim = |enabled: bool, layer| {
            enabled.then_some(()).and_then(|()| {
                armor_trim_sprite(registries, saddle_slot_item()?, "minecraft:saddle", layer)
            })
        };
        let saddle_foil =
            |enabled: bool| enabled && saddle_slot_item().is_some_and(ItemStackSummary::has_foil);
        let pig_saddle = identity.entity_type_id == VANILLA_ENTITY_TYPE_PIG_ID
            && saddle_slot_contains_saddle_item();
        let pig_saddle_trim = saddle_trim(pig_saddle, ArmorTrimLayer::PigSaddle);
        let pig_saddle_foil = saddle_foil(pig_saddle);
        let equine_saddle = || -> bool {
            vanilla_entity_renderer_uses_abstract_horse_renderer(identity.entity_type_id)
                && saddle_slot_contains_saddle_item()
        };
        let equine_saddle = equine_saddle();
        let equine_saddle_trim_layer = match identity.entity_type_id {
            VANILLA_ENTITY_TYPE_HORSE_ID => Some(ArmorTrimLayer::HorseSaddle),
            VANILLA_ENTITY_TYPE_DONKEY_ID => Some(ArmorTrimLayer::DonkeySaddle),
            VANILLA_ENTITY_TYPE_MULE_ID => Some(ArmorTrimLayer::MuleSaddle),
            VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => Some(ArmorTrimLayer::SkeletonHorseSaddle),
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID => Some(ArmorTrimLayer::ZombieHorseSaddle),
            _ => None,
        };
        let equine_saddle_trim =
            equine_saddle_trim_layer.and_then(|layer| saddle_trim(equine_saddle, layer));
        let equine_saddle_foil = saddle_foil(equine_saddle);
        let equine_saddle_ridden = equine_saddle
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let strider_ridden = identity.entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let strider_saddle = identity.entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID
            && saddle_slot_contains_saddle_item();
        let strider_saddle_trim = saddle_trim(strider_saddle, ArmorTrimLayer::StriderSaddle);
        let strider_saddle_foil = saddle_foil(strider_saddle);
        let camel_saddle = vanilla_camel_saddle_type(identity.entity_type_id)
            && saddle_slot_contains_saddle_item();
        let camel_saddle_trim_layer =
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID {
                ArmorTrimLayer::CamelHuskSaddle
            } else {
                ArmorTrimLayer::CamelSaddle
            };
        let camel_saddle_trim = saddle_trim(camel_saddle, camel_saddle_trim_layer);
        let camel_saddle_foil = saddle_foil(camel_saddle);
        let camel_saddle_ridden = camel_saddle
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let nautilus_saddle = is_vanilla_abstract_nautilus_type(identity.entity_type_id)
            && saddle_slot_contains_saddle_item();
        let nautilus_saddle_trim = saddle_trim(nautilus_saddle, ArmorTrimLayer::NautilusSaddle);
        let nautilus_saddle_foil = saddle_foil(nautilus_saddle);
        let body_slot_item = || -> Option<&ItemStackSummary> {
            equipment
                .as_ref()
                .and_then(|equipment| {
                    equipment
                        .equipment
                        .iter()
                        .find(|update| update.slot == ProtocolEquipmentSlot::Body)
                })
                .map(|update| &update.item)
                .filter(|item| item.count > 0)
        };
        let body_slot_item_id =
            || -> Option<i32> { body_slot_item().and_then(|item| item.item_id) };
        let body_slot_armor_dye =
            || -> Option<i32> { body_slot_item().and_then(|item| item.component_patch.dyed_color) };
        let happy_ghast_body_item = identity.entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            && body_slot_item().is_some();
        let happy_ghast_harness = if identity.entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
            body_slot_item_id()
                .and_then(|item_id| happy_ghast_harness_colors.get(&item_id).copied())
        } else {
            None
        };
        let happy_ghast_harness_trim = happy_ghast_harness.and_then(|color| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                color.equipment_asset_id(),
                ArmorTrimLayer::HappyGhastBody,
            )
        });
        let happy_ghast_harness_foil = happy_ghast_harness
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let happy_ghast_ridden = identity.entity_type_id == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            && mount
                .as_ref()
                .is_some_and(|mount| !mount.passengers.is_empty());
        let llama_body_decor = if is_vanilla_llama_type(identity.entity_type_id)
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id().and_then(|item_id| llama_body_decor_colors.get(&item_id).copied())
        } else {
            None
        };
        let llama_body_decor_trim = llama_body_decor.and_then(|color| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                color.equipment_asset_id(),
                ArmorTrimLayer::LlamaBody,
            )
        });
        let llama_body_decor_foil = llama_body_decor
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let nautilus_body_armor = if is_vanilla_abstract_nautilus_type(identity.entity_type_id)
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id()
                .and_then(|item_id| nautilus_body_armor_materials.get(&item_id).copied())
        } else {
            None
        };
        let nautilus_body_armor_trim = nautilus_body_armor.and_then(|material| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                material.equipment_asset_id(),
                ArmorTrimLayer::NautilusBody,
            )
        });
        let nautilus_body_armor_foil = nautilus_body_armor
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let horse_body_armor = if is_vanilla_can_wear_horse_armor_type(identity.entity_type_id)
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id()
                .and_then(|item_id| horse_body_armor_materials.get(&item_id).copied())
        } else {
            None
        };
        let horse_body_armor_trim = horse_body_armor.and_then(|material| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                material.equipment_asset_id(),
                ArmorTrimLayer::HorseBody,
            )
        });
        let horse_body_armor_dye = horse_body_armor.and_then(|_| body_slot_armor_dye());
        let horse_body_armor_foil = horse_body_armor
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        let wolf_body_armor = if identity.entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID
            && !vanilla_is_baby(identity.entity_type_id, &metadata.data_values)
        {
            body_slot_item_id().and_then(|item_id| wolf_body_armor_materials.get(&item_id).copied())
        } else {
            None
        };
        let wolf_body_armor_trim = wolf_body_armor.and_then(|material| {
            armor_trim_sprite(
                registries,
                body_slot_item()?,
                material.equipment_asset_id(),
                ArmorTrimLayer::WolfBody,
            )
        });
        let wolf_body_armor_dye = wolf_body_armor.and_then(|_| body_slot_armor_dye());
        let wolf_body_armor_crackiness = wolf_body_armor
            .and_then(|_| body_slot_item())
            .map(|item| wolf_armor_crackiness(item, default_item_max_damage))
            .unwrap_or(WolfArmorCrackiness::None);
        let wolf_body_armor_foil = wolf_body_armor
            .and_then(|_| body_slot_item())
            .is_some_and(ItemStackSummary::has_foil);
        // Vanilla `LivingEntityRenderer.isShaking` (base) is `Entity.isFullyFrozen`
        // (`getTicksFrozen() >= 140`), and only living entities shake.
        let is_fully_frozen =
            vanilla_living_entity_type(identity.entity_type_id) && self.is_fully_frozen(id);
        // Vanilla `LivingEntityRenderer.extractRenderState`: `isInvisibleToPlayer =
        // state.isInvisible && entity.isInvisibleTo(minecraft.player)`. The store-local baseline is
        // the non-spectator / no-friendly-team-override branch, so the world aggregation can clear it
        // for spectator viewers.
        let invisible_to_player = self
            .metadata_byte(
                id,
                entity_accessors.shared_flags_data_id,
                entity_accessors.shared_flags_default,
            )
            .unwrap_or(entity_accessors.shared_flags_default)
            & entity_accessors.invisible_mask
            != 0;
        let appears_glowing = self
            .metadata_byte(
                id,
                entity_accessors.shared_flags_data_id,
                entity_accessors.shared_flags_default,
            )
            .unwrap_or(entity_accessors.shared_flags_default)
            & entity_accessors.glowing_mask
            != 0;
        // Vanilla renderer `extractRenderState` assignments project `Mob.isAggressive()` for the
        // zombie/zombie-villager/zombified-piglin, skeleton, and illager paths. PiglinRenderer
        // instead projects `entity.getArmPose()`; bbb resolves that pose from the same synced Mob
        // flag plus held-item state, so the generated AbstractPiglin class membership is composed
        // here. Other entities do not consume the bit through a supported renderer pose.
        let is_aggressive =
            (vanilla_entity_renderer_extracts_mob_aggressive(identity.entity_type_id)
                || vanilla_entity_metadata_type_is_abstract_piglin(identity.entity_type_id))
                && vanilla_entity_metadata_mob_accessors_for_entity_type(identity.entity_type_id)
                    .is_some_and(|accessors| {
                        self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                            .unwrap_or(accessors.flags_default)
                            & accessors.aggressive_mask
                            != 0
                    });
        // Vanilla EvokerRenderer / IllusionerRenderer copy
        // `state.isCastingSpell = entity.isCastingSpell()`. Intersect that renderer assignment
        // with the generated SpellcasterIllager accessor owner before reading the shared byte id;
        // Pillager uses the same id for its BOOLEAN crossbow flag.
        let illager_spellcasting =
            vanilla_entity_renderer_extracts_casting_spell(identity.entity_type_id)
                && vanilla_entity_metadata_spellcaster_casting_accessor_for_entity_type(
                    identity.entity_type_id,
                )
                .is_some_and(|(data_id, default)| {
                    self.metadata_byte(id, data_id, default).unwrap_or(default) > 0
                });
        // Vanilla `VillagerRenderer` / `WanderingTraderRenderer.extractRenderState`:
        // `state.isUnhappy = entity.getUnhappyCounter() > 0`. PandaRenderer has the same assignment
        // for Panda.UNHAPPY_COUNTER, so intersect the renderer fact with the generated
        // AbstractVillager accessor owner rather than treating the shared field spelling as one type.
        let villager_unhappy =
            vanilla_entity_renderer_extracts_unhappy_counter(identity.entity_type_id)
                && vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(
                    identity.entity_type_id,
                )
                .is_some_and(|(data_id, default)| {
                    self.metadata_int(id, data_id, default).unwrap_or(default) > 0
                });
        // Vanilla `EndermanModel.setupAnim`: a carried block (`!carriedBlock.isEmpty()`,
        // the synced `DATA_CARRY_STATE`) poses both arms forward, and `isCreepy` (the
        // synced `DATA_CREEPY`) drops the head / raises the hat. Only the enderman defines
        // these accessors, so the projections are gated to it and default off otherwise.
        let enderman_accessors = vanilla_entity_metadata_enderman_render_accessors_for_entity_type(
            identity.entity_type_id,
        );
        let enderman_carried_block_state_id = enderman_accessors.and_then(|accessors| {
            self.metadata_optional_block_state(id, accessors.carried_block_state_data_id)
                .unwrap_or(accessors.carried_block_state_default)
        });
        let enderman_carrying = enderman_carried_block_state_id.is_some();
        let enderman_carried_block = enderman_carried_block_state_id
            .and_then(|state_id| registries.block_state(state_id))
            .map(|state| EntityBlockModelState {
                name: state.name.clone(),
                properties: state.properties.clone(),
            });
        let enderman_creepy = enderman_accessors.is_some_and(|accessors| {
            self.metadata_bool(id, accessors.creepy_data_id, accessors.creepy_default)
                .unwrap_or(accessors.creepy_default)
        });
        // Vanilla `BatModel.setupAnim` swaps to the `BAT_RESTING` hanging pose while
        // `Bat.isResting` (`DATA_ID_FLAGS & FLAG_RESTING`). Only the bat defines that flags byte, so
        // the projection is gated to it and defaults to flying otherwise.
        let bat_resting =
            vanilla_entity_metadata_bat_flags_accessor_for_entity_type(identity.entity_type_id)
                .is_some_and(|(data_id, default, resting_mask)| {
                    self.metadata_byte(id, data_id, default).unwrap_or(default) & resting_mask != 0
                });
        // Vanilla `BeeModel.setupAnim` hides the stinger cube once `Bee.hasStung`
        // (`DATA_FLAGS_ID & 4`). Only the bee defines that flags byte; every other entity keeps
        // its stinger field at the `true` default (and never renders a stinger anyway).
        let bee_has_stinger =
            vanilla_entity_metadata_bee_accessors_for_entity_type(identity.entity_type_id)
                .is_none_or(|accessors| {
                    self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                        .unwrap_or(accessors.flags_default)
                        & accessors.has_stung_mask
                        == 0
                });
        // Vanilla `FoxModel.setupAnim` reads the fox's synced `DATA_FLAGS_ID` (19) crouch / sleep /
        // sit / pounce / faceplant bits directly (no easing) to pick its pose branch. Only the fox
        // defines that flags byte, so the five bool projections are gated to it and default off. The
        // interest bit (8) is read by the animation layer instead, to drive the `headRollAngle`
        // accumulator below.
        let fox_accessors =
            vanilla_entity_metadata_fox_accessors_for_entity_type(identity.entity_type_id);
        let fox_flags = fox_accessors
            .map(|accessors| {
                self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                    .unwrap_or(accessors.flags_default)
            })
            .unwrap_or(0);
        let fox_is_crouching =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.crouching_mask != 0);
        let fox_is_sleeping =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.sleeping_mask != 0);
        let fox_is_sitting =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.sitting_mask != 0);
        let fox_is_pouncing =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.pouncing_mask != 0);
        let fox_is_faceplanted =
            fox_accessors.is_some_and(|accessors| fox_flags & accessors.faceplanted_mask != 0);
        let is_feline = vanilla_entity_renderer_uses_abstract_feline_model(identity.entity_type_id);
        // Vanilla `CatRenderer` / `OcelotRenderer.extractRenderState` copies
        // `Entity.isCrouching()` and `Entity.isSprinting()` into `FelineRenderState`. The former is the
        // synced `Pose.CROUCHING`; the latter is shared entity flags bit 3. `CatRenderer` additionally
        // projects `Cat.isInSittingPose()` from the generated TamableAnimal sitting mask;
        // `OcelotRenderer` leaves `isSitting` at its default.
        let feline_is_crouching = is_feline
            && entity_data_pose(&metadata.data_values) == VANILLA_ENTITY_POSE_CROUCHING_ID;
        let feline_is_sprinting = is_feline
            && self
                .metadata_byte(
                    id,
                    entity_accessors.shared_flags_data_id,
                    entity_accessors.shared_flags_default,
                )
                .unwrap_or(entity_accessors.shared_flags_default)
                & entity_accessors.sprinting_mask
                != 0;
        let feline_is_sitting = vanilla_is_cat(identity.entity_type_id)
            && vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
                identity.entity_type_id,
            )
            .is_some_and(|accessors| {
                self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                    .unwrap_or(accessors.flags_default)
                    & accessors.sitting_mask
                    != 0
            });
        // Vanilla `VexModel.setupAnim` levels the body (`xRot = 0`) and raises both arms
        // (`setArmsCharging`) while `Vex.isCharging` (`DATA_FLAGS_ID & FLAG_IS_CHARGING`). The
        // generated accessor owns the Vex membership, wire id, default, and named mask, so every
        // other type stays idle.
        let vex_charging =
            vanilla_entity_metadata_vex_flags_accessor_for_entity_type(identity.entity_type_id)
                .map(|(data_id, default, charging_mask)| {
                    self.metadata_byte(id, data_id, default).unwrap_or(default) & charging_mask != 0
                })
                .unwrap_or(false);
        // Vanilla `SnowGolemRenderer.extractRenderState`: when `SnowGolem.hasPumpkin()` is true, the
        // renderer resolves `Blocks.CARVED_PUMPKIN.defaultBlockState()` into `state.headBlock`. The
        // generated synced-byte default has the pumpkin mask set, so a newly spawned golem shows the
        // pumpkin until the server clears that bit.
        let snow_golem_pumpkin =
            vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
                identity.entity_type_id,
            )
            .is_some_and(|(data_id, default, pumpkin_mask)| {
                self.metadata_byte(id, data_id, default).unwrap_or(default) & pumpkin_mask != 0
            });
        // Vanilla `WitherBossRenderer.extractRenderState`: `state.invulnerableTicks =
        // invulnerableTicks > 0 ? invulnerableTicks - partialTicks : 0.0` (the synced `DATA_ID_INV`
        // spawn countdown, lerped against the partial tick). It drives both the `scale()` spawn-charge
        // shrink and the `getTextureLocation` `wither_invulnerable.png` flicker. Only the wither
        // defines that accessor, so every other entity holds `0.0`.
        let wither_invulnerable_accessor =
            vanilla_entity_metadata_wither_invulnerable_ticks_accessor_for_entity_type(
                identity.entity_type_id,
            );
        let wither_invulnerable_ticks =
            if let Some((data_id, default)) = wither_invulnerable_accessor {
                let ticks = self.metadata_int(id, data_id, default).unwrap_or(default);
                if ticks > 0 {
                    ticks as f32 - partial_ticks
                } else {
                    0.0
                }
            } else {
                0.0
            };
        // Vanilla `LivingEntity.isAutoSpinAttack` (the named spin-attack bit in
        // `DATA_LIVING_ENTITY_FLAGS`):
        // a living entity mid riptide-trident spin. Non-living entities have no
        // living-entity flags byte, so they never spin.
        let living_accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            identity.entity_type_id,
        );
        let is_auto_spin_attack = living_accessors.is_some_and(|accessors| {
            self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                .unwrap_or(accessors.flags_default)
                & accessors.spin_attack_mask
                != 0
        });
        // Vanilla `LivingEntity.isUsingItem` plus `getUsedItemHand` read the named using/off-hand
        // bits in `DATA_LIVING_ENTITY_FLAGS`; use-item arm poses (spyglass / horn / brush) read both. Non-living
        // entities have no flags byte, so they never count as using an item.
        let (living_entity_flags, is_using_mask, off_hand_mask) = living_accessors
            .map(|accessors| {
                (
                    self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                        .unwrap_or(accessors.flags_default),
                    accessors.is_using_mask,
                    accessors.off_hand_mask,
                )
            })
            .unwrap_or((0, 0, 0));
        let is_using_item = living_entity_flags & is_using_mask != 0;
        let use_item_off_hand = living_entity_flags & off_hand_mask != 0;
        let main_arm_left = self.entity_main_arm_left(id, identity.entity_type_id);
        // Vanilla `LivingEntityRenderer.isEntityUpsideDown`: a non-player living
        // entity whose custom name is `Dinnerbone`/`Grumm`. The player variant
        // (`AvatarRenderer.isPlayerUpsideDown`) keys off the GameProfile name and
        // the cape model part instead, which need the player-info list, so the
        // player type is excluded here and that path stays deferred.
        let is_upside_down = vanilla_living_entity_type(identity.entity_type_id)
            && identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
            && self
                .metadata_optional_component(id, entity_accessors.custom_name_data_id)
                .is_some_and(|name| {
                    vanilla_living_entity_renderer_name_is_upside_down(name.as_str())
                });
        // Vanilla `EntityRenderState.boundingBoxHeight` (`Entity.getBbHeight`): the
        // pick-bounds AABB height, used to lift the upside-down model before flipping.
        let bounding_box_height = self
            .pick_bounds(id)
            .map(|bounds| bounds.max[1] - bounds.min[1])
            .unwrap_or(0.0);
        // Vanilla `LivingEntityRenderState.hasPose(Pose.SLEEPING)`: only living
        // entities lie down. The bed orientation/offset are resolved spatially by
        // the WorldStore aggregation (which owns the block data); the per-entity
        // source defaults to the no-bed fallback.
        let is_sleeping = vanilla_living_entity_type(identity.entity_type_id)
            && entity_data_pose(&metadata.data_values) == VANILLA_ENTITY_POSE_SLEEPING_ID;
        // Vanilla `Entity.isCrouching` (`Pose.CROUCHING`): only the player is ever put into the
        // crouch pose by the server, and only the player model has the `HumanoidModel.setupAnim`
        // crouch, so the projection is gated to the player.
        let is_crouching = identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && entity_data_pose(&metadata.data_values) == VANILLA_ENTITY_POSE_CROUCHING_ID;
        // Vanilla `LivingEntityRenderState.scale` (`LivingEntity.getScale`, the SCALE
        // attribute): only living entities carry a render scale; everything else
        // renders at its default size.
        let scale = if vanilla_entity_metadata_type_is_living(identity.entity_type_id) {
            vanilla_render_scale(identity.entity_type_id, &attributes.attributes)
        } else {
            1.0
        };
        // Vanilla `Sniffer.onSyncedDataUpdated`: the active `Sniffer.State` one-shot `AnimationState`
        // as `(state ordinal, elapsed seconds)`, or `(-1, -1.0)` for an idling/searching sniffer and
        // every non-sniffer (only the sniffer is given a sniffer animation state).
        let (sniffer_animation_id, sniffer_animation_seconds) = client_animations
            .animations
            .sniffer_animation(partial_ticks);
        let walk_animation_position = client_animations
            .animations
            .walk_animation_position(partial_ticks);
        let (player_cape_flap, player_cape_lean, player_cape_lean2) =
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                client_animations
                    .animations
                    .player_cape_state(partial_ticks, transform.y_rot)
            } else {
                (0.0, 0.0, 0.0)
            };
        let (player_left_shoulder_parrot, player_right_shoulder_parrot) =
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
                (
                    self.metadata_optional_unsigned_int(
                        id,
                        VANILLA_PLAYER_SHOULDER_PARROT_LEFT_DATA_ID,
                    )
                    .unwrap_or(None),
                    self.metadata_optional_unsigned_int(
                        id,
                        VANILLA_PLAYER_SHOULDER_PARROT_RIGHT_DATA_ID,
                    )
                    .unwrap_or(None),
                )
            } else {
                (None, None)
            };
        let is_baby = vanilla_is_baby(identity.entity_type_id, &metadata.data_values);
        let is_panda = identity.entity_type_id == VANILLA_ENTITY_TYPE_PANDA_ID;
        let panda_roll_amount = if is_panda && !is_baby {
            client_animations
                .animations
                .panda_roll_amount(partial_ticks)
        } else {
            0.0
        };
        let is_passenger = mount
            .as_ref()
            .is_some_and(|mount| mount.vehicle_id.is_some());
        let boat_paddles = mount.as_ref().map_or([false; 2], |mount| {
            boat_paddle_states(
                identity.entity_type_id,
                &metadata.data_values,
                !mount.passengers.is_empty(),
            )
        });
        let boat_rowing_times = client_animations.animations.boat_rowing_times(
            partial_ticks,
            boat_paddles[0],
            boat_paddles[1],
        );
        let (boat_hurt_time, boat_hurt_dir, boat_damage_time) = client_animations
            .animations
            .boat_damage_state(partial_ticks);
        // `AbstractMinecart` chooses NewMinecartBehavior from the level's
        // enabled feature set at construction, before any step packet arrives.
        let minecart_new_render =
            is_vanilla_minecart_type(identity.entity_type_id) && minecart_improvements_enabled;
        let mut source_position = position;
        let is_shulker = identity.entity_type_id == VANILLA_ENTITY_TYPE_SHULKER_ID;
        let render_head_y_rot = if is_shulker {
            transform.y_head_rot
        } else {
            transform.render_y_head_rot(partial_ticks)
        };
        let mut source_y_rot = if client_animations.animations.age_ticks == 0 {
            client_animations
                .animations
                .spawn_body_y_rot
                .unwrap_or_else(|| transform.render_body_y_rot(partial_ticks))
        } else if is_shulker {
            transform.y_rot
        } else if vanilla_living_entity_type(identity.entity_type_id) {
            transform.render_body_y_rot(partial_ticks)
        } else {
            transform.render_y_rot(partial_ticks)
        };
        if vanilla_living_entity_type(identity.entity_type_id) {
            if let Some(vehicle_body_y_rot) = mount
                .as_ref()
                .and_then(|mount| mount.vehicle_id)
                .and_then(|vehicle_id| self.living_body_render_y_rot(vehicle_id, partial_ticks))
            {
                source_y_rot =
                    solve_living_passenger_body_y_rot(render_head_y_rot, vehicle_body_y_rot);
            }
        }
        let mut source_x_rot = transform.render_x_rot(partial_ticks);
        if minecart_new_render {
            if let Some(render_step) = minecart_lerp
                .as_ref()
                .and_then(|lerp| lerp.render_step(partial_ticks))
            {
                source_position = super::movement::entity_vec3(render_step.position);
                source_y_rot = render_step.y_rot;
                source_x_rot = render_step.x_rot;
            }
        }
        if let Some(passenger_position) = mount
            .as_ref()
            .and_then(|mount| mount.vehicle_id)
            .and_then(|vehicle_id| {
                self.turtle_passenger_render_position(id, vehicle_id, partial_ticks)
            })
        {
            source_position = passenger_position;
        }
        if let Some(offset) = minecart_improvements_enabled
            .then_some(())
            .and_then(|()| mount.as_ref())
            .and_then(|mount| mount.vehicle_id)
            .and_then(|vehicle_id| {
                self.new_minecart_passenger_render_offset(vehicle_id, partial_ticks)
            })
        {
            // Vanilla `EntityRenderer.extractRenderState` adds this via `passengerOffset`
            // when a new-behavior minecart's weighted render position diverges from its
            // normal xOld/getX interpolation.
            source_position = add_entity_vec3(source_position, offset);
        }
        if is_shulker {
            if let Some(offset) = client_animations.animations.shulker_render_offset(
                transform.position,
                partial_ticks,
                scale,
            ) {
                source_position = add_entity_vec3(source_position, offset);
            }
        }
        let boat_bubble_angle = client_animations
            .animations
            .boat_bubble_angle(partial_ticks);
        let (wither_x_head_rots, wither_y_head_rots) = wither_invulnerable_accessor
            .map(|_| client_animations.animations.wither_head_rotations())
            .unwrap_or(([0.0; 2], [0.0; 2]));
        // Vanilla `LivingEntityRenderer.extractRenderState`: worn skull animation normally mirrors
        // the entity walk position, but while riding a living entity it reads the vehicle's walk
        // animation position even though the passenger's own limb swing is stopped.
        let worn_head_animation_position = mount
            .as_ref()
            .and_then(|mount| mount.vehicle_id)
            .and_then(|vehicle_id| {
                self.living_entity_walk_animation_position(vehicle_id, partial_ticks)
            })
            .unwrap_or(walk_animation_position);
        // Vanilla `LivingEntity.recreateFromPacket` seeds squid `yBodyRot` from
        // the add-entity head yaw; `Squid.aiStep` then refines it from movement
        // while in water. Keep it as renderer-only body yaw so the synced
        // transform yaw remains canonical.
        let squid_y_body_rot = if vanilla_entity_type_is_squid(identity.entity_type_id) {
            client_animations
                .animations
                .squid_y_body_rot(partial_ticks)
                .unwrap_or(transform.y_head_rot)
        } else {
            0.0
        };
        Some(EntityModelSourceState {
            entity_id: identity.id,
            uuid: identity.uuid,
            entity_type_id: identity.entity_type_id,
            position: source_position,
            y_rot: source_y_rot,
            x_rot: source_x_rot,
            y_head_rot: render_head_y_rot,
            arrow_shake: client_animations.animations.arrow_shake(partial_ticks),
            is_passenger,
            age_ticks: if identity.entity_type_id == VANILLA_ENTITY_TYPE_END_CRYSTAL_ID {
                client_animations.animations.end_crystal_age_ticks()
            } else {
                client_animations.animations.age_ticks
            },
            boat_rowing_time_left: boat_rowing_times[0],
            boat_rowing_time_right: boat_rowing_times[1],
            boat_hurt_time,
            boat_hurt_dir,
            boat_damage_time,
            minecart_new_render,
            minecart_pos_on_rail: None,
            minecart_front_pos: None,
            minecart_back_pos: None,
            minecart_tnt_fuse_remaining_in_ticks: client_animations
                .animations
                .minecart_tnt_fuse_remaining_in_ticks(partial_ticks),
            boat_bubble_angle,
            boat_underwater: false,
            wither_x_head_rots,
            wither_y_head_rots,
            is_fully_frozen,
            invisible_to_player,
            appears_glowing,
            outline_color: 0,
            is_aggressive,
            illager_spellcasting,
            villager_unhappy,
            enderman_carrying,
            enderman_carried_block,
            enderman_creepy,
            bat_resting,
            bat_flying_seconds: client_animations
                .animations
                .bat_flying_seconds(partial_ticks),
            bee_has_stinger,
            bee_roll_amount: client_animations.animations.bee_roll_amount(partial_ticks),
            panda_sit_amount: client_animations.animations.panda_sit_amount(partial_ticks),
            panda_lie_on_back_amount: client_animations
                .animations
                .panda_lie_on_back_amount(partial_ticks),
            panda_roll_amount,
            panda_roll_time: client_animations.animations.panda_roll_time(partial_ticks),
            // Vanilla `Frog.croakAnimationState`: the elapsed seconds since `Pose.CROAKING` started,
            // or `-1.0` for a non-croaking frog (only the frog is given a croak animation state).
            frog_croak_seconds: client_animations
                .animations
                .frog_croak_seconds(partial_ticks),
            // Vanilla `Frog.tongueAnimationState`: the elapsed seconds since `Pose.USING_TONGUE`
            // started, or `-1.0` for a frog not using its tongue (only the frog is given a tongue
            // animation state).
            frog_tongue_seconds: client_animations
                .animations
                .frog_tongue_seconds(partial_ticks),
            // Vanilla `Frog.jumpAnimationState`: the elapsed seconds since `Pose.LONG_JUMPING`
            // started, or `-1.0` for a non-jumping frog (only the frog is given a jump animation
            // state).
            frog_jump_seconds: client_animations
                .animations
                .frog_jump_seconds(partial_ticks),
            // Vanilla `Frog.swimIdleAnimationState`: the elapsed seconds since the in-water idle
            // started (`Frog.tick` drives it off the per-tick `isInWater()` + `!isMoving()`), or
            // `-1.0` for a dry/moving frog (only the frog is given a swim-idle animation state).
            frog_swim_idle_seconds: client_animations
                .animations
                .frog_swim_idle_seconds(partial_ticks),
            // Vanilla `Camel.dashAnimationState`: the elapsed seconds since the dash started (the
            // synced `DASH` boolean rising edge), or `-1.0` for a non-dashing camel and every other
            // entity (only the camel is given a dash animation state).
            camel_dash_seconds: client_animations
                .animations
                .camel_dash_seconds(partial_ticks),
            // Vanilla `Camel.idleAnimationState`: client-random timeout-driven idle keyframe elapsed
            // seconds, or `-1.0` for every non-camel.
            camel_idle_seconds: client_animations
                .animations
                .camel_idle_seconds(partial_ticks),
            copper_golem_idle_seconds: client_animations
                .animations
                .copper_golem_idle_seconds(partial_ticks),
            copper_golem_get_item_seconds: client_animations
                .animations
                .copper_golem_get_item_seconds(partial_ticks),
            copper_golem_get_no_item_seconds: client_animations
                .animations
                .copper_golem_get_no_item_seconds(partial_ticks),
            copper_golem_drop_item_seconds: client_animations
                .animations
                .copper_golem_drop_item_seconds(partial_ticks),
            copper_golem_drop_no_item_seconds: client_animations
                .animations
                .copper_golem_drop_no_item_seconds(partial_ticks),
            camel_jump_cooldown: client_animations
                .animations
                .camel_jump_cooldown(partial_ticks),
            sniffer_animation_id,
            sniffer_animation_seconds,
            // Vanilla `Sniffer.isSearching()`: gates the renderer's swap of the base walk for the
            // looping `SNIFFER_SNIFF_SEARCH` search-walk. `false` for every non-searching sniffer and
            // every other entity.
            sniffer_is_searching: client_animations.animations.sniffer_is_searching(),
            // Vanilla `Armadillo.shouldHideInShell` + the rollUp/rollOut transition timers: the
            // shell-ball hide window and the curl-in / un-curl elapsed seconds. `false`/`-1.0` for
            // every non-armadillo (only the armadillo is given a roll animation state).
            armadillo_is_hiding_in_shell: client_animations
                .animations
                .armadillo_is_hiding_in_shell(),
            armadillo_roll_up_seconds: client_animations
                .animations
                .armadillo_roll_up_seconds(partial_ticks),
            armadillo_roll_out_seconds: client_animations
                .animations
                .armadillo_roll_out_seconds(partial_ticks),
            armadillo_peek_seconds: client_animations
                .animations
                .armadillo_peek_seconds(partial_ticks),
            // Vanilla `Fox.getHeadRollAngle` / `getCrouchAmount`: the lerped client accumulators that
            // drive the head tilt and the crouch body drop. `0.0` for every non-fox (only the fox is
            // given a fox animation state).
            fox_head_roll_angle: client_animations
                .animations
                .fox_head_roll_angle(partial_ticks),
            fox_crouch_amount: client_animations
                .animations
                .fox_crouch_amount(partial_ticks),
            wolf_wet_shade: client_animations.animations.wolf_wet_shade(partial_ticks),
            wolf_shake_anim: client_animations.animations.wolf_shake_anim(partial_ticks),
            wolf_head_roll_angle: client_animations
                .animations
                .wolf_head_roll_angle(partial_ticks),
            fox_is_crouching,
            fox_is_sleeping,
            fox_is_sitting,
            fox_is_pouncing,
            fox_is_faceplanted,
            feline_is_crouching,
            feline_is_sprinting,
            feline_is_sitting,
            feline_lie_down_amount: client_animations
                .animations
                .feline_lie_down_amount(partial_ticks),
            feline_lie_down_amount_tail: client_animations
                .animations
                .feline_lie_down_amount_tail(partial_ticks),
            feline_relax_state_one_amount: client_animations
                .animations
                .feline_relax_state_one_amount(partial_ticks),
            feline_is_lying_on_top_of_sleeping_player: false,
            vex_charging,
            wither_invulnerable_ticks,
            is_crouching,
            // Vanilla `HumanoidMobRenderer.extractHumanoidRenderState` copies
            // `LivingEntity.elytraAnimationState.getRotX/Y/Z(partialTick)` into
            // `HumanoidRenderState`; `WingsLayer` consumes those rotations when
            // chest equipment renders a WINGS layer.
            elytra_rot_x: client_animations.animations.elytra_rot_x(partial_ticks),
            elytra_rot_y: client_animations.animations.elytra_rot_y(partial_ticks),
            elytra_rot_z: client_animations.animations.elytra_rot_z(partial_ticks),
            player_cape_flap,
            player_cape_lean,
            player_cape_lean2,
            player_left_shoulder_parrot,
            player_right_shoulder_parrot,
            show_extra_ears: false,
            is_auto_spin_attack,
            is_using_item,
            use_item_off_hand,
            is_upside_down,
            bounding_box_height,
            is_sleeping,
            sleeping_bed_yaw: None,
            sleeping_bed_offset: [0.0, 0.0],
            scale,
            age_scale: vanilla_age_scale(identity.entity_type_id, &metadata.data_values),
            swim_amount: client_animations
                .animations
                .living_swim_amount(partial_ticks),
            sheep_eat_animation_tick: client_animations.animations.sheep_eat_animation_tick(),
            goat_lower_head_tick: client_animations.animations.goat_lower_head_tick(),
            iron_golem_attack_ticks_remaining: client_animations
                .animations
                .iron_golem_attack_ticks_remaining(partial_ticks),
            iron_golem_offer_flower_tick: client_animations
                .animations
                .iron_golem_offer_flower_tick(),
            snow_golem_pumpkin,
            ravager_stunned_ticks_remaining: client_animations
                .animations
                .ravager_stunned_ticks_remaining(partial_ticks),
            ravager_attack_ticks_remaining: client_animations
                .animations
                .ravager_attack_ticks_remaining(partial_ticks),
            ravager_roar_animation: client_animations
                .animations
                .ravager_roar_animation(partial_ticks),
            hoglin_attack_animation_tick: client_animations
                .animations
                .hoglin_attack_animation_tick(),
            armor_stand_wiggle: client_animations
                .animations
                .armor_stand_wiggle(partial_ticks),
            polar_bear_stand_scale: client_animations
                .animations
                .polar_bear_stand_scale(partial_ticks),
            painting_variant_projection: (*metadata.painting_variant_projection).clone(),
            painting_facing: vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(
                identity.entity_type_id,
            )
            .and_then(|accessor| {
                super::dimensions::painting_facing(accessor, identity.data, &metadata.data_values)
            }),
            // Vanilla `Shulker.getClientPeekAmount(partialTick)`: the lerped client peek that
            // drives `ShulkerModel.setupAnim`'s lid open/close. `0.0` (closed/bind pose) for
            // every non-shulker (only the shulker is given a peek animation state).
            shulker_peek: client_animations
                .animations
                .shulker_peek_amount(partial_ticks),
            shulker_attach_face: shulker_attach_face(
                identity.entity_type_id,
                &metadata.data_values,
            ),
            // Spatial light is sampled by the WorldStore aggregation, which owns
            // the chunk light data; the per-entity source defaults to full bright.
            light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
            // `in_water` is likewise resolved by the WorldStore aggregation (it owns the
            // chunk fluid data); the per-entity source defaults to dry.
            in_water: false,
            // Vanilla `Entity.onGround()`: the last synced movement ground flag (default
            // `false` until a movement packet sets it).
            on_ground: transform.on_ground.unwrap_or(false),
            // Vanilla `DolphinRenderState.isMoving` (`getDeltaMovement().horizontalDistanceSqr() >
            // 1.0e-7`): projected from the synced velocity.
            is_moving: {
                let delta = transform.delta_movement;
                delta.x * delta.x + delta.z * delta.z > 1.0e-7
            },
            has_red_overlay: client_animations.animations.has_red_overlay(),
            death_time: client_animations.animations.death_time(partial_ticks),
            ender_dragon_death_time: client_animations
                .animations
                .ender_dragon_death_time(partial_ticks),
            creeper_swelling: client_animations.animations.creeper_swelling(partial_ticks),
            tendril_animation: client_animations
                .animations
                .warden_tendril_animation(partial_ticks),
            heart_animation: client_animations
                .animations
                .warden_heart_animation(partial_ticks),
            // Vanilla `Warden`'s four triggered combat keyframe one-shots (roar/sniff pose-driven,
            // attack/sonic-boom event-driven): the elapsed seconds since each started, or `-1.0`
            // when stopped (only the warden is given a combat animation state).
            warden_roar_seconds: client_animations
                .animations
                .warden_roar_seconds(partial_ticks),
            warden_sniff_seconds: client_animations
                .animations
                .warden_sniff_seconds(partial_ticks),
            warden_attack_seconds: client_animations
                .animations
                .warden_attack_seconds(partial_ticks),
            warden_sonic_boom_seconds: client_animations
                .animations
                .warden_sonic_boom_seconds(partial_ticks),
            warden_emerge_seconds: client_animations
                .animations
                .warden_emerge_seconds(partial_ticks),
            warden_dig_seconds: client_animations
                .animations
                .warden_dig_seconds(partial_ticks),
            rabbit_hop_seconds: client_animations
                .animations
                .rabbit_hop_seconds(partial_ticks),
            // Vanilla `Creaking`: the directly-synced `canMove()` walk gate plus the three triggered
            // keyframe one-shots (attack/invulnerable event-driven, death on the synced
            // `isTearingDown()`), each the elapsed seconds since it started or `-1.0` when stopped. The
            // The generated accessor gates the `canMove` read on Creaking membership and defaults
            // every other entity to `true`, because this field only controls the Creaking model.
            creaking_can_move: creaking_can_move(identity.entity_type_id, &metadata.data_values),
            creaking_attack_seconds: client_animations
                .animations
                .creaking_attack_seconds(partial_ticks),
            creaking_invulnerable_seconds: client_animations
                .animations
                .creaking_invulnerable_seconds(partial_ticks),
            creaking_death_seconds: client_animations
                .animations
                .creaking_death_seconds(partial_ticks),
            walk_animation_position,
            walk_animation_speed: client_animations
                .animations
                .walk_animation_speed(partial_ticks),
            worn_head_animation_position,
            // Vanilla `HumanoidRenderState.attackTime` / `ArmedEntityRenderState.mainArm` /
            // `attackArm`: the lerped melee swing progress, which arm is the main arm, and which arm
            // swings. Attack progress is `0.0` for an entity that is not mid-swing.
            attack_anim: client_animations.animations.attack_anim(partial_ticks),
            main_arm_left,
            attack_arm_off_hand: client_animations.animations.attack_arm_off_hand(),
            is_swinging: client_animations.animations.is_swinging(),
            // Vanilla `SquidRenderer.extractRenderState`: the lerped tentacle flex
            // angle plus body pitch/yaw/roll. `0.0` for every non-squid entity
            // (only the squid/glow squid is given a squid animation state).
            squid_tentacle_angle: client_animations
                .animations
                .squid_tentacle_angle(partial_ticks),
            squid_x_body_rot: client_animations.animations.squid_x_body_rot(partial_ticks),
            squid_y_body_rot,
            squid_z_body_rot: client_animations.animations.squid_z_body_rot(partial_ticks),
            // Vanilla `GuardianRenderer.extractRenderState`: the lerped tail-sway
            // phase. `0.0` for every non-guardian entity (only the guardian / elder
            // guardian is given a tail animation state).
            guardian_tail_animation: client_animations
                .animations
                .guardian_tail_animation(partial_ticks),
            // Vanilla `GuardianRenderState.spikesAnimation`: the lerped spike-withdrawal phase
            // (`withdrawal = (1 - it) · 0.55`). `1.0` (withdrawal `0`, fully extended) for every
            // non-guardian entity (only the guardian / elder guardian is given a spikes state).
            guardian_spikes_animation: client_animations
                .animations
                .guardian_spikes_animation(partial_ticks),
            // Vanilla `Breeze`'s pose-driven action one-shots (shoot/slide/slideBack/inhale/longJump):
            // the elapsed seconds since each started, or `-1.0` when stopped (only the breeze is given
            // a breeze animation state).
            breeze_shoot_seconds: client_animations
                .animations
                .breeze_shoot_seconds(partial_ticks),
            breeze_slide_seconds: client_animations
                .animations
                .breeze_slide_seconds(partial_ticks),
            breeze_slide_back_seconds: client_animations
                .animations
                .breeze_slide_back_seconds(partial_ticks),
            breeze_inhale_seconds: client_animations
                .animations
                .breeze_inhale_seconds(partial_ticks),
            breeze_long_jump_seconds: client_animations
                .animations
                .breeze_long_jump_seconds(partial_ticks),
            // Vanilla `ChickenRenderer.extractRenderState`: the lerped wing-flap
            // phase and amplitude. `0.0` for every non-chicken entity (only the
            // chicken is given a flap animation state).
            chicken_flap: client_animations.animations.chicken_flap(partial_ticks),
            chicken_flap_speed: client_animations
                .animations
                .chicken_flap_speed(partial_ticks),
            // Vanilla `SlimeRenderer.extractRenderState`: the lerped squish amount
            // driving the slime/magma-cube body's non-uniform stretch. `0.0` for
            // every other entity (only the slime/magma cube is given a squish state).
            slime_squish: client_animations.animations.slime_squish(partial_ticks),
            // Vanilla `EvokerFangsRenderer.extractRenderState`: the `0..1` bite ramp
            // driving the fang's jaw snap, rise, and vanish. `0.0` (hidden) for every
            // other entity and a fang whose attack has not started.
            evoker_fangs_bite_progress: client_animations
                .animations
                .evoker_fangs_bite_progress(partial_ticks),
            // Vanilla `AllayModel.setupAnim`: the dance state driven by the synced
            // `DATA_DANCING` flag. `dancing` gates the dance pose, `spinning` selects
            // the spin sub-pose, and `spinning_progress` is the `0..1` lerped spin
            // blend. All inert (`false`/`0.0`) for every non-allay entity.
            allay_dancing: client_animations.animations.allay_is_dancing(),
            allay_spinning: client_animations.animations.allay_is_spinning(),
            allay_spinning_progress: client_animations
                .animations
                .allay_spinning_progress(partial_ticks),
            allay_holding_item_progress: client_animations
                .animations
                .allay_holding_item_progress(partial_ticks),
            // Vanilla render-state `ticksUsingItem`, reconstructed from the shared charge/use counter.
            // Crossbow draw poses and player spear kinetic use sway consume it with their own durations.
            // `0.0` for anything not mid-draw/use.
            crossbow_charge_ticks: client_animations
                .animations
                .crossbow_charge_ticks_using_item(partial_ticks),
            // Vanilla `LivingEntityRenderState.ticksSinceKineticHitFeedback`
            // (`LivingEntity.getTicksSinceLastKineticHitFeedback(partialTicks)`): entity event `2`
            // starts the local feedback timer that `SpearAnimations.thirdPersonUseItem` consumes.
            ticks_since_kinetic_hit_feedback: client_animations
                .animations
                .ticks_since_kinetic_hit_feedback(partial_ticks),
            // Vanilla `AxolotlRenderer.extractRenderState`: the four `BinaryAnimator` factors
            // (`Axolotl.{playingDead,inWater,onGround,moving}Animator.getFactor`) that
            // `AdultAxolotlModel.setupAnim` blends into its swim / hover / crawl / lay-still /
            // play-dead sub-animations. All `0.0` for every non-axolotl entity.
            axolotl_playing_dead_factor: client_animations
                .animations
                .axolotl_playing_dead_factor(partial_ticks),
            axolotl_in_water_factor: client_animations
                .animations
                .axolotl_in_water_factor(partial_ticks),
            axolotl_on_ground_factor: client_animations
                .animations
                .axolotl_on_ground_factor(partial_ticks),
            axolotl_moving_factor: client_animations
                .animations
                .axolotl_moving_factor(partial_ticks),
            // Vanilla `ParrotRenderer.extractRenderState`: the lerped, combined
            // wing-flap angle. `0.0` for every non-parrot entity (only the parrot is
            // given a flap animation state).
            parrot_flap_angle: client_animations
                .animations
                .parrot_flap_angle(partial_ticks),
            parrot_party: false,
            // Vanilla `HumanoidArmorLayer`: the worn armor item in each armor slot resolved to its
            // equipment-asset material against the item registry map (threaded from the WorldStore).
            head_armor: armor_material(ProtocolEquipmentSlot::Head),
            chest_armor: armor_material(ProtocolEquipmentSlot::Chest),
            legs_armor: armor_material(ProtocolEquipmentSlot::Legs),
            feet_armor: armor_material(ProtocolEquipmentSlot::Feet),
            head_armor_trim: armor_trim(ProtocolEquipmentSlot::Head),
            chest_armor_trim: armor_trim(ProtocolEquipmentSlot::Chest),
            legs_armor_trim: armor_trim(ProtocolEquipmentSlot::Legs),
            feet_armor_trim: armor_trim(ProtocolEquipmentSlot::Feet),
            head_armor_dye: armor_dye(ProtocolEquipmentSlot::Head),
            chest_armor_dye: armor_dye(ProtocolEquipmentSlot::Chest),
            legs_armor_dye: armor_dye(ProtocolEquipmentSlot::Legs),
            feet_armor_dye: armor_dye(ProtocolEquipmentSlot::Feet),
            head_armor_foil: armor_foil(ProtocolEquipmentSlot::Head),
            chest_armor_foil: armor_foil(ProtocolEquipmentSlot::Chest),
            legs_armor_foil: armor_foil(ProtocolEquipmentSlot::Legs),
            feet_armor_foil: armor_foil(ProtocolEquipmentSlot::Feet),
            chest_wings_trim,
            chest_wings_foil,
            wolf_body_armor,
            wolf_body_armor_trim,
            wolf_body_armor_dye,
            wolf_body_armor_crackiness,
            wolf_body_armor_foil,
            pig_saddle,
            pig_saddle_trim,
            pig_saddle_foil,
            equine_saddle,
            equine_saddle_trim,
            equine_saddle_foil,
            equine_saddle_ridden,
            equine_animate_tail: client_animations.animations.equine_animate_tail(),
            equine_eat_animation: client_animations
                .animations
                .equine_eat_animation(partial_ticks),
            equine_stand_animation: client_animations
                .animations
                .equine_stand_animation(partial_ticks),
            equine_feeding_animation: client_animations
                .animations
                .equine_feeding_animation(partial_ticks),
            equine_body_armor: horse_body_armor,
            equine_body_armor_trim: horse_body_armor_trim,
            equine_body_armor_dye: horse_body_armor_dye,
            equine_body_armor_foil: horse_body_armor_foil,
            strider_ridden,
            strider_saddle,
            strider_saddle_trim,
            strider_saddle_foil,
            camel_saddle,
            camel_saddle_trim,
            camel_saddle_foil,
            camel_saddle_ridden,
            nautilus_saddle,
            nautilus_saddle_trim,
            nautilus_saddle_foil,
            happy_ghast_body_item,
            happy_ghast_harness,
            happy_ghast_harness_trim,
            happy_ghast_harness_foil,
            happy_ghast_ridden,
            guardian_beam: self.guardian_beam_source(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                client_animations.animations,
                position,
                partial_ticks,
            ),
            end_crystal_beam: self.end_crystal_beam_source(
                identity.entity_type_id,
                &metadata.data_values,
                position,
            ),
            ender_dragon_beam: self.ender_dragon_beam_source(
                identity.entity_type_id,
                position,
                partial_ticks,
            ),
            llama_body_decor,
            llama_body_decor_trim,
            llama_body_decor_foil,
            nautilus_body_armor,
            nautilus_body_armor_trim,
            nautilus_body_armor_foil,
            data_values: (*metadata.data_values).clone(),
        })
    }

    /// Vanilla `GuardianRenderer.extractRenderState` attack beam: when this entity is a guardian whose
    /// synced `DATA_ID_ATTACK_TARGET` names a live target, project the world eye→target vector and the
    /// attack timing. `position` is the guardian's interpolated feet position; the target is resolved
    /// cross-entity by its protocol id. `None` for a guardian with no target (or a missing target) and
    /// every non-guardian.
    #[allow(clippy::too_many_arguments)]
    fn guardian_beam_source(
        &self,
        entity_type_id: i32,
        add_entity_data: i32,
        data_values: &[bbb_protocol::packets::EntityDataValue],
        attributes: &[ProtocolAttributeSnapshot],
        client_animations: super::EntityClientAnimationState,
        position: super::EntityVec3,
        partial_ticks: f32,
    ) -> Option<super::GuardianBeamSource> {
        if !vanilla_entity_metadata_type_is_guardian(entity_type_id) {
            return None;
        }
        let attack_target_id = guardian_attack_target_id(entity_type_id, data_values);
        if attack_target_id == 0 {
            return None;
        }
        // The target may not be tracked client-side (out of range / not yet spawned); then no beam.
        let target_transform = self.transform(attack_target_id)?;
        let target_bounds = self.pick_bounds(attack_target_id)?;
        let eye_height = vanilla_eye_height_for_entity_data(
            entity_type_id,
            add_entity_data,
            data_values,
            attributes,
            Some(client_animations),
            None,
        )?;
        let attack = client_animations.guardian_attack.unwrap_or_default();
        let attack_duration = guardian_attack_duration(entity_type_id);
        // Vanilla `getPosition(target, getBbHeight() * 0.5)` is the target center; `getEyePosition()`
        // is the guardian eye. bbb uses the latest (un-interpolated) positions, so the lerp collapses.
        let target_half_height = f64::from((target_bounds.max[1] - target_bounds.min[1]) * 0.5);
        let eye_to_target = [
            (target_transform.position.x - position.x) as f32,
            (target_transform.position.y + target_half_height
                - (position.y + f64::from(eye_height))) as f32,
            (target_transform.position.z - position.z) as f32,
        ];
        Some(super::GuardianBeamSource {
            eye_to_target,
            eye_height,
            attack_time: attack.attack_time(partial_ticks),
            attack_scale: attack.attack_scale(partial_ticks, attack_duration),
        })
    }

    /// Vanilla `EndCrystalRenderer.extractRenderState`: when an end crystal has a synced
    /// `DATA_BEAM_TARGET`, project the target block center relative to the crystal's interpolated
    /// position. `None` for crystals without a target and every non-crystal entity.
    fn end_crystal_beam_source(
        &self,
        entity_type_id: i32,
        data_values: &[bbb_protocol::packets::EntityDataValue],
        position: super::EntityVec3,
    ) -> Option<super::EndCrystalBeamSource> {
        let accessors =
            vanilla_entity_metadata_end_crystal_accessors_for_entity_type(entity_type_id)?;
        let target = data_values
            .iter()
            .find(|value| value.data_id == accessors.beam_target_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalBlockPos(Some(pos)) => Some(pos),
                _ => None,
            })?;
        Some(super::EndCrystalBeamSource {
            beam_offset: [
                (f64::from(target.x) + 0.5 - position.x) as f32,
                (f64::from(target.y) + 0.5 - position.y) as f32,
                (f64::from(target.z) + 0.5 - position.z) as f32,
            ],
        })
    }

    /// Vanilla `EnderDragon.checkCrystals` tracks the nearest end crystal from
    /// `getBoundingBox().inflate(32)`, and `EnderDragonRenderer.extractRenderState` projects that
    /// crystal's bobbed render position relative to the dragon position as `beamOffset`. The exact
    /// random 10-tick refresh cadence is not replayed client-side here; the rendered state is projected
    /// from the nearest currently tracked crystal intersecting the vanilla search box.
    fn ender_dragon_beam_source(
        &self,
        entity_type_id: i32,
        position: super::EntityVec3,
        partial_ticks: f32,
    ) -> Option<super::EnderDragonBeamSource> {
        if entity_type_id != VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
            return None;
        }

        let mut nearest: Option<(f64, [f32; 3])> = None;
        let mut query = self
            .ecs
            .query::<(&EntityIdentity, &EntityTransform, &EntityClientAnimations)>();
        for (_, (identity, transform, client_animations)) in query.iter() {
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_END_CRYSTAL_ID {
                continue;
            }
            if !end_crystal_intersects_ender_dragon_search_box(position, transform.position) {
                continue;
            }
            let dx = transform.position.x - position.x;
            let dy = transform.position.y - position.y;
            let dz = transform.position.z - position.z;
            let distance_sqr = dx * dx + dy * dy + dz * dz;
            if nearest.is_some_and(|(nearest_distance_sqr, _)| distance_sqr >= nearest_distance_sqr)
            {
                continue;
            }
            let crystal_age =
                client_animations.animations.end_crystal_age_ticks() as f32 + partial_ticks;
            let crystal_y = transform.position.y + f64::from(end_crystal_renderer_y(crystal_age));
            nearest = Some((
                distance_sqr,
                [dx as f32, (crystal_y - position.y) as f32, dz as f32],
            ));
        }

        nearest.map(|(_, beam_offset)| super::EnderDragonBeamSource { beam_offset })
    }

    pub(crate) fn camera_pose_state(&self, id: i32) -> Option<EntityCameraPoseState> {
        self.camera_pose_state_at_partial_tick(id, 1.0)
    }

    pub(crate) fn camera_pose_state_at_partial_tick(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraPoseState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let eye_height = vanilla_eye_height_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(client_animations.animations),
            metadata.painting_variant_projection.as_ref(),
        )?;
        Some(EntityCameraPoseState {
            id: identity.id,
            position: transform.render_position(partial_ticks),
            y_rot: transform.render_y_rot(partial_ticks),
            x_rot: transform.render_x_rot(partial_ticks),
            eye_height,
        })
    }

    pub(crate) fn camera_hurt_state(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraHurtState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if !vanilla_entity_metadata_type_is_living(identity.entity_type_id) {
            return None;
        }
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let (hurt_time, hurt_duration) = client_animations
            .animations
            .camera_hurt_time(partial_ticks)
            .unwrap_or((-1.0, 10.0));
        let death_time = client_animations.animations.death_time(partial_ticks);
        let hurt_dir = if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID {
            self.ecs
                .get::<&EntityTransientEvents>(entity)
                .ok()
                .and_then(|events| events.last_hurt_yaw)
                .unwrap_or(0.0)
        } else {
            // `LivingEntity.getHurtDir()` is zero; only `Player` overrides it.
            0.0
        };
        Some(EntityCameraHurtState {
            hurt_time,
            hurt_duration,
            hurt_dir,
            death_time,
            is_dead_or_dying: death_time > 0.0,
        })
    }

    fn living_entity_walk_animation_position(&self, id: i32, partial_ticks: f32) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if !vanilla_living_entity_type(identity.entity_type_id) {
            return None;
        }
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(
            client_animations
                .animations
                .walk_animation_position(partial_ticks),
        )
    }

    pub(crate) fn living_entity_previous_feet_position(
        &self,
        id: i32,
    ) -> Option<Option<super::EntityVec3>> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if !vanilla_living_entity_type(identity.entity_type_id) {
            return None;
        }
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(
            client_animations
                .animations
                .walk_animation
                .and_then(|walk| walk.previous_feet_position),
        )
    }

    fn wither_head_targets_by_id(&self) -> HashMap<i32, [WitherHeadTargetRotations; 2]> {
        let mut withers = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityAttributes,
        )>();
        for (_, (identity, transform, metadata, attributes)) in query.iter() {
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_WITHER_ID {
                continue;
            }

            withers.push((
                identity.id,
                transform.position,
                transform.y_rot,
                vanilla_render_scale(identity.entity_type_id, &attributes.attributes),
                wither_side_head_target_ids(identity.entity_type_id, &metadata.data_values),
            ));
        }
        drop(query);

        let mut eye_positions = HashMap::new();
        for target_id in withers
            .iter()
            .flat_map(|(_, _, _, _, target_ids)| target_ids)
            .copied()
            .filter(|target_id| *target_id > 0)
        {
            if eye_positions.contains_key(&target_id) {
                continue;
            }
            let Some(entity) = self.by_protocol_id.get(&target_id).copied() else {
                continue;
            };
            let (Ok(identity), Ok(transform), Ok(metadata), Ok(attributes), Ok(client_animations)) = (
                self.ecs.get::<&EntityIdentity>(entity),
                self.ecs.get::<&EntityTransform>(entity),
                self.ecs.get::<&EntityMetadata>(entity),
                self.ecs.get::<&EntityAttributes>(entity),
                self.ecs.get::<&EntityClientAnimations>(entity),
            ) else {
                continue;
            };
            if let Some(eye_height) = vanilla_eye_height_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(client_animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) {
                eye_positions.insert(target_id, (transform.position, eye_height));
            }
        }

        withers
            .into_iter()
            .map(|(id, position, y_body_rot, scale, target_ids)| {
                let mut targets = [
                    WitherHeadTargetRotations::fallback_to_body(y_body_rot),
                    WitherHeadTargetRotations::fallback_to_body(y_body_rot),
                ];
                for (head_index, target_id) in target_ids.into_iter().enumerate() {
                    if target_id <= 0 {
                        continue;
                    }
                    if let Some((target_position, target_eye_height)) =
                        eye_positions.get(&target_id).copied()
                    {
                        targets[head_index] = wither_side_head_target_rotation(
                            position,
                            y_body_rot,
                            scale,
                            head_index,
                            target_position,
                            target_eye_height,
                        );
                    }
                }
                (id, targets)
            })
            .collect()
    }

    pub(crate) fn mount(&self, id: i32) -> Option<EntityMount> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMount>(entity)
            .ok()
            .map(|mount| (*mount).clone())
    }

    pub(crate) fn debug_passenger_vehicle_target(
        &self,
        passenger_id: i32,
        game_time: i64,
    ) -> Option<super::EntityPickTargetState> {
        let passenger_identity = self.identity(passenger_id)?;
        let passenger_bounds = self.pick_bounds(passenger_id)?;
        let passenger_mount = self.mount(passenger_id)?;
        let vehicle_id = passenger_mount.vehicle_id?;
        let vehicle_bounds = self.pick_bounds(vehicle_id)?;
        let vehicle_entity = self.by_protocol_id.get(&vehicle_id).copied()?;
        let Ok(mut query) = self.ecs.query_one::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMount,
            &EntityMetadata,
            &EntityAttributes,
            Option<&EntityClientAnimations>,
        )>(vehicle_entity) else {
            return None;
        };
        let Some((
            vehicle_identity,
            vehicle_transform,
            vehicle_mount,
            vehicle_metadata,
            vehicle_attributes,
            vehicle_client_animations,
        )) = query.get()
        else {
            return None;
        };
        let (position, bounds) = vanilla_passenger_vehicle_debug_target_for_entity_data(
            vehicle_identity.entity_type_id,
            vehicle_transform.position,
            vehicle_transform.y_rot,
            vehicle_bounds,
            &vehicle_metadata.data_values,
            &vehicle_attributes.attributes,
            vehicle_client_animations.map(|animations| animations.animations),
            &vehicle_mount.passengers,
            passenger_id,
            passenger_identity.entity_type_id,
            passenger_bounds,
            game_time,
        )?;
        Some(super::EntityPickTargetState {
            entity_id: passenger_id,
            position,
            bounds,
        })
    }

    pub(crate) fn ender_dragon_part_parent_id(&self, part_id: i32) -> Option<i32> {
        ender_dragon_part_parent_id(part_id, |parent_id| {
            self.identity(parent_id).is_some_and(|identity| {
                identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID
            })
        })
    }

    #[cfg(test)]
    pub(crate) fn leash(&self, id: i32) -> Option<EntityLeash> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityLeash>(entity)
            .ok()
            .map(|leash| *leash)
    }

    pub(crate) fn mob_effects(&self, id: i32) -> Option<EntityMobEffects> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMobEffects>(entity)
            .ok()
            .map(|effects| (*effects).clone())
    }

    pub(crate) fn damage(&self, id: i32) -> Option<EntityDamage> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityDamage>(entity)
            .ok()
            .map(|damage| (*damage).clone())
    }

    #[cfg(test)]
    pub(crate) fn minecart_lerp(&self, id: i32) -> Option<EntityMinecartLerp> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMinecartLerp>(entity)
            .ok()
            .map(|lerp| (*lerp).clone())
    }

    pub(crate) fn hurting_projectile(&self, id: i32) -> Option<EntityHurtingProjectile> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityHurtingProjectile>(entity)
            .ok()
            .map(|projectile| *projectile)
    }

    pub(crate) fn transform_states(&self) -> Vec<EntityTransformState> {
        let mut transforms = Vec::with_capacity(self.by_protocol_id.len());
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            if let Some(transform) = self.transform_state_for_entity(entity) {
                transforms.push(transform);
            }
        }
        transforms
    }

    pub(crate) fn item_entity_stacks(&self) -> Vec<ItemEntityStackState> {
        self.item_stacks_for_entity_types(&[VANILLA_ENTITY_TYPE_ITEM_ID])
    }

    pub(crate) fn item_stack_for_entity(&self, id: i32) -> Option<ItemStackSummary> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
            identity.entity_type_id,
        )?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        metadata_item_stack(&metadata.data_values, data_id)
            .filter(|stack| stack.item_id.is_some() && stack.count > 0)
            .cloned()
    }

    pub(crate) fn raw_item_stack_for_entity(&self, id: i32) -> Option<ItemStackSummary> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let accessor = vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
            identity.entity_type_id,
            "ITEM_STACK",
        )?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        metadata_item_stack(&metadata.data_values, accessor.data_id).cloned()
    }

    pub(crate) fn entity_age_ticks(&self, id: i32) -> Option<u32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(client_animations.animations.age_ticks)
    }

    pub(crate) fn item_bob_offset(&self, id: i32) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        (identity.entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID).then_some(())?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(client_animations.animations.item_bob_offset())
    }

    pub(crate) fn experience_orb_icon_for_entity(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let (data_id, default) =
            vanilla_entity_metadata_experience_orb_value_accessor_for_entity_type(
                identity.entity_type_id,
            )?;
        let value = self
            .ecs
            .get::<&EntityMetadata>(entity)
            .ok()
            .and_then(|metadata| {
                metadata
                    .data_values
                    .iter()
                    .find(|value| value.data_id == data_id)
                    .and_then(|value| match &value.value {
                        EntityDataValueKind::Int(value) => Some(*value),
                        _ => None,
                    })
            })
            .unwrap_or(default);
        Some(super::experience_orb_icon(value))
    }

    /// The render state of every item-frame / glow-item-frame entity: its resolved wall center, the
    /// facing wall, the `0..=7` item rotation, glow/invisible flags, the framed item, and framed map id.
    /// Drives the 3D item-frame render (vanilla `ItemFrameRenderer`).
    pub(crate) fn item_frame_render_states(&self) -> Vec<ItemFrameRenderState> {
        let mut frames = Vec::new();
        let entity_accessors = vanilla_entity_metadata_entity_accessors()
            .expect("generated base Entity metadata accessors must resolve");
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            let Some(accessors) = vanilla_entity_metadata_item_frame_accessors_for_entity_type(
                identity.entity_type_id,
            ) else {
                continue;
            };
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            frames.push(ItemFrameRenderState {
                entity_id: identity.id,
                center: transform.position,
                facing: item_frame_facing(accessors, identity.data, &metadata.data_values),
                light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
                rotation: item_frame_rotation(accessors, &metadata.data_values),
                glow: vanilla_entity_renderer_minimum_block_light(identity.entity_type_id)
                    .is_some(),
                invisible: self
                    .metadata_byte(
                        identity.id,
                        entity_accessors.shared_flags_data_id,
                        entity_accessors.shared_flags_default,
                    )
                    .unwrap_or(entity_accessors.shared_flags_default)
                    & entity_accessors.invisible_mask
                    != 0,
                item: item_frame_item(accessors, &metadata.data_values).cloned(),
                map_id: item_frame_map_id(accessors, &metadata.data_values),
            });
        }
        frames
    }

    /// The item a humanoid entity holds in its main (`off_hand = false`) or off hand (vanilla
    /// `EntityEquipment` `MainHand`/`OffHand` slot), or `None` for an empty hand / no equipment. Drives
    /// the third-person held-item render.
    pub(crate) fn held_item(&self, id: i32, off_hand: bool) -> Option<ItemStackSummary> {
        let slot = if off_hand {
            ProtocolEquipmentSlot::OffHand
        } else {
            ProtocolEquipmentSlot::MainHand
        };
        self.equipment_item(id, slot)
    }

    /// The item in an arbitrary synced equipment slot, or `None` for an empty slot / no equipment.
    pub(crate) fn equipment_item(
        &self,
        id: i32,
        slot: ProtocolEquipmentSlot,
    ) -> Option<ItemStackSummary> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let equipment = self.ecs.get::<&EntityEquipment>(entity).ok()?;
        let item = equipment
            .equipment
            .iter()
            .find(|update| update.slot == slot)?
            .item
            .clone();
        item.item_id.map(|_| item)
    }

    /// Collects the unique generated ITEM_STACK accessor carried by every requested entity type.
    /// This covers dropped items and owner-specific thrown-item projectile accessors without treating
    /// their coincident wire indices as shared ownership.
    pub(crate) fn item_stacks_for_entity_types(
        &self,
        type_ids: &[i32],
    ) -> Vec<ItemEntityStackState> {
        let mut items = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if !type_ids.contains(&identity.entity_type_id) {
                continue;
            }
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            let Ok(client_animations) = self.ecs.get::<&EntityClientAnimations>(entity) else {
                continue;
            };
            let Some(data_id) = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
                identity.entity_type_id,
            )
            .or_else(|| {
                vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
                    identity.entity_type_id,
                    "ITEM_STACK",
                )
                .map(|accessor| accessor.data_id)
            }) else {
                continue;
            };
            let Some(stack) = metadata_item_stack(&metadata.data_values, data_id)
                .filter(|stack| stack.item_id.is_some() && stack.count > 0)
            else {
                continue;
            };
            items.push(ItemEntityStackState {
                entity_id: identity.id,
                position: transform.position,
                light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
                bob_offset: client_animations.animations.item_bob_offset(),
                stack: stack.clone(),
            });
        }
        items
    }

    /// Collects the firework rocket item render state. Vanilla
    /// `FireworkRocketEntity.DATA_ID_FIREWORKS_ITEM` uses item-stack metadata id 8, followed by
    /// `DATA_ATTACHED_TO_TARGET` (id 9) and `DATA_SHOT_AT_ANGLE` (id 10).
    pub(crate) fn firework_rocket_item_states(&self) -> Vec<FireworkRocketItemState> {
        let mut items = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            let Some(accessors) = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
                identity.entity_type_id,
            ) else {
                continue;
            };
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            if firework_rocket_attached_to_entity(&metadata.data_values, accessors) {
                continue;
            }
            let Some(stack) = firework_rocket_render_stack(&metadata.data_values, accessors) else {
                continue;
            };
            items.push(FireworkRocketItemState {
                entity_id: identity.id,
                position: transform.position,
                light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
                stack: stack.clone(),
                shot_at_angle: firework_rocket_shot_at_angle(&metadata.data_values, accessors),
            });
        }
        items
    }

    pub(crate) fn firework_rocket_explosion_particle_state(
        &self,
        id: i32,
    ) -> Option<FireworkRocketExplosionParticleState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
            identity.entity_type_id,
        )?;
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let stack = self
            .ecs
            .get::<&EntityMetadata>(entity)
            .ok()
            .and_then(|metadata| {
                firework_rocket_render_stack(&metadata.data_values, accessors).cloned()
            });
        let has_explosions = stack
            .as_ref()
            .is_some_and(firework_rocket_stack_has_explosions);
        Some(FireworkRocketExplosionParticleState {
            entity_id: identity.id,
            position: transform.position,
            delta_movement: transform.delta_movement,
            has_explosions,
            explosions: stack
                .map(|stack| stack.component_patch.fireworks_explosions)
                .unwrap_or_default(),
        })
    }

    pub(crate) fn fox_eat_particle_state(&self, id: i32) -> Option<FoxEatParticleState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_FOX_ID {
            return None;
        }
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let item_stack = self.equipment_item(id, ProtocolEquipmentSlot::MainHand)?;
        Some(FoxEatParticleState {
            entity_id: identity.id,
            position: transform.position,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            item_stack,
        })
    }

    /// Collects the ominous item spawner item-cluster render state. The generated
    /// `OminousItemSpawner.DATA_ITEM` accessor owns membership and the item-stack wire index.
    pub(crate) fn ominous_item_spawner_item_states_at_partial_tick(
        &self,
        partial_ticks: f32,
    ) -> Vec<OminousItemSpawnerItemState> {
        let mut items = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            let Ok(client_animations) = self.ecs.get::<&EntityClientAnimations>(entity) else {
                continue;
            };
            let Some(stack) =
                ominous_item_spawner_render_stack(identity.entity_type_id, &metadata.data_values)
            else {
                continue;
            };
            items.push(OminousItemSpawnerItemState {
                entity_id: identity.id,
                position: transform.position,
                age_ticks: client_animations.animations.age_ticks as f32 + partial_ticks,
                stack: stack.clone(),
            });
        }
        items
    }

    pub(crate) fn append_ominous_item_spawner_particle_states(&mut self) {
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID {
                continue;
            }
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            self.pending_ominous_item_spawner_particles
                .push(OminousItemSpawnerParticleState {
                    entity_id: identity.id,
                    position: transform.position,
                });
        }
    }

    #[cfg(test)]
    pub(crate) fn metadata(&self, id: i32) -> Option<EntityMetadata> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMetadata>(entity)
            .ok()
            .map(|metadata| (*metadata).clone())
    }

    #[cfg(test)]
    pub(crate) fn equipment(&self, id: i32) -> Option<EntityEquipment> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityEquipment>(entity)
            .ok()
            .map(|equipment| (*equipment).clone())
    }

    #[cfg(test)]
    pub(crate) fn attributes(&self, id: i32) -> Option<EntityAttributes> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityAttributes>(entity)
            .ok()
            .map(|attributes| (*attributes).clone())
    }

    #[cfg(test)]
    pub(crate) fn transient_events(&self, id: i32) -> Option<EntityTransientEvents> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityTransientEvents>(entity)
            .ok()
            .map(|events| *events)
    }

    pub(crate) fn with_transform_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityTransform) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut transform = self.ecs.get::<&mut EntityTransform>(entity).ok()?;
        let result = update(&mut transform);
        Some(result)
    }

    fn with_client_animations_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityClientAnimations) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        Some(update(&mut animations))
    }

    pub(crate) fn with_metadata_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMetadata) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut metadata = self.ecs.get::<&mut EntityMetadata>(entity).ok()?;
        let result = update(&mut metadata);
        Some(result)
    }

    pub(crate) fn with_equipment_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityEquipment) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut equipment = self.ecs.get::<&mut EntityEquipment>(entity).ok()?;
        let result = update(&mut equipment);
        Some(result)
    }

    pub(crate) fn with_attributes_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityAttributes) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut attributes = self.ecs.get::<&mut EntityAttributes>(entity).ok()?;
        let result = update(&mut attributes);
        Some(result)
    }

    pub(crate) fn with_transient_events_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityTransientEvents) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut events = self.ecs.get::<&mut EntityTransientEvents>(entity).ok()?;
        let result = update(&mut events);
        Some(result)
    }

    pub(crate) fn with_mount_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMount) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut mount = self.ecs.get::<&mut EntityMount>(entity).ok()?;
        let result = update(&mut mount);
        Some(result)
    }

    pub(crate) fn with_leash_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityLeash) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut leash = self.ecs.get::<&mut EntityLeash>(entity).ok()?;
        let result = update(&mut leash);
        Some(result)
    }

    pub(crate) fn with_mob_effects_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMobEffects) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut effects = self.ecs.get::<&mut EntityMobEffects>(entity).ok()?;
        let result = update(&mut effects);
        Some(result)
    }

    pub(crate) fn with_damage_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityDamage) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut damage = self.ecs.get::<&mut EntityDamage>(entity).ok()?;
        let result = update(&mut damage);
        Some(result)
    }

    pub(crate) fn supporting_block(&self, id: i32) -> Option<EntitySupportingBlock> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntitySupportingBlock>(entity)
            .ok()
            .map(|support| *support)
    }

    pub(crate) fn with_supporting_block_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntitySupportingBlock) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut support = self.ecs.get::<&mut EntitySupportingBlock>(entity).ok()?;
        let result = update(&mut support);
        Some(result)
    }

    pub(crate) fn rebuild_local_player_entity_for_respawn(
        &mut self,
        id: i32,
        keep_entity_data: bool,
        keep_attribute_modifiers: bool,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;

        if let Ok(mut identity) = self.ecs.get::<&mut EntityIdentity>(entity) {
            identity.data = 0;
        }
        if let Ok(mut transform) = self.ecs.get::<&mut EntityTransform>(entity) {
            let (delta_movement, y_rot, x_rot) = if keep_entity_data {
                (transform.delta_movement, transform.y_rot, transform.x_rot)
            } else {
                (EntityVec3::default(), -180.0, 0.0)
            };
            *transform = EntityTransform {
                position: EntityVec3::default(),
                old_position: EntityVec3::default(),
                position_base: EntityVec3::default(),
                delta_movement,
                y_rot,
                old_y_rot: y_rot,
                x_rot,
                old_x_rot: x_rot,
                y_head_rot: 0.0,
                old_y_head_rot: 0.0,
                on_ground: None,
                interpolation: None,
            };
        }
        if !keep_entity_data {
            if let Ok(mut metadata) = self.ecs.get::<&mut EntityMetadata>(entity) {
                *metadata = EntityMetadata::default();
            }
        }
        if let Ok(mut equipment) = self.ecs.get::<&mut EntityEquipment>(entity) {
            *equipment = EntityEquipment::default();
        }
        if let Ok(mut attributes) = self.ecs.get::<&mut EntityAttributes>(entity) {
            if !keep_attribute_modifiers {
                for attribute in &mut *attributes.attributes {
                    attribute.modifiers.clear();
                }
            }
        }
        if let Ok(mut events) = self.ecs.get::<&mut EntityTransientEvents>(entity) {
            *events = EntityTransientEvents::default();
        }
        if let Ok(mut mount) = self.ecs.get::<&mut EntityMount>(entity) {
            *mount = EntityMount::default();
        }
        if let Ok(mut leash) = self.ecs.get::<&mut EntityLeash>(entity) {
            *leash = EntityLeash::default();
        }
        if let Ok(mut effects) = self.ecs.get::<&mut EntityMobEffects>(entity) {
            *effects = EntityMobEffects::default();
        }
        if let Ok(mut animations) = self.ecs.get::<&mut EntityClientAnimations>(entity) {
            *animations = EntityClientAnimations::default();
        }
        if let Ok(mut damage) = self.ecs.get::<&mut EntityDamage>(entity) {
            *damage = EntityDamage::default();
        }
        if let Ok(mut support) = self.ecs.get::<&mut EntitySupportingBlock>(entity) {
            *support = EntitySupportingBlock::default();
        }
        if keep_entity_data {
            let _ = self.sync_client_animation_targets_from_metadata(id);
        }
        Some(())
    }

    pub(crate) fn with_minecart_lerp_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityMinecartLerp) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut lerp = self.ecs.get::<&mut EntityMinecartLerp>(entity).ok()?;
        let result = update(&mut lerp);
        Some(result)
    }

    pub(crate) fn with_hurting_projectile_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut EntityHurtingProjectile) -> R,
    ) -> Option<R> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut projectile = self.ecs.get::<&mut EntityHurtingProjectile>(entity).ok()?;
        let result = update(&mut projectile);
        Some(result)
    }

    pub(crate) fn sync_client_animation_targets_from_metadata(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?.clone();
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?.clone();
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations
            .animations
            .sync_targets_from_metadata(identity.entity_type_id, &metadata.data_values);
        Some(())
    }

    pub(crate) fn sync_client_animation_events_from_metadata_update(
        &mut self,
        id: i32,
        updated_values: &[bbb_protocol::packets::EntityDataValue],
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?.clone();
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?.clone();
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations.animations.sync_events_from_metadata_update(
            identity.entity_type_id,
            updated_values,
            &metadata.data_values,
        );
        Some(())
    }

    pub(crate) fn apply_client_animation_entity_event(
        &mut self,
        id: i32,
        event_id: i8,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        let event_kind = vanilla_entity_event_kind(entity_type_id, event_id)?;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations
            .animations
            .handle_entity_event(entity_type_id, id, event_kind);
        Some(())
    }

    #[cfg(test)]
    pub(crate) fn minecart_spawner_spawn_delay(&self, id: i32) -> Option<i32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID {
            return None;
        }
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(animations.animations.minecart_spawner_spawn_delay())
    }

    /// Applies the canonical state mutation from `LivingEntity.handleEntityEvent(3)`.
    ///
    /// Vanilla plays the death sound for Players too, but deliberately skips `setHealth(0)` and
    /// `die(generic)` for every `Player` instance. The caller owns the sound side effect; this
    /// method owns only the non-Player health/death-state transition.
    pub(crate) fn apply_client_living_death_entity_event(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        let accessors =
            vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)?;
        if vanilla_entity_metadata_type_uses_player_entity_event_handler(entity_type_id) {
            return Some(());
        }

        {
            let mut metadata = self.ecs.get::<&mut EntityMetadata>(entity).ok()?;
            let health = ProtocolEntityDataValue {
                data_id: accessors.health_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(0.0),
            };
            if let Some(existing) = metadata
                .data_values
                .iter_mut()
                .find(|value| value.data_id == accessors.health_data_id)
            {
                *existing = health;
            } else {
                metadata.data_values.push(health);
                metadata.data_values.sort_by_key(|value| value.data_id);
            }
        }

        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?.clone();
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations
            .animations
            .sync_targets_from_metadata(entity_type_id, &metadata.data_values);
        Some(())
    }

    pub(crate) fn apply_client_animation_hurt(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        if vanilla_entity_metadata_type_is_living(entity_type_id) {
            animations.animations.trigger_hurt();
            Some(())
        } else if is_vanilla_vehicle_entity_type(entity_type_id) {
            animations.animations.trigger_vehicle_hurt();
            Some(())
        } else {
            None
        }
    }

    pub(crate) fn lerp_head_to(
        &mut self,
        id: i32,
        target_y_head_rot: f32,
        steps: i32,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        if vanilla_entity_metadata_type_is_living(entity_type_id) {
            let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
            animations
                .animations
                .start_head_rotation_lerp(target_y_head_rot, steps);
        } else {
            let mut transform = self.ecs.get::<&mut EntityTransform>(entity).ok()?;
            transform.y_head_rot = target_y_head_rot;
        }
        Some(())
    }

    pub(crate) fn trigger_client_animation_hurt(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations.animations.trigger_hurt();
        Some(())
    }

    /// Arms the melee swing for `id` (vanilla `LivingEntity.swing`), triggered by the
    /// `ClientboundAnimate` packet's main-hand (`off_hand = false`) / off-hand actions.
    pub(crate) fn trigger_client_animation_swing(
        &mut self,
        id: i32,
        off_hand: bool,
        duration: i32,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations.animations.trigger_swing(off_hand, duration);
        Some(())
    }

    pub(crate) fn active_swing_off_hand(&self, id: i32) -> Option<bool> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        animations.animations.active_swing_off_hand()
    }

    pub(crate) fn attack_swing_state(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<LocalPlayerAttackSwingState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(LocalPlayerAttackSwingState {
            attack_anim: animations.animations.attack_anim(partial_ticks),
            off_hand: animations.animations.attack_arm_off_hand(),
        })
    }

    pub(crate) fn ticks_since_kinetic_hit_feedback(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(
            animations
                .animations
                .ticks_since_kinetic_hit_feedback(partial_ticks),
        )
    }

    pub(crate) fn refresh_client_animation_swing_duration(
        &mut self,
        id: i32,
        duration: i32,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations.animations.refresh_swing_duration(duration);
        Some(())
    }

    pub(crate) fn interpolation_collision_inputs(&self) -> Vec<EntityInterpolationCollisionInput> {
        let mut inputs = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityAttributes,
            &EntityClientAnimations,
        )>();
        for (_, (identity, transform, metadata, attributes, animations)) in query.iter() {
            let Some(interpolation) = transform.interpolation else {
                continue;
            };
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
            let delta = EntityVec3 {
                x: transform.position.x - interpolation.previous_position.x,
                y: transform.position.y - interpolation.previous_position.y,
                z: transform.position.z - interpolation.previous_position.z,
            };
            inputs.push(EntityInterpolationCollisionInput {
                entity_id: identity.id,
                entity_type_id: identity.entity_type_id,
                current_position: transform.position,
                target_position: EntityVec3 {
                    x: interpolation.target_position.x + delta.x,
                    y: interpolation.target_position.y + delta.y,
                    z: interpolation.target_position.z + delta.z,
                },
                bounds,
            });
        }
        inputs
    }

    /// Gathers the world AABBs the per-entity `in_water` map is built from, for the
    /// entity types whose client-tick animation reads `isInWater()`
    /// ([`entity_animation_uses_in_water`]). Each tuple is `(entity_id, aabb_min,
    /// aabb_max)` with the AABB resolved from the SAME bounds source the projection
    /// uses ([`Self::pick_bounds`] → `vanilla_pick_bounds_for_entity_data`), so the
    /// fluid overlap matches the projected `in_water`. `WorldStore` (which holds the
    /// chunk/fluid data) turns each AABB into a `bool`.
    pub(crate) fn in_water_aabb_inputs(&self) -> Vec<(i32, [f64; 3], [f64; 3])> {
        let mut inputs = Vec::new();
        let mut query = self.ecs.query::<(
            &EntityIdentity,
            &EntityTransform,
            &EntityMetadata,
            &EntityAttributes,
            &EntityClientAnimations,
        )>();
        for (_, (identity, transform, metadata, attributes, animations)) in query.iter() {
            if !entity_animation_uses_in_water(identity.entity_type_id)
                && !super::client_movement::client_movement_uses_in_water(identity.entity_type_id)
            {
                continue;
            }
            let Some(bounds) = vanilla_pick_bounds_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(animations.animations),
                metadata.painting_variant_projection.as_ref(),
            ) else {
                continue;
            };
            let aabb_min = [
                transform.position.x + f64::from(bounds.min[0]),
                transform.position.y + f64::from(bounds.min[1]),
                transform.position.z + f64::from(bounds.min[2]),
            ];
            let aabb_max = [
                transform.position.x + f64::from(bounds.max[0]),
                transform.position.y + f64::from(bounds.max[1]),
                transform.position.z + f64::from(bounds.max[2]),
            ];
            inputs.push((identity.id, aabb_min, aabb_max));
        }
        inputs
    }

    pub(crate) fn advance_client_animations<F, G>(
        &mut self,
        ticks: u32,
        runs_normally: bool,
        spectator_player_uuids: &HashSet<Uuid>,
        in_water_by_id: &HashMap<i32, bool>,
        client_movement_collisions_by_id: &HashMap<
            i32,
            super::client_movement::EntityClientMovementCollisionResult,
        >,
        minecart_improvements_enabled: bool,
        death_tick_player_chunk: Option<(i32, i32)>,
        death_tick_simulation_distance: Option<i32>,
        is_ticking_entity: &F,
        can_carry_interpolation_target: &G,
    ) -> EntityClientTickWork
    where
        F: Fn(i32, i32, EntityVec3) -> bool,
        G: Fn(i32) -> bool,
    {
        let mut work = EntityClientTickWork::default();
        let preparation_started_at = Instant::now();
        let wither_head_targets_by_id = self.wither_head_targets_by_id();
        work.preparation_duration += preparation_started_at.elapsed();
        for _ in 0..ticks {
            let mut tick_scratch = std::mem::take(&mut self.client_tick_scratch);
            let schedule_started_at = Instant::now();
            self.fill_client_tick_schedule(&mut tick_scratch, runs_normally, is_ticking_entity);
            self.prepare_scheduled_passengers_for_ride_tick(&tick_scratch.scheduled_entities);
            work.schedule_duration += schedule_started_at.elapsed();
            let scheduled = &tick_scratch.scheduled_set;
            let projectile_started_at = Instant::now();
            self.advance_projectile_lifecycles(1, spectator_player_uuids, scheduled);
            work.projectile_duration += projectile_started_at.elapsed();
            let interpolation_started_at = Instant::now();
            for (_, (identity, transform)) in self
                .ecs
                .query_mut::<(&EntityIdentity, &mut EntityTransform)>()
            {
                let is_ticking = scheduled.contains(&identity.id);
                if !is_ticking {
                    transform.advance_interpolation_tick(true, false, false);
                    continue;
                }
                transform.begin_client_tick();
                let has_interpolation_handler = vanilla_entity_metadata_type_interpolation_steps(
                    identity.entity_type_id,
                    minecart_improvements_enabled,
                )
                .is_some();
                transform.advance_interpolation_tick(
                    has_interpolation_handler,
                    true,
                    can_carry_interpolation_target(identity.id),
                );
            }
            work.interpolation_duration += interpolation_started_at.elapsed();
            let movement_started_at = Instant::now();
            self.advance_direct_client_movement(
                &tick_scratch.scheduled_entities,
                in_water_by_id,
                client_movement_collisions_by_id,
            );
            work.projectile_duration += movement_started_at.elapsed();
            let animation_started_at = Instant::now();
            work.scanned_entities = work
                .scanned_entities
                .saturating_add(self.by_protocol_id.len());
            for &entity in &tick_scratch.scheduled_entities {
                let Ok((
                    identity,
                    transform,
                    mount,
                    metadata,
                    attributes,
                    equipment,
                    animations,
                    minecart_lerp,
                )) = self.ecs.query_one_mut::<(
                    &EntityIdentity,
                    &mut EntityTransform,
                    &EntityMount,
                    &EntityMetadata,
                    &EntityAttributes,
                    Option<&EntityEquipment>,
                    &mut EntityClientAnimations,
                    Option<&mut EntityMinecartLerp>,
                )>(entity)
                else {
                    continue;
                };
                animations
                    .animations
                    .advance_head_rotation_lerp(&mut transform.y_head_rot);
                // Vanilla `LivingEntity.calculateEntityAnimation` gates the limb
                // swing on `!isPassenger()` and scales the position by `3` for a
                // baby (`updateWalkAnimation`).
                let is_passenger = mount.vehicle_id.is_some();
                let should_tick_death = within_death_tick_distance(
                    transform.position,
                    death_tick_player_chunk,
                    death_tick_simulation_distance,
                );
                let entity_type_id = identity.entity_type_id;
                let is_living = vanilla_entity_metadata_type_is_living(entity_type_id);
                let is_baby = is_living && vanilla_is_baby(entity_type_id, &metadata.data_values);
                let pose = is_living.then(|| entity_data_pose(&metadata.data_values));
                let is_fall_flying = is_living && entity_is_fall_flying(&metadata.data_values);
                let is_crouching = pose == Some(VANILLA_ENTITY_POSE_CROUCHING_ID);
                let bat_is_resting = entity_type_id == VANILLA_ENTITY_TYPE_BAT_ID
                    && bat_is_resting(entity_type_id, &metadata.data_values);
                let is_swimming = entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                    && pose == Some(VANILLA_ENTITY_POSE_SWIMMING_ID);
                // The per-tick world fact (`isInWater()`) the world resolved before
                // this mutable pass, defaulting to `false` for non-consumers, and the
                // synced `Guardian.DATA_ID_MOVING` flag read from the metadata here.
                let in_water = entity_animation_uses_in_water(entity_type_id)
                    && in_water_by_id.get(&identity.id).copied().unwrap_or(false);
                let is_guardian = vanilla_entity_metadata_type_is_guardian(entity_type_id);
                let is_moving =
                    is_guardian && guardian_is_moving(entity_type_id, &metadata.data_values);
                let warden_heartbeat_delay = if entity_type_id == VANILLA_ENTITY_TYPE_WARDEN_ID {
                    warden_heartbeat_delay(entity_type_id, &metadata.data_values)
                } else {
                    0
                };
                let guardian_attack_target_id = if is_guardian {
                    guardian_attack_target_id(entity_type_id, &metadata.data_values)
                } else {
                    0
                };
                let wither_head_targets = (entity_type_id == VANILLA_ENTITY_TYPE_WITHER_ID)
                    .then(|| wither_head_targets_by_id.get(&identity.id).copied())
                    .flatten();
                let is_camel = vanilla_entity_metadata_type_is_camel(entity_type_id);
                let camel_is_standing = is_camel && pose == Some(VANILLA_ENTITY_POSE_STANDING_ID);
                let camel_is_dashing =
                    is_camel && camel_is_dashing(entity_type_id, &metadata.data_values);
                let copper_golem_animation =
                    if entity_type_id == VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID {
                        copper_golem_animation_flags(entity_type_id, &metadata.data_values)
                    } else {
                        Default::default()
                    };
                let allay_is_dancing = entity_type_id == VANILLA_ENTITY_TYPE_ALLAY_ID
                    && allay_is_dancing(entity_type_id, &metadata.data_values);
                let allay_has_item_in_hand = entity_type_id == VANILLA_ENTITY_TYPE_ALLAY_ID
                    && equipment.is_some_and(|equipment| {
                        equipment.equipment.iter().any(|update| {
                            update.slot == ProtocolEquipmentSlot::MainHand
                                && update.item.item_id.is_some()
                        })
                    });
                let axolotl_is_playing_dead = entity_type_id == VANILLA_ENTITY_TYPE_AXOLOTL_ID
                    && axolotl_is_playing_dead(entity_type_id, &metadata.data_values);
                let creaking_is_tearing_down = entity_type_id == VANILLA_ENTITY_TYPE_CREAKING_ID
                    && creaking_is_tearing_down(entity_type_id, &metadata.data_values);
                let pillager_is_charging_crossbow = entity_type_id
                    == VANILLA_ENTITY_TYPE_PILLAGER_ID
                    && pillager_is_charging_crossbow(entity_type_id, &metadata.data_values);
                let piglin_is_charging_crossbow = entity_type_id == VANILLA_ENTITY_TYPE_PIGLIN_ID
                    && piglin_is_charging_crossbow(entity_type_id, &metadata.data_values);
                let player_is_using_item = entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
                    && player_is_using_item(entity_type_id, &metadata.data_values);
                let wolf_is_interested = entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID
                    && wolf_is_interested(entity_type_id, &metadata.data_values);
                let cat_is_lying = entity_type_id == VANILLA_ENTITY_TYPE_CAT_ID
                    && cat_is_lying(entity_type_id, &metadata.data_values);
                let cat_relax_state_one = entity_type_id == VANILLA_ENTITY_TYPE_CAT_ID
                    && cat_relax_state_one(entity_type_id, &metadata.data_values);
                let is_boat = is_vanilla_boat_type(entity_type_id);
                let boat_paddles = if is_boat {
                    boat_paddle_states(
                        entity_type_id,
                        &metadata.data_values,
                        !mount.passengers.is_empty(),
                    )
                } else {
                    [false; 2]
                };
                let boat_bubble_time_value = if is_boat {
                    boat_bubble_time(entity_type_id, &metadata.data_values)
                } else {
                    0
                };
                let client_animations_before_tick = animations
                    .animations
                    .needs_particle_trigger_bounds_snapshot()
                    .then_some(animations.animations);
                let particle_triggers = animations.animations.advance_client_tick(
                    identity.entity_type_id,
                    identity.id,
                    *transform,
                    should_tick_death,
                    is_passenger,
                    is_baby,
                    is_fall_flying,
                    is_crouching,
                    bat_is_resting,
                    in_water,
                    is_moving,
                    warden_heartbeat_delay,
                    guardian_attack_target_id,
                    wither_head_targets,
                    camel_is_standing,
                    camel_is_dashing,
                    copper_golem_animation.idle,
                    copper_golem_animation.getting_item,
                    copper_golem_animation.getting_no_item,
                    copper_golem_animation.dropping_item,
                    copper_golem_animation.dropping_no_item,
                    allay_is_dancing,
                    allay_has_item_in_hand,
                    axolotl_is_playing_dead,
                    creaking_is_tearing_down,
                    pillager_is_charging_crossbow,
                    piglin_is_charging_crossbow,
                    player_is_using_item,
                    wolf_is_interested,
                    cat_is_lying,
                    cat_relax_state_one,
                    boat_paddles[0],
                    boat_paddles[1],
                    boat_bubble_time_value,
                    is_swimming,
                );
                append_client_tick_particle_triggers(
                    &mut self.pending_ravager_stun_particles,
                    &mut self.pending_evoker_fangs_crit_particles,
                    identity,
                    transform,
                    &metadata.data_values,
                    &attributes.attributes,
                    client_animations_before_tick,
                    particle_triggers,
                );
                // `advance_client_tick` always advances `age_ticks`, including
                // entities without a specialized animation family.
                work.mutated_entities = work.mutated_entities.saturating_add(1);
                if minecart_improvements_enabled {
                    if let Some(minecart_lerp) = minecart_lerp {
                        minecart_lerp.advance_client_tick(transform);
                    }
                }
            }
            work.animation_duration += animation_started_at.elapsed();
            self.client_tick_scratch = tick_scratch;
        }
        work
    }

    fn fill_client_tick_schedule<F>(
        &self,
        scratch: &mut EntityClientTickScratch,
        runs_normally: bool,
        is_ticking_entity: &F,
    ) where
        F: Fn(i32, i32, EntityVec3) -> bool,
    {
        scratch.clear();
        if !runs_normally {
            self.fill_player_passenger_ancestors(scratch);
        }
        for &id in &self.order {
            let Some(entity) = self.by_protocol_id.get(&id).copied() else {
                continue;
            };
            let (Ok(identity), Ok(transform), Ok(mount)) = (
                self.ecs.get::<&EntityIdentity>(entity),
                self.ecs.get::<&EntityTransform>(entity),
                self.ecs.get::<&EntityMount>(entity),
            ) else {
                continue;
            };
            if mount.vehicle_id.is_some()
                || !is_ticking_entity(identity.id, identity.entity_type_id, transform.position)
                || (!runs_normally
                    && identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
                    && !scratch.player_passenger_ancestors.contains(&identity.id))
            {
                continue;
            }
            self.append_client_tick_passenger_tree(id, scratch, is_ticking_entity);
        }
    }

    fn prepare_scheduled_passengers_for_ride_tick(&mut self, scheduled_entities: &[Entity]) {
        for &entity in scheduled_entities {
            let Ok((mount, transform)) = self
                .ecs
                .query_one_mut::<(&EntityMount, &mut EntityTransform)>(entity)
            else {
                continue;
            };
            if mount.vehicle_id.is_some() {
                // Vanilla `Entity.rideTick` clears motion immediately before
                // calling the entity's ordinary `tick`. Specialized client
                // movement therefore starts from zero rather than retaining a
                // stale packet velocity. Root entities keep their velocity.
                transform.delta_movement = EntityVec3::default();
            }
        }
    }

    fn fill_player_passenger_ancestors(&self, scratch: &mut EntityClientTickScratch) {
        let max_depth = self.by_protocol_id.len();
        for &id in &self.order {
            let Some(entity) = self.by_protocol_id.get(&id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
                continue;
            }

            let mut passenger_id = identity.id;
            for _ in 0..max_depth {
                let Some(passenger) = self.by_protocol_id.get(&passenger_id).copied() else {
                    break;
                };
                let Ok(mount) = self.ecs.get::<&EntityMount>(passenger) else {
                    break;
                };
                let Some(vehicle_id) = mount.vehicle_id else {
                    break;
                };
                scratch.player_passenger_ancestors.insert(vehicle_id);
                passenger_id = vehicle_id;
            }
        }
    }

    fn append_client_tick_passenger_tree<F>(
        &self,
        id: i32,
        scratch: &mut EntityClientTickScratch,
        is_ticking_entity: &F,
    ) where
        F: Fn(i32, i32, EntityVec3) -> bool,
    {
        let Some(entity) = self.by_protocol_id.get(&id).copied() else {
            return;
        };
        if !scratch.scheduled_set.insert(id) {
            return;
        }
        scratch.scheduled_entities.push(entity);
        let Ok(mount) = self.ecs.get::<&EntityMount>(entity) else {
            return;
        };

        for &passenger_id in mount.passengers.iter() {
            let Some(passenger) = self.by_protocol_id.get(&passenger_id).copied() else {
                continue;
            };
            let (Ok(identity), Ok(transform), Ok(mount)) = (
                self.ecs.get::<&EntityIdentity>(passenger),
                self.ecs.get::<&EntityTransform>(passenger),
                self.ecs.get::<&EntityMount>(passenger),
            ) else {
                continue;
            };
            if mount.vehicle_id != Some(id)
                || (identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID
                    && !is_ticking_entity(identity.id, identity.entity_type_id, transform.position))
            {
                continue;
            }
            self.append_client_tick_passenger_tree(passenger_id, scratch, is_ticking_entity);
        }
    }

    pub(crate) fn publication_snapshot_observation(&self) -> EntityPublicationSnapshotObservation {
        self.publication_snapshot_observation
    }

    pub(crate) fn for_each_mount_mut(&mut self, mut update: impl FnMut(i32, &mut EntityMount)) {
        // Every entity spawns with an `EntityMount`, so iterating the ecs directly visits the same
        // set as `self.order` without cloning it. All callers apply an order-independent per-entity
        // mutation, so the archetype iteration order is not observable.
        for (_, (identity, mount)) in self.ecs.query_mut::<(&EntityIdentity, &mut EntityMount)>() {
            update(identity.id, mount);
        }
    }

    pub(crate) fn for_each_leash_mut(&mut self, mut update: impl FnMut(i32, &mut EntityLeash)) {
        let ids = self.order.clone();
        for id in ids {
            let _ = self.with_leash_mut(id, |leash| update(id, leash));
        }
    }

    pub(crate) fn states(&self) -> Vec<EntityState> {
        self.order
            .iter()
            .filter_map(|id| self.by_protocol_id.get(id).copied())
            .filter_map(|entity| self.project_entity(entity))
            .collect()
    }

    pub(crate) fn total_mob_effects(&self) -> usize {
        self.order
            .iter()
            .filter_map(|id| self.by_protocol_id.get(id).copied())
            .filter_map(|entity| self.ecs.get::<&EntityMobEffects>(entity).ok())
            .map(|effects| effects.effects.len())
            .sum()
    }

    pub(crate) fn total_minecart_lerp_steps(&self) -> usize {
        self.order
            .iter()
            .filter_map(|id| self.by_protocol_id.get(id).copied())
            .filter_map(|entity| self.ecs.get::<&EntityMinecartLerp>(entity).ok())
            .map(|lerp| lerp.pending_steps.len() + lerp.steps.len())
            .sum()
    }

    pub(crate) fn len(&self) -> usize {
        self.by_protocol_id.len()
    }

    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn remove_ids(&mut self, ids: &[i32]) -> usize {
        let mut removed = 0;
        for id in ids {
            let Some(entity) = self.by_protocol_id.remove(id) else {
                continue;
            };
            let _ = self.ecs.despawn(entity);
            removed += 1;
        }
        if removed > 0 {
            self.order.retain(|id| self.by_protocol_id.contains_key(id));
        }
        removed
    }

    fn transform_state_for_entity(&self, entity: Entity) -> Option<EntityTransformState> {
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        Some(EntityTransformState::from_components(&identity, *transform))
    }

    fn sync_components_from_state(&mut self, entity: Entity, state: &EntityState) {
        if let Ok(mut identity) = self.ecs.get::<&mut EntityIdentity>(entity) {
            *identity = EntityIdentity::from(state);
        }
        if let Ok(mut transform) = self.ecs.get::<&mut EntityTransform>(entity) {
            *transform = EntityTransform::from(state);
        }
        if let Ok(mut metadata) = self.ecs.get::<&mut EntityMetadata>(entity) {
            *metadata = EntityMetadata::from(state);
        }
        if let Ok(mut equipment) = self.ecs.get::<&mut EntityEquipment>(entity) {
            *equipment = EntityEquipment::from(state);
        }
        if let Ok(mut attributes) = self.ecs.get::<&mut EntityAttributes>(entity) {
            *attributes = EntityAttributes::from(state);
        }
        if let Ok(mut events) = self.ecs.get::<&mut EntityTransientEvents>(entity) {
            *events = EntityTransientEvents::from(state);
        }
        if let Ok(mut mount) = self.ecs.get::<&mut EntityMount>(entity) {
            *mount = EntityMount::from(state);
        }
        if let Ok(mut leash) = self.ecs.get::<&mut EntityLeash>(entity) {
            *leash = EntityLeash::from(state);
        }
        if let Ok(mut effects) = self.ecs.get::<&mut EntityMobEffects>(entity) {
            *effects = EntityMobEffects::from(state);
        }
        if let Ok(mut animations) = self.ecs.get::<&mut EntityClientAnimations>(entity) {
            *animations = EntityClientAnimations::from(state);
        }
        if let Ok(mut damage) = self.ecs.get::<&mut EntityDamage>(entity) {
            *damage = EntityDamage::from(state);
        }
        if let Ok(mut support) = self.ecs.get::<&mut EntitySupportingBlock>(entity) {
            *support = EntitySupportingBlock::from(state);
        }
        // Keep `EntityMinecartLerp` attached only while the entity is a vanilla minecart. A re-add
        // packet can in principle change an existing id's type, so mirror
        // `sync_hurting_projectile_from_state` and insert / update / remove the component to match.
        if is_vanilla_minecart_type(state.entity_type_id) {
            let updated = {
                if let Ok(mut lerp) = self.ecs.get::<&mut EntityMinecartLerp>(entity) {
                    *lerp = EntityMinecartLerp::from(state);
                    true
                } else {
                    false
                }
            };
            if !updated {
                let _ = self.ecs.insert_one(entity, EntityMinecartLerp::from(state));
            }
        } else {
            let _ = self.ecs.remove_one::<EntityMinecartLerp>(entity);
        }
        self.sync_hurting_projectile_from_state(entity, state);
        self.sync_projectile_lifecycle_from_state(entity, state);
    }

    fn sync_hurting_projectile_from_state(&mut self, entity: Entity, state: &EntityState) {
        if let Some(projectile) =
            entity_hurting_projectile_from_state(state.entity_type_id, state.hurting_projectile)
        {
            let updated = {
                if let Ok(mut existing) = self.ecs.get::<&mut EntityHurtingProjectile>(entity) {
                    *existing = projectile;
                    true
                } else {
                    false
                }
            };
            if !updated {
                let _ = self.ecs.insert_one(entity, projectile);
            }
        } else {
            let _ = self.ecs.remove_one::<EntityHurtingProjectile>(entity);
        }
    }

    fn sync_projectile_lifecycle_from_state(&mut self, entity: Entity, state: &EntityState) {
        let projectile = entity_projectile_lifecycle_from_state(
            state.entity_type_id,
            state.projectile_lifecycle,
            self.uuid_for_protocol_id(state.data),
        );
        if let Some(projectile) = projectile {
            let updated = {
                if let Ok(mut existing) = self.ecs.get::<&mut EntityProjectileLifecycle>(entity) {
                    *existing = projectile;
                    true
                } else {
                    false
                }
            };
            if !updated {
                let _ = self.ecs.insert_one(entity, projectile);
            }
        } else {
            let _ = self.ecs.remove_one::<EntityProjectileLifecycle>(entity);
        }
    }

    fn project_entity(&self, entity: Entity) -> Option<EntityState> {
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let equipment = self.ecs.get::<&EntityEquipment>(entity).ok()?;
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        let events = self.ecs.get::<&EntityTransientEvents>(entity).ok()?;
        let mount = self.ecs.get::<&EntityMount>(entity).ok()?;
        let leash = self.ecs.get::<&EntityLeash>(entity).ok()?;
        let effects = self.ecs.get::<&EntityMobEffects>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        let damage = self.ecs.get::<&EntityDamage>(entity).ok()?;
        let supporting_block = self.ecs.get::<&EntitySupportingBlock>(entity).ok()?;
        let minecart_lerp = self.ecs.get::<&EntityMinecartLerp>(entity).ok();
        let hurting_projectile = self.ecs.get::<&EntityHurtingProjectile>(entity).ok();
        let projectile_lifecycle = self.ecs.get::<&EntityProjectileLifecycle>(entity).ok();

        let mut state = EntityState {
            id: identity.id,
            uuid: identity.uuid,
            entity_type_id: identity.entity_type_id,
            data: identity.data,
            position: transform.position,
            position_base: transform.position_base,
            delta_movement: transform.delta_movement,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            y_head_rot: transform.y_head_rot,
            on_ground: transform.on_ground,
            interpolation: None,
            main_supporting_block_pos: None,
            on_ground_no_blocks: false,
            data_values: Vec::new(),
            painting_variant_projection: None,
            equipment: Vec::new(),
            attributes: Vec::new(),
            vehicle_id: None,
            passengers: Vec::new(),
            leash_holder_id: None,
            last_animation_action: None,
            last_event_id: None,
            last_hurt_yaw: None,
            mob_effects: BTreeMap::new(),
            client_animations: Default::default(),
            last_damage: None,
            minecart_lerp_pending_steps: Vec::new(),
            minecart_lerp_steps: Vec::new(),
            minecart_lerp_old_step: None,
            minecart_lerp_delay: 0,
            hurting_projectile: None,
            projectile_lifecycle: None,
        };
        (*transform).write_to_state(&mut state);
        (*metadata).clone().write_to_state(&mut state);
        (*equipment).clone().write_to_state(&mut state);
        (*attributes).clone().write_to_state(&mut state);
        (*events).write_to_state(&mut state);
        (*mount).clone().write_to_state(&mut state);
        (*leash).write_to_state(&mut state);
        (*effects).clone().write_to_state(&mut state);
        (*client_animations).write_to_state(&mut state);
        (*damage).clone().write_to_state(&mut state);
        (*supporting_block).write_to_state(&mut state);
        if let Some(minecart_lerp) = minecart_lerp {
            (*minecart_lerp).clone().write_to_state(&mut state);
        }
        if let Some(projectile) = hurting_projectile {
            (*projectile).write_to_state(&mut state);
        }
        if let Some(projectile) = projectile_lifecycle {
            (*projectile).write_to_state(&mut state);
        }
        Some(state)
    }
}

impl Default for EntityStore {
    fn default() -> Self {
        Self {
            ecs: World::new(),
            by_protocol_id: BTreeMap::new(),
            order: Vec::new(),
            pending_ravager_stun_particles: Vec::new(),
            pending_evoker_fangs_crit_particles: Vec::new(),
            pending_entity_client_movement_particles: Vec::new(),
            pending_ominous_item_spawner_particles: Vec::new(),
            client_tick_scratch: EntityClientTickScratch::default(),
            publication_snapshot_observation: EntityPublicationSnapshotObservation::default(),
        }
    }
}

impl Clone for EntityStore {
    fn clone(&self) -> Self {
        let snapshot_started_at = Instant::now();
        // Clone component-by-component instead of round-tripping every entity
        // through a full `EntityState` projection. Immutable simulation
        // publications still rebuild this ECS explicitly: keeping that cost at
        // publication avoids a whole-store COW that would merely move the same
        // work into the next entity mutation.
        let mut store = Self::default();
        for (&id, &entity) in &self.by_protocol_id {
            let (
                Ok(identity),
                Ok(transform),
                Ok(metadata),
                Ok(equipment),
                Ok(attributes),
                Ok(events),
                Ok(mount),
                Ok(leash),
                Ok(effects),
                Ok(animations),
                Ok(damage),
                Ok(supporting_block),
            ) = (
                self.ecs.get::<&EntityIdentity>(entity),
                self.ecs.get::<&EntityTransform>(entity),
                self.ecs.get::<&EntityMetadata>(entity),
                self.ecs.get::<&EntityEquipment>(entity),
                self.ecs.get::<&EntityAttributes>(entity),
                self.ecs.get::<&EntityTransientEvents>(entity),
                self.ecs.get::<&EntityMount>(entity),
                self.ecs.get::<&EntityLeash>(entity),
                self.ecs.get::<&EntityMobEffects>(entity),
                self.ecs.get::<&EntityClientAnimations>(entity),
                self.ecs.get::<&EntityDamage>(entity),
                self.ecs.get::<&EntitySupportingBlock>(entity),
            )
            else {
                continue;
            };
            let cloned = store.ecs.spawn((
                (*identity).clone(),
                *transform,
                (*metadata).clone(),
                (*equipment).clone(),
                (*attributes).clone(),
                *events,
                (*mount).clone(),
                *leash,
                (*effects).clone(),
                *animations,
                (*damage).clone(),
                *supporting_block,
            ));
            if let Ok(lerp) = self.ecs.get::<&EntityMinecartLerp>(entity) {
                let _ = store.ecs.insert_one(cloned, (*lerp).clone());
            }
            if let Ok(projectile) = self.ecs.get::<&EntityHurtingProjectile>(entity) {
                let _ = store.ecs.insert_one(cloned, *projectile);
            }
            if let Ok(projectile) = self.ecs.get::<&EntityProjectileLifecycle>(entity) {
                let _ = store.ecs.insert_one(cloned, *projectile);
            }
            store.by_protocol_id.insert(id, cloned);
        }
        store.order = self.order.clone();
        store.pending_ravager_stun_particles = self.pending_ravager_stun_particles.clone();
        store.pending_evoker_fangs_crit_particles =
            self.pending_evoker_fangs_crit_particles.clone();
        store.pending_entity_client_movement_particles =
            self.pending_entity_client_movement_particles.clone();
        store.pending_ominous_item_spawner_particles =
            self.pending_ominous_item_spawner_particles.clone();
        store.publication_snapshot_observation = EntityPublicationSnapshotObservation {
            duration: snapshot_started_at.elapsed(),
            rebuilt_entities: store.by_protocol_id.len(),
        };
        store
    }
}

impl fmt::Debug for EntityStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let states = self.states();
        f.debug_struct("EntityStore")
            .field("entities", &states)
            .finish()
    }
}

fn minecart_display_offset_for_components(
    entity_type_id: i32,
    data_values: &[ProtocolEntityDataValue],
    registries: &RegistrySet,
) -> Option<i32> {
    if !is_vanilla_minecart_type(entity_type_id) {
        return None;
    }
    let accessors =
        vanilla_entity_metadata_abstract_minecart_accessors_for_entity_type(entity_type_id)?;
    let custom_state_id = data_values
        .iter()
        .find(|value| value.data_id == accessors.custom_display_block_data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::OptionalBlockState(state) => *state,
            _ => None,
        });
    let has_display = if let Some(state_id) = custom_state_id {
        registries
            .block_state(state_id)
            .is_some_and(|state| state.name != "minecraft:air")
    } else {
        default_minecart_display_block_kind(entity_type_id).is_some()
    };
    has_display.then(|| {
        data_values
            .iter()
            .find(|value| value.data_id == accessors.display_offset_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::Int(value) => Some(*value),
                _ => None,
            })
            .unwrap_or(accessors.display_offset_default)
    })
}

fn default_minecart_display_block_state(
    entity_type_id: i32,
    furnace_has_fuel: bool,
    registries: &RegistrySet,
) -> Option<EntityBlockModelState> {
    match default_minecart_display_block_kind(entity_type_id)? {
        DefaultMinecartDisplayBlockKind::Chest => registered_block_model_state(
            registries,
            "minecraft:chest",
            BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("type".to_string(), "single".to_string()),
                ("waterlogged".to_string(), "false".to_string()),
            ]),
        ),
        DefaultMinecartDisplayBlockKind::CommandBlock => registered_block_model_state(
            registries,
            "minecraft:command_block",
            BTreeMap::from([
                ("conditional".to_string(), "false".to_string()),
                ("facing".to_string(), "north".to_string()),
            ]),
        ),
        DefaultMinecartDisplayBlockKind::Furnace => registered_block_model_state(
            registries,
            "minecraft:furnace",
            BTreeMap::from([
                ("facing".to_string(), "north".to_string()),
                ("lit".to_string(), furnace_has_fuel.to_string()),
            ]),
        ),
        DefaultMinecartDisplayBlockKind::Hopper => registered_block_model_state(
            registries,
            "minecraft:hopper",
            BTreeMap::from([
                ("enabled".to_string(), "true".to_string()),
                ("facing".to_string(), "down".to_string()),
            ]),
        ),
        DefaultMinecartDisplayBlockKind::Spawner => {
            registered_block_model_state(registries, "minecraft:spawner", BTreeMap::new())
        }
        DefaultMinecartDisplayBlockKind::Tnt => registered_block_model_state(
            registries,
            "minecraft:tnt",
            BTreeMap::from([("unstable".to_string(), "false".to_string())]),
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DefaultMinecartDisplayBlockKind {
    Chest,
    CommandBlock,
    Furnace,
    Hopper,
    Spawner,
    Tnt,
}

fn default_minecart_display_block_kind(
    entity_type_id: i32,
) -> Option<DefaultMinecartDisplayBlockKind> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_CHEST_MINECART_ID => Some(DefaultMinecartDisplayBlockKind::Chest),
        VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID => {
            Some(DefaultMinecartDisplayBlockKind::CommandBlock)
        }
        VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID => Some(DefaultMinecartDisplayBlockKind::Furnace),
        VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID => Some(DefaultMinecartDisplayBlockKind::Hopper),
        VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID => Some(DefaultMinecartDisplayBlockKind::Spawner),
        VANILLA_ENTITY_TYPE_TNT_MINECART_ID => Some(DefaultMinecartDisplayBlockKind::Tnt),
        _ => None,
    }
}

fn default_primed_tnt_block_state(
    registries: &RegistrySet,
    default: crate::entities::schema::VanillaEntityMetadataDefault,
) -> Option<EntityBlockModelState> {
    let crate::entities::schema::VanillaEntityMetadataDefault::BlockStateDefault {
        resource_id,
        properties,
    } = default
    else {
        unreachable!("generated PrimedTnt block-state default has the wrong kind")
    };
    registered_block_model_state(
        registries,
        resource_id,
        properties
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect(),
    )
}

fn registered_block_model_state(
    registries: &RegistrySet,
    name: &str,
    properties: BTreeMap<String, String>,
) -> Option<EntityBlockModelState> {
    registries.block_state_id_by_name_and_properties(name, &properties)?;
    Some(EntityBlockModelState {
        name: name.to_string(),
        properties,
    })
}

fn entity_block_pos(position: super::EntityVec3) -> crate::BlockPos {
    crate::BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

fn append_client_tick_particle_triggers(
    pending_ravager_stun_particles: &mut Vec<RavagerStunParticleState>,
    pending_evoker_fangs_crit_particles: &mut Vec<EvokerFangsCritParticleState>,
    identity: &EntityIdentity,
    transform: &EntityTransform,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    attributes: &[ProtocolAttributeSnapshot],
    client_animations: Option<EntityClientAnimationState>,
    triggers: EntityClientTickParticleTriggers,
) {
    if triggers.ravager_stun.is_none() && triggers.evoker_fangs_crit.is_none() {
        return;
    }
    let Some(client_animations) = client_animations else {
        debug_assert!(false, "particle bounds trigger lacked its pre-tick state");
        return;
    };
    let Some(bounds) = vanilla_model_source_bounds_for_entity_data(
        identity.entity_type_id,
        identity.data,
        data_values,
        attributes,
        Some(client_animations),
        None,
    ) else {
        return;
    };

    if let Some(offset) = triggers.ravager_stun {
        let width = f64::from(bounds.max[0] - bounds.min[0]);
        let height = f64::from(bounds.max[1] - bounds.min[1]);
        let y_body_rot = f64::from(transform.y_rot).to_radians();
        pending_ravager_stun_particles.push(RavagerStunParticleState {
            entity_id: identity.id,
            position: super::EntityVec3 {
                x: transform.position.x - width * y_body_rot.sin() + offset.jitter_x,
                y: transform.position.y + height - 0.3,
                z: transform.position.z + width * y_body_rot.cos() + offset.jitter_z,
            },
        });
    }

    if let Some(offsets) = triggers.evoker_fangs_crit {
        let half_width = f64::from(bounds.max[0] - bounds.min[0]) * 0.5;
        pending_evoker_fangs_crit_particles.extend(offsets.into_iter().map(|offset| {
            EvokerFangsCritParticleState {
                entity_id: identity.id,
                position: super::EntityVec3 {
                    x: transform.position.x + offset.x_unit * half_width,
                    y: transform.position.y + offset.y_offset,
                    z: transform.position.z + offset.z_unit * half_width,
                },
                velocity: offset.velocity,
            }
        }));
    }
}

fn block_pos_dist_sqr(left: crate::BlockPos, right: crate::BlockPos) -> f64 {
    let dx = f64::from(left.x) - f64::from(right.x);
    let dy = f64::from(left.y) - f64::from(right.y);
    let dz = f64::from(left.z) - f64::from(right.z);
    dx * dx + dy * dy + dz * dz
}

fn add_entity_vec3(left: super::EntityVec3, right: super::EntityVec3) -> super::EntityVec3 {
    super::EntityVec3 {
        x: left.x + right.x,
        y: left.y + right.y,
        z: left.z + right.z,
    }
}

fn expand_bounds_towards_offset(
    mut bounds: super::EntityPickBoundsState,
    offset: super::EntityVec3,
) -> super::EntityPickBoundsState {
    for (axis, value) in [offset.x, offset.y, offset.z].into_iter().enumerate() {
        let value = value as f32;
        bounds.min[axis] = bounds.min[axis].min(bounds.min[axis] + value);
        bounds.max[axis] = bounds.max[axis].max(bounds.max[axis] + value);
    }
    bounds
}

fn sub_entity_vec3(left: super::EntityVec3, right: super::EntityVec3) -> super::EntityVec3 {
    super::EntityVec3 {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

fn lerp_entity_vec3(
    alpha: f32,
    from: super::EntityVec3,
    to: super::EntityVec3,
) -> super::EntityVec3 {
    let alpha = f64::from(alpha);
    super::EntityVec3 {
        x: from.x + (to.x - from.x) * alpha,
        y: from.y + (to.y - from.y) * alpha,
        z: from.z + (to.z - from.z) * alpha,
    }
}

fn solve_living_passenger_body_y_rot(head_y_rot: f32, vehicle_body_y_rot: f32) -> f32 {
    let head_difference = wrap_degrees_f32(head_y_rot - vehicle_body_y_rot).clamp(-85.0, 85.0);
    let mut body_y_rot = head_y_rot - head_difference;
    if head_difference.abs() > 50.0 {
        body_y_rot += head_difference * 0.2;
    }
    body_y_rot
}

fn wrap_degrees_f32(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

fn vanilla_attribute_value(attribute: &ProtocolAttributeSnapshot) -> f64 {
    let mut base = attribute.base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id != VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID
            && modifier.operation_id != VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID
        {
            base += modifier.amount;
        }
    }

    let mut result = base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID {
            result += base * modifier.amount;
        }
    }
    for modifier in &attribute.modifiers {
        if modifier.operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID {
            result *= 1.0 + modifier.amount;
        }
    }
    result
}

/// Exact Vanilla `DefaultAttributes` supplier value, used until a
/// `ClientboundUpdateAttributesPacket` overrides it. Membership and family
/// overrides come from the generated EntityType profile; the Attribute
/// registry default is consulted only for the one-argument builder overload.
fn vanilla_default_attribute_base(entity_type_id: i32, attribute_id: i32) -> Option<f64> {
    let entity_default = vanilla_entity_type_default_attribute(entity_type_id, attribute_id)?;
    entity_default
        .base_value_override_bits
        .map(f64::from_bits)
        .or_else(|| {
            vanilla_attribute_type(attribute_id)
                .map(|attribute| f64::from_bits(attribute.default_value_bits))
        })
}

fn shulker_attach_face(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> EntityAttachmentFace {
    let Some(accessors) = vanilla_entity_metadata_shulker_accessors_for_entity_type(entity_type_id)
    else {
        return EntityAttachmentFace::Down;
    };
    data_values
        .iter()
        .find_map(|value| {
            if value.data_id != accessors.attach_face_data_id {
                return None;
            }
            let EntityDataValueKind::Direction(direction) = &value.value else {
                return None;
            };
            Some(EntityAttachmentFace::from_3d_data(*direction))
        })
        .unwrap_or_else(|| EntityAttachmentFace::from_3d_data(accessors.attach_face_default))
}

fn firework_rocket_render_stack(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    accessors: VanillaFireworkRocketMetadataAccessors,
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    metadata_item_stack(data_values, accessors.item_data_id)
        .filter(|stack| stack.item_id.is_some() && stack.count > 0)
}

fn ominous_item_spawner_render_stack(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    let data_id =
        vanilla_entity_metadata_ominous_item_spawner_item_accessor_for_entity_type(entity_type_id)?;
    metadata_item_stack(data_values, data_id)
        .filter(|stack| stack.item_id.is_some() && stack.count > 0)
}

fn metadata_item_stack(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    data_id: u8,
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    data_values.iter().find_map(|value| {
        if value.data_id != data_id {
            return None;
        }
        let EntityDataValueKind::ItemStack(stack) = &value.value else {
            return None;
        };
        Some(stack)
    })
}

fn firework_rocket_attached_to_entity(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    accessors: VanillaFireworkRocketMetadataAccessors,
) -> bool {
    data_values.iter().any(|value| {
        value.data_id == accessors.attached_to_target_data_id
            && matches!(
                &value.value,
                EntityDataValueKind::OptionalUnsignedInt(Some(_))
            )
    })
}

fn firework_rocket_shot_at_angle(
    data_values: &[bbb_protocol::packets::EntityDataValue],
    accessors: VanillaFireworkRocketMetadataAccessors,
) -> bool {
    data_values.iter().any(|value| {
        value.data_id == accessors.shot_at_angle_data_id
            && matches!(&value.value, EntityDataValueKind::Boolean(true))
    })
}

fn firework_rocket_stack_has_explosions(stack: &ItemStackSummary) -> bool {
    stack
        .component_patch
        .fireworks_explosions_count
        .unwrap_or(stack.component_patch.fireworks_explosions.len())
        > 0
}

impl Serialize for EntityStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.states().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EntityStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let states = Vec::<EntityState>::deserialize(deserializer)?;
        let mut store = EntityStore::default();
        for state in states {
            store.insert_or_replace(state);
        }
        Ok(store)
    }
}
