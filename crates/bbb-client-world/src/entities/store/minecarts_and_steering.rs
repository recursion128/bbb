use super::*;

impl EntityStore {
    pub(crate) fn minecart_command_block(&self, id: i32) -> Option<EntityMinecartCommandBlock> {
        self.record(id)?.family.minecart_command_block().cloned()
    }

    pub(crate) fn add_client_furnace_minecart_fuel(
        &mut self,
        id: i32,
        interacting_position: ProtocolVec3d,
    ) -> Option<bool> {
        const FUEL_TICKS_PER_ITEM: i32 = 3_600;
        const MAX_FUEL_TICKS: i32 = 32_000;

        let entity = self.record_mut(id)?;
        let cart_position = entity.transform.position;
        let furnace = entity.family.minecart_furnace_mut()?;
        let Some(next_fuel) = furnace.fuel.checked_add(FUEL_TICKS_PER_ITEM) else {
            return Some(false);
        };
        if next_fuel > MAX_FUEL_TICKS {
            return Some(false);
        }
        furnace.fuel = next_fuel;
        furnace.push = EntityVec3 {
            x: cart_position.x - interacting_position.x,
            y: 0.0,
            z: cart_position.z - interacting_position.z,
        };
        Some(true)
    }

    pub(crate) fn sync_minecart_command_block_from_metadata_update(
        &mut self,
        id: i32,
        values: &[ProtocolEntityDataValue],
    ) -> bool {
        let Some(entity) = self.record_mut(id) else {
            return false;
        };
        let Some(command_block) = entity.family.minecart_command_block_mut() else {
            return false;
        };
        let mut updated = false;
        for value in values {
            match (value.data_id, &value.value) {
                (
                    VANILLA_COMMAND_BLOCK_MINECART_COMMAND_DATA_ID,
                    EntityDataValueKind::String(command),
                ) => {
                    command_block.command.clone_from(command);
                    updated = true;
                }
                (
                    VANILLA_COMMAND_BLOCK_MINECART_LAST_OUTPUT_DATA_ID,
                    EntityDataValueKind::Component(output),
                ) => {
                    command_block.last_output = Some(output.clone());
                    updated = true;
                }
                _ => {}
            }
        }
        updated
    }

    pub(crate) fn clear_minecart_command_block_last_output(&mut self, id: i32) -> bool {
        let Some(entity) = self.record_mut(id) else {
            return false;
        };
        let Some(command_block) = entity.family.minecart_command_block_mut() else {
            return false;
        };
        command_block.last_output = None;
        true
    }

    pub(crate) fn set_minecart_command_block_track_output(
        &mut self,
        id: i32,
        track_output: bool,
    ) -> bool {
        let Some(entity) = self.record_mut(id) else {
            return false;
        };
        let Some(command_block) = entity.family.minecart_command_block_mut() else {
            return false;
        };
        command_block.track_output = track_output;
        true
    }

    pub(crate) fn player_rideable_jumping_cooldown(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let client_animations = &entity.animations;
        if vanilla_entity_metadata_type_is_camel(identity.entity_type_id) {
            return Some(client_animations.camel_jump_cooldown(partial_ticks));
        }
        if is_vanilla_abstract_nautilus_type(identity.entity_type_id) {
            return Some(client_animations.nautilus_jump_cooldown(partial_ticks));
        }
        Some(0.0)
    }

    pub(crate) fn ridden_jump_state(&self, id: i32) -> Option<RiddenJumpState> {
        let entity = self.record(id)?;
        let animations = &entity.animations;
        animations.ridden_jump
    }

    pub(crate) fn camel_jump_cooldown_ticks(&self, id: i32) -> Option<i32> {
        let entity = self.record(id)?;
        let animations = &entity.animations;
        Some(
            animations
                .camel_kind()
                .and_then(|k| k.dash_cooldown)
                .map(|cooldown| cooldown.cooldown_ticks())
                .unwrap_or(0),
        )
    }

    pub(crate) fn nautilus_jump_cooldown_ticks(&self, id: i32) -> Option<i32> {
        let entity = self.record(id)?;
        let animations = &entity.animations;
        Some(
            animations
                .nautilus_kind()
                .and_then(|k| k.dash_cooldown)
                .map(|cooldown| cooldown.cooldown_ticks())
                .unwrap_or(0),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn item_based_steering_controller_rotation(
        &self,
        vehicle_id: i32,
        entity_type_id: i32,
        profiles: &crate::ItemProfiles,
        local_player_id: Option<i32>,
        local_player_rotation: Option<(f32, f32)>,
        local_player_holds_carrot_on_a_stick: bool,
        local_player_holds_warped_fungus_on_a_stick: bool,
    ) -> Option<(f32, f32)> {
        vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
            entity_type_id,
        )?;
        if !self.saddle_slot_contains_saddle_item(vehicle_id, profiles) {
            return None;
        }
        let controller_id = self.mount(vehicle_id)?.passengers.first().copied()?;
        let required_item_id = match entity_type_id {
            VANILLA_ENTITY_TYPE_PIG_ID => VANILLA_ITEM_CARROT_ON_A_STICK_ID,
            VANILLA_ENTITY_TYPE_STRIDER_ID => VANILLA_ITEM_WARPED_FUNGUS_ON_A_STICK_ID,
            _ => return None,
        };
        if local_player_id == Some(controller_id) {
            let holds_required_item = match entity_type_id {
                VANILLA_ENTITY_TYPE_PIG_ID => local_player_holds_carrot_on_a_stick,
                VANILLA_ENTITY_TYPE_STRIDER_ID => local_player_holds_warped_fungus_on_a_stick,
                _ => false,
            };
            if !holds_required_item {
                return None;
            }
            return local_player_rotation;
        }
        if self.entity_type_id(controller_id) != Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
            || ![false, true].into_iter().any(|off_hand| {
                self.held_item(controller_id, off_hand)
                    .and_then(|item| item.item_id)
                    == Some(required_item_id)
            })
        {
            return None;
        }
        let transform = self.transform(controller_id)?;
        Some((transform.x_rot, transform.y_rot))
    }

    #[cfg(test)]
    pub(crate) fn item_based_steering_debug_state(&self, id: i32) -> Option<(bool, i32)> {
        let entity = self.record(id)?;
        let animations = &entity.animations;
        animations
            .item_based_steering
            .map(|steering| steering.debug_state())
    }

    pub(crate) fn advance_item_based_steering_tick(&mut self, id: i32) -> Option<f32> {
        let entity = self.record_mut(id)?;
        let identity = &entity.identity;
        let metadata = &entity.metadata;
        let animations = &mut entity.animations;
        let (data_id, default) =
            vanilla_entity_metadata_item_based_steering_boost_time_accessor_for_entity_type(
                identity.entity_type_id,
            )?;
        let total = entity_data_int(&metadata.data_values, data_id, default);
        Some(animations.advance_item_based_steering_tick(total))
    }

    /// Mirrors `Strider.setSuffocating`: the synced render flag and the
    /// transient movement-speed modifier are one canonical client-side update.
    /// A server attribute snapshot may already contain the named modifier, so
    /// replace by semantic id instead of stacking a second copy.
    pub(crate) fn set_strider_suffocating(&mut self, id: i32, suffocating: bool) -> Option<()> {
        const MODIFIER_ID: &str = "minecraft:suffocating";
        const MODIFIER_AMOUNT: f64 = -0.34_f32 as f64;

        let entity_type_id = self.entity_type_id(id)?;
        let (data_id, default) =
            vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(entity_type_id)?;
        let default_base =
            vanilla_default_attribute_base(entity_type_id, VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)?;
        let current_flag = self.metadata_bool_for_entity(id, data_id, default)?;
        let entity = self.record_mut(id)?;
        let current_modifier = entity
            .attributes
            .attributes
            .iter()
            .find(|attribute| attribute.attribute_id == VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            .and_then(|attribute| {
                attribute
                    .modifiers
                    .iter()
                    .find(|modifier| modifier.id == MODIFIER_ID)
            })
            .map(|modifier| (modifier.amount, modifier.operation_id));
        let expected_modifier = suffocating.then_some((
            MODIFIER_AMOUNT,
            VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
        ));
        if current_flag == suffocating && current_modifier == expected_modifier {
            return Some(());
        }
        let metadata = &mut entity.metadata;
        let attributes = &mut entity.attributes;
        upsert_metadata_value(
            metadata,
            ProtocolEntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(suffocating),
            },
        );
        let attributes = &mut *attributes.attributes;
        let movement = if let Some(movement) = attributes
            .iter_mut()
            .find(|attribute| attribute.attribute_id == VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        {
            movement
        } else {
            attributes.push(ProtocolAttributeSnapshot {
                attribute_id: VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID,
                base: default_base,
                modifiers: Vec::new(),
            });
            attributes.last_mut()?
        };
        movement
            .modifiers
            .retain(|modifier| modifier.id != MODIFIER_ID);
        if suffocating {
            movement.modifiers.push(ProtocolAttributeModifier {
                id: MODIFIER_ID.to_string(),
                amount: MODIFIER_AMOUNT,
                operation_id: VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
            });
        }
        attributes.sort_by_key(|attribute| attribute.attribute_id);
        Some(())
    }

    pub(crate) fn start_local_nautilus_dash(&mut self, id: i32) -> Option<()> {
        self.with_client_animations_mut(id, |animations| {
            animations.start_local_nautilus_dash();
        })
    }

    pub(crate) fn advance_deferred_nautilus_subclass_tick(
        &mut self,
        id: i32,
        in_water: bool,
    ) -> Option<()> {
        let entity = self.record_mut(id)?;
        let (identity, transform, triggers) = {
            let identity = &entity.identity;
            let transform = &entity.transform;
            let metadata = &mut entity.metadata;
            let animations = &mut entity.animations;
            let is_dashing = entity_data_bool(
                &metadata.data_values,
                VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
                VANILLA_ABSTRACT_NAUTILUS_DEFAULT_DASH,
            );
            let triggers = animations.advance_nautilus_subclass_tick(
                identity.id,
                is_dashing,
                in_water,
                transform.delta_movement,
            );
            if triggers.nautilus_clear_dash {
                upsert_metadata_value(
                    metadata,
                    ProtocolEntityDataValue {
                        data_id: VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                        value: EntityDataValueKind::Boolean(false),
                    },
                );
            }
            (identity.clone(), *transform, triggers)
        };
        append_nautilus_tick_triggers(
            &mut self.pending_entity_client_movement_effects,
            &identity,
            &transform,
            &triggers,
        );
        Some(())
    }

    pub(crate) fn start_local_camel_dash_after_tick(&mut self, id: i32) -> Option<()> {
        self.with_client_animations_mut(id, |animations| {
            animations.start_local_camel_dash_after_tick();
        })
    }

    pub(crate) fn stop_local_camel_dash(&mut self, id: i32) -> Option<()> {
        self.with_client_animations_mut(id, |animations| {
            animations.stop_local_camel_dash();
        })
    }

    /// Advances the shared client-local `AbstractHorse.random` cursor for
    /// `handleEating`'s three `getRandom*` calls and resolves the resulting
    /// happy-villager particle position against the entity's current scaled
    /// dimensions.
    pub(crate) fn next_local_equine_feeding_particle_position(
        &mut self,
        id: i32,
    ) -> Option<super::super::EntityVec3> {
        let entity_type_id = self.identity(id)?.entity_type_id;
        vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)?;
        let position = self.record(id)?.transform.position;
        let bounds = self.pick_bounds(id)?;
        let width = f64::from(bounds.max[0] - bounds.min[0]);
        let height = f64::from(bounds.max[1] - bounds.min[1]);
        let units = self.with_client_animations_mut(id, |animations| {
            animations.next_equine_feeding_particle_units(id)
        })?;
        Some(super::super::EntityVec3 {
            x: position.x + width * (2.0 * units[0] - 1.0),
            y: position.y + height * units[1] + 0.5,
            z: position.z + width * (2.0 * units[2] - 1.0),
        })
    }

    pub(crate) fn with_ridden_jump_state_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut RiddenJumpState) -> R,
    ) -> Option<R> {
        self.with_client_animations_mut(id, |animations| {
            animations.ridden_jump.as_mut().map(update)
        })?
    }
}
