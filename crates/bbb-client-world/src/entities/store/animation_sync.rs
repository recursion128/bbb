use super::*;

impl EntityStore {
    pub(crate) fn sync_client_animation_targets_from_metadata(&mut self, id: i32) -> Option<()> {
        let content_registry = Arc::clone(&self.content_registry);
        let entity = self.record_mut(id)?;
        entity.animations.sync_targets_from_metadata_with_registry(
            entity.identity.entity_type_id,
            &entity.metadata.data_values,
            &content_registry,
        );
        Some(())
    }

    pub(crate) fn sync_client_animation_events_from_metadata_update(
        &mut self,
        id: i32,
        updated_values: &[bbb_protocol::packets::EntityDataValue],
    ) -> Option<()> {
        let mut enderman_stare_pos = None;
        {
            let entity = self.record_mut(id)?;
            let entity_type_id = entity.identity.entity_type_id;
            entity.animations.sync_events_from_metadata_update(
                entity_type_id,
                updated_values,
                &entity.metadata.data_values,
            );
            // Vanilla `EnderMan.onSyncedDataUpdated(DATA_CREEPY)` (`EnderMan.java:173-189`):
            // plays the local stare sound when `DATA_STARED_AT` is true, with a 400-tick cooldown.
            if entity_type_id == VANILLA_ENTITY_TYPE_ENDERMAN_ID
                && updated_values
                    .iter()
                    .any(|value| value.data_id == VANILLA_ENDERMAN_CREEPY_DATA_ID)
                && entity_data_bool(
                    &entity.metadata.data_values,
                    VANILLA_ENDERMAN_STARED_AT_DATA_ID,
                    VANILLA_ENDERMAN_DEFAULT_STARED_AT,
                )
            {
                let is_silent = vanilla_entity_metadata_entity_accessors().is_some_and(|acc| {
                    entity_data_bool(
                        &entity.metadata.data_values,
                        acc.silent_data_id,
                        acc.silent_default,
                    )
                });
                if entity.animations.try_trigger_enderman_stare_sound() && !is_silent {
                    let eye_height = vanilla_eye_height_for_entity_data(
                        entity_type_id,
                        entity.identity.data,
                        &entity.metadata.data_values,
                        &entity.attributes.attributes,
                        Some(entity.animations),
                        entity.metadata.painting_variant_projection.as_ref(),
                    )
                    .unwrap_or(2.55);
                    enderman_stare_pos = Some(super::super::EntityVec3 {
                        x: entity.transform.position.x,
                        y: entity.transform.position.y + f64::from(eye_height),
                        z: entity.transform.position.z,
                    });
                }
            }
        }
        if let Some(position) = enderman_stare_pos {
            self.pending_entity_client_movement_effects.push(
                EntityClientMovementEffectState::Sound(
                    super::super::EntityClientMovementSoundState {
                        entity_id: id,
                        kind: super::super::EntityClientMovementSoundKind::EndermanStare,
                        position,
                    },
                ),
            );
        }
        Some(())
    }

    /// Applies `FishingHook.onSyncedDataUpdated` in packet field order. The
    /// hooked target is captured by UUID so a later protocol-id reuse cannot
    /// retarget an old hook instance, while the biting impulse consumes the
    /// hook's retained synchronized RNG cursor immediately like Vanilla.
    pub(crate) fn sync_fishing_hook_from_metadata_update(
        &mut self,
        id: i32,
        updated_values: &[bbb_protocol::packets::EntityDataValue],
    ) -> Option<()> {
        self.record(id)?.family.fishing_hook()?;
        for value in updated_values {
            if value.data_id == VANILLA_FISHING_HOOK_HOOKED_ENTITY_DATA_ID {
                let EntityDataValueKind::Int(target_data) = &value.value else {
                    continue;
                };
                let hooked_entity = target_data
                    .checked_sub(1)
                    .and_then(|target_id| self.record(target_id))
                    .map(|target| target.identity.uuid);
                let movement = self.record_mut(id)?.family.fishing_hook_mut()?;
                movement.hooked_entity_uuid = hooked_entity;
            } else if value.data_id == VANILLA_FISHING_HOOK_BITING_DATA_ID {
                let EntityDataValueKind::Boolean(biting) = &value.value else {
                    continue;
                };
                let entity = self.record_mut(id)?;
                let movement = entity.family.fishing_hook_mut()?;
                movement.biting = *biting;
                if *biting {
                    let random =
                        super::super::animations::java_random_next_float(&mut movement.random_seed);
                    let downward = -0.4_f32 * (0.6_f32 + random * (1.0_f32 - 0.6_f32));
                    entity.transform.delta_movement.y = f64::from(downward);
                }
            }
        }
        Some(())
    }

    pub(crate) fn apply_client_animation_entity_event(
        &mut self,
        id: i32,
        event_id: i8,
    ) -> Option<()> {
        let content_registry = Arc::clone(&self.content_registry);
        let entity = self.record_mut(id)?;
        let entity_type_id = entity.identity.entity_type_id;
        let event_kind = super::super::events::entity_event_kind_with_registry(
            entity_type_id,
            event_id,
            &content_registry,
        )?;
        entity
            .animations
            .handle_entity_event(entity_type_id, id, event_kind);
        Some(())
    }

    #[cfg(test)]
    pub(crate) fn minecart_spawner_spawn_delay(&self, id: i32) -> Option<i32> {
        let entity = self.record(id)?;
        (entity.identity.entity_type_id == VANILLA_ENTITY_TYPE_SPAWNER_MINECART_ID)
            .then(|| entity.animations.minecart_spawner_spawn_delay())
    }

    /// Applies the canonical state mutation from `LivingEntity.handleEntityEvent(3)`.
    ///
    /// Vanilla plays the death sound for Players too, but deliberately skips `setHealth(0)` and
    /// `die(generic)` for every `Player` instance. The caller owns the sound side effect; this
    /// method owns only the non-Player health/death-state transition.
    pub(crate) fn apply_client_living_death_entity_event(
        &mut self,
        id: i32,
        wake: Option<LivingDeathWakeTransition>,
        game_time: i64,
    ) -> Option<()> {
        let content_registry = Arc::clone(&self.content_registry);
        let entity = self.record_mut(id)?;
        let entity_type_id = entity.identity.entity_type_id;
        let accessors = super::super::content_facts::entity_living_accessors(
            entity_type_id,
            &content_registry,
        )?;
        if vanilla_entity_metadata_type_uses_player_entity_event_handler(entity_type_id) {
            return Some(());
        }

        upsert_metadata_value(
            &mut entity.metadata,
            ProtocolEntityDataValue {
                data_id: accessors.health_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(0.0),
            },
        );

        // These virtual overrides execute before `LivingEntity.die`, so they
        // run on repeated event-3 calls even after the base `dead` guard closes.
        if entity_type_id == VANILLA_ENTITY_TYPE_WOLF_ID {
            entity.animations.clear_wolf_wet_on_death();
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_SNIFFER_ID {
            let (state_data_id, _) =
                vanilla_entity_metadata_sniffer_state_accessor_for_entity_type(entity_type_id)?;
            upsert_metadata_value(
                &mut entity.metadata,
                ProtocolEntityDataValue {
                    data_id: state_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_SNIFFER_STATE_ID,
                    value: EntityDataValueKind::EnumId {
                        serializer: EntityDataEnumSerializer::SnifferState,
                        id: VANILLA_SNIFFER_STATE_IDLING_ID,
                    },
                },
            );
        }

        if !entity.family.living()?.dead {
            let sleeping = entity.metadata.data_values.iter().any(|value| {
                value.data_id == accessors.sleeping_pos_data_id
                    && matches!(value.value, EntityDataValueKind::OptionalBlockPos(Some(_)))
            });
            if sleeping {
                if let Some(wake) = wake {
                    let transform = &mut entity.transform;
                    transform.position = wake.position;
                    transform.set_y_rot(wake.y_rot);
                    transform.set_x_rot(0.0);
                }
                upsert_metadata_value(
                    &mut entity.metadata,
                    ProtocolEntityDataValue {
                        data_id: accessors.sleeping_pos_data_id,
                        serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
                        value: EntityDataValueKind::OptionalBlockPos(None),
                    },
                );
                set_villager_last_woken(entity, game_time);
            }

            // `stopSleeping` temporarily writes STANDING. The immediately
            // following base `die` write is the only retained packet-boundary
            // state, so publish the final DYING pose directly.
            let entity_accessors = vanilla_entity_metadata_entity_accessors()?;
            upsert_metadata_value(
                &mut entity.metadata,
                ProtocolEntityDataValue {
                    data_id: entity_accessors.pose_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
                    value: EntityDataValueKind::Pose(VANILLA_ENTITY_POSE_DYING_ID),
                },
            );

            // The client side of `stopUsingItem` clears only unsynchronized
            // use-item payload/ticks. bbb has no parallel remote payload owner;
            // the synchronized using flag remains server-owned, as in Vanilla.
            entity.family.living_mut()?.dead = true;
        }

        entity.animations.sync_targets_from_metadata_with_registry(
            entity_type_id,
            &entity.metadata.data_values,
            &content_registry,
        );
        Some(())
    }

    pub(super) fn advance_player_sleep_counter(&mut self, id: i32) {
        let Some(entity) = self.record_mut(id) else {
            return;
        };
        let Some(counter) = entity.family.player_sleep_counter_mut() else {
            return;
        };
        let Some(accessors) = vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_PLAYER_ID,
        ) else {
            return;
        };
        let sleeping = entity.metadata.data_values.iter().any(|value| {
            value.data_id == accessors.sleeping_pos_data_id
                && matches!(value.value, EntityDataValueKind::OptionalBlockPos(Some(_)))
        });
        if sleeping {
            *counter = counter.saturating_add(1).min(100);
        } else if *counter > 0 {
            *counter = counter.saturating_add(1);
            if *counter >= 110 {
                *counter = 0;
            }
        }
    }

    /// Arms the melee swing for `id` (vanilla `LivingEntity.swing`), triggered by the
    /// `ClientboundAnimate` packet's main-hand (`off_hand = false`) / off-hand actions.
    pub(crate) fn trigger_client_animation_swing(
        &mut self,
        id: i32,
        off_hand: bool,
        duration: i32,
    ) -> Option<()> {
        self.record_mut(id)?
            .animations
            .trigger_swing(off_hand, duration);
        Some(())
    }

    pub(crate) fn active_swing_off_hand(&self, id: i32) -> Option<bool> {
        self.record(id)?.animations.active_swing_off_hand()
    }

    pub(crate) fn attack_swing_state(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<LocalPlayerAttackSwingState> {
        let animations = &self.record(id)?.animations;
        Some(LocalPlayerAttackSwingState {
            attack_anim: animations.attack_anim(partial_ticks),
            off_hand: animations.attack_arm_off_hand(),
        })
    }

    pub(crate) fn ticks_since_kinetic_hit_feedback(
        &self,
        id: i32,
        partial_ticks: f32,
    ) -> Option<f32> {
        Some(
            self.record(id)?
                .animations
                .ticks_since_kinetic_hit_feedback(partial_ticks),
        )
    }

    /// Vanilla `LivingEntity.onKineticHit` (`LivingEntity.java:2125`): accepts a new
    /// kinetic hit sound only when more than 10 ticks elapsed since the previous one.
    pub(crate) fn can_trigger_kinetic_hit_feedback(&self, id: i32) -> bool {
        self.record(id)
            .is_some_and(|entity| entity.animations.can_trigger_kinetic_hit_feedback())
    }

    pub(crate) fn refresh_client_animation_swing_duration(
        &mut self,
        id: i32,
        duration: i32,
    ) -> Option<()> {
        self.record_mut(id)?
            .animations
            .refresh_swing_duration(duration);
        Some(())
    }
}

/// `Villager.stopSleeping` records `LAST_WOKEN` in the client brain.
pub(super) fn set_villager_last_woken(entity: &mut EntityRecord, game_time: i64) {
    if entity.identity.entity_type_id != VANILLA_ENTITY_TYPE_VILLAGER_ID {
        return;
    }
    if let Some(living) = entity.family.living_mut() {
        living.brain.last_woken = Some(game_time);
    }
}
