use super::*;

impl WorldStore {
    pub(super) fn predict_local_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        interact_owner: VanillaEntityOwner,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        let Some(health) = self.entities.living_entity_health(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if health <= 0.0 || health.is_nan() {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }

        let held_item_id = self
            .local_item_in_hand(hand)
            .and_then(|stack| stack.item_id);
        if let Some(item_id) = held_item_id {
            let Some(item_type) = vanilla_item_type(item_id) else {
                return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
            };
            if item_type.interact_living_entity_override
                == Some(VanillaItemInteractLivingEntityOverride::NameTagItem)
            {
                let result = self.predict_local_item_stack_interact_living_entity(
                    entity_id,
                    entity_type_id,
                    self.local_item_in_hand(hand)
                        .expect("held item id came from this stack"),
                );
                if result != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
                    return result;
                }
            }
            if item_type.use_override == Some(VanillaItemUseOverride::SpawnEggItem) {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::server_item_interaction(),
                ));
            }
        }

        let base_result = self.predict_local_base_entity_interact(
            entity_id,
            entity_type_id,
            held_item_id,
            secondary_use_active,
        );
        if base_result != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
            return base_result;
        }

        let Some(owner) = vanilla_entity_metadata_mob_interact_owner(entity_type_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if interact_owner == VANILLA_CAMEL_HUSK_INTERACT_OWNER
            && owner == VANILLA_CAMEL_MOB_INTERACT_OWNER
        {
            return self.predict_local_camel_generated_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                secondary_use_active,
                true,
            );
        }
        if interact_owner == VANILLA_ZOMBIE_HORSE_INTERACT_OWNER
            && owner == VANILLA_ZOMBIE_HORSE_INTERACT_OWNER
        {
            return self.predict_local_zombie_horse_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                secondary_use_active,
            );
        }
        if interact_owner == VANILLA_NAUTILUS_INTERACT_OWNER
            && owner == VANILLA_NAUTILUS_INTERACT_OWNER
        {
            return self.predict_local_nautilus_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                secondary_use_active,
            );
        }
        if held_item_id == Some(VANILLA_ITEM_SHEARS_ID) {
            if let Some(result) = self.predict_local_specialized_shearable_mob_interact(
                entity_id,
                entity_type_id,
                owner,
            ) {
                return result;
            }
        }
        if let Some(result) = self.predict_local_specialized_generated_mob_interact(
            entity_id,
            entity_type_id,
            hand,
            held_item_id,
            owner,
            secondary_use_active,
        ) {
            return result;
        }
        if owner == VANILLA_ANIMAL_MOB_INTERACT_OWNER {
            return self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                owner,
            );
        }
        if owner == VANILLA_AGEABLE_MOB_INTERACT_OWNER {
            return self.predict_local_ageable_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                owner,
            );
        }
        if owner != VANILLA_MOB_ENTITY_INTERACT_OWNER {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    }

    pub(super) fn predict_local_animal_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        owner: VanillaEntityOwner,
    ) -> LocalTargetUsePrediction {
        self.predict_local_animal_mob_interact_with_can_age_up(
            entity_id,
            entity_type_id,
            hand,
            held_item_id,
            owner,
            true,
        )
    }

    /// Mirrors the virtual `Mob.usePlayerItem` call used by animal feeding.
    ///
    /// Axolotl and Nautilus override the method for bucket food; every other
    /// mob inherits the base `USE_REMAINDER` conversion.
    pub(super) fn use_local_mob_player_item(
        &mut self,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        owner: VanillaEntityOwner,
    ) -> Result<bool, Box<LocalTargetUsePrediction>> {
        let unsupported = || Box::new(LocalTargetUsePrediction::UnsupportedEntityInteract(owner));
        let water_bucket = || ProtocolItemStackSummary {
            item_id: Some(VANILLA_ITEM_WATER_BUCKET_ID),
            count: 1,
            component_patch: ProtocolDataComponentPatchSummary::default().into(),
        };
        if entity_type_id == VANILLA_ENTITY_TYPE_AXOLOTL_ID
            && held_item_id == Some(VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID)
        {
            return self
                .replace_local_item_with_filled_result(
                    CreativeFilledResult::AddIfMissing,
                    hand,
                    VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID,
                    water_bucket(),
                )
                .ok_or_else(unsupported);
        }
        if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_NAUTILUS_ID | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
        ) {
            let Some(bucket_food_ids) = self
                .registry_holder_ids_for_tag("minecraft:item", VANILLA_NAUTILUS_BUCKET_FOOD_TAG)
            else {
                return Err(unsupported());
            };
            if held_item_id.is_some_and(|item_id| bucket_food_ids.contains(&item_id)) {
                let expected_item_id = held_item_id.expect("matched a present held item id");
                return self
                    .replace_local_item_with_filled_result(
                        CreativeFilledResult::AddIfMissing,
                        hand,
                        expected_item_id,
                        water_bucket(),
                    )
                    .ok_or_else(unsupported);
            }
        }

        let Some(stack) = self.local_item_in_hand(hand) else {
            return Err(unsupported());
        };
        let remainder = if self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID)
        {
            let patch = self.items.component_source(
                stack.item_id,
                &stack.component_patch,
                VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
            );
            let Some(template) = patch.use_remainder.as_deref() else {
                return Err(unsupported());
            };
            Some(ProtocolItemStackSummary {
                item_id: Some(template.item_id),
                count: template.count,
                component_patch: template.component_patch.clone(),
            })
        } else {
            None
        };
        self.consume_local_mob_player_item(hand, remainder)
            .then_some(false)
            .ok_or_else(unsupported)
    }

    pub(super) fn predict_local_animal_mob_interact_with_can_age_up(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        owner: VanillaEntityOwner,
        can_age_up: bool,
    ) -> LocalTargetUsePrediction {
        let is_food = match vanilla_entity_metadata_food_predicate(entity_type_id) {
            Some(VanillaEntityFoodPredicate::Never) => false,
            None => return LocalTargetUsePrediction::UnsupportedEntityInteract(owner),
            Some(VanillaEntityFoodPredicate::ItemTag(tag)) => {
                let Some(item_ids) = self.registry_holder_ids_for_tag("minecraft:item", tag) else {
                    return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
                };
                held_item_id.is_some_and(|item_id| item_ids.contains(&item_id))
            }
            Some(VanillaEntityFoodPredicate::Nautilus {
                untamed_adult_tag,
                default_tag,
            }) => {
                let Some(is_baby) = self.entities.is_baby(entity_id) else {
                    return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                };
                let Some(accessors) =
                    vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
                        entity_type_id,
                    )
                else {
                    return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                };
                let Some(flags) = self.entities.metadata_byte_for_entity(
                    entity_id,
                    accessors.flags_data_id,
                    accessors.flags_default,
                ) else {
                    return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                };
                let tag = if !is_baby && flags & accessors.tame_mask == 0 {
                    untamed_adult_tag
                } else {
                    default_tag
                };
                let Some(item_ids) = self.registry_holder_ids_for_tag("minecraft:item", tag) else {
                    return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
                };
                held_item_id.is_some_and(|item_id| item_ids.contains(&item_id))
            }
        };
        if !is_food {
            return self.predict_local_ageable_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                owner,
            );
        }

        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let can_age_up = if is_baby && can_age_up {
            let Some((data_id, default)) =
                vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)
            else {
                return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
            };
            let Some(age_locked) = self
                .entities
                .metadata_bool_for_entity(entity_id, data_id, default)
            else {
                return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
            };
            !age_locked
        } else {
            false
        };
        if !can_age_up {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false),
            ));
        }
        let dropped =
            match self.use_local_mob_player_item(entity_type_id, hand, held_item_id, owner) {
                Ok(dropped) => dropped,
                Err(prediction) => return *prediction,
            };
        let mut success = LocalInteractionSuccess::client_item_interaction();
        success.extra_swing = dropped.then_some(InteractionHand::MainHand);
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(success))
    }

    pub(super) fn predict_local_ageable_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        owner: VanillaEntityOwner,
    ) -> LocalTargetUsePrediction {
        if held_item_id != Some(VANILLA_ITEM_GOLDEN_DANDELION_ID) {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(timer_ticks) = self.entities.age_lock_particle_timer_ticks(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if !is_baby || timer_ticks != 0 {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        let Some(excluded_entity_type_ids) = self
            .registry_holder_ids_for_tag("minecraft:entity_type", VANILLA_CANNOT_BE_AGE_LOCKED_TAG)
        else {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
        };
        if excluded_entity_type_ids.contains(&entity_type_id) {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        if self.entities.apply_local_age_lock_use(entity_id).is_none()
            || !self.consume_local_item_in_hand(hand, 1)
        {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ))
    }

    pub(super) fn predict_local_specialized_shearable_mob_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        owner: VanillaEntityOwner,
    ) -> Option<LocalTargetUsePrediction> {
        let client_success = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        };
        let pass = || LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        Some(match owner {
            VANILLA_BOGGED_MOB_INTERACT_OWNER => {
                let Some((data_id, default)) =
                    vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(entity_type_id)
                else {
                    return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                        entity_type_id,
                    )));
                };
                let Some(sheared) = self
                    .entities
                    .metadata_bool_for_entity(entity_id, data_id, default)
                else {
                    return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                        entity_type_id,
                    )));
                };
                if sheared {
                    pass()
                } else {
                    client_success()
                }
            }
            VANILLA_COPPER_GOLEM_MOB_INTERACT_OWNER => {
                let Some(antenna) = self
                    .entities
                    .equipment_item(entity_id, EquipmentSlot::Saddle)
                else {
                    return Some(pass());
                };
                let Some(antenna_item_id) = antenna.item_id else {
                    return Some(pass());
                };
                let Some(item_ids) = self.registry_holder_ids_for_tag(
                    "minecraft:item",
                    VANILLA_COPPER_GOLEM_SHEARABLE_ITEMS_TAG,
                ) else {
                    return Some(LocalTargetUsePrediction::UnsupportedEntityInteract(owner));
                };
                if item_ids.contains(&antenna_item_id) {
                    client_success()
                } else {
                    pass()
                }
            }
            VANILLA_MOOSHROOM_MOB_INTERACT_OWNER => {
                let Some(is_baby) = self.entities.is_baby(entity_id) else {
                    return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                        entity_type_id,
                    )));
                };
                if is_baby {
                    return None;
                } else {
                    client_success()
                }
            }
            VANILLA_SHEEP_MOB_INTERACT_OWNER => LocalTargetUsePrediction::Result(
                LocalInteractionResult::Success(LocalInteractionSuccess::consume(false)),
            ),
            VANILLA_SNOW_GOLEM_MOB_INTERACT_OWNER => {
                let Some((data_id, default, pumpkin_mask)) =
                    vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
                        entity_type_id,
                    )
                else {
                    return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                        entity_type_id,
                    )));
                };
                let Some(flags) = self
                    .entities
                    .metadata_byte_for_entity(entity_id, data_id, default)
                else {
                    return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                        entity_type_id,
                    )));
                };
                if flags & pumpkin_mask != 0 {
                    client_success()
                } else {
                    pass()
                }
            }
            _ => return None,
        })
    }

    pub(super) fn predict_local_zombie_horse_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(accessors) =
            vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)
        else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(flags) = self.entities.metadata_byte_for_entity(
            entity_id,
            accessors.flags_data_id,
            accessors.flags_default,
        ) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let is_tamed = flags & accessors.tame_mask != 0;
        let is_vehicle = self.entities.entity_has_passengers(entity_id);
        let should_open_inventory = !is_baby && is_tamed && secondary_use_active;

        if !is_vehicle && !should_open_inventory {
            if let Some(item_id) = held_item_id {
                let Some(food_item_ids) = self
                    .registry_holder_ids_for_tag("minecraft:item", VANILLA_ZOMBIE_HORSE_FOOD_TAG)
                else {
                    return LocalTargetUsePrediction::UnsupportedEntityInteract(
                        VANILLA_ZOMBIE_HORSE_INTERACT_OWNER,
                    );
                };
                if food_item_ids.contains(&item_id) {
                    return self.predict_local_abstract_horse_fed_food(
                        entity_id,
                        entity_type_id,
                        hand,
                        item_id,
                        VANILLA_ZOMBIE_HORSE_INTERACT_OWNER,
                    );
                }
                if !is_tamed {
                    // `makeMad` is server-only; the client still returns
                    // `InteractionResult.SUCCESS`.
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                        LocalInteractionSuccess::client_item_interaction(),
                    ));
                }
            }
        }

        self.predict_local_abstract_horse_mob_interact(
            entity_id,
            entity_type_id,
            hand,
            held_item_id,
            secondary_use_active,
            VANILLA_ZOMBIE_HORSE_INTERACT_OWNER,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn predict_local_abstract_horse_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
        owner: VanillaEntityOwner,
        can_age_up: bool,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let is_vehicle = self.entities.entity_has_passengers(entity_id);
        if is_vehicle || is_baby {
            return self.predict_local_animal_mob_interact_with_can_age_up(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                owner,
                can_age_up,
            );
        }

        let Some(accessors) =
            vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)
        else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(flags) = self.entities.metadata_byte_for_entity(
            entity_id,
            accessors.flags_data_id,
            accessors.flags_default,
        ) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if flags & accessors.tame_mask != 0 && secondary_use_active {
            // `openCustomInventoryScreen` is server-only, but the method
            // returns client `SUCCESS` before consulting the held stack.
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ));
        }

        if let Some(item_id) = held_item_id {
            let Some(stack) = self.local_item_in_hand(hand) else {
                return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
            };
            let item_interaction = self.predict_local_item_stack_interact_living_entity(
                entity_id,
                entity_type_id,
                stack,
            );
            if item_interaction != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
                return item_interaction;
            }

            let equippable = match self
                .local_item_stack_is_equippable_in_horse_body_slot(entity_type_id, stack)
            {
                Ok(equippable) => equippable,
                Err(prediction) => return *prediction,
            };
            if equippable {
                let wearing_body_armor = if let Some(body_item) =
                    self.entities.equipment_item(entity_id, EquipmentSlot::Body)
                {
                    match self.local_item_stack_is_equippable_in_horse_body_slot(
                        entity_type_id,
                        body_item,
                    ) {
                        Ok(equippable) => equippable,
                        Err(prediction) => return *prediction,
                    }
                } else {
                    false
                };
                if !wearing_body_armor {
                    let mut equipped = stack.clone();
                    equipped.count = 1;
                    if !self.consume_local_item_in_hand(hand, 1)
                        || !self.entities.set_local_equipment_item(
                            entity_id,
                            EquipmentSlot::Body,
                            equipped,
                        )
                    {
                        return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
                    }
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                        LocalInteractionSuccess::client_item_interaction(),
                    ));
                }
            }
        }

        if self
            .entities
            .apply_local_horse_ride_pose_reset(entity_id)
            .is_none()
        {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ))
    }

    pub(super) fn predict_local_abstract_horse_fed_food(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: i32,
        owner: VanillaEntityOwner,
    ) -> LocalTargetUsePrediction {
        let effect = vanilla_abstract_horse_food_effect(held_item_id);
        let ate_food = if let Some(effect) = effect {
            let Some(health) = self.entities.living_entity_health(entity_id) else {
                return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
            };
            let Some(max_health) = self
                .entities
                .attribute_value(entity_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            else {
                return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
            };
            if health < max_health as f32 {
                let Some(healed) = self
                    .entities
                    .apply_local_living_heal(entity_id, effect.heal)
                else {
                    return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                };
                healed
            } else {
                false
            }
        } else {
            false
        };

        let particle_action = if let Some(effect) = effect {
            let Some(is_baby) = self.entities.is_baby(entity_id) else {
                return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
            };
            if is_baby && effect.age_up > 0 {
                let Some((data_id, default)) =
                    vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)
                else {
                    return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                };
                let Some(age_locked) = self
                    .entities
                    .metadata_bool_for_entity(entity_id, data_id, default)
                else {
                    return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                };
                if !age_locked {
                    let Some(position) = self
                        .entities
                        .next_local_equine_feeding_particle_position(entity_id)
                    else {
                        return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                    };
                    Some(LocalInteractionParticleAction::EquineBabyFeeding {
                        position_bits: [
                            position.x.to_bits(),
                            position.y.to_bits(),
                            position.z.to_bits(),
                        ],
                    })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if ate_food && !self.consume_local_item_in_hand(hand, 1) {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(LocalInteractionSuccess {
            particle_action,
            ..LocalInteractionSuccess::server_item_interaction()
        }))
    }

    pub(super) fn predict_local_villager_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        // Spawn eggs and dead mobs have already returned from `Mob.interact`.
        // `AbstractVillager.tradingPlayer` is written only by server-side
        // branches, so the client has no speculative trading state here.
        if self.entities.sleeping_pos(entity_id).is_some() {
            return self.predict_local_ageable_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                VANILLA_VILLAGER_MOB_INTERACT_OWNER,
            );
        }
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if is_baby
            && self
                .entities
                .set_local_abstract_villager_unhappy_counter(entity_id, 40)
                .is_none()
        {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ))
    }

    pub(super) fn predict_local_wandering_trader_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        // Both writes to `tradingPlayer` are server-only. Adult traders thus
        // return client `SUCCESS`, while babies inherit `AgeableMob`.
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if is_baby {
            self.predict_local_ageable_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                VANILLA_WANDERING_TRADER_MOB_INTERACT_OWNER,
            )
        } else {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        }
    }

    pub(super) fn predict_local_nautilus_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(accessors) =
            vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id)
        else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(flags) = self.entities.metadata_byte_for_entity(
            entity_id,
            accessors.flags_data_id,
            accessors.flags_default,
        ) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let is_tamed = flags & accessors.tame_mask != 0;

        if is_baby {
            return self.predict_local_animal_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                VANILLA_NAUTILUS_INTERACT_OWNER,
            );
        }
        if is_tamed && secondary_use_active {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ));
        }
        let Some(item_id) = held_item_id else {
            return if is_tamed {
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_item_interaction(),
                ))
            } else {
                LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
            };
        };
        let food_tag = if !is_tamed {
            VANILLA_NAUTILUS_TAMING_ITEMS_TAG
        } else {
            VANILLA_NAUTILUS_FOOD_TAG
        };
        let Some(food_item_ids) = self.registry_holder_ids_for_tag("minecraft:item", food_tag)
        else {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_NAUTILUS_INTERACT_OWNER,
            );
        };
        let is_food = food_item_ids.contains(&item_id);

        if is_food {
            let Some(health) = self.entities.living_entity_health(entity_id) else {
                return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
            };
            let Some(max_health) = self
                .entities
                .attribute_value(entity_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            else {
                return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
            };
            if health < max_health as f32 {
                let Some(stack) = self.local_item_in_hand(hand) else {
                    return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
                };
                let heal = self
                    .effective_item_stack(stack)
                    .component_patch
                    .food_nutrition
                    .map_or(1.0, |nutrition| nutrition as f32 * 2.0);
                let dropped = match self.use_local_mob_player_item(
                    entity_type_id,
                    hand,
                    held_item_id,
                    VANILLA_NAUTILUS_INTERACT_OWNER,
                ) {
                    Ok(dropped) => dropped,
                    Err(prediction) => return *prediction,
                };
                if self
                    .entities
                    .apply_local_living_heal(entity_id, heal)
                    .is_none()
                {
                    return LocalTargetUsePrediction::UnsupportedEntityInteract(
                        VANILLA_NAUTILUS_INTERACT_OWNER,
                    );
                }
                let mut success = LocalInteractionSuccess::client_item_interaction();
                success.extra_swing = dropped.then_some(InteractionHand::MainHand);
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(success));
            }
        }
        let Some(stack) = self.local_item_in_hand(hand) else {
            return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
        };
        let item_interaction =
            self.predict_local_item_stack_interact_living_entity(entity_id, entity_type_id, stack);
        if item_interaction != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
            return item_interaction;
        }
        if is_food {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false),
            ));
        }
        if is_tamed && !secondary_use_active {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ));
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    }

    pub(super) fn effective_equipment_can_be_sheared(
        &self,
        item: &ProtocolItemStackSummary,
    ) -> Option<bool> {
        let patch = &item.component_patch;
        if patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            return Some(false);
        }
        if patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            return patch
                .equippable
                .as_ref()
                .map(|equippable| equippable.can_be_sheared);
        }
        Some(
            self.items
                .component_source(
                    item.item_id,
                    patch,
                    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                )
                .equippable
                .as_ref()
                .is_some_and(|equippable| equippable.can_be_sheared),
        )
    }

    pub(super) fn local_mob_can_shear_equipment(
        &self,
        entity_id: i32,
        entity_type_id: i32,
    ) -> Option<bool> {
        if entity_type_id != VANILLA_ENTITY_TYPE_WOLF_ID {
            return Some(!self.entities.entity_has_passengers(entity_id));
        }
        let accessors =
            vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id)?;
        let owner = self
            .entities
            .metadata_optional_living_entity_reference_for_entity(
                entity_id,
                accessors.owner_data_id,
                accessors.owner_default,
            )?;
        match owner {
            None => Some(false),
            Some(owner) => self
                .local_player_profile_id()
                .map(|local_player| local_player == owner),
        }
    }

    pub(super) fn equipment_prevents_armor_change(
        &self,
        item: &ProtocolItemStackSummary,
    ) -> Option<bool> {
        let patch = &item.component_patch;
        if patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)
            || patch.enchantments.is_empty()
        {
            return Some(false);
        }
        for enchantment in &patch.enchantments {
            if self.enchantment_prevents_armor_change(enchantment.holder_id)? {
                return Some(true);
            }
        }
        Some(false)
    }
}
