use super::*;

const SPATIAL_INDEX_MIN_ENTITIES: usize = 8;
const SPATIAL_INDEX_MAX_ENTITY_SECTIONS_PER_AXIS: i32 = 4;
const SPATIAL_INDEX_MAX_QUERY_SECTIONS: i64 = 64;

fn spatial_section_coord(value: f64) -> Option<i32> {
    if !value.is_finite() {
        return None;
    }
    let floored = value.floor();
    if floored < f64::from(i32::MIN) || floored > f64::from(i32::MAX) {
        return None;
    }
    Some((floored as i32) >> 4)
}

impl EntityStore {
    fn metadata_byte_for_record(entity: &EntityRecord, data_id: u8, default: i8) -> i8 {
        entity_data_byte(&entity.metadata.data_values, data_id, default)
    }

    pub(super) fn pick_bounds_for_record(
        &self,
        entity: &EntityRecord,
    ) -> Option<super::super::EntityPickBoundsState> {
        let identity = &entity.identity;
        let metadata = &entity.metadata;
        let attributes = &entity.attributes;
        let client_animations = &entity.animations;
        super::super::content_facts::pick_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(*client_animations),
            metadata.painting_variant_projection.as_ref(),
            &self.content_registry,
        )
    }

    pub(super) fn debug_hitbox_bounds_for_record(
        &self,
        entity: &EntityRecord,
    ) -> Option<super::super::EntityPickBoundsState> {
        let identity = &entity.identity;
        let metadata = &entity.metadata;
        let attributes = &entity.attributes;
        let client_animations = &entity.animations;
        super::super::content_facts::bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(*client_animations),
            metadata.painting_variant_projection.as_ref(),
            &self.content_registry,
        )
    }

    fn spatial_index(&self) -> &EntityChunkSpatialIndex {
        self.spatial_index
            .get_or_init(|| Arc::new(self.build_spatial_index()))
    }

    fn build_spatial_index(&self) -> EntityChunkSpatialIndex {
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
                spatial_section_coord(min_x),
                spatial_section_coord(min_y),
                spatial_section_coord(min_z),
                spatial_section_coord(max_x),
                spatial_section_coord(max_y),
                spatial_section_coord(max_z),
            )
            else {
                oversized_slots.push(slot_idx);
                continue;
            };
            if max_sx.saturating_sub(min_sx) >= SPATIAL_INDEX_MAX_ENTITY_SECTIONS_PER_AXIS
                || max_sy.saturating_sub(min_sy) >= SPATIAL_INDEX_MAX_ENTITY_SECTIONS_PER_AXIS
                || max_sz.saturating_sub(min_sz) >= SPATIAL_INDEX_MAX_ENTITY_SECTIONS_PER_AXIS
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

    fn candidate_slots_in_aabb(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> Option<Vec<usize>> {
        if self.len() <= SPATIAL_INDEX_MIN_ENTITIES {
            return None;
        }
        let min_sx = spatial_section_coord(query_min[0])?;
        let min_sy = spatial_section_coord(query_min[1])?;
        let min_sz = spatial_section_coord(query_min[2])?;
        let max_sx = spatial_section_coord(query_max[0])?;
        let max_sy = spatial_section_coord(query_max[1])?;
        let max_sz = spatial_section_coord(query_max[2])?;
        let span_x = i64::from(max_sx) - i64::from(min_sx) + 1;
        let span_y = i64::from(max_sy) - i64::from(min_sy) + 1;
        let span_z = i64::from(max_sz) - i64::from(min_sz) + 1;
        let total_cells = span_x.checked_mul(span_y)?.checked_mul(span_z)?;
        if total_cells > SPATIAL_INDEX_MAX_QUERY_SECTIONS || total_cells as usize > self.len() {
            return None;
        }
        let index = self.spatial_index();
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

    fn push_pick_target_for_record(
        &self,
        entity: &EntityRecord,
        partial_ticks: f32,
        is_spectator_player: &impl Fn(Uuid) -> bool,
        targets: &mut Vec<super::super::EntityPickTargetState>,
    ) {
        let identity = &entity.identity;
        if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && is_spectator_player(identity.uuid)
        {
            return;
        }
        let transform = &entity.transform;
        if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
            let dragon_animation = entity.animations.ender_dragon_kind().and_then(|k| k.state);
            targets.extend(ender_dragon_part_pick_targets_at_partial_tick(
                identity.id,
                *transform,
                dragon_animation,
                partial_ticks,
            ));
        } else if let Some(bounds) = self.pick_bounds_for_record(entity) {
            targets.push(super::super::EntityPickTargetState {
                entity_id: identity.id,
                position: transform.render_position(partial_ticks),
                bounds,
            });
        }
    }

    fn push_debug_hitbox_target_for_record(
        &self,
        entity: &EntityRecord,
        partial_ticks: f32,
        entity_accessors: Option<crate::entities::schema::VanillaEntityMetadataAccessors>,
        is_spectator_player: &impl Fn(Uuid) -> bool,
        targets: &mut Vec<super::super::EntityPickTargetState>,
    ) {
        let identity = &entity.identity;
        if identity.entity_type_id == VANILLA_ENTITY_TYPE_PLAYER_ID
            && is_spectator_player(identity.uuid)
        {
            return;
        }
        let transform = &entity.transform;
        if let Some(accessors) = entity_accessors {
            let invisible = Self::metadata_byte_for_record(
                entity,
                accessors.shared_flags_data_id,
                accessors.shared_flags_default,
            ) & accessors.invisible_mask
                != 0;
            if invisible {
                return;
            }
        }
        if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
            targets.push(ender_dragon_parent_pick_target(
                identity.id,
                *transform,
                partial_ticks,
            ));
            let dragon_animation = entity.animations.ender_dragon_kind().and_then(|k| k.state);
            targets.extend(ender_dragon_part_pick_targets_at_partial_tick(
                identity.id,
                *transform,
                dragon_animation,
                partial_ticks,
            ));
        } else if let Some(bounds) = self.debug_hitbox_bounds_for_record(entity) {
            targets.push(super::super::EntityPickTargetState {
                entity_id: identity.id,
                position: transform.render_position(partial_ticks),
                bounds,
            });
        }
    }

    pub(crate) fn pose(&self, id: i32) -> Option<i32> {
        let entity = self.record(id)?;
        let metadata = &entity.metadata;
        Some(entity_data_pose(&metadata.data_values))
    }

    /// Vanilla `Entity.isOnFire`: private client ignition or the synced fire bit,
    /// with `Blaze.isOnFire()` (`Blaze.java:137-143`) overriding `fireImmune` via
    /// its charged `DATA_FLAGS_ID` bit 0.
    pub(crate) fn on_fire(&self, id: i32) -> Option<bool> {
        let entity = self.record(id)?;
        if entity.identity.entity_type_id == VANILLA_ENTITY_TYPE_BLAZE_ID {
            let (flags_id, flags_default) =
                vanilla_entity_metadata_blaze_flags_accessor_for_entity_type(
                    entity.identity.entity_type_id,
                )?;
            let blaze_flags =
                entity_data_byte(&entity.metadata.data_values, flags_id, flags_default);
            return Some((blaze_flags & 1) != 0);
        }
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let shared_flags = entity_data_byte(
            &entity.metadata.data_values,
            entity_accessors.shared_flags_data_id,
            entity_accessors.shared_flags_default,
        );
        Some(
            !super::super::content_facts::entity_is_fire_immune(
                entity.identity.entity_type_id,
                &self.content_registry,
            ) && (entity.supporting_block.block_effects.remaining_fire_ticks > 0
                || shared_flags & entity_accessors.on_fire_mask != 0),
        )
    }

    pub(crate) fn pick_bounds(&self, id: i32) -> Option<super::super::EntityPickBoundsState> {
        let entity = self.record(id)?;
        self.pick_bounds_for_record(entity)
    }

    pub(crate) fn body_anchor_y_offset(
        &self,
        id: i32,
        game_time: i64,
        is_front: bool,
        partial_ticks: f32,
    ) -> Option<f32> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let metadata = &entity.metadata;
        let attributes = &entity.attributes;
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
    ) -> Option<super::super::EntityPickBoundsState> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let metadata = &entity.metadata;
        let attributes = &entity.attributes;
        let client_animations = &entity.animations;
        let mut bounds = super::super::content_facts::model_source_bounds_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(*client_animations),
            metadata.painting_variant_projection.as_ref(),
            &self.content_registry,
        )?;
        if let Some(display) = self.minecart_display_block_state(identity.id, registries) {
            bounds = bounds.expand_towards_y(display.display_offset as f32 * 0.75 / 16.0);
        }
        Some(bounds)
    }

    pub(crate) fn pick_targets_at_partial_tick_filtered(
        &self,
        partial_ticks: f32,
        is_spectator_player: impl Fn(Uuid) -> bool,
    ) -> Vec<super::super::EntityPickTargetState> {
        let mut targets = Vec::with_capacity(self.len());
        for entity in self.records() {
            self.push_pick_target_for_record(
                entity,
                partial_ticks,
                &is_spectator_player,
                &mut targets,
            );
        }
        targets
    }

    pub(crate) fn pick_targets_in_aabb_at_partial_tick_filtered(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
        partial_ticks: f32,
        is_spectator_player: impl Fn(Uuid) -> bool,
    ) -> Vec<super::super::EntityPickTargetState> {
        if !(0.0..=1.0).contains(&partial_ticks) {
            return self.pick_targets_at_partial_tick_filtered(partial_ticks, is_spectator_player);
        }
        let Some(candidate_slots) = self.candidate_slots_in_aabb(query_min, query_max) else {
            return self.pick_targets_at_partial_tick_filtered(partial_ticks, is_spectator_player);
        };
        let mut targets = Vec::with_capacity(candidate_slots.len());
        for slot in candidate_slots {
            if let Some(entity) = self.slots[slot].as_ref() {
                self.push_pick_target_for_record(
                    entity,
                    partial_ticks,
                    &is_spectator_player,
                    &mut targets,
                );
            }
        }
        targets
    }

    /// Vanilla `EntityHitboxDebugRenderer.emitGizmos` walks every rendered entity and draws its
    /// bounding box unless `entity.isInvisible()`; the pickability gate only applies to actual
    /// pick targeting, so non-pickable entities (dropped items, experience orbs, area effect
    /// clouds) still show their boxes here.
    pub(crate) fn debug_hitbox_targets_at_partial_tick_filtered(
        &self,
        partial_ticks: f32,
        is_spectator_player: impl Fn(Uuid) -> bool,
    ) -> Vec<super::super::EntityPickTargetState> {
        let entity_accessors = vanilla_entity_metadata_entity_accessors();
        let mut targets = Vec::with_capacity(self.len());
        for entity in self.records() {
            self.push_debug_hitbox_target_for_record(
                entity,
                partial_ticks,
                entity_accessors,
                &is_spectator_player,
                &mut targets,
            );
        }
        targets
    }

    pub(crate) fn debug_hitbox_targets_in_aabb_at_partial_tick_filtered(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
        partial_ticks: f32,
        is_spectator_player: impl Fn(Uuid) -> bool,
    ) -> Vec<super::super::EntityPickTargetState> {
        if !(0.0..=1.0).contains(&partial_ticks) {
            return self
                .debug_hitbox_targets_at_partial_tick_filtered(partial_ticks, is_spectator_player);
        }
        let Some(candidate_slots) = self.candidate_slots_in_aabb(query_min, query_max) else {
            return self
                .debug_hitbox_targets_at_partial_tick_filtered(partial_ticks, is_spectator_player);
        };
        let entity_accessors = vanilla_entity_metadata_entity_accessors();
        let mut targets = Vec::with_capacity(candidate_slots.len());
        for slot in candidate_slots {
            if let Some(entity) = self.slots[slot].as_ref() {
                self.push_debug_hitbox_target_for_record(
                    entity,
                    partial_ticks,
                    entity_accessors,
                    &is_spectator_player,
                    &mut targets,
                );
            }
        }
        targets
    }

    /// The debug-hitbox bounds skip the `isPickable` gate that `pick_bounds` applies.
    pub(crate) fn debug_hitbox_bounds(
        &self,
        id: i32,
    ) -> Option<super::super::EntityPickBoundsState> {
        let entity = self.record(id)?;
        self.debug_hitbox_bounds_for_record(entity)
    }

    /// Mirrors the strict `AABB.intersects` test used by `Level.getEntities`
    /// for a caller-selected entity family. Every retained entity must have
    /// canonical dimensions; a missing projection keeps prediction explicit.
    pub(crate) fn any_entity_bounds_intersecting(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
        type_filter: impl Fn(i32) -> bool,
    ) -> Option<bool> {
        self.any_entity_bounds_intersecting_except(query_min, query_max, None, type_filter)
    }

    fn entity_record_intersects_query(
        &self,
        entity: &EntityRecord,
        query_min: [f64; 3],
        query_max: [f64; 3],
        excluded_id: Option<i32>,
        type_filter: &impl Fn(i32) -> bool,
    ) -> Option<bool> {
        let id = entity.identity.id;
        if excluded_id == Some(id) || !type_filter(entity.identity.entity_type_id) {
            return Some(false);
        }
        let transform = &entity.transform;
        let bounds = self.debug_hitbox_bounds_for_record(entity)?;
        let entity_min = [
            transform.position.x + f64::from(bounds.min[0]),
            transform.position.y + f64::from(bounds.min[1]),
            transform.position.z + f64::from(bounds.min[2]),
        ];
        let entity_max = [
            transform.position.x + f64::from(bounds.max[0]),
            transform.position.y + f64::from(bounds.max[1]),
            transform.position.z + f64::from(bounds.max[2]),
        ];
        Some(
            (0..3).all(|axis| {
                entity_min[axis] < query_max[axis] && entity_max[axis] > query_min[axis]
            }),
        )
    }

    pub(crate) fn any_entity_bounds_intersecting_except(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
        excluded_id: Option<i32>,
        type_filter: impl Fn(i32) -> bool,
    ) -> Option<bool> {
        if !(0..3).all(|axis| {
            query_min[axis].is_finite()
                && query_max[axis].is_finite()
                && query_min[axis] < query_max[axis]
        }) {
            return None;
        }
        if let Some(candidate_slots) = self.candidate_slots_in_aabb(query_min, query_max) {
            for slot in candidate_slots {
                if let Some(entity) = self.slots[slot].as_ref() {
                    if self.entity_record_intersects_query(
                        entity,
                        query_min,
                        query_max,
                        excluded_id,
                        &type_filter,
                    )? {
                        return Some(true);
                    }
                }
            }
            return Some(false);
        }
        for entity in self.records() {
            if self.entity_record_intersects_query(
                entity,
                query_min,
                query_max,
                excluded_id,
                &type_filter,
            )? {
                return Some(true);
            }
        }
        Some(false)
    }

    /// Returns the building-blocking entities whose canonical boxes strictly
    /// intersect one prospective block collision box.
    ///
    /// Vanilla `EntityGetter.isUnobstructed(null, shape)` first applies the
    /// `blocksBuilding` instance flag and then intersects the entity AABB with
    /// the block's collision shape. The only dynamic flag exception in 26.1 is
    /// the synchronized marker armor stand.
    pub(crate) fn block_placement_obstruction_ids_intersecting(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> Option<Vec<i32>> {
        self.entity_bounds_intersection_ids(query_min, query_max, false, |entity_type_id| {
            super::super::vanilla_entity_type_blocks_building(entity_type_id)
        })
    }

    /// Returns every canonical entity AABB intersecting one query, including
    /// degenerate marker points. Callers retain player-info ownership for
    /// Vanilla's spectator filter.
    pub(crate) fn all_entity_bounds_intersection_ids(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
    ) -> Option<Vec<i32>> {
        self.entity_bounds_intersection_ids(query_min, query_max, true, |_| true)
    }

    fn collect_entity_bounds_intersection_id(
        &self,
        entity: &EntityRecord,
        query_min: [f64; 3],
        query_max: [f64; 3],
        include_marker_points: bool,
        type_filter: &impl Fn(i32) -> bool,
        obstructing: &mut Vec<i32>,
    ) -> Option<()> {
        let identity = &entity.identity;
        if !type_filter(identity.entity_type_id) {
            return Some(());
        }
        let transform = &entity.transform;
        let entity_position = [
            transform.position.x,
            transform.position.y,
            transform.position.z,
        ];
        if identity.entity_type_id == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
            let accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
                identity.entity_type_id,
            )?;
            let flags = Self::metadata_byte_for_record(
                entity,
                accessors.client_flags_data_id,
                accessors.client_flags_default,
            );
            if flags & accessors.marker_mask != 0 {
                if include_marker_points
                    && (0..3).all(|axis| {
                        entity_position[axis] < query_max[axis]
                            && entity_position[axis] > query_min[axis]
                    })
                {
                    obstructing.push(identity.id);
                }
                return Some(());
            }
        }
        let bounds = self.debug_hitbox_bounds_for_record(entity)?;
        let entity_min = [
            entity_position[0] + f64::from(bounds.min[0]),
            entity_position[1] + f64::from(bounds.min[1]),
            entity_position[2] + f64::from(bounds.min[2]),
        ];
        let entity_max = [
            entity_position[0] + f64::from(bounds.max[0]),
            entity_position[1] + f64::from(bounds.max[1]),
            entity_position[2] + f64::from(bounds.max[2]),
        ];
        if (0..3)
            .all(|axis| entity_min[axis] < query_max[axis] && entity_max[axis] > query_min[axis])
        {
            obstructing.push(identity.id);
            return Some(());
        }
        if identity.entity_type_id == VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
            let client_animations = &entity.animations;
            let dragon_animation = client_animations.ender_dragon_kind().and_then(|k| k.state);
            if ender_dragon_part_pick_targets_at_partial_tick(
                identity.id,
                *transform,
                dragon_animation,
                1.0,
            )
            .into_iter()
            .any(|part| {
                let part_position = [part.position.x, part.position.y, part.position.z];
                let part_min: [f64; 3] = std::array::from_fn(|axis| {
                    part_position[axis] + f64::from(part.bounds.min[axis])
                });
                let part_max: [f64; 3] = std::array::from_fn(|axis| {
                    part_position[axis] + f64::from(part.bounds.max[axis])
                });
                (0..3).all(|axis| {
                    part_min[axis] < query_max[axis] && part_max[axis] > query_min[axis]
                })
            }) {
                obstructing.push(identity.id);
            }
        }
        Some(())
    }

    pub(super) fn entity_bounds_intersection_ids(
        &self,
        query_min: [f64; 3],
        query_max: [f64; 3],
        include_marker_points: bool,
        type_filter: impl Fn(i32) -> bool,
    ) -> Option<Vec<i32>> {
        if !(0..3).all(|axis| {
            query_min[axis].is_finite()
                && query_max[axis].is_finite()
                && query_min[axis] < query_max[axis]
        }) {
            return None;
        }
        let mut obstructing = Vec::new();
        if let Some(candidate_slots) = self.candidate_slots_in_aabb(query_min, query_max) {
            for slot in candidate_slots {
                if let Some(entity) = self.slots[slot].as_ref() {
                    self.collect_entity_bounds_intersection_id(
                        entity,
                        query_min,
                        query_max,
                        include_marker_points,
                        &type_filter,
                        &mut obstructing,
                    )?;
                }
            }
            return Some(obstructing);
        }
        for entity in self.records() {
            self.collect_entity_bounds_intersection_id(
                entity,
                query_min,
                query_max,
                include_marker_points,
                &type_filter,
                &mut obstructing,
            )?;
        }
        Some(obstructing)
    }

    pub(crate) fn fill_model_candidates_at_partial_tick(
        &self,
        partial_ticks: f32,
        game_time: i64,
        minecart_improvements_enabled: bool,
        registries: &RegistrySet,
        candidates: &mut Vec<super::super::EntityRenderCandidateState>,
    ) {
        candidates.clear();
        candidates.reserve(self.len());
        for entity in self.records() {
            let (identity, transform, metadata, equipment, attributes, animations) = (
                &entity.identity,
                &entity.transform,
                &entity.metadata,
                &entity.equipment,
                &entity.attributes,
                &entity.animations,
            );
            let entity_accessors = vanilla_entity_metadata_entity_accessors();
            let shared_flags = entity_accessors.map(|accessors| {
                metadata
                    .data_values
                    .iter()
                    .find(|value| value.data_id == accessors.shared_flags_data_id)
                    .and_then(|value| match &value.value {
                        EntityDataValueKind::Byte(value) => Some(*value),
                        _ => None,
                    })
                    .unwrap_or(accessors.shared_flags_default)
            });
            let is_invisible = entity_accessors
                .zip(shared_flags)
                .is_some_and(|(accessors, flags)| flags & accessors.invisible_mask != 0);
            let has_glowing_flag = entity_accessors
                .zip(shared_flags)
                .is_some_and(|(accessors, flags)| flags & accessors.glowing_mask != 0);
            let custom_name_visible = entity_accessors.is_some_and(|accessors| {
                entity_data_bool(
                    &metadata.data_values,
                    accessors.custom_name_visible_data_id,
                    accessors.custom_name_visible_default,
                )
            });
            let has_always_visible_name = matches!(
                identity.entity_type_id,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID
                    | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID
            ) || custom_name_visible;
            let Some(mut bounds) = super::super::content_facts::model_source_bounds_for_entity_data(
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
            if let Some(display_offset) = minecart_display_offset_for_components(
                identity.entity_type_id,
                &metadata.data_values,
                registries,
            ) {
                bounds = bounds.expand_towards_y(display_offset as f32 * 0.75 / 16.0);
            }
            candidates.push(super::super::EntityRenderCandidateState {
                entity_id: identity.id,
                uuid: identity.uuid,
                entity_type_id: identity.entity_type_id,
                position: transform.render_position(partial_ticks),
                current_position: transform.position,
                bounds,
                vehicle_id: entity.mount.vehicle_id,
                leash_holder_id: entity.leash.holder_id,
                fishing_hook_owner_id: (identity.entity_type_id
                    == VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID)
                    .then_some(identity.data),
                is_spectator: false,
                is_invisible,
                is_sleeping: super::super::content_facts::entity_is_living(
                    identity.entity_type_id,
                    &self.content_registry,
                ) && entity_data_pose(&metadata.data_values)
                    == VANILLA_ENTITY_POSE_SLEEPING_ID,
                wears_dragon_head: equipment.equipment.iter().any(|update| {
                    update.slot == ProtocolEquipmentSlot::Head
                        && update.item.item_id == Some(VANILLA_ITEM_DRAGON_HEAD_ID)
                }),
                has_glowing_flag,
                has_always_visible_name,
                display_culling: display_entity_culling_state(
                    identity.entity_type_id,
                    &metadata.data_values,
                ),
                special_culling: renderer_special_culling_state(
                    identity.entity_type_id,
                    identity.data,
                    transform.position,
                    &metadata.data_values,
                    &attributes.attributes,
                    *animations,
                ),
            });
        }
        self.update_passenger_candidate_positions(
            partial_ticks,
            game_time,
            minecart_improvements_enabled,
            candidates,
            true,
        );
    }

    pub(crate) fn update_passenger_candidate_positions(
        &self,
        partial_ticks: f32,
        game_time: i64,
        minecart_improvements_enabled: bool,
        candidates: &mut [super::super::EntityRenderCandidateState],
        update_current: bool,
    ) {
        let passenger_ids = candidates
            .iter()
            .filter(|candidate| candidate.vehicle_id.is_some())
            .map(|candidate| candidate.entity_id)
            .collect::<Vec<_>>();
        if passenger_ids.is_empty() {
            return;
        }
        let positions = |tick| {
            self.render_positions_at_partial_tick(
                &passenger_ids,
                tick,
                game_time,
                minecart_improvements_enabled,
            )
        };
        let render_positions = positions(partial_ticks);
        let current_positions = update_current.then(|| positions(1.0));
        for candidate in candidates.iter_mut() {
            if candidate.vehicle_id.is_none() {
                continue;
            }
            candidate.position = render_positions
                .get(&candidate.entity_id)
                .copied()
                .unwrap_or(candidate.position);
            if let Some(current) = current_positions
                .as_ref()
                .and_then(|positions| positions.get(&candidate.entity_id))
            {
                candidate.current_position = *current;
            }
        }
    }

    pub(crate) fn display_render_source_at_partial_tick(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<DisplayEntityRenderSourceState> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let transform = entity.transform;
        let metadata = &entity.metadata;
        display_entity_render_source(identity, transform, metadata, partial_ticks)
    }

    pub(crate) fn fishing_hook_render_source_at_partial_tick(
        &self,
        hook_id: i32,
        partial_ticks: f32,
    ) -> Option<FishingHookRenderSourceState> {
        let hook = self.record(hook_id)?;
        let hook_identity = &hook.identity;
        if hook_identity.entity_type_id != VANILLA_ENTITY_TYPE_FISHING_BOBBER_ID {
            return None;
        }
        let owner_id = hook_identity.data;
        let hook_transform = &hook.transform;

        let owner = self.record(owner_id)?;
        let owner_identity = &owner.identity;
        if owner_identity.entity_type_id != VANILLA_ENTITY_TYPE_PLAYER_ID {
            return None;
        }
        let owner_transform = &owner.transform;
        let owner_metadata = &owner.metadata;
        let owner_attributes = &owner.attributes;
        let owner_animations = &owner.animations;
        let owner_equipment = &owner.equipment;

        let owner_position = owner_transform.render_position(partial_ticks);
        let eye_height = vanilla_eye_height_for_entity_data(
            owner_identity.entity_type_id,
            owner_identity.data,
            &owner_metadata.data_values,
            &owner_attributes.attributes,
            Some(*owner_animations),
            owner_metadata.painting_variant_projection.as_ref(),
        )?;
        let owner_eye_position = EntityVec3 {
            y: owner_position.y + f64::from(eye_height),
            ..owner_position
        };
        let owner_scale =
            vanilla_render_scale(owner_identity.entity_type_id, &owner_attributes.attributes);
        let main_arm_left = self.entity_main_arm_left(owner_id, owner_identity.entity_type_id);
        let main_hand_holds_rod = owner_equipment.equipment.iter().any(|update| {
            update.slot == ProtocolEquipmentSlot::MainHand
                && update.item.item_id == Some(VANILLA_ITEM_FISHING_ROD_ID)
        });

        Some(FishingHookRenderSourceState {
            hook_position: hook_transform.render_position(partial_ticks),
            owner_id,
            owner_eye_position,
            owner_body_y_rot: owner_transform.render_body_y_rot(partial_ticks),
            owner_scale,
            owner_is_crouching: entity_data_pose(&owner_metadata.data_values)
                == VANILLA_ENTITY_POSE_CROUCHING_ID,
            holding_arm_left: if main_hand_holds_rod {
                main_arm_left
            } else {
                !main_arm_left
            },
            owner_attack_anim: owner_animations.attack_anim(partial_ticks),
        })
    }

    #[cfg(test)]
    pub(crate) fn model_candidates_at_partial_tick(
        &self,
        partial_ticks: f32,
        registries: &RegistrySet,
    ) -> Vec<super::super::EntityRenderCandidateState> {
        let mut candidates = Vec::new();
        self.fill_model_candidates_at_partial_tick(
            partial_ticks,
            0,
            false,
            registries,
            &mut candidates,
        );
        candidates
    }
}
