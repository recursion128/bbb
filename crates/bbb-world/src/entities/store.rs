use std::{collections::BTreeMap, fmt};

use bbb_protocol::packets::AttributeSnapshot as ProtocolAttributeSnapshot;
use hecs::{Entity, World};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use bbb_protocol::packets::EntityDataValueKind;

use super::{
    EntityAttributes, EntityCameraPoseState, EntityClientAnimations, EntityDamage, EntityEquipment,
    EntityHurtingProjectile, EntityIdentity, EntityLeash, EntityMetadata, EntityMinecartLerp,
    EntityMobEffects, EntityModelSourceState, EntityMount, EntityState, EntityTransform,
    EntityTransformState, EntityTransientEvents, ItemEntityStackState,
    VANILLA_ENTITY_NO_GRAVITY_DATA_ID, VANILLA_ENTITY_SILENT_DATA_ID,
    VANILLA_ENTITY_TICKS_FROZEN_DATA_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
    VANILLA_ITEM_ENTITY_STACK_DATA_ID,
};
use crate::entities::dimensions::{
    entity_data_pose, vanilla_client_position_for_entity_data, vanilla_eye_height_for_entity_data,
    vanilla_pick_bounds_for_entity_data,
};
use crate::entities::dragon::{
    ender_dragon_part_pick_targets_at_partial_tick, VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
};
use crate::entities::projectiles::entity_hurting_projectile_from_state;

pub(crate) struct EntityStore {
    ecs: World,
    by_protocol_id: BTreeMap<i32, Entity>,
    order: Vec<i32>,
}

impl EntityStore {
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

    fn spawn_components(&mut self, state: EntityState) {
        let id = state.id;
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
            EntityMinecartLerp::from(&state),
        ));
        if let Some(projectile) =
            entity_hurting_projectile_from_state(state.entity_type_id, state.hurting_projectile)
        {
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

    pub(crate) fn identity(&self, id: i32) -> Option<EntityIdentity> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityIdentity>(entity)
            .ok()
            .map(|identity| (*identity).clone())
    }

    pub(crate) fn is_silent(&self, id: i32) -> Option<bool> {
        self.metadata_bool(id, VANILLA_ENTITY_SILENT_DATA_ID, false)
    }

    pub(crate) fn no_gravity(&self, id: i32) -> Option<bool> {
        self.metadata_bool(id, VANILLA_ENTITY_NO_GRAVITY_DATA_ID, false)
    }

    pub(crate) fn ticks_frozen(&self, id: i32) -> Option<i32> {
        self.metadata_int(id, VANILLA_ENTITY_TICKS_FROZEN_DATA_ID, 0)
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
        )
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
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
                let dragon_animation = self
                    .ecs
                    .get::<&EntityClientAnimations>(entity)
                    .ok()
                    .and_then(|animations| animations.animations.ender_dragon);
                targets.extend(ender_dragon_part_pick_targets_at_partial_tick(
                    identity.id,
                    *transform,
                    dragon_animation,
                    partial_ticks,
                ));
            } else if let Some(bounds) = self.pick_bounds(identity.id) {
                targets.push(super::EntityPickTargetState {
                    entity_id: identity.id,
                    position: transform.position,
                    bounds,
                });
            }
        }
        targets
    }

    pub(crate) fn refresh_client_position_from_entity_data(&mut self, id: i32) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?.clone();
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?.clone();
        let packet_position = {
            let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
            transform.position_base
        };
        let position = vanilla_client_position_for_entity_data(
            identity.entity_type_id,
            packet_position,
            identity.data,
            &metadata.data_values,
        )?;
        let mut transform = self.ecs.get::<&mut EntityTransform>(entity).ok()?;
        transform.position = position;
        Some(())
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
        let attributes = self.ecs.get::<&EntityAttributes>(entity).ok()?;
        attributes
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == attribute_id)
            .map(vanilla_attribute_value)
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

    pub(crate) fn model_source(
        &self,
        id: i32,
        position: super::EntityVec3,
        partial_ticks: f32,
    ) -> Option<EntityModelSourceState> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let identity = self.ecs.get::<&EntityIdentity>(entity).ok()?;
        let transform = self.ecs.get::<&EntityTransform>(entity).ok()?;
        let metadata = self.ecs.get::<&EntityMetadata>(entity).ok()?;
        let client_animations = self.ecs.get::<&EntityClientAnimations>(entity).ok()?;
        Some(EntityModelSourceState {
            entity_id: identity.id,
            entity_type_id: identity.entity_type_id,
            position,
            y_rot: transform.y_rot,
            age_ticks: client_animations.animations.age_ticks,
            sheep_eat_animation_tick: client_animations.animations.sheep_eat_animation_tick(),
            polar_bear_stand_scale: client_animations
                .animations
                .polar_bear_stand_scale(partial_ticks),
            // Spatial light is sampled by the WorldStore aggregation, which owns
            // the chunk light data; the per-entity source defaults to full bright.
            light: super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
            data_values: metadata.data_values.clone(),
        })
    }

    pub(crate) fn camera_pose_state(&self, id: i32) -> Option<EntityCameraPoseState> {
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
        )?;
        Some(EntityCameraPoseState {
            id: identity.id,
            position: transform.position,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            eye_height,
        })
    }

    pub(crate) fn mount(&self, id: i32) -> Option<EntityMount> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        self.ecs
            .get::<&EntityMount>(entity)
            .ok()
            .map(|mount| (*mount).clone())
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
            .map(|damage| *damage)
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
        let mut items = Vec::new();
        for id in &self.order {
            let Some(entity) = self.by_protocol_id.get(id).copied() else {
                continue;
            };
            let Ok(identity) = self.ecs.get::<&EntityIdentity>(entity) else {
                continue;
            };
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_ITEM_ID {
                continue;
            }
            let Ok(transform) = self.ecs.get::<&EntityTransform>(entity) else {
                continue;
            };
            let Ok(metadata) = self.ecs.get::<&EntityMetadata>(entity) else {
                continue;
            };
            let Some(stack) = item_entity_render_stack(&metadata.data_values) else {
                continue;
            };
            items.push(ItemEntityStackState {
                entity_id: identity.id,
                position: transform.position,
                stack: stack.clone(),
            });
        }
        items
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

    pub(crate) fn apply_client_animation_entity_event(
        &mut self,
        id: i32,
        event_id: i8,
    ) -> Option<()> {
        let entity = self.by_protocol_id.get(&id).copied()?;
        let entity_type_id = self.ecs.get::<&EntityIdentity>(entity).ok()?.entity_type_id;
        let mut animations = self.ecs.get::<&mut EntityClientAnimations>(entity).ok()?;
        animations
            .animations
            .handle_entity_event(entity_type_id, event_id);
        Some(())
    }

    pub(crate) fn advance_client_animations(&mut self, ticks: u32) {
        for _ in 0..ticks {
            for (_, (identity, transform, animations)) in self.ecs.query_mut::<(
                &EntityIdentity,
                &EntityTransform,
                &mut EntityClientAnimations,
            )>() {
                animations
                    .animations
                    .advance_client_tick(identity.entity_type_id, *transform);
            }
        }
    }

    pub(crate) fn for_each_mount_mut(&mut self, mut update: impl FnMut(i32, &mut EntityMount)) {
        let ids = self.order.clone();
        for id in ids {
            let _ = self.with_mount_mut(id, |mount| update(id, mount));
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
            .map(|lerp| lerp.steps.len())
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
        if let Ok(mut lerp) = self.ecs.get::<&mut EntityMinecartLerp>(entity) {
            *lerp = EntityMinecartLerp::from(state);
        }
        self.sync_hurting_projectile_from_state(entity, state);
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
        let minecart_lerp = self.ecs.get::<&EntityMinecartLerp>(entity).ok()?;
        let hurting_projectile = self.ecs.get::<&EntityHurtingProjectile>(entity).ok();

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
            data_values: Vec::new(),
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
            minecart_lerp_steps: Vec::new(),
            hurting_projectile: None,
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
        (*damage).write_to_state(&mut state);
        (*minecart_lerp).clone().write_to_state(&mut state);
        if let Some(projectile) = hurting_projectile {
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
        }
    }
}

impl Clone for EntityStore {
    fn clone(&self) -> Self {
        let mut store = Self::default();
        for state in self.states() {
            store.insert_or_replace(state);
        }
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

fn vanilla_attribute_value(attribute: &ProtocolAttributeSnapshot) -> f64 {
    let mut base = attribute.base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id != 1 && modifier.operation_id != 2 {
            base += modifier.amount;
        }
    }

    let mut result = base;
    for modifier in &attribute.modifiers {
        if modifier.operation_id == 1 {
            result += base * modifier.amount;
        }
    }
    for modifier in &attribute.modifiers {
        if modifier.operation_id == 2 {
            result *= 1.0 + modifier.amount;
        }
    }
    result
}

fn item_entity_render_stack(
    data_values: &[bbb_protocol::packets::EntityDataValue],
) -> Option<&bbb_protocol::packets::ItemStackSummary> {
    data_values.iter().find_map(|value| {
        if value.data_id != VANILLA_ITEM_ENTITY_STACK_DATA_ID {
            return None;
        }
        let EntityDataValueKind::ItemStack(stack) = &value.value else {
            return None;
        };
        if stack.item_id.is_some() && stack.count > 0 {
            Some(stack)
        } else {
            None
        }
    })
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
