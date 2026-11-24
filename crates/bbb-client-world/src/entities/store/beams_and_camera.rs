use super::*;

impl EntityStore {
    pub(crate) fn camera_pose_state(&self, id: i32) -> Option<EntityCameraPoseState> {
        self.camera_pose_state_at_partial_tick(id, 1.0)
    }

    pub(crate) fn camera_pose_state_at_partial_tick(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraPoseState> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let transform = &entity.transform;
        let metadata = &entity.metadata;
        let attributes = &entity.attributes;
        let client_animations = &entity.animations;
        let eye_height = vanilla_eye_height_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(*client_animations),
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

    pub(crate) fn camera_view_bob_state(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<EntityCameraViewBobState> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
            return None;
        }
        let client_animations = &entity.animations;
        Some(client_animations.player_camera_view_bob(partial_ticks))
    }

    pub(super) fn living_entity_walk_animation_position(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        if !super::super::content_facts::entity_is_living(
            identity.entity_type_id,
            &self.content_registry,
        ) {
            return None;
        }
        let client_animations = &entity.animations;
        Some(client_animations.walk_animation_position(partial_ticks))
    }

    pub(crate) fn living_entity_previous_feet_position(
        &self,
        id: i32,
    ) -> Option<Option<super::super::EntityVec3>> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        if !super::super::content_facts::entity_is_living(
            identity.entity_type_id,
            &self.content_registry,
        ) {
            return None;
        }
        let client_animations = &entity.animations;
        Some(
            client_animations
                .walk_animation
                .and_then(|walk| walk.previous_feet_position),
        )
    }

    /// Vanilla `GuardianRenderer.extractRenderState` attack beam: when this entity is a guardian whose
    /// synced `DATA_ID_ATTACK_TARGET` names a live target, project the world eye→target vector and the
    /// attack timing. `position` is the guardian's interpolated feet position; the target is resolved
    /// cross-entity by its protocol id. `None` for a guardian with no target (or a missing target) and
    /// every non-guardian.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn guardian_beam_source(
        &self,
        entity_type_id: i32,
        add_entity_data: i32,
        data_values: &[bbb_protocol::packets::EntityDataValue],
        attributes: &[ProtocolAttributeSnapshot],
        client_animations: crate::entities::EntityClientAnimationState,
        position: crate::entities::EntityVec3,
        partial_ticks: f32,
    ) -> Option<crate::entities::GuardianBeamRenderState> {
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
        let attack = client_animations
            .guardian_kind()
            .and_then(|k| k.attack)
            .unwrap_or_default();
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
        Some(crate::entities::GuardianBeamRenderState {
            eye_to_target,
            eye_height,
            attack_time: attack.attack_time(partial_ticks),
            attack_scale: attack.attack_scale(partial_ticks, attack_duration),
        })
    }

    /// Vanilla `EndCrystalRenderer.extractRenderState`: when an end crystal has a synced
    /// `DATA_BEAM_TARGET`, project the target block center relative to the crystal's interpolated
    /// position. `None` for crystals without a target and every non-crystal entity.
    pub(super) fn end_crystal_beam_source(
        &self,
        entity_type_id: i32,
        data_values: &[bbb_protocol::packets::EntityDataValue],
        position: crate::entities::EntityVec3,
    ) -> Option<crate::entities::EndCrystalBeamRenderState> {
        let accessors =
            vanilla_entity_metadata_end_crystal_accessors_for_entity_type(entity_type_id)?;
        let target = data_values
            .iter()
            .find(|value| value.data_id == accessors.beam_target_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalBlockPos(Some(pos)) => Some(pos),
                _ => None,
            })?;
        Some(crate::entities::EndCrystalBeamRenderState {
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
    pub(super) fn ender_dragon_beam_source(
        &self,
        entity_type_id: i32,
        position: crate::entities::EntityVec3,
        partial_ticks: f32,
    ) -> Option<crate::entities::EnderDragonBeamRenderState> {
        if entity_type_id != VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
            return None;
        }

        let mut nearest: Option<(f64, [f32; 3])> = None;
        for entity in self.records() {
            let (identity, transform, client_animations) =
                (&entity.identity, &entity.transform, &entity.animations);
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
            let crystal_age = client_animations.end_crystal_age_ticks() as f32 + partial_ticks;
            let crystal_y = transform.position.y + f64::from(end_crystal_renderer_y(crystal_age));
            nearest = Some((
                distance_sqr,
                [dx as f32, (crystal_y - position.y) as f32, dz as f32],
            ));
        }

        nearest.map(|(_, beam_offset)| crate::entities::EnderDragonBeamRenderState { beam_offset })
    }
}
