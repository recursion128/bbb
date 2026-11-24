use super::*;

impl EntityStore {
    pub(crate) fn mount(&self, id: i32) -> Option<EntityMount> {
        self.record(id).map(|entity| entity.mount.clone())
    }

    pub(crate) fn debug_passenger_vehicle_target(
        &self,
        passenger_id: i32,
        game_time: i64,
    ) -> Option<super::super::EntityPickTargetState> {
        let passenger_identity = self.identity(passenger_id)?;
        let passenger_bounds = self.pick_bounds(passenger_id)?;
        let passenger_mount = self.mount(passenger_id)?;
        let vehicle_id = passenger_mount.vehicle_id?;
        let vehicle_bounds = self.pick_bounds(vehicle_id)?;
        let vehicle_entity = self.record(vehicle_id)?;
        let vehicle_identity = &vehicle_entity.identity;
        let vehicle_transform = &vehicle_entity.transform;
        let vehicle_mount = &vehicle_entity.mount;
        let vehicle_metadata = &vehicle_entity.metadata;
        let vehicle_attributes = &vehicle_entity.attributes;
        let (position, bounds) = vanilla_passenger_vehicle_debug_target_for_entity_data(
            vehicle_identity.entity_type_id,
            vehicle_transform.position,
            vehicle_transform.y_rot,
            vehicle_bounds,
            &vehicle_metadata.data_values,
            &vehicle_attributes.attributes,
            Some(vehicle_entity.animations),
            &vehicle_mount.passengers,
            passenger_id,
            passenger_identity.entity_type_id,
            passenger_bounds,
            game_time,
        )?;
        Some(super::super::EntityPickTargetState {
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
        self.record(id).map(|entity| entity.leash)
    }

    pub(crate) fn leash_holder_id(&self, id: i32) -> Option<i32> {
        self.record(id)?.leash.holder_id
    }

    /// Applies the client-side state mutation from vanilla 26.1
    /// `Entity.dropAllLeashConnections`: clear this leashable entity's own
    /// connection and every leashable entity connected to it within the
    /// 32×32×32 `Leashable.leashableInArea` search box.
    pub(crate) fn drop_local_leash_connections(&mut self, id: i32) -> Option<bool> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let transform = &entity.transform;
        let bounds = self.debug_hitbox_bounds(id)?;
        let search_center = [
            transform.position.x + f64::from((bounds.min[0] + bounds.max[0]) / 2.0),
            transform.position.y + f64::from((bounds.min[1] + bounds.max[1]) / 2.0),
            transform.position.z + f64::from((bounds.min[2] + bounds.max[2]) / 2.0),
        ];

        let mut connected_ids = Vec::new();
        if vanilla_entity_metadata_type_is_leashable(identity.entity_type_id)
            && self.leash_holder_id(id).is_some()
        {
            connected_ids.push(id);
        }

        for candidate_id in self.ids().filter(|candidate| *candidate != id) {
            if self.leash_holder_id(candidate_id) != Some(id) {
                continue;
            }
            let candidate = self.record(candidate_id)?;
            let candidate_identity = &candidate.identity;
            if !vanilla_entity_metadata_type_is_leashable(candidate_identity.entity_type_id) {
                continue;
            }
            let candidate_transform = &candidate.transform;
            let candidate_bounds = self.debug_hitbox_bounds(candidate_id)?;
            let candidate_min = [
                candidate_transform.position.x + f64::from(candidate_bounds.min[0]),
                candidate_transform.position.y + f64::from(candidate_bounds.min[1]),
                candidate_transform.position.z + f64::from(candidate_bounds.min[2]),
            ];
            let candidate_max = [
                candidate_transform.position.x + f64::from(candidate_bounds.max[0]),
                candidate_transform.position.y + f64::from(candidate_bounds.max[1]),
                candidate_transform.position.z + f64::from(candidate_bounds.max[2]),
            ];
            if (0..3).all(|axis| {
                candidate_max[axis] > search_center[axis] - 16.0
                    && candidate_min[axis] < search_center[axis] + 16.0
            }) {
                connected_ids.push(candidate_id);
            }
        }

        if connected_ids.is_empty() {
            return Some(false);
        }
        for connected_id in connected_ids {
            self.with_leash_mut(connected_id, |leash| leash.holder_id = None)?;
        }
        Some(true)
    }

    pub(crate) fn leash_endpoint_inputs(
        &self,
        id: i32,
        partial_ticks: f32,
        game_time: i64,
        minecart_improvements_enabled: bool,
    ) -> Option<super::super::leash::LeashEndpointInputs> {
        let entity = self.record(id)?;
        let entity_type_id = entity.identity.entity_type_id;
        let transform = entity.transform;
        let bounds = self.debug_hitbox_bounds(id)?;
        let eye_height = self
            .camera_pose_state_at_partial_tick(id, partial_ticks)?
            .eye_height;
        let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
        let shared_flags = self
            .metadata_byte(
                id,
                entity_accessors.shared_flags_data_id,
                entity_accessors.shared_flags_default,
            )
            .unwrap_or(entity_accessors.shared_flags_default);
        let metadata = &entity.metadata;
        let attributes = &entity.attributes;
        let pose = metadata
            .data_values
            .iter()
            .find(|value| value.data_id == entity_accessors.pose_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::Pose(pose) => Some(*pose),
                _ => None,
            })
            .unwrap_or(entity_accessors.pose_default);
        let is_auto_spin_attack =
            vanilla_entity_metadata_living_entity_accessors_for_entity_type(entity_type_id)
                .is_some_and(|accessors| {
                    self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)
                        .unwrap_or(accessors.flags_default)
                        & accessors.spin_attack_mask
                        != 0
                });
        // Vanilla `Avatar.DEFAULT_MAIN_HAND` is right (id 1).
        let main_arm_right = self
            .metadata_humanoid_arm(id, VANILLA_AVATAR_MAIN_HAND_DATA_ID, 1)
            .unwrap_or(1)
            != 0;
        Some(super::super::leash::LeashEndpointInputs {
            entity_type_id,
            leash_offset_owner: vanilla_entity_metadata_leash_offset_owner(entity_type_id),
            rope_hold_position_owner: vanilla_entity_metadata_rope_hold_position_owner(
                entity_type_id,
            )?,
            position: self.render_position_at_partial_tick(
                id,
                partial_ticks,
                game_time,
                minecart_improvements_enabled,
            )?,
            body_y_rot: transform.render_body_y_rot(partial_ticks),
            // Vanilla `Player.getRopeHoldPosition` pitch is
            // `Mth.lerp(partialTicks * 0.5, getXRot(), xRotO)` — the reversed
            // half lerp equals the ordinary render lerp at `1 - 0.5 * t`.
            rope_hold_x_rot: transform.render_x_rot(1.0 - 0.5 * partial_ticks.clamp(0.0, 1.0)),
            view_x_rot: transform.render_x_rot(partial_ticks),
            view_y_rot: transform.render_y_rot(partial_ticks),
            bounds_width: bounds.max[0] - bounds.min[0],
            bounds_height: bounds.max[1] - bounds.min[1],
            eye_height,
            camel_leash_offset: vanilla_camel_leash_offset_for_entity_data(
                entity_type_id,
                &metadata.data_values,
                &attributes.attributes,
                game_time,
                partial_ticks,
            ),
            on_fire: shared_flags & entity_accessors.on_fire_mask != 0,
            fall_flying_or_spin: shared_flags & entity_accessors.fall_flying_mask != 0
                || is_auto_spin_attack,
            visually_swimming: pose == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID,
            crouching: pose == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID,
            main_arm_right,
            delta_movement: transform.delta_movement,
        })
    }

    pub(crate) fn mob_effects(&self, id: i32) -> Option<EntityMobEffects> {
        self.record(id).map(|entity| entity.effects.clone())
    }

    pub(crate) fn mob_effect_amplifier(&self, id: i32, effect_id: i32) -> Option<i32> {
        let entity = self.record(id)?;
        let effects = &entity.effects;
        effects
            .effects
            .get(&effect_id)
            .map(|effect| effect.amplifier)
    }

    pub(crate) fn damage(&self, id: i32) -> Option<EntityDamage> {
        self.record(id).map(|entity| entity.damage.clone())
    }

    #[cfg(test)]
    pub(crate) fn minecart_lerp(&self, id: i32) -> Option<EntityMinecartLerp> {
        self.record(id)?.family.minecart_lerp().cloned()
    }

    pub(crate) fn hurting_projectile(
        &self,
        id: i32,
    ) -> Option<super::super::HurtingProjectileState> {
        self.record(id)?.family.hurting_projectile().copied()
    }

    pub(crate) fn transform_states(&self) -> Vec<EntityTransformState> {
        self.records()
            .map(|entity| EntityTransformState::from_components(&entity.identity, entity.transform))
            .collect()
    }

    pub(crate) fn lightning_bolt_render_sources(&self) -> Vec<LightningBoltRenderSourceState> {
        let mut sources = Vec::new();
        for entity in self.records() {
            let identity = &entity.identity;
            let transform = &entity.transform;
            let animations = &entity.animations;
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID {
                continue;
            }
            let Some(seed) = animations.lightning_bolt_kind().and_then(|k| k.render_seed) else {
                continue;
            };
            sources.push(LightningBoltRenderSourceState {
                entity_id: identity.id,
                position: transform.position,
                seed,
            });
        }
        sources
    }

    pub(crate) fn item_entity_stacks(&self) -> impl Iterator<Item = ItemEntityStackState<'_>> {
        self.item_stacks_for_entity_types(&[VANILLA_ENTITY_TYPE_ITEM_ID])
    }

    pub(crate) fn item_frame_has_item(&self, id: i32) -> Option<bool> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let accessors =
            vanilla_entity_metadata_item_frame_accessors_for_entity_type(identity.entity_type_id)?;
        let metadata = &entity.metadata;
        Some(item_frame_item(accessors, &metadata.data_values).is_some())
    }

    /// Vanilla `ComparatorBlock.getItemFrame`: selects exactly one item frame
    /// whose canonical AABB intersects the queried block and whose hanging
    /// direction matches the comparator facing. An empty matching frame has
    /// analog output `0`; zero or multiple matches produce no frame input.
    pub(crate) fn comparator_item_frame_analog_output(
        &self,
        pos: crate::BlockPos,
        facing: super::super::ItemFrameFacing,
    ) -> Option<Option<i32>> {
        let query_min = [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)];
        let query_max = [
            f64::from(pos.x) + 1.0,
            f64::from(pos.y) + 1.0,
            f64::from(pos.z) + 1.0,
        ];
        let mut matched = None;
        for entity in self.records() {
            let id = &entity.identity.id;
            let identity = &entity.identity;
            let Some(accessors) = vanilla_entity_metadata_item_frame_accessors_for_entity_type(
                identity.entity_type_id,
            ) else {
                continue;
            };
            let transform = &entity.transform;
            let metadata = &entity.metadata;
            if item_frame_facing(accessors, identity.data, &metadata.data_values) != facing {
                continue;
            }
            let bounds = self.debug_hitbox_bounds(*id)?;
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
            if !(0..3).all(|axis| {
                entity_min[axis] < query_max[axis] && entity_max[axis] > query_min[axis]
            }) {
                continue;
            }
            if matched.is_some() {
                return Some(None);
            }
            matched = Some(
                item_frame_item(accessors, &metadata.data_values).map_or(0, |_| {
                    i32::from(item_frame_rotation(accessors, &metadata.data_values)) + 1
                }),
            );
        }
        Some(matched)
    }

    /// Vanilla `ChestBlock.isCatSittingOnChest`: any sitting cat whose
    /// canonical AABB intersects the one-block volume directly above the
    /// chest blocks container access and therefore comparator output.
    pub(crate) fn sitting_cat_intersects_chest_top(&self, pos: crate::BlockPos) -> Option<bool> {
        let query_min = [f64::from(pos.x), f64::from(pos.y) + 1.0, f64::from(pos.z)];
        let query_max = [
            f64::from(pos.x) + 1.0,
            f64::from(pos.y) + 2.0,
            f64::from(pos.z) + 1.0,
        ];
        for entity in self.records() {
            let id = &entity.identity.id;
            let identity = &entity.identity;
            if identity.entity_type_id != VANILLA_ENTITY_TYPE_CAT_ID {
                continue;
            }
            let accessors = vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
                identity.entity_type_id,
            )?;
            let flags = self.metadata_byte(
                identity.id,
                accessors.flags_data_id,
                accessors.flags_default,
            )?;
            if flags & accessors.sitting_mask == 0 {
                continue;
            }
            let transform = &entity.transform;
            let bounds = self.debug_hitbox_bounds(*id)?;
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
            if (0..3).all(|axis| {
                entity_min[axis] < query_max[axis] && entity_max[axis] > query_min[axis]
            }) {
                return Some(true);
            }
        }
        Some(false)
    }

    pub(crate) fn entity_age_ticks(&self, id: i32) -> Option<u32> {
        let entity = self.record(id)?;
        let client_animations = &entity.animations;
        Some(client_animations.age_ticks)
    }

    pub(crate) fn item_bob_offset(&self, id: i32) -> Option<f32> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        (identity.entity_type_id == VANILLA_ENTITY_TYPE_ITEM_ID).then_some(())?;
        let client_animations = &entity.animations;
        Some(client_animations.item_bob_offset())
    }

    /// The item in an arbitrary synced equipment slot, or `None` for an empty slot / no equipment.
    pub(crate) fn equipment_item(
        &self,
        id: i32,
        slot: ProtocolEquipmentSlot,
    ) -> Option<&ItemStackSummary> {
        let entity = self.record(id)?;
        let equipment = &entity.equipment;
        let item = &equipment
            .equipment
            .iter()
            .find(|update| update.slot == slot)?
            .item;
        item.item_id.map(|_| item)
    }

    pub(crate) fn clear_local_equipment_item(
        &mut self,
        id: i32,
        slot: ProtocolEquipmentSlot,
    ) -> bool {
        self.with_equipment_mut(id, |equipment| {
            let Some(update) = equipment
                .equipment
                .iter_mut()
                .find(|update| update.slot == slot && update.item.item_id.is_some())
            else {
                return false;
            };
            update.item = ItemStackSummary::empty();
            true
        })
        .unwrap_or(false)
    }

    pub(crate) fn set_local_equipment_item(
        &mut self,
        id: i32,
        slot: ProtocolEquipmentSlot,
        item: ItemStackSummary,
    ) -> bool {
        self.with_equipment_mut(id, |equipment| {
            if let Some(update) = equipment
                .equipment
                .iter_mut()
                .find(|update| update.slot == slot)
            {
                update.item = item;
            } else {
                equipment
                    .equipment
                    .push(bbb_protocol::packets::EquipmentSlotUpdate { slot, item });
                equipment
                    .equipment
                    .sort_by_key(|update| update.slot.ordinal());
            }
        })
        .is_some()
    }

    /// Applies the client half of `AbstractHorse.doPlayerRide`: clear the
    /// eating and standing flags immediately, reset the client-owned stand
    /// counter, and retarget the existing pose interpolation. Passenger state
    /// remains server-authoritative because `startRiding` is server-only.
    pub(crate) fn apply_local_horse_ride_pose_reset(&mut self, id: i32) -> Option<()> {
        let entity_type_id = self.entity_type_id(id)?;
        let accessors =
            vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)?;
        let flags = self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)?;
        let reset_flags = flags & !(accessors.eating_mask | accessors.standing_mask);
        let entity = self.record_mut(id)?;
        let metadata = &mut entity.metadata;
        let animations = &mut entity.animations;
        if reset_flags != flags {
            upsert_metadata_value(
                metadata,
                ProtocolEntityDataValue {
                    data_id: accessors.flags_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(reset_flags),
                },
            );
        }
        if let Some(ridden_jump) = animations.ridden_jump.as_mut() {
            ridden_jump.local_stand_ticks_remaining = 0;
        }
        animations.sync_targets_from_metadata(entity_type_id, &metadata.data_values);
        Some(())
    }

    /// Collects the unique generated ITEM_STACK accessor carried by every requested entity type.
    /// This covers dropped items and owner-specific thrown-item projectile accessors without treating
    /// their coincident wire indices as shared ownership.
    pub(crate) fn item_stacks_for_entity_types<'a>(
        &'a self,
        type_ids: &'a [i32],
    ) -> impl Iterator<Item = ItemEntityStackState<'a>> + 'a {
        self.records()
            .filter(move |entity| type_ids.contains(&entity.identity.entity_type_id))
            .filter_map(|entity| self.item_stack(entity.identity.id))
    }

    pub(crate) fn item_stack(&self, id: i32) -> Option<ItemEntityStackState<'_>> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let transform = &entity.transform;
        let metadata = &entity.metadata;
        let client_animations = &entity.animations;
        let data_id = vanilla_entity_metadata_item_entity_stack_accessor_for_entity_type(
            identity.entity_type_id,
        )
        .or_else(|| {
            vanilla_entity_metadata_unique_accessor_for_entity_type_and_serializer(
                identity.entity_type_id,
                "ITEM_STACK",
            )
            .map(|accessor| accessor.data_id)
        })?;
        let stack = metadata_item_stack(&metadata.data_values, data_id)
            .filter(|stack| stack.item_id.is_some() && stack.count > 0)?;
        Some(ItemEntityStackState {
            entity_id: identity.id,
            position: transform.position,
            light: super::super::ENTITY_LIGHT_PROBE_FULL_BRIGHT,
            bob_offset: client_animations.item_bob_offset(),
            stack,
        })
    }

    pub(crate) fn firework_rocket_explosion_particle_state(
        &self,
        id: i32,
    ) -> Option<FireworkRocketExplosionParticleState> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let accessors = vanilla_entity_metadata_firework_rocket_accessors_for_entity_type(
            identity.entity_type_id,
        )?;
        let transform = &entity.transform;
        let explosions = firework_rocket_render_stack(&entity.metadata.data_values, accessors)
            .map(|stack| stack.component_patch.fireworks_explosions.clone())
            .unwrap_or_default();
        Some(FireworkRocketExplosionParticleState {
            entity_id: identity.id,
            position: transform.position,
            delta_movement: transform.delta_movement,
            has_explosions: !explosions.is_empty(),
            explosions,
        })
    }

    pub(crate) fn fox_eat_particle_state(&self, id: i32) -> Option<FoxEatParticleState> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        if identity.entity_type_id != VANILLA_ENTITY_TYPE_FOX_ID {
            return None;
        }
        let transform = &entity.transform;
        let item_stack = self.equipment_item(id, ProtocolEquipmentSlot::MainHand)?;
        Some(FoxEatParticleState {
            entity_id: identity.id,
            position: transform.position,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            item_stack: item_stack.clone(),
        })
    }
}
