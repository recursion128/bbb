use super::*;
use bbb_protocol::{
    entity_types::{
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_DONKEY_ID,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_MULE_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_PIG_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
        VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    },
    equipment_slots::{vanilla_equipment_slot_by_id, EquipmentSlot},
    item_types::VanillaItemEquipOnInteract,
};

const ENTITY_TYPE_REGISTRY: &str = "minecraft:entity_type";
const CAN_EQUIP_SADDLE_TAG: &str = "minecraft:can_equip_saddle";
const CAN_EQUIP_HARNESS_TAG: &str = "minecraft:can_equip_harness";

impl WorldStore {
    pub(super) fn predict_local_equippable_equip_on_target(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        stack: &ProtocolItemStackSummary,
    ) -> Option<LocalTargetUsePrediction> {
        if !self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            return None;
        }

        let resolved = if stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            let Some(equippable) = stack.component_patch.equippable.as_ref() else {
                return Some(unsupported_target_equippable());
            };
            if !equippable.equip_on_interact {
                return None;
            }
            let Some(slot) =
                vanilla_equipment_slot_by_id(equippable.slot_id).map(|entry| entry.slot)
            else {
                return Some(unsupported_target_equippable());
            };
            let Some(allowed) = self.local_equippable_holder_set_accepts(
                equippable.allowed_entities.as_ref(),
                entity_type_id,
            ) else {
                return Some(unsupported_target_equippable());
            };
            (slot, allowed)
        } else {
            let item_type = stack.item_id.and_then(vanilla_item_type)?;
            match item_type.equip_on_interact? {
                VanillaItemEquipOnInteract::Saddle => {
                    let Some(allowed) =
                        self.local_entity_type_tag_contains(CAN_EQUIP_SADDLE_TAG, entity_type_id)
                    else {
                        return Some(unsupported_target_equippable());
                    };
                    (EquipmentSlot::Saddle, allowed)
                }
                VanillaItemEquipOnInteract::Harness => {
                    let Some(allowed) =
                        self.local_entity_type_tag_contains(CAN_EQUIP_HARNESS_TAG, entity_type_id)
                    else {
                        return Some(unsupported_target_equippable());
                    };
                    (EquipmentSlot::Body, allowed)
                }
            }
        };
        let (slot, allowed) = resolved;
        if !allowed {
            return Some(target_equippable_pass());
        }
        let Some(alive) = self
            .entities
            .living_entity_health(entity_id)
            .map(|health| health > 0.0)
        else {
            return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                entity_type_id,
            )));
        };
        let Some(can_use_slot) =
            self.local_living_entity_can_use_equipment_slot(entity_id, entity_type_id, slot)
        else {
            return Some(unsupported_target_equippable());
        };
        if !alive || !can_use_slot || self.entities.equipment_item(entity_id, slot).is_some() {
            return Some(target_equippable_pass());
        }
        Some(LocalTargetUsePrediction::Result(
            LocalInteractionResult::Success(LocalInteractionSuccess::client_item_interaction()),
        ))
    }

    fn local_equippable_holder_set_accepts(
        &self,
        allowed_entities: Option<&bbb_protocol::packets::EntityTypeHolderSetSummary>,
        entity_type_id: i32,
    ) -> Option<bool> {
        let Some(allowed_entities) = allowed_entities else {
            return Some(true);
        };
        match (
            allowed_entities.tag.as_deref(),
            allowed_entities.registry_ids.as_slice(),
        ) {
            (Some(tag), []) => self.local_entity_type_tag_contains(tag, entity_type_id),
            (None, ids) => Some(ids.contains(&entity_type_id)),
            (Some(_), _) => None,
        }
    }

    fn local_entity_type_tag_contains(&self, tag: &str, entity_type_id: i32) -> Option<bool> {
        self.registry_holder_ids_for_tag(ENTITY_TYPE_REGISTRY, tag)
            .map(|ids| ids.contains(&entity_type_id))
    }

    fn local_living_entity_can_use_equipment_slot(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        slot: EquipmentSlot,
    ) -> Option<bool> {
        if entity_type_id == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
            return match slot {
                EquipmentSlot::Body | EquipmentSlot::Saddle => Some(false),
                EquipmentSlot::MainHand | EquipmentSlot::OffHand => {
                    let accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
                        entity_type_id,
                    )?;
                    self.entities
                        .metadata_byte_for_entity(
                            entity_id,
                            accessors.client_flags_data_id,
                            accessors.client_flags_default,
                        )
                        .map(|flags| flags & accessors.show_arms_mask != 0)
                }
                _ => Some(true),
            };
        }
        match (entity_type_id, slot) {
            (
                VANILLA_ENTITY_TYPE_HORSE_ID
                | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
                | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
                _,
            ) => Some(true),
            (
                VANILLA_ENTITY_TYPE_PIG_ID | VANILLA_ENTITY_TYPE_STRIDER_ID,
                EquipmentSlot::Saddle,
            )
            | (VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, EquipmentSlot::Body) => {
                self.entities.is_baby(entity_id).map(|is_baby| !is_baby)
            }
            (
                VANILLA_ENTITY_TYPE_CAMEL_ID | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
                EquipmentSlot::Saddle,
            ) => self.entities.is_baby(entity_id).map(|is_baby| !is_baby),
            (
                VANILLA_ENTITY_TYPE_DONKEY_ID | VANILLA_ENTITY_TYPE_MULE_ID,
                EquipmentSlot::Saddle,
            ) => {
                let is_baby = self.entities.is_baby(entity_id)?;
                self.local_abstract_horse_is_tamed(entity_id, entity_type_id)
                    .map(|is_tamed| !is_baby && is_tamed)
            }
            (
                VANILLA_ENTITY_TYPE_NAUTILUS_ID | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
                EquipmentSlot::Saddle | EquipmentSlot::Body,
            ) => {
                let is_baby = self.entities.is_baby(entity_id)?;
                self.local_tamable_animal_is_tamed(entity_id, entity_type_id)
                    .map(|is_tamed| !is_baby && is_tamed)
            }
            _ => Some(true),
        }
    }

    fn local_abstract_horse_is_tamed(&self, entity_id: i32, entity_type_id: i32) -> Option<bool> {
        let accessors =
            vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)?;
        self.entities
            .metadata_byte_for_entity(entity_id, accessors.flags_data_id, accessors.flags_default)
            .map(|flags| flags & accessors.tame_mask != 0)
    }

    fn local_tamable_animal_is_tamed(&self, entity_id: i32, entity_type_id: i32) -> Option<bool> {
        let accessors =
            vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id)?;
        self.entities
            .metadata_byte_for_entity(entity_id, accessors.flags_data_id, accessors.flags_default)
            .map(|flags| flags & accessors.tame_mask != 0)
    }

    pub(super) fn predict_local_swappable_equippable_use(
        &mut self,
        hand: InteractionHand,
        stack: &ProtocolItemStackSummary,
    ) -> Option<LocalItemUsePrediction> {
        if !self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            return None;
        }

        let (equipment_slot, player_allowed) = match self.local_swappable_equippable_slot(stack) {
            Ok(Some(resolved)) => resolved,
            Ok(None) => return None,
            Err(prediction) => return Some(prediction),
        };
        if !player_allowed {
            return Some(LocalItemUsePrediction::Result(LocalInteractionResult::Pass));
        }

        let Some(in_equipment_slot) = self.local_player_equipment_item(equipment_slot) else {
            return Some(LocalItemUsePrediction::MissingComponentValue(
                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            ));
        };
        if !self.local_player_is_creative() {
            let Some(prevents_armor_change) =
                self.equipment_prevents_armor_change(&in_equipment_slot)
            else {
                return Some(LocalItemUsePrediction::MissingComponentValue(
                    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                ));
            };
            if prevents_armor_change {
                return Some(LocalItemUsePrediction::Result(LocalInteractionResult::Fail));
            }
        }
        if same_item_same_components(stack, &in_equipment_slot) {
            return Some(LocalItemUsePrediction::Result(LocalInteractionResult::Fail));
        }

        let Some(dropped) = self.swap_local_item_with_equipment_slot(hand, equipment_slot) else {
            return Some(LocalItemUsePrediction::MissingComponentValue(
                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            ));
        };
        Some(LocalItemUsePrediction::Result(
            LocalInteractionResult::Success(LocalInteractionSuccess::equippable_swap(dropped)),
        ))
    }

    fn local_swappable_equippable_slot(
        &self,
        stack: &ProtocolItemStackSummary,
    ) -> Result<Option<(EquipmentSlot, bool)>, LocalItemUsePrediction> {
        if stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            let Some(equippable) = stack.component_patch.equippable.as_ref() else {
                return Err(LocalItemUsePrediction::MissingComponentValue(
                    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                ));
            };
            if !equippable.swappable {
                return Ok(None);
            }
            let Some(equipment_slot) =
                vanilla_equipment_slot_by_id(equippable.slot_id).map(|entry| entry.slot)
            else {
                return Err(LocalItemUsePrediction::MissingComponentValue(
                    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                ));
            };
            let player_allowed = match &equippable.allowed_entities {
                None => true,
                Some(allowed) if allowed.tag.is_some() && !allowed.registry_ids.is_empty() => {
                    return Err(LocalItemUsePrediction::MissingComponentValue(
                        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                    ));
                }
                Some(allowed) => {
                    if let Some(tag) = allowed.tag.as_deref() {
                        let Some(entity_type_ids) =
                            self.registry_holder_ids_for_tag(ENTITY_TYPE_REGISTRY, tag)
                        else {
                            return Err(LocalItemUsePrediction::MissingComponentValue(
                                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                            ));
                        };
                        entity_type_ids.contains(&VANILLA_ENTITY_TYPE_PLAYER_ID)
                    } else {
                        allowed
                            .registry_ids
                            .contains(&VANILLA_ENTITY_TYPE_PLAYER_ID)
                    }
                }
            };
            return Ok(Some((equipment_slot, player_allowed)));
        }

        let Some(item_id) = stack.item_id else {
            return Err(LocalItemUsePrediction::UnknownItemType(None));
        };
        if !self
            .items
            .default_swappable_equipment_item_ids
            .contains(&item_id)
        {
            return Ok(None);
        }
        let Some(equipment_slot) = self
            .items
            .default_item_equipment_slots
            .get(&item_id)
            .copied()
            .map(ItemEquipmentSlot::protocol_slot)
        else {
            return Err(LocalItemUsePrediction::MissingComponentValue(
                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            ));
        };
        Ok(Some((
            equipment_slot,
            self.items
                .default_player_swappable_equipment_item_ids
                .contains(&item_id),
        )))
    }
}

fn unsupported_target_equippable() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::UnsupportedItemStackInteractLivingEntityComponent(
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    )
}

fn target_equippable_pass() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        data_component_types::VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        entity_types::{
            VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
            VANILLA_ENTITY_TYPE_PIG_ID,
        },
        equipment_slots::{
            VANILLA_EQUIPMENT_SLOT_BODY_ID, VANILLA_EQUIPMENT_SLOT_CHEST_ID,
            VANILLA_EQUIPMENT_SLOT_MAINHAND_ID, VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
            VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
        },
        game_types::{VANILLA_GAME_TYPE_CREATIVE_ID, VANILLA_GAME_TYPE_SURVIVAL_ID},
        item_types::{
            VANILLA_ITEM_IRON_CHESTPLATE_ID, VANILLA_ITEM_SADDLE_ID, VANILLA_ITEM_SHIELD_ID,
            VANILLA_ITEM_STONE_ID, VANILLA_ITEM_WHITE_HARNESS_ID,
        },
        packets::{
            AddEntity, CommonPlayerSpawnInfo, DataComponentPatchSummary,
            EntityTypeHolderSetSummary, EquipmentSlotUpdate, EquippableSummary,
            ItemEnchantmentSummary, PlayLogin, PlayerAbilities, RegistryData, RegistryDataEntry,
            RegistryTags, SetEquipment, SetPlayerInventory, SoundEventSummary, TagNetworkPayload,
            UpdateTags, Vec3d,
        },
    };
    use std::collections::{BTreeMap, BTreeSet};
    use uuid::Uuid;

    fn stack(item_id: i32, count: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count,
            component_patch: DataComponentPatchSummary::default(),
        }
    }

    fn explicit_equippable_stack(
        count: i32,
        swappable: bool,
        allowed_entities: Option<EntityTypeHolderSetSummary>,
    ) -> ProtocolItemStackSummary {
        let sound = || SoundEventSummary {
            registry_id: Some(0),
            sound_id: None,
            fixed_range_bits: None,
        };
        ProtocolItemStackSummary {
            item_id: Some(VANILLA_ITEM_STONE_ID),
            count,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID],
                equippable: Some(EquippableSummary {
                    slot_id: VANILLA_EQUIPMENT_SLOT_CHEST_ID,
                    equip_sound: sound(),
                    asset_id: None,
                    camera_overlay: None,
                    allowed_entities,
                    dispensable: true,
                    swappable,
                    damage_on_hurt: true,
                    equip_on_interact: false,
                    can_be_sheared: false,
                    shearing_sound: sound(),
                }),
                ..DataComponentPatchSummary::default()
            },
        }
    }

    fn world_with_game_type(game_type: i32) -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_login(&PlayLogin {
            player_id: 7,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: i8::try_from(game_type).unwrap(),
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        world
    }

    fn set_inventory_item(world: &mut WorldStore, slot: i32, item: ProtocolItemStackSummary) {
        assert!(world.apply_set_player_inventory(SetPlayerInventory { slot, item }));
    }

    fn inventory_item(world: &WorldStore, slot: i32) -> ProtocolItemStackSummary {
        world
            .inventory()
            .player_slots
            .iter()
            .find(|entry| entry.slot == slot)
            .map(|entry| entry.item.clone())
            .unwrap_or_else(ProtocolItemStackSummary::empty)
    }

    fn add_entity(world: &mut WorldStore, id: i32, entity_type_id: i32) {
        world.apply_add_entity(AddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position: Vec3d::default(),
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
    }

    fn install_default_equip_on_interact_tags(world: &mut WorldStore) {
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: ENTITY_TYPE_REGISTRY.to_string(),
                tags: vec![
                    TagNetworkPayload {
                        tag: CAN_EQUIP_SADDLE_TAG.to_string(),
                        entries: vec![VANILLA_ENTITY_TYPE_PIG_ID],
                    },
                    TagNetworkPayload {
                        tag: CAN_EQUIP_HARNESS_TAG.to_string(),
                        entries: vec![VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID],
                    },
                ],
            }],
        });
    }

    #[test]
    fn default_target_equippables_use_generated_owner_live_tags_and_empty_slot() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 20, VANILLA_ENTITY_TYPE_PIG_ID);
        add_entity(&mut world, 21, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID);
        install_default_equip_on_interact_tags(&mut world);
        world.set_default_item_component_type_ids(BTreeMap::from([
            (
                VANILLA_ITEM_SADDLE_ID,
                BTreeSet::from([VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID]),
            ),
            (
                VANILLA_ITEM_WHITE_HARNESS_ID,
                BTreeSet::from([VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID]),
            ),
        ]));
        let success = Some(LocalTargetUsePrediction::Result(
            LocalInteractionResult::Success(LocalInteractionSuccess::client_item_interaction()),
        ));
        let pass = Some(target_equippable_pass());
        let saddle = stack(VANILLA_ITEM_SADDLE_ID, 1);
        let harness = stack(VANILLA_ITEM_WHITE_HARNESS_ID, 1);

        assert_eq!(
            world.predict_local_equippable_equip_on_target(20, VANILLA_ENTITY_TYPE_PIG_ID, &saddle),
            success
        );
        assert_eq!(
            world.predict_local_equippable_equip_on_target(
                20,
                VANILLA_ENTITY_TYPE_PIG_ID,
                &harness
            ),
            pass
        );
        assert_eq!(
            world.predict_local_equippable_equip_on_target(
                21,
                VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
                &harness
            ),
            success
        );

        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: 20,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Saddle,
                item: stack(VANILLA_ITEM_SADDLE_ID, 1),
            }],
        }));
        assert_eq!(
            world.predict_local_equippable_equip_on_target(20, VANILLA_ENTITY_TYPE_PIG_ID, &saddle),
            pass
        );
    }

    #[test]
    fn patched_target_equippable_honors_flag_holders_slot_and_removal() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 30, VANILLA_ENTITY_TYPE_COW_ID);
        let mut held = explicit_equippable_stack(
            2,
            true,
            Some(EntityTypeHolderSetSummary {
                registry_ids: vec![VANILLA_ENTITY_TYPE_COW_ID],
                tag: None,
            }),
        );
        held.component_patch
            .equippable
            .as_mut()
            .unwrap()
            .equip_on_interact = true;
        assert!(matches!(
            world.predict_local_equippable_equip_on_target(30, VANILLA_ENTITY_TYPE_COW_ID, &held),
            Some(LocalTargetUsePrediction::Result(
                LocalInteractionResult::Success(_)
            ))
        ));
        assert_eq!(held.count, 2, "target equip is server-authoritative");

        held.component_patch
            .equippable
            .as_mut()
            .unwrap()
            .allowed_entities = Some(EntityTypeHolderSetSummary {
            registry_ids: vec![VANILLA_ENTITY_TYPE_PIG_ID],
            tag: None,
        });
        assert_eq!(
            world.predict_local_equippable_equip_on_target(30, VANILLA_ENTITY_TYPE_COW_ID, &held),
            Some(target_equippable_pass())
        );

        held.component_patch
            .equippable
            .as_mut()
            .unwrap()
            .allowed_entities = None;
        held.component_patch.equippable.as_mut().unwrap().slot_id = i32::MAX;
        assert_eq!(
            world.predict_local_equippable_equip_on_target(30, VANILLA_ENTITY_TYPE_COW_ID, &held),
            Some(unsupported_target_equippable())
        );

        let removed_default = ProtocolItemStackSummary {
            component_patch: DataComponentPatchSummary {
                removed_type_ids: vec![VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID],
                ..DataComponentPatchSummary::default()
            },
            ..stack(VANILLA_ITEM_SADDLE_ID, 1)
        };
        world.set_default_item_component_type_ids(BTreeMap::from([(
            VANILLA_ITEM_SADDLE_ID,
            BTreeSet::from([VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID]),
        )]));
        assert_eq!(
            world.predict_local_equippable_equip_on_target(
                30,
                VANILLA_ENTITY_TYPE_COW_ID,
                &removed_default
            ),
            None
        );

        let mut no_target_equip = explicit_equippable_stack(1, true, None);
        no_target_equip
            .component_patch
            .equippable
            .as_mut()
            .unwrap()
            .slot_id = VANILLA_EQUIPMENT_SLOT_BODY_ID;
        assert_eq!(
            world.predict_local_equippable_equip_on_target(
                30,
                VANILLA_ENTITY_TYPE_COW_ID,
                &no_target_equip
            ),
            None
        );
    }

    #[test]
    fn explicit_single_item_swap_honors_player_holder_and_same_stack_fail() {
        let allowed_player = EntityTypeHolderSetSummary {
            registry_ids: vec![VANILLA_ENTITY_TYPE_PLAYER_ID],
            tag: None,
        };
        let mut world = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        let held = explicit_equippable_stack(1, true, Some(allowed_player.clone()));
        set_inventory_item(&mut world, 0, held.clone());
        set_inventory_item(
            &mut world,
            VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
            stack(VANILLA_ITEM_SHIELD_ID, 1),
        );

        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::equippable_swap(false)
            ))
        );
        assert_eq!(
            inventory_item(&world, 0).item_id,
            Some(VANILLA_ITEM_SHIELD_ID)
        );
        assert_eq!(
            inventory_item(&world, VANILLA_PLAYER_INVENTORY_CHEST_SLOT),
            held
        );

        let mut same = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        let held = explicit_equippable_stack(2, true, Some(allowed_player));
        let mut equipped = held.clone();
        equipped.count = 1;
        set_inventory_item(&mut same, 0, held.clone());
        set_inventory_item(&mut same, VANILLA_PLAYER_INVENTORY_CHEST_SLOT, equipped);
        assert_eq!(
            same.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
        );
        assert_eq!(inventory_item(&same, 0), held);
    }

    #[test]
    fn explicit_allowed_entity_pass_and_unswappable_fallthrough_preserve_order() {
        let mut world = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        let disallowed = EntityTypeHolderSetSummary {
            registry_ids: vec![],
            tag: None,
        };
        let held = explicit_equippable_stack(1, true, Some(disallowed));
        set_inventory_item(&mut world, 0, held.clone());
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert_eq!(inventory_item(&world, 0), held);

        let mut unswappable = explicit_equippable_stack(1, false, None);
        unswappable.component_patch.added = 2;
        unswappable
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID);
        set_inventory_item(&mut world, 0, unswappable);
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true)
            ))
        );
    }

    #[test]
    fn allowed_entity_tag_uses_live_holder_membership_and_missing_tag_is_explicit() {
        let mut missing = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        let tagged = explicit_equippable_stack(
            1,
            true,
            Some(EntityTypeHolderSetSummary {
                registry_ids: vec![],
                tag: Some("example:players".to_string()),
            }),
        );
        set_inventory_item(&mut missing, 0, tagged.clone());
        assert_eq!(
            missing.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::MissingComponentValue(
                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID
            )
        );

        missing.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: ENTITY_TYPE_REGISTRY.to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "example:players".to_string(),
                    entries: vec![VANILLA_ENTITY_TYPE_PLAYER_ID],
                }],
            }],
        });
        assert_eq!(
            missing.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::equippable_swap(false)
            ))
        );
        assert_eq!(
            inventory_item(&missing, VANILLA_PLAYER_INVENTORY_CHEST_SLOT),
            tagged
        );
    }

    #[test]
    fn stacked_swap_adds_displaced_item_or_emits_full_inventory_drop_swing() {
        let mut world = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        let held = explicit_equippable_stack(2, true, None);
        set_inventory_item(&mut world, 0, held.clone());
        set_inventory_item(
            &mut world,
            VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
            stack(VANILLA_ITEM_SHIELD_ID, 1),
        );
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::equippable_swap(false)
            ))
        );
        assert_eq!(inventory_item(&world, 0).count, 1);
        assert_eq!(
            inventory_item(&world, 1).item_id,
            Some(VANILLA_ITEM_SHIELD_ID)
        );
        assert_eq!(
            inventory_item(&world, VANILLA_PLAYER_INVENTORY_CHEST_SLOT).count,
            1
        );

        let mut full = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        for slot in 0..36 {
            set_inventory_item(&mut full, slot, stack(VANILLA_ITEM_SHIELD_ID, 1));
        }
        set_inventory_item(&mut full, 0, held);
        set_inventory_item(
            &mut full,
            VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
            stack(VANILLA_ITEM_SADDLE_ID, 1),
        );
        assert_eq!(
            full.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::equippable_swap(true)
            ))
        );
        assert_eq!(inventory_item(&full, 0).count, 1);
        assert!(full.inventory().player_slots.iter().all(|slot| {
            slot.slot == VANILLA_PLAYER_INVENTORY_CHEST_SLOT
                || slot.item.item_id != Some(VANILLA_ITEM_SADDLE_ID)
        }));
    }

    #[test]
    fn stacked_hand_slot_swap_splits_displaced_stack_without_overfilling() {
        let mut world = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        world.set_default_item_max_stack_sizes(BTreeMap::from([
            (VANILLA_ITEM_STONE_ID, 64),
            (VANILLA_ITEM_SHIELD_ID, 1),
        ]));
        set_inventory_item(&mut world, 0, stack(VANILLA_ITEM_STONE_ID, 64));
        set_inventory_item(&mut world, 1, stack(VANILLA_ITEM_STONE_ID, 63));
        for slot in 2..36 {
            set_inventory_item(&mut world, slot, stack(VANILLA_ITEM_SHIELD_ID, 1));
        }
        let mut held = explicit_equippable_stack(2, true, None);
        held.component_patch.equippable.as_mut().unwrap().slot_id =
            VANILLA_EQUIPMENT_SLOT_MAINHAND_ID;
        set_inventory_item(&mut world, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT, held);

        assert_eq!(
            world.predict_local_item_use(InteractionHand::OffHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::equippable_swap(true)
            ))
        );
        assert_eq!(inventory_item(&world, 0).count, 1);
        assert_eq!(inventory_item(&world, 1).count, 64);
        assert_eq!(
            inventory_item(&world, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT).count,
            1
        );
    }

    #[test]
    fn creative_game_type_and_infinite_materials_drive_distinct_swap_branches() {
        let mut creative = world_with_game_type(VANILLA_GAME_TYPE_CREATIVE_ID);
        let held = explicit_equippable_stack(1, true, None);
        set_inventory_item(&mut creative, 0, held.clone());
        assert_eq!(
            creative.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::equippable_swap(false)
            ))
        );
        assert_eq!(inventory_item(&creative, 0), held);
        assert_eq!(
            inventory_item(&creative, VANILLA_PLAYER_INVENTORY_CHEST_SLOT),
            held
        );

        let mut infinite_survival = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        infinite_survival.apply_player_abilities(PlayerAbilities {
            invulnerable: false,
            flying: false,
            can_fly: false,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        let stacked = explicit_equippable_stack(2, true, None);
        set_inventory_item(&mut infinite_survival, 0, stacked.clone());
        set_inventory_item(
            &mut infinite_survival,
            VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
            stack(VANILLA_ITEM_SHIELD_ID, 1),
        );
        assert_eq!(
            infinite_survival.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::equippable_swap(false)
            ))
        );
        assert_eq!(inventory_item(&infinite_survival, 0), stacked);
        assert_eq!(
            inventory_item(&infinite_survival, 1).item_id,
            Some(VANILLA_ITEM_SHIELD_ID)
        );
    }

    #[test]
    fn binding_curse_blocks_survival_swap_and_creative_game_type_bypasses_it() {
        let binding_target = || ProtocolItemStackSummary {
            item_id: Some(VANILLA_ITEM_SHIELD_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID],
                enchantments: vec![ItemEnchantmentSummary {
                    holder_id: 0,
                    level: 1,
                }],
                ..DataComponentPatchSummary::default()
            },
        };
        let install_enchantments = |world: &mut WorldStore| {
            world.record_registry_data(RegistryData {
                registry: "minecraft:enchantment".to_string(),
                entries: vec![RegistryDataEntry {
                    id: "minecraft:binding_curse".to_string(),
                    raw_data: None,
                }],
                raw_payload_len: 0,
            });
        };

        let mut survival = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        install_enchantments(&mut survival);
        set_inventory_item(&mut survival, 0, explicit_equippable_stack(1, true, None));
        set_inventory_item(
            &mut survival,
            VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
            binding_target(),
        );
        assert_eq!(
            survival.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
        );

        let mut creative = world_with_game_type(VANILLA_GAME_TYPE_CREATIVE_ID);
        install_enchantments(&mut creative);
        set_inventory_item(&mut creative, 0, explicit_equippable_stack(1, true, None));
        set_inventory_item(
            &mut creative,
            VANILLA_PLAYER_INVENTORY_CHEST_SLOT,
            binding_target(),
        );
        assert!(matches!(
            creative.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
    }

    #[test]
    fn default_profiles_distinguish_player_equipment_animal_equipment_and_fallthrough() {
        let configure = |world: &mut WorldStore| {
            world.set_default_item_component_type_ids(BTreeMap::from([
                (
                    VANILLA_ITEM_IRON_CHESTPLATE_ID,
                    BTreeSet::from([VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID]),
                ),
                (
                    VANILLA_ITEM_SADDLE_ID,
                    BTreeSet::from([VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID]),
                ),
                (
                    VANILLA_ITEM_SHIELD_ID,
                    BTreeSet::from([
                        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
                    ]),
                ),
            ]));
            world.set_default_item_equipment_slots(BTreeMap::from([
                (VANILLA_ITEM_IRON_CHESTPLATE_ID, ItemEquipmentSlot::Chest),
                (VANILLA_ITEM_SADDLE_ID, ItemEquipmentSlot::Saddle),
                (VANILLA_ITEM_SHIELD_ID, ItemEquipmentSlot::OffHand),
            ]));
            world.set_default_swappable_equipment_item_ids(BTreeSet::from([
                VANILLA_ITEM_IRON_CHESTPLATE_ID,
                VANILLA_ITEM_SADDLE_ID,
            ]));
            world.set_default_player_swappable_equipment_item_ids(BTreeSet::from([
                VANILLA_ITEM_IRON_CHESTPLATE_ID,
            ]));
        };

        let mut armor = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        configure(&mut armor);
        set_inventory_item(&mut armor, 0, stack(VANILLA_ITEM_IRON_CHESTPLATE_ID, 1));
        assert!(matches!(
            armor.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        assert_eq!(
            inventory_item(&armor, VANILLA_PLAYER_INVENTORY_CHEST_SLOT).item_id,
            Some(VANILLA_ITEM_IRON_CHESTPLATE_ID)
        );

        let mut saddle = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        configure(&mut saddle);
        set_inventory_item(&mut saddle, 0, stack(VANILLA_ITEM_SADDLE_ID, 1));
        assert_eq!(
            saddle.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
        );

        let mut shield = world_with_game_type(VANILLA_GAME_TYPE_SURVIVAL_ID);
        configure(&mut shield);
        set_inventory_item(&mut shield, 0, stack(VANILLA_ITEM_SHIELD_ID, 1));
        set_inventory_item(
            &mut shield,
            VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            stack(VANILLA_ITEM_STONE_ID, 1),
        );
        assert_eq!(
            shield.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true)
            ))
        );
        assert_eq!(
            inventory_item(&shield, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT).item_id,
            Some(VANILLA_ITEM_STONE_ID)
        );
    }
}
