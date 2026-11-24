use super::*;
use crate::entities::entity_data::{
    entity_data_bool, entity_data_byte, entity_data_float, entity_data_humanoid_arm,
    entity_data_int, entity_data_long,
};

impl EntityStore {
    pub(crate) fn saddle_slot_contains_saddle_item(
        &self,
        id: i32,
        profiles: &crate::ItemProfiles,
    ) -> bool {
        let Some(entity) = self.record(id) else {
            return false;
        };
        let equipment = &entity.equipment;
        equipment_contains_saddle_item(Some(equipment), profiles)
    }

    pub(crate) fn equipment_slot_contains_nonempty_item(
        &self,
        id: i32,
        slot: ProtocolEquipmentSlot,
    ) -> bool {
        let Some(entity) = self.record(id) else {
            return false;
        };
        let equipment = &entity.equipment;
        equipment.equipment.iter().any(|update| {
            update.slot == slot && update.item.count > 0 && update.item.item_id.is_some()
        })
    }

    pub(crate) fn first_entity_id_with_type_and_data(
        &self,
        entity_type_id: i32,
        data: i32,
    ) -> Option<i32> {
        for entity in self.records() {
            let identity = &entity.identity;
            if identity.entity_type_id == entity_type_id && identity.data == data {
                return Some(identity.id);
            }
        }
        None
    }

    pub(crate) fn identity(&self, id: i32) -> Option<EntityIdentity> {
        self.record(id).map(|entity| entity.identity.clone())
    }

    /// Returns the hand actively using an item according to `LivingEntity`'s
    /// `DATA_LIVING_ENTITY_FLAGS` metadata (`LivingEntity.isUsingItem` / `getUsedItemHand`).
    pub(crate) fn active_use_item_hand(
        &self,
        id: i32,
    ) -> Option<bbb_protocol::packets::InteractionHand> {
        let entity = self.record(id)?;
        let accessors = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            entity.identity.entity_type_id,
        )?;
        let flags = entity_data_byte(
            &entity.metadata.data_values,
            accessors.flags_data_id,
            accessors.flags_default,
        );
        if flags & accessors.is_using_mask == 0 {
            return None;
        }
        Some(if flags & accessors.off_hand_mask != 0 {
            bbb_protocol::packets::InteractionHand::OffHand
        } else {
            bbb_protocol::packets::InteractionHand::MainHand
        })
    }

    pub(crate) fn take_local_discards(&mut self) -> Vec<i32> {
        std::mem::take(&mut self.pending_local_discards)
    }

    pub(crate) fn main_arm_left(&self, id: i32) -> Option<bool> {
        let entity_type_id = self.entity_type_id(id)?;
        Some(self.entity_main_arm_left(id, entity_type_id))
    }

    pub(crate) fn guardian_attack_sound_state(
        &self,
        id: i32,
    ) -> Option<(super::super::EntityVec3, f32)> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        if !vanilla_entity_metadata_type_is_guardian(identity.entity_type_id) {
            return None;
        }
        let transform = &entity.transform;
        let animations = &entity.animations;
        let attack = animations
            .guardian_kind()
            .and_then(|k| k.attack)
            .unwrap_or_default();
        let duration = guardian_attack_duration(identity.entity_type_id);
        Some((transform.position, attack.attack_scale(0.0, duration)))
    }

    pub(crate) fn sniffer_can_play_digging_sound(&self, id: i32) -> Option<bool> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let metadata = &entity.metadata;
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

    pub(super) fn advance_villager_unhappy_counter(&mut self, id: i32) {
        let Some(entity) = self.record_mut(id) else {
            return;
        };
        if entity.identity.entity_type_id != VANILLA_ENTITY_TYPE_VILLAGER_ID {
            return;
        }
        let Some((data_id, default)) =
            vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(
                entity.identity.entity_type_id,
            )
        else {
            return;
        };
        let counter = entity_data_int(&entity.metadata.data_values, data_id, default);
        if counter > 0 {
            upsert_metadata_value(
                &mut entity.metadata,
                ProtocolEntityDataValue {
                    data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                    value: EntityDataValueKind::Int(counter - 1),
                },
            );
        }
    }

    pub(crate) fn living_entity_dead(&self, id: i32) -> Option<bool> {
        Some(self.record(id)?.family.living()?.dead)
    }

    pub(crate) fn set_allay_liked_player(&mut self, id: i32, player: Option<Uuid>) -> bool {
        let Some(entity) = self.record_mut(id) else {
            return false;
        };
        if entity.identity.entity_type_id != VANILLA_ENTITY_TYPE_ALLAY_ID {
            return false;
        }
        let Some(living) = entity.family.living_mut() else {
            return false;
        };
        living.brain.liked_player = player;
        true
    }

    pub(crate) fn player_sleep_counter(&self, id: i32) -> Option<i32> {
        self.record(id)?.family.player_sleep_counter()
    }

    pub(crate) fn is_baby(&self, id: i32) -> Option<bool> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let metadata = &entity.metadata;
        Some(vanilla_is_baby(
            identity.entity_type_id,
            &metadata.data_values,
        ))
    }

    pub(crate) fn on_ground(&self, id: i32) -> Option<Option<bool>> {
        let entity = self.record(id)?;
        let transform = &entity.transform;
        Some(transform.on_ground)
    }

    pub(crate) fn age_lock_particle_timer_ticks(&self, id: i32) -> Option<u8> {
        let entity = self.record(id)?;
        let animations = &entity.animations;
        Some(animations.age_lock_particle_timer_ticks())
    }

    /// Applies the client half of `AgeableMob.setAgeLocked`: toggles the generated synchronized
    /// boolean immediately and starts the local 40-tick particle/interaction timer. The next
    /// server metadata update remains authoritative and replaces the local boolean.
    pub(crate) fn apply_local_age_lock_use(&mut self, id: i32) -> Option<()> {
        let entity_type_id = self.entity_type_id(id)?;
        let (data_id, default) =
            vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)?;
        let age_locked = self.metadata_bool(id, data_id, default)?;
        let entity = self.record_mut(id)?;
        let animations = &mut entity.animations;
        upsert_metadata_value(
            &mut entity.metadata,
            ProtocolEntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(!age_locked),
            },
        );
        animations.start_age_lock_particle_timer();
        Some(())
    }

    pub(crate) fn toggle_local_ordered_to_sit(&mut self, id: i32) -> Option<bool> {
        self.with_client_animations_mut(id, |animations| {
            animations.ordered_to_sit = !animations.ordered_to_sit;
            animations.ordered_to_sit
        })
    }

    #[cfg(test)]
    pub(crate) fn local_ordered_to_sit(&self, id: i32) -> Option<bool> {
        let entity = self.record(id)?;
        let animations = &entity.animations;
        Some(animations.ordered_to_sit)
    }

    pub(crate) fn local_mooshroom_stew_ingredient(&self, id: i32) -> Option<Option<i32>> {
        let entity = self.record(id)?;
        let animations = &entity.animations;
        Some(animations.mooshroom_stew_ingredient_item_id)
    }

    pub(crate) fn set_local_mooshroom_stew_ingredient(
        &mut self,
        id: i32,
        ingredient_item_id: Option<i32>,
    ) -> Option<()> {
        self.with_client_animations_mut(id, |animations| {
            animations.mooshroom_stew_ingredient_item_id = ingredient_item_id;
        })
    }

    pub(crate) fn local_tadpole_age_ticks(&self, id: i32) -> Option<i32> {
        let entity = self.record(id)?;
        let animations = &entity.animations;
        Some(animations.tadpole_age_ticks)
    }

    pub(crate) fn feed_local_tadpole(&mut self, id: i32) -> Option<()> {
        self.with_client_animations_mut(id, |animations| {
            let remaining = (24_000 - animations.tadpole_age_ticks).max(0);
            let speed_up_seconds = ((remaining / 20) as f32 * 0.1) as i32;
            animations.tadpole_age_ticks = animations
                .tadpole_age_ticks
                .saturating_add(speed_up_seconds.saturating_mul(20));
        })
    }

    pub(crate) fn reset_local_tadpole_age(&mut self, id: i32) -> Option<()> {
        self.with_client_animations_mut(id, |animations| {
            animations.tadpole_age_ticks = 0;
        })
    }

    pub(crate) fn next_local_interaction_random_float(&mut self, id: i32) -> Option<f32> {
        self.with_client_animations_mut(id, |animations| {
            animations.next_local_interaction_random_float(id)
        })
    }

    pub(crate) fn next_local_interaction_random_double(&mut self, id: i32) -> Option<f64> {
        self.with_client_animations_mut(id, |animations| {
            animations.next_local_interaction_random_double(id)
        })
    }

    pub(crate) fn with_entity_random_mut<R>(
        &mut self,
        id: i32,
        update: impl FnOnce(&mut crate::LevelEventSoundRandomState) -> R,
    ) -> Option<R> {
        self.with_client_animations_mut(id, |animations| update(animations.entity_random_mut(id)))
    }

    pub(crate) fn local_interaction_particle_origin(
        &self,
        id: i32,
        y_fraction: f64,
    ) -> Option<super::super::EntityVec3> {
        let entity = self.record(id)?;
        let position = &entity.transform.position;
        let bounds = self.pick_bounds(id)?;
        let height = f64::from(bounds.max[1] - bounds.min[1]);
        Some(super::super::EntityVec3 {
            x: position.x,
            y: position.y + height * y_fraction,
            z: position.z,
        })
    }

    pub(crate) fn local_interaction_random_position(
        &mut self,
        id: i32,
        horizontal_scale: f64,
        y_offset: f64,
    ) -> Option<super::super::EntityVec3> {
        let position = self.record(id)?.transform.position;
        let bounds = self.pick_bounds(id)?;
        let width = f64::from(bounds.max[0] - bounds.min[0]);
        let height = f64::from(bounds.max[1] - bounds.min[1]);
        let x = self.next_local_interaction_random_double(id)?;
        let y = self.next_local_interaction_random_double(id)?;
        let z = self.next_local_interaction_random_double(id)?;
        Some(super::super::EntityVec3 {
            x: position.x + width * horizontal_scale * (2.0 * x - 1.0),
            y: position.y + height * y + y_offset,
            z: position.z + width * horizontal_scale * (2.0 * z - 1.0),
        })
    }

    pub(crate) fn is_fall_flying(&self, id: i32) -> Option<bool> {
        let entity = self.record(id)?;
        let metadata = &entity.metadata;
        Some(entity_is_fall_flying(&metadata.data_values))
    }

    /// Vanilla `LivingEntity.isAutoSpinAttack`: the synced riptide spin bit.
    pub(crate) fn is_auto_spin_attack(&self, id: i32) -> Option<bool> {
        let accessors = super::super::content_facts::entity_living_accessors(
            self.record(id)?.identity.entity_type_id,
            &self.content_registry,
        )?;
        let flags = self.metadata_byte(id, accessors.flags_data_id, accessors.flags_default)?;
        Some(flags & accessors.spin_attack_mask != 0)
    }

    /// Vanilla `LivingEntity.stopFallFlying` mutates shared flag 7 locally on
    /// the client too (the `travelFallFlying` climbable branch); the server
    /// remains authoritative and re-syncs the flag on its own tick.
    pub(crate) fn stop_fall_flying(&mut self, id: i32) -> Option<()> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        let current = self.metadata_byte(
            id,
            accessors.shared_flags_data_id,
            accessors.shared_flags_default,
        )?;
        self.set_local_metadata_byte(
            id,
            accessors.shared_flags_data_id,
            current & !accessors.fall_flying_mask,
        )
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

    /// Vanilla `Player.getScore()`: the synced `DATA_SCORE_ID` int, defaulting
    /// to the define-time `0` (Player.java).
    pub(crate) fn player_score(&self, id: i32) -> Option<i32> {
        let entity_type_id = self.entity_type_id(id)?;
        let (data_id, default) =
            vanilla_entity_metadata_player_score_accessor_for_entity_type(entity_type_id)?;
        self.metadata_int(id, data_id, default)
    }

    pub(crate) fn metadata_int_for_entity(
        &self,
        id: i32,
        data_id: u8,
        default: i32,
    ) -> Option<i32> {
        self.metadata_int(id, data_id, default)
    }

    pub(crate) fn metadata_byte_for_entity(&self, id: i32, data_id: u8, default: i8) -> Option<i8> {
        self.metadata_byte(id, data_id, default)
    }

    pub(crate) fn metadata_bool_for_entity(
        &self,
        id: i32,
        data_id: u8,
        default: bool,
    ) -> Option<bool> {
        self.metadata_bool(id, data_id, default)
    }

    pub(crate) fn set_local_metadata_bool(
        &mut self,
        id: i32,
        data_id: u8,
        value: bool,
    ) -> Option<()> {
        upsert_metadata_value(
            &mut self.record_mut(id)?.metadata,
            ProtocolEntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(value),
            },
        );
        Some(())
    }

    pub(crate) fn set_local_metadata_byte(
        &mut self,
        id: i32,
        data_id: u8,
        value: i8,
    ) -> Option<()> {
        upsert_metadata_value(
            &mut self.record_mut(id)?.metadata,
            ProtocolEntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(value),
            },
        );
        Some(())
    }

    pub(crate) fn set_local_metadata_int(
        &mut self,
        id: i32,
        data_id: u8,
        value: i32,
    ) -> Option<()> {
        upsert_metadata_value(
            &mut self.record_mut(id)?.metadata,
            ProtocolEntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(value),
            },
        );
        Some(())
    }

    pub(crate) fn metadata_optional_living_entity_reference_for_entity(
        &self,
        id: i32,
        data_id: u8,
        default: Option<Uuid>,
    ) -> Option<Option<Uuid>> {
        let entity = self.record(id)?;
        let metadata = &entity.metadata;
        Some(
            metadata
                .data_values
                .iter()
                .find(|value| value.data_id == data_id)
                .and_then(|value| match &value.value {
                    EntityDataValueKind::OptionalLivingEntityReference(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(default),
        )
    }

    pub(crate) fn metadata_registry_reference_for_entity(
        &self,
        id: i32,
        data_id: u8,
        serializer: EntityDataRegistryHolder,
    ) -> Option<Option<EntityDataRegistryReference>> {
        let entity = self.record(id)?;
        let metadata = &entity.metadata;
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

    pub(in crate::entities) fn metadata_byte(
        &self,
        id: i32,
        data_id: u8,
        default: i8,
    ) -> Option<i8> {
        let entity = self.record(id)?;
        Some(entity_data_byte(
            &entity.metadata.data_values,
            data_id,
            default,
        ))
    }

    pub(crate) fn metadata_optional_rich_component_override(
        &self,
        id: i32,
        data_id: u8,
    ) -> Option<Option<bbb_protocol::RichTextComponent>> {
        let entity = self.record(id)?;
        let metadata = &entity.metadata;
        metadata
            .data_values
            .iter()
            .find(|value| value.data_id == data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalComponent(component) => Some(component.clone()),
                _ => None,
            })
    }

    pub(super) fn metadata_optional_component(&self, id: i32, data_id: u8) -> Option<String> {
        self.metadata_optional_rich_component_override(id, data_id)?
            .map(|component| component.text().to_string())
    }

    pub(crate) fn entity_has_passengers(&self, id: i32) -> bool {
        self.record(id)
            .is_some_and(|entity| !entity.mount.passengers.is_empty())
    }

    pub(crate) fn entity_passenger_count(&self, id: i32) -> Option<usize> {
        let entity = self.record(id)?;
        let mount = &entity.mount;
        Some(mount.passengers.len())
    }

    /// Vanilla `LivingEntity.getSleepingPos`: the synced bed position the entity is
    /// sleeping in (`SLEEPING_POS_ID`), or `None` when it is not in a bed.
    pub(crate) fn sleeping_pos(&self, id: i32) -> Option<crate::BlockPos> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let accessors = super::super::content_facts::entity_living_accessors(
            identity.entity_type_id,
            &self.content_registry,
        )?;
        let metadata = &entity.metadata;
        metadata
            .data_values
            .iter()
            .find(|value| value.data_id == accessors.sleeping_pos_data_id)
            .and_then(|value| match &value.value {
                EntityDataValueKind::OptionalBlockPos(Some(pos)) => Some(*pos),
                _ => None,
            })
    }

    /// Vanilla `AvatarRenderer.isEntityUpsideDown` inputs for a player entity: its
    /// profile UUID (to resolve the GameProfile name from the player-info list) and
    /// whether the `CAPE` model part is shown (`DATA_PLAYER_MODE_CUSTOMISATION & 1`).
    /// `None` for non-player entities, which never use the avatar upside-down path.
    pub(crate) fn avatar_upside_down_inputs(&self, id: i32) -> Option<(Uuid, bool)> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
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

    /// Vanilla `Entity.getEyeHeight(Pose.STANDING)` used by the sleeping bed
    /// head-offset translate: the eye height resolved with the synced pose stripped
    /// so the dimensions fall back to standing rather than `SLEEPING_DIMENSIONS`.
    pub(crate) fn standing_eye_height(&self, id: i32) -> Option<f32> {
        let accessors = vanilla_entity_metadata_entity_accessors()?;
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let metadata = &entity.metadata;
        let attributes = &entity.attributes;
        let client_animations = &entity.animations;
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
            Some(*client_animations),
            metadata.painting_variant_projection.as_ref(),
        )
    }

    /// Vanilla `Entity.getEyeHeight()` at the entity's currently synced pose.
    pub(crate) fn pose_eye_height(&self, id: i32) -> Option<f32> {
        let entity = self.record(id)?;
        let identity = &entity.identity;
        let metadata = &entity.metadata;
        let attributes = &entity.attributes;
        let client_animations = &entity.animations;
        super::super::content_facts::eye_height_for_entity_data(
            identity.entity_type_id,
            identity.data,
            &metadata.data_values,
            &attributes.attributes,
            Some(*client_animations),
            metadata.painting_variant_projection.as_ref(),
            &self.content_registry,
        )
    }

    pub(super) fn metadata_bool(&self, id: i32, data_id: u8, default: bool) -> Option<bool> {
        let entity = self.record(id)?;
        Some(entity_data_bool(
            &entity.metadata.data_values,
            data_id,
            default,
        ))
    }

    pub(super) fn metadata_int(&self, id: i32, data_id: u8, default: i32) -> Option<i32> {
        let entity = self.record(id)?;
        Some(entity_data_int(
            &entity.metadata.data_values,
            data_id,
            default,
        ))
    }

    pub(in crate::entities) fn metadata_long(
        &self,
        id: i32,
        data_id: u8,
        default: i64,
    ) -> Option<i64> {
        let entity = self.record(id)?;
        Some(entity_data_long(
            &entity.metadata.data_values,
            data_id,
            default,
        ))
    }

    pub(super) fn metadata_float(&self, id: i32, data_id: u8, default: f32) -> Option<f32> {
        let entity = self.record(id)?;
        Some(entity_data_float(
            &entity.metadata.data_values,
            data_id,
            default,
        ))
    }

    pub(super) fn metadata_humanoid_arm(&self, id: i32, data_id: u8, default: i32) -> Option<i32> {
        let entity = self.record(id)?;
        Some(entity_data_humanoid_arm(
            &entity.metadata.data_values,
            data_id,
            default,
        ))
    }

    pub(super) fn metadata_optional_unsigned_int(
        &self,
        id: i32,
        data_id: u8,
    ) -> Option<Option<i32>> {
        let entity = self.record(id)?;
        let metadata = &entity.metadata;
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
    pub(super) fn metadata_optional_block_state(
        &self,
        id: i32,
        data_id: u8,
    ) -> Option<Option<i32>> {
        let entity = self.record(id)?;
        let metadata = &entity.metadata;
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

    pub(super) fn metadata_block_state(&self, id: i32, data_id: u8) -> Option<Option<i32>> {
        let entity = self.record(id)?;
        let metadata = &entity.metadata;
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
}
