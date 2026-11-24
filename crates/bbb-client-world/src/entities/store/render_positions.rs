use super::*;

impl EntityStore {
    pub(crate) fn refresh_client_position_from_entity_data(&mut self, id: i32) -> Option<()> {
        let entity = self.record_mut(id)?;
        let old_position = {
            let identity = &entity.identity;
            let metadata = &entity.metadata;
            let transform = &mut entity.transform;
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
        old_position: super::super::EntityVec3,
    ) -> Option<bool> {
        let entity = self.record_mut(id)?;
        let identity = &entity.identity;
        let mount = &entity.mount;
        let transform = &mut entity.transform;
        let animations = &mut entity.animations;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_SHULKER_ID {
            return Some(false);
        }
        if mount.vehicle_id.is_none() {
            // Vanilla `Shulker.setPos` keeps the packet codec base untouched,
            // but snaps a non-passenger's canonical position to its attachment
            // block center before starting the client render offset.
            transform.position = vanilla_shulker_attached_position(transform.position);
        }
        let started = animations.record_shulker_position_change(
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
            animations.shulker_started_riding();
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
            animations.shulker_stopped_riding(position);
        });
    }

    pub(crate) fn transform(&self, id: i32) -> Option<EntityTransform> {
        self.record(id).map(|entity| entity.transform)
    }

    pub(crate) fn attribute_value(&self, id: i32, attribute_id: i32) -> Option<f64> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let attributes = &entity.attributes;
        attributes
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == attribute_id)
            .map(vanilla_attribute_value)
            .or_else(|| vanilla_default_attribute_base(identity.entity_type_id, attribute_id))
    }

    pub(crate) fn render_scale(&self, id: i32) -> Option<f32> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let attributes = &entity.attributes;
        Some(vanilla_render_scale(
            identity.entity_type_id,
            &attributes.attributes,
        ))
    }

    /// The attribute value with one modifier left out, for a modifier the
    /// client owns while the server may also echo it.
    pub(crate) fn attribute_value_without_modifier(
        &self,
        id: i32,
        attribute_id: i32,
        modifier_id: &str,
    ) -> Option<f64> {
        let entity = self.record(id)?;
        let Some(attribute) = entity
            .attributes
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == attribute_id)
        else {
            return vanilla_default_attribute_base(entity.identity.entity_type_id, attribute_id);
        };
        let mut attribute = attribute.clone();
        attribute
            .modifiers
            .retain(|modifier| modifier.id != modifier_id);
        Some(vanilla_attribute_value(&attribute))
    }

    pub(crate) fn attribute_base_value(&self, id: i32, attribute_id: i32) -> Option<f64> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let attributes = &entity.attributes;
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
        let Some(entity) = self.record(id) else {
            return false;
        };
        let attributes = &entity.attributes;
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
        let entity = self.record(id)?;
        Some(EntityTransformState::from_components(
            &entity.identity,
            entity.transform,
        ))
    }

    pub(crate) fn minecart_has_render_steps(&self, id: i32) -> bool {
        self.record(id)
            .and_then(|entity| entity.family.minecart_lerp())
            .is_some_and(|lerp| !lerp.steps.is_empty())
    }

    pub(super) fn render_position_and_y_rot_at_partial_tick(
        &self,
        id: i32,
        partial_ticks: f32,
        minecart_improvements_enabled: bool,
    ) -> Option<(super::super::EntityVec3, f32)> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let transform = &entity.transform;
        let minecart_lerp = entity.family.minecart_lerp();
        if minecart_improvements_enabled && is_vanilla_minecart_type(identity.entity_type_id) {
            if let Some(render_step) =
                minecart_lerp.and_then(|lerp| lerp.render_step(partial_ticks))
            {
                return Some((render_step.position, render_step.y_rot));
            }
        }
        Some((
            transform.render_position(partial_ticks),
            transform.render_y_rot(partial_ticks),
        ))
    }

    pub(super) fn passenger_position_from_vehicle_render_pose(
        &self,
        passenger_id: i32,
        vehicle_id: i32,
        vehicle_position: super::super::EntityVec3,
        vehicle_y_rot: f32,
        partial_ticks: f32,
        game_time: i64,
    ) -> Option<super::super::EntityVec3> {
        let passenger_bounds = self.pick_bounds(passenger_id)?;
        let passenger_entity = self.record(passenger_id)?;
        let passenger_identity = &passenger_entity.identity;
        let passenger_transform = &passenger_entity.transform;
        let passenger_metadata = &passenger_entity.metadata;

        let vehicle_bounds = self.pick_bounds(vehicle_id)?;
        let vehicle_entity = self.record(vehicle_id)?;
        let vehicle_identity = &vehicle_entity.identity;
        let vehicle_mount = &vehicle_entity.mount;
        let vehicle_metadata = &vehicle_entity.metadata;
        let vehicle_attributes = &vehicle_entity.attributes;
        vanilla_passenger_position_for_entity_data(
            vehicle_identity.entity_type_id,
            vehicle_position,
            vehicle_y_rot,
            vehicle_bounds,
            &vehicle_metadata.data_values,
            &vehicle_attributes.attributes,
            Some(vehicle_entity.animations),
            &vehicle_mount.passengers,
            passenger_id,
            passenger_identity.entity_type_id,
            passenger_transform.render_y_rot(partial_ticks),
            passenger_bounds,
            &passenger_metadata.data_values,
            game_time,
        )
    }

    pub(super) fn render_positions_at_partial_tick(
        &self,
        ids: &[i32],
        partial_ticks: f32,
        game_time: i64,
        minecart_improvements_enabled: bool,
    ) -> FxHashMap<i32, super::super::EntityVec3> {
        let mut resolved = FxHashMap::with_capacity_and_hasher(ids.len(), Default::default());
        let mut path = Vec::new();
        let mut path_ids = FxHashSet::default();
        for &id in ids {
            if resolved.contains_key(&id) {
                continue;
            }
            path.clear();
            path_ids.clear();
            let mut cursor = id;
            let base = loop {
                if let Some(position) = resolved.get(&cursor).copied() {
                    break Some(position);
                }
                if !path_ids.insert(cursor) || path.len() >= self.len() {
                    break None;
                }
                let Some(own_position) = self
                    .render_position_and_y_rot_at_partial_tick(
                        cursor,
                        partial_ticks,
                        minecart_improvements_enabled,
                    )
                    .map(|(position, _)| position)
                else {
                    break None;
                };
                let Some(vehicle_id) = self.mount(cursor).and_then(|mount| mount.vehicle_id) else {
                    resolved.insert(cursor, own_position);
                    break Some(own_position);
                };
                path.push((cursor, vehicle_id, own_position));
                cursor = vehicle_id;
            };

            let Some(mut position) = base else {
                // Broken/missing/cyclic graphs preserve every packet-owned
                // render endpoint already discovered on this walk.
                for &(entity_id, _, own_position) in &path {
                    resolved.entry(entity_id).or_insert(own_position);
                }
                continue;
            };
            for &(passenger_id, vehicle_id, own_position) in path.iter().rev() {
                let next_position = self
                    .render_position_and_y_rot_at_partial_tick(
                        vehicle_id,
                        partial_ticks,
                        minecart_improvements_enabled,
                    )
                    .and_then(|(_, vehicle_y_rot)| {
                        self.passenger_position_from_vehicle_render_pose(
                            passenger_id,
                            vehicle_id,
                            position,
                            vehicle_y_rot,
                            partial_ticks,
                            game_time,
                        )
                    })
                    .unwrap_or(own_position);
                resolved.insert(passenger_id, next_position);
                position = next_position;
            }
        }
        resolved
    }

    /// Projects one entity from the current render pose of the root vehicle
    /// through each immediate passenger edge. The packet-owned transforms are
    /// never mutated. A malformed cycle or broken edge falls back to the
    /// entity's own interpolated transform after at most one pass over the
    /// loaded entity count.
    pub(crate) fn render_position_at_partial_tick(
        &self,
        id: i32,
        partial_ticks: f32,
        game_time: i64,
        minecart_improvements_enabled: bool,
    ) -> Option<super::super::EntityVec3> {
        self.render_positions_at_partial_tick(
            &[id],
            partial_ticks,
            game_time,
            minecart_improvements_enabled,
        )
        .remove(&id)
    }

    pub(super) fn living_body_render_y_rot(&self, id: i32, partial_ticks: f32) -> Option<f32> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let transform = &entity.transform;
        let animations = &entity.animations;
        if !super::super::content_facts::entity_is_living(
            identity.entity_type_id,
            &self.content_registry,
        ) {
            return None;
        }
        Some(animations.render_body_y_rot(*transform, partial_ticks))
    }
}
