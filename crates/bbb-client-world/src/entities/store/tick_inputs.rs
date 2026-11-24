use super::*;

impl EntityStore {
    pub(crate) fn interpolation_collision_inputs(&self) -> Vec<EntityInterpolationCollisionInput> {
        let mut inputs = Vec::new();
        for entity in self.records() {
            let (identity, transform, metadata, attributes, animations) = (
                &entity.identity,
                &entity.transform,
                &entity.metadata,
                &entity.attributes,
                &entity.animations,
            );
            let Some(interpolation) = transform.interpolation else {
                continue;
            };
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

    pub(super) fn fluid_contact_input_from_components(
        identity: &EntityIdentity,
        transform: &EntityTransform,
        metadata: &EntityMetadata,
        attributes: &EntityAttributes,
        animations: &EntityClientAnimationState,
    ) -> Option<EntityInWaterTickInput> {
        let bounds = vanilla_pick_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(*animations),
            metadata.painting_variant_projection.as_ref(),
        )?;
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
        let eye_height = if is_vanilla_abstract_nautilus_type(identity.entity_type_id) {
            vanilla_eye_height_for_entity_data(
                identity.entity_type_id,
                identity.data,
                &metadata.data_values,
                &attributes.attributes,
                Some(*animations),
                metadata.painting_variant_projection.as_ref(),
            )
            .unwrap_or(bounds.max[1])
        } else {
            0.0
        };
        let guardian_below_pos =
            (vanilla_entity_metadata_type_is_guardian(identity.entity_type_id)
                && transform.delta_movement.y < 0.0)
                .then_some(crate::BlockPos {
                    x: transform.position.x.floor() as i32,
                    y: transform.position.y.floor() as i32 - 1,
                    z: transform.position.z.floor() as i32,
                });
        let guardian_attack_target_id =
            if vanilla_entity_metadata_type_is_guardian(identity.entity_type_id) {
                guardian_attack_target_id(identity.entity_type_id, &metadata.data_values)
            } else {
                0
            };
        Some(EntityInWaterTickInput {
            entity_id: identity.id,
            entity_type_id: identity.entity_type_id,
            aabb_min,
            aabb_max,
            position: transform.position,
            eye_position: [
                transform.position.x,
                transform.position.y + f64::from(eye_height),
                transform.position.z,
            ],
            guardian_below_pos,
            guardian_attack_target_id,
        })
    }

    /// Resolves one entity through the same canonical bounds and eye-height
    /// projection used by batched client fluid facts. Packet-time consumers do
    /// not widen the per-tick animation filter just to sample one entity.
    pub(crate) fn fluid_contact_input(&self, id: i32) -> Option<EntityInWaterTickInput> {
        let entity = self.record(id)?;
        Self::fluid_contact_input_from_components(
            &entity.identity,
            &entity.transform,
            &entity.metadata,
            &entity.attributes,
            &entity.animations,
        )
    }

    /// Gathers the world AABBs the per-entity `in_water` map is built from, for the
    /// entity types whose client-tick animation reads `isInWater()`
    /// ([`entity_animation_uses_in_water`]). Each input carries the entity id,
    /// AABB, and narrow Guardian-only support/target facts. The AABB is resolved from the SAME bounds source the projection
    /// uses ([`Self::pick_bounds`] → `vanilla_pick_bounds_for_entity_data`), so the
    /// fluid overlap matches the projected `in_water`. `WorldStore` (which holds the
    /// chunk/fluid data) turns each AABB into a `bool`.
    pub(crate) fn in_water_aabb_inputs(&self) -> Vec<EntityInWaterTickInput> {
        let mut inputs = Vec::new();
        for entity in self.records() {
            let (identity, transform, metadata, attributes, animations) = (
                &entity.identity,
                &entity.transform,
                &entity.metadata,
                &entity.attributes,
                &entity.animations,
            );
            if !entity_animation_uses_in_water(identity.entity_type_id)
                && !super::super::client_movement::client_movement_uses_in_water(
                    identity.entity_type_id,
                )
                && identity.entity_type_id != VANILLA_ENTITY_TYPE_STRIDER_ID
            {
                continue;
            }
            if let Some(input) = Self::fluid_contact_input_from_components(
                identity, transform, metadata, attributes, animations,
            ) {
                inputs.push(input);
            }
        }
        inputs
    }

    /// Gathers the fixed hull facts required by `AbstractBoat.tick` before the
    /// mutable ordered entity pass. Boat dimensions come from the generated
    /// Vanilla entity catalog and do not depend on entity metadata.
    pub(crate) fn prepare_client_tick_schedule<F>(
        &mut self,
        runs_normally: bool,
        is_ticking_entity: &F,
    ) -> EntityClientTickScheduleInputs
    where
        F: Fn(i32, i32, EntityVec3) -> bool,
    {
        let mut schedule = std::mem::take(&mut self.client_tick_scratch);
        let mut inputs = EntityClientTickScheduleInputs::default();
        self.fill_client_tick_schedule(
            &mut schedule,
            runs_normally,
            is_ticking_entity,
            Some(&mut inputs),
        );
        self.client_tick_scratch = schedule;
        inputs
    }

    /// Resolves only unique active Guardian targets collected by the existing
    /// water-input query. Guardians sharing a target reuse one protocol-id and
    /// component lookup; unrelated living entities are never projected.
    pub(super) fn guardian_attack_target_facts(
        &self,
        target_ids: &FxHashSet<i32>,
    ) -> FxHashMap<i32, GuardianAttackTargetTickInput> {
        let mut targets = FxHashMap::with_capacity_and_hasher(target_ids.len(), Default::default());
        for &target_id in target_ids {
            let Some(entity) = self.record(target_id) else {
                continue;
            };
            let (identity, transform, metadata, attributes, animations) = (
                &entity.identity,
                &entity.transform,
                &entity.metadata,
                &entity.attributes,
                &entity.animations,
            );
            if !super::super::content_facts::entity_is_living(
                identity.entity_type_id,
                &self.content_registry,
            ) {
                continue;
            }
            let Some(bounds) = super::super::content_facts::bounds_for_entity_data(
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
            let Some(eye_height) = super::super::content_facts::eye_height_for_entity_data(
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
            targets.insert(
                target_id,
                GuardianAttackTargetTickInput {
                    position: transform.position,
                    eye_height,
                    half_height: (bounds.max[1] - bounds.min[1]) * 0.5,
                },
            );
        }
        targets
    }

    pub(crate) fn client_movement_fluid_inputs<S: std::hash::BuildHasher>(
        &self,
        can_carry_interpolation_target: &std::collections::HashSet<i32, S>,
    ) -> Vec<super::super::client_movement::EntityClientMovementFluidInput> {
        let mut inputs = Vec::new();
        for entity in self.records() {
            let (identity, transform, metadata, attributes, animations) = (
                &entity.identity,
                &entity.transform,
                &entity.metadata,
                &entity.attributes,
                &entity.animations,
            );
            if !super::super::client_movement::client_movement_uses_fluid_contact(
                identity.entity_type_id,
            ) {
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
            let mut world_fact_transform = *transform;
            if identity.entity_type_id == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID {
                world_fact_transform.advance_interpolation_tick(
                    true,
                    true,
                    can_carry_interpolation_target.contains(&identity.id),
                );
            }
            let position = world_fact_transform.position;
            inputs.push(
                super::super::client_movement::EntityClientMovementFluidInput {
                    entity_id: identity.id,
                    entity_type_id: identity.entity_type_id,
                    position,
                    bounds,
                    aabb_min: [
                        position.x + f64::from(bounds.min[0]),
                        position.y + f64::from(bounds.min[1]),
                        position.z + f64::from(bounds.min[2]),
                    ],
                    aabb_max: [
                        position.x + f64::from(bounds.max[0]),
                        position.y + f64::from(bounds.max[1]),
                        position.z + f64::from(bounds.max[2]),
                    ],
                    eye_position: [position.x, position.y + f64::from(eye_height), position.z],
                },
            );
        }
        inputs
    }

    pub(crate) fn client_movement_has_experience_orb(&self) -> bool {
        self.records()
            .any(|entity| entity.family.experience_orb().is_some())
    }

    pub(crate) fn client_movement_entity_collision_targets(
        &self,
    ) -> Vec<super::super::EntityPickTargetState> {
        self.entity_collision_targets(false)
    }

    pub(crate) fn client_movement_entity_collision_targets_in_aabb(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> Vec<super::super::EntityPickTargetState> {
        self.entity_collision_targets_in_aabb(query_min, query_max, false)
    }

    pub(crate) fn vehicle_entity_collision_targets(
        &self,
    ) -> Vec<super::super::EntityPickTargetState> {
        self.entity_collision_targets(true)
    }

    pub(crate) fn vehicle_entity_collision_targets_in_aabb(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> Vec<super::super::EntityPickTargetState> {
        self.entity_collision_targets_in_aabb(query_min, query_max, true)
    }

    pub(super) fn entity_collision_targets(
        &self,
        include_pushable: bool,
    ) -> Vec<super::super::EntityPickTargetState> {
        if self.collidable_entity_count == 0
            && (!include_pushable || self.pushable_entity_count == 0)
        {
            return Vec::new();
        }
        let mut targets = Vec::new();
        for entity in self.records() {
            Self::push_collision_target_for_record(entity, include_pushable, &mut targets);
        }
        targets
    }

    fn entity_collision_targets_in_aabb(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
        include_pushable: bool,
    ) -> Vec<super::super::EntityPickTargetState> {
        if self.collidable_entity_count == 0
            && (!include_pushable || self.pushable_entity_count == 0)
        {
            return Vec::new();
        }
        let Some(candidate_slots) = self.spatial_candidate_slots_in_aabb(query_min, query_max)
        else {
            return self.entity_collision_targets(include_pushable);
        };
        let mut targets = Vec::with_capacity(candidate_slots.len());
        for slot in candidate_slots {
            if let Some(entity) = self.slots[slot].as_ref() {
                Self::push_collision_target_for_record(entity, include_pushable, &mut targets);
            }
        }
        targets
    }

    fn push_collision_target_for_record(
        entity: &EntityRecord,
        include_pushable: bool,
        targets: &mut Vec<super::super::EntityPickTargetState>,
    ) {
        let (identity, transform, metadata, attributes, animations) = (
            &entity.identity,
            &entity.transform,
            &entity.metadata,
            &entity.attributes,
            &entity.animations,
        );
        let never_collidable = matches!(
            vanilla_entity_metadata_type_collidability_kind(identity.entity_type_id),
            None | Some(VanillaEntityCollidabilityKind::Never)
        );
        let never_pushable = matches!(
            vanilla_entity_metadata_type_pushability_kind(identity.entity_type_id),
            None | Some(VanillaEntityPushabilityKind::Never)
        );
        if never_collidable && (!include_pushable || never_pushable) {
            return;
        }
        let Some(bounds) = vanilla_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(*animations),
            metadata.painting_variant_projection.as_ref(),
        ) else {
            return;
        };
        targets.push(super::super::EntityPickTargetState {
            entity_id: identity.id,
            position: transform.position,
            bounds,
        });
    }

    /// Filters scheduled local-player pushers down to entities whose bounds
    /// (covering current position, velocity step, and active interpolation
    /// target) can overlap `query_min..query_max` on this tick.
    pub(crate) fn local_player_pusher_candidates(
        &self,
        pushers: &[EntityLocalPlayerPushTickInput],
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> Vec<EntityLocalPlayerPushTickInput> {
        if pushers.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        // When many pushers exist, use the cached EntityChunkSpatialIndex for
        // stationary entities while still checking any moving/interpolating
        // entity whose trajectory could enter the player's AABB on this tick.
        if pushers.len() > TICK_SPATIAL_INDEX_MIN_ENTITIES {
            let expanded_min = [query_min[0] - 1.0, query_min[1] - 1.0, query_min[2] - 1.0];
            let expanded_max = [query_max[0] + 1.0, query_max[1] + 1.0, query_max[2] + 1.0];
            if let Some(candidate_slots) =
                self.spatial_candidate_slots_in_aabb(expanded_min, expanded_max)
            {
                for &input in pushers {
                    let Some(&slot) = self.slot_by_id.get(&input.entity_id) else {
                        continue;
                    };
                    let Some(entity) = self.slots[slot].as_ref() else {
                        continue;
                    };
                    let transform = &entity.transform;
                    let may_cross_section = transform.interpolation.is_some()
                        || transform.delta_movement.x.abs() > 1.0
                        || transform.delta_movement.y.abs() > 1.0
                        || transform.delta_movement.z.abs() > 1.0;
                    if !may_cross_section && candidate_slots.binary_search(&slot).is_err() {
                        continue;
                    }
                    if self.record_can_overlap_aabb_this_tick(entity, query_min, query_max) {
                        out.push(input);
                    }
                }
                return out;
            }
        }
        for &input in pushers {
            let Some(entity) = self.record(input.entity_id) else {
                continue;
            };
            if self.record_can_overlap_aabb_this_tick(entity, query_min, query_max) {
                out.push(input);
            }
        }
        out
    }

    fn record_can_overlap_aabb_this_tick(
        &self,
        entity: &EntityRecord,
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> bool {
        let Some(bounds) = vanilla_bounds_for_entity_data(
            entity.identity.entity_type_id,
            entity.identity.data,
            &entity.metadata.data_values,
            &entity.attributes.attributes,
            Some(entity.animations),
            entity.metadata.painting_variant_projection.as_ref(),
        ) else {
            return true;
        };
        let pos = entity.transform.position;
        let vel = entity.transform.delta_movement;
        let mut min_x = pos.x.min(pos.x + vel.x);
        let mut min_y = pos.y.min(pos.y + vel.y);
        let mut min_z = pos.z.min(pos.z + vel.z);
        let mut max_x = pos.x.max(pos.x + vel.x);
        let mut max_y = pos.y.max(pos.y + vel.y);
        let mut max_z = pos.z.max(pos.z + vel.z);
        if let Some(interp) = entity.transform.interpolation {
            min_x = min_x.min(interp.target_position.x);
            min_y = min_y.min(interp.target_position.y);
            min_z = min_z.min(interp.target_position.z);
            max_x = max_x.max(interp.target_position.x);
            max_y = max_y.max(interp.target_position.y);
            max_z = max_z.max(interp.target_position.z);
        }
        let entity_min = [
            min_x + f64::from(bounds.min[0]),
            min_y + f64::from(bounds.min[1]),
            min_z + f64::from(bounds.min[2]),
        ];
        let entity_max = [
            max_x + f64::from(bounds.max[0]),
            max_y + f64::from(bounds.max[1]),
            max_z + f64::from(bounds.max[2]),
        ];
        entity_min[0] <= query_max[0]
            && entity_max[0] >= query_min[0]
            && entity_min[1] <= query_max[1]
            && entity_max[1] >= query_min[1]
            && entity_min[2] <= query_max[2]
            && entity_max[2] >= query_min[2]
    }

    fn spatial_candidate_slots_in_aabb(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> Option<Vec<usize>> {
        if self.len() <= TICK_SPATIAL_INDEX_MIN_ENTITIES {
            return None;
        }
        let min_sx = tick_spatial_section_coord(query_min[0])?;
        let min_sy = tick_spatial_section_coord(query_min[1])?;
        let min_sz = tick_spatial_section_coord(query_min[2])?;
        let max_sx = tick_spatial_section_coord(query_max[0])?;
        let max_sy = tick_spatial_section_coord(query_max[1])?;
        let max_sz = tick_spatial_section_coord(query_max[2])?;
        let span_x = i64::from(max_sx) - i64::from(min_sx) + 1;
        let span_y = i64::from(max_sy) - i64::from(min_sy) + 1;
        let span_z = i64::from(max_sz) - i64::from(min_sz) + 1;
        let total_cells = span_x.checked_mul(span_y)?.checked_mul(span_z)?;
        if total_cells > TICK_SPATIAL_INDEX_MAX_QUERY_SECTIONS || total_cells as usize > self.len()
        {
            return None;
        }
        let index = self
            .spatial_index
            .get_or_init(|| Arc::new(self.build_tick_spatial_index()));
        let mut candidate_slots = Vec::with_capacity(index.oversized_slots.len() + 8);
        candidate_slots.extend_from_slice(&index.oversized_slots);
        for sy in min_sy..=max_sy {
            for sz in min_sz..=max_sz {
                for sx in min_sx..=max_sx {
                    if let Some(slots) = index.sections.get(&(sx, sy, sz)) {
                        candidate_slots.extend_from_slice(slots);
                    }
                }
            }
        }
        candidate_slots.sort_unstable();
        candidate_slots.dedup();
        Some(candidate_slots)
    }

    fn build_tick_spatial_index(&self) -> EntityChunkSpatialIndex {
        let mut sections: FxHashMap<(i32, i32, i32), Vec<usize>> = FxHashMap::default();
        let mut oversized_slots = Vec::new();
        for (slot_idx, slot) in self.slots.iter().enumerate() {
            let Some(entity) = slot.as_ref() else {
                continue;
            };
            if entity.identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
                oversized_slots.push(slot_idx);
                continue;
            }
            let Some(bounds) = self.debug_hitbox_bounds_for_record(entity) else {
                oversized_slots.push(slot_idx);
                continue;
            };
            let pos = entity.transform.position;
            let old = entity.transform.old_position;
            let pick_pad = f64::from(bounds.pick_radius.max(0.0));
            let min_x = pos.x.min(old.x) + f64::from(bounds.min[0]).min(0.0) - pick_pad;
            let min_y = pos.y.min(old.y) + f64::from(bounds.min[1]).min(0.0) - pick_pad;
            let min_z = pos.z.min(old.z) + f64::from(bounds.min[2]).min(0.0) - pick_pad;
            let max_x = pos.x.max(old.x) + f64::from(bounds.max[0]).max(0.0) + pick_pad;
            let max_y = pos.y.max(old.y) + f64::from(bounds.max[1]).max(0.0) + pick_pad;
            let max_z = pos.z.max(old.z) + f64::from(bounds.max[2]).max(0.0) + pick_pad;
            let (
                Some(min_sx),
                Some(min_sy),
                Some(min_sz),
                Some(max_sx),
                Some(max_sy),
                Some(max_sz),
            ) = (
                tick_spatial_section_coord(min_x),
                tick_spatial_section_coord(min_y),
                tick_spatial_section_coord(min_z),
                tick_spatial_section_coord(max_x),
                tick_spatial_section_coord(max_y),
                tick_spatial_section_coord(max_z),
            )
            else {
                oversized_slots.push(slot_idx);
                continue;
            };
            if max_sx.saturating_sub(min_sx) >= TICK_SPATIAL_INDEX_MAX_ENTITY_SECTIONS_PER_AXIS
                || max_sy.saturating_sub(min_sy) >= TICK_SPATIAL_INDEX_MAX_ENTITY_SECTIONS_PER_AXIS
                || max_sz.saturating_sub(min_sz) >= TICK_SPATIAL_INDEX_MAX_ENTITY_SECTIONS_PER_AXIS
            {
                oversized_slots.push(slot_idx);
                continue;
            }
            for sy in min_sy..=max_sy {
                for sz in min_sz..=max_sz {
                    for sx in min_sx..=max_sx {
                        sections.entry((sx, sy, sz)).or_default().push(slot_idx);
                    }
                }
            }
        }
        EntityChunkSpatialIndex {
            sections,
            oversized_slots,
        }
    }
}

const TICK_SPATIAL_INDEX_MIN_ENTITIES: usize = 8;
const TICK_SPATIAL_INDEX_MAX_ENTITY_SECTIONS_PER_AXIS: i32 = 4;
const TICK_SPATIAL_INDEX_MAX_QUERY_SECTIONS: i64 = 64;

fn tick_spatial_section_coord(value: f64) -> Option<i32> {
    if !value.is_finite() {
        return None;
    }
    let floored = value.floor();
    if floored < f64::from(i32::MIN) || floored > f64::from(i32::MAX) {
        return None;
    }
    Some((floored as i32) >> 4)
}
