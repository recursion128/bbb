use super::*;

impl WorldStore {
    pub(super) fn predict_local_armor_stand_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        let Some(accessors) =
            vanilla_entity_metadata_armor_stand_accessors_for_entity_type(entity_type_id)
        else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(flags) = self.entities.metadata_byte_for_entity(
            entity_id,
            accessors.client_flags_data_id,
            accessors.client_flags_default,
        ) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let held_item_type = match held_item_id {
            Some(item_id) => {
                let Some(item_type) = vanilla_item_type(item_id) else {
                    return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
                };
                Some(item_type)
            }
            None => None,
        };
        let marker = flags & accessors.marker_mask != 0;
        let held_is_name_tag = held_item_type.is_some_and(|item_type| {
            item_type.interact_living_entity_override
                == Some(VanillaItemInteractLivingEntityOverride::NameTagItem)
        });
        if marker || held_is_name_tag {
            self.predict_local_base_entity_interact(
                entity_id,
                entity_type_id,
                held_item_id,
                secondary_use_active,
            )
        } else {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::server_item_interaction(),
            ))
        }
    }

    pub(super) fn predict_local_interaction_entity_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
    ) -> LocalTargetUsePrediction {
        let Some(accessors) =
            vanilla_entity_metadata_interaction_accessors_for_entity_type(entity_type_id)
        else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(response) = self.entities.metadata_bool_for_entity(
            entity_id,
            accessors.response_data_id,
            accessors.response_default,
        ) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(if response {
            LocalInteractionSuccess::client_item_interaction()
        } else {
            LocalInteractionSuccess::consume(false)
        }))
    }

    pub(super) fn predict_local_item_frame_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some(frame_has_item) = self.entities.item_frame_has_item(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if !frame_has_item && held_item_id.is_none() {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        } else {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        }
    }

    pub(super) fn predict_local_minecart_interact(
        &mut self,
        owner: VanillaEntityOwner,
        entity_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> Option<LocalTargetUsePrediction> {
        let success = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        };
        match owner {
            VANILLA_MINECART_INTERACT_OWNER => Some(
                if secondary_use_active || self.entities.entity_has_passengers(entity_id) {
                    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
                } else {
                    success()
                },
            ),
            VANILLA_MINECART_CONTAINER_INTERACT_OWNER | VANILLA_MINECART_CHEST_INTERACT_OWNER => {
                Some(success())
            }
            VANILLA_MINECART_FURNACE_INTERACT_OWNER => {
                let Some(item_id) = held_item_id else {
                    return Some(success());
                };
                let Some(fuel_items) = self.registry_holder_ids_for_tag(
                    "minecraft:item",
                    "minecraft:furnace_minecart_fuel",
                ) else {
                    return Some(LocalTargetUsePrediction::UnsupportedEntityInteract(owner));
                };
                if !fuel_items.contains(&item_id) {
                    return Some(success());
                }
                let Some(player_position) = self.local_player_pose().map(|pose| pose.position)
                else {
                    return Some(LocalTargetUsePrediction::UnsupportedEntityInteract(owner));
                };
                match self
                    .entities
                    .add_client_furnace_minecart_fuel(entity_id, player_position)
                {
                    Some(true) => {
                        if !self.consume_local_item_in_hand(hand, 1) {
                            return Some(LocalTargetUsePrediction::UnsupportedEntityInteract(
                                owner,
                            ));
                        }
                        Some(success())
                    }
                    Some(false) => Some(success()),
                    None => Some(LocalTargetUsePrediction::UnknownEntityType(
                        self.entity_type_id(entity_id),
                    )),
                }
            }
            VANILLA_MINECART_COMMAND_BLOCK_INTERACT_OWNER => {
                Some(if self.local_player_can_use_gamemaster_blocks() {
                    success()
                } else {
                    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
                })
            }
            _ => None,
        }
    }

    pub(super) fn predict_local_boat_interact(
        &mut self,
        owner: VanillaEntityOwner,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> Option<LocalTargetUsePrediction> {
        if !matches!(
            owner,
            VANILLA_BOAT_INTERACT_OWNER | VANILLA_CHEST_BOAT_INTERACT_OWNER
        ) {
            return None;
        }
        let base_result = self.predict_local_base_entity_interact(
            entity_id,
            entity_type_id,
            held_item_id,
            secondary_use_active,
        );
        if base_result != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
            return Some(base_result);
        }

        let success = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        };
        if owner == VANILLA_CHEST_BOAT_INTERACT_OWNER {
            // `AbstractChestBoat.interact` opens its container whenever
            // `AbstractBoat.interact` passes on the client: secondary use
            // intentionally skips riding, while 60 submerged ticks imply
            // `canAddPassenger == false`.
            return Some(success());
        }
        let Some(movement) = self.entities.boat_movement(entity_id) else {
            return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                entity_type_id,
            )));
        };
        Some(
            if secondary_use_active || movement.out_of_control_ticks >= 60.0 {
                LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
            } else {
                success()
            },
        )
    }

    pub(super) fn predict_local_base_entity_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        if held_item_id == Some(VANILLA_ITEM_SHEARS_ID) {
            let dropped_leashes = self.entities.drop_local_leash_connections(entity_id);
            match dropped_leashes {
                Some(true) => {
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                        LocalInteractionSuccess::client_item_interaction(),
                    ));
                }
                None => {
                    return LocalTargetUsePrediction::UnsupportedEntityInteract(
                        VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                    );
                }
                Some(false) => {}
            }
            if vanilla_entity_metadata_mob_interact_owner(entity_type_id).is_some()
                && !secondary_use_active
            {
                let Some(can_shear_equipment) =
                    self.local_mob_can_shear_equipment(entity_id, entity_type_id)
                else {
                    return LocalTargetUsePrediction::UnsupportedEntityInteract(
                        VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                    );
                };
                if !can_shear_equipment {
                    return self.predict_local_base_leash_interact(
                        entity_id,
                        entity_type_id,
                        held_item_id,
                    );
                }
                for slot in VANILLA_EQUIPMENT_SLOTS.iter().map(|entry| entry.slot) {
                    let Some(item) = self.entities.equipment_item(entity_id, slot) else {
                        continue;
                    };
                    let Some(can_be_sheared) = self.effective_equipment_can_be_sheared(item) else {
                        return LocalTargetUsePrediction::UnsupportedEntityInteract(
                            VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                        );
                    };
                    if !can_be_sheared {
                        continue;
                    }
                    if !self.local_player_is_creative() {
                        let Some(prevents_armor_change) =
                            self.equipment_prevents_armor_change(item)
                        else {
                            return LocalTargetUsePrediction::UnsupportedEntityInteract(
                                VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                            );
                        };
                        if prevents_armor_change {
                            continue;
                        }
                    }
                    if !self.entities.clear_local_equipment_item(entity_id, slot) {
                        return LocalTargetUsePrediction::UnsupportedEntityInteract(
                            VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                        );
                    }
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                        LocalInteractionSuccess::client_item_interaction(),
                    ));
                }
            }
        }
        self.predict_local_base_leash_interact(entity_id, entity_type_id, held_item_id)
    }

    pub(super) fn predict_local_base_leash_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        if !vanilla_entity_metadata_type_is_leashable(entity_type_id) {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        if let Some(holder_id) = self.entities.leash_holder_id(entity_id) {
            let Some(local_player_id) = self.local_player_id else {
                return LocalTargetUsePrediction::UnsupportedEntityInteract(
                    VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                );
            };
            if holder_id == local_player_id {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_without_item(),
                ));
            }
        }
        if held_item_id == Some(VANILLA_ITEM_LEAD_ID) {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
            );
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    }
}
