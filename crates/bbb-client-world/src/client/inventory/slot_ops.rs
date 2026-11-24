//! Slot-level container and menu machinery behind the local click paths.
//!
//! Free functions (plus their private helper types) shared by
//! [`InventoryState`]'s click/quick-move application methods and by the
//! per-menu submodules; split out of `inventory.rs` to keep the state module
//! focused on the store itself.

use super::*;
use crate::entities::entity_data::{entity_data_bool, entity_data_byte};

mod bundle;
mod quick_move;
pub(super) use bundle::*;
pub(super) use quick_move::*;

pub(super) enum ResultTakeDestination<'a> {
    Cursor(&'a mut ProtocolItemStackSummary, i8),
    Slot(i16),
    Drop,
    PlayerInventory {
        container_id: i32,
        player_slots: std::ops::Range<i16>,
        profiles: &'a ItemProfiles,
    },
}

impl ResultTakeDestination<'_> {
    pub(super) fn may_take(&self) -> bool {
        match self {
            Self::Cursor(item, button_num) => *button_num == 0 && item_stack_is_empty(item),
            Self::Slot(_) | Self::Drop | Self::PlayerInventory { .. } => true,
        }
    }

    pub(super) fn take(
        self,
        slots: &mut [ContainerSlot],
        source_index: usize,
        input_indices: &[usize],
    ) -> bool {
        let mut trial = None;
        let target = match self {
            Self::Cursor(item, _) => {
                *item = slots[source_index].item.clone();
                &mut *slots
            }
            Self::Slot(dest_slot_num) => {
                let Some(dest_index) = slots.iter().position(|slot| slot.slot == dest_slot_num)
                else {
                    return false;
                };
                if dest_index == source_index || !item_stack_is_empty(&slots[dest_index].item) {
                    return false;
                }
                slots[dest_index].item = slots[source_index].item.clone();
                slots[dest_index].normalize_selection();
                &mut *slots
            }
            Self::Drop => &mut *slots,
            Self::PlayerInventory {
                container_id,
                player_slots,
                profiles,
            } => {
                let candidate = trial.insert(slots.to_vec());
                let mut moving = slots[source_index].item.clone();
                if !move_item_stack_to_slots(
                    container_id,
                    candidate,
                    source_index,
                    &mut moving,
                    player_slots.start,
                    player_slots.end,
                    true,
                    profiles,
                ) || !item_stack_is_empty(&moving)
                {
                    return false;
                }
                candidate.as_mut_slice()
            }
        };
        for &index in std::iter::once(&source_index).chain(input_indices) {
            target[index].item = ProtocolItemStackSummary::empty();
            target[index].normalize_selection();
        }
        if let Some(trial) = trial {
            slots.clone_from_slice(&trial);
        }
        true
    }
}

pub(super) fn mount_horse_saddle_slot_is_active(
    entity_type_id: i32,
    data_values: &[ProtocolEntityDataValue],
) -> bool {
    if !crate::entities::is_vanilla_can_equip_saddle_type(entity_type_id) {
        return false;
    }
    if crate::entities::vanilla_entity_type_can_use_every_equipment_slot(entity_type_id) {
        return true;
    }
    let Some(accessors) =
        vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)
    else {
        return false;
    };
    !mount_entity_is_ageable_baby(entity_type_id, data_values)
        && (entity_data_byte(
            data_values,
            accessors.flags_data_id,
            accessors.flags_default,
        ) & accessors.tame_mask)
            != 0
}

pub(super) fn mount_horse_body_slot_kind(entity_type_id: i32) -> Option<MountArmorSlotKind> {
    if crate::entities::is_vanilla_llama_type(entity_type_id) {
        Some(MountArmorSlotKind::Llama)
    } else if crate::entities::is_vanilla_can_wear_horse_armor_type(entity_type_id) {
        Some(MountArmorSlotKind::Horse)
    } else {
        None
    }
}

pub(super) fn mount_nautilus_can_use_equipment_slots(
    entity_type_id: i32,
    data_values: &[ProtocolEntityDataValue],
) -> bool {
    let Some(accessors) =
        vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id)
    else {
        return false;
    };
    !mount_entity_is_ageable_baby(entity_type_id, data_values)
        && (entity_data_byte(
            data_values,
            accessors.flags_data_id,
            accessors.flags_default,
        ) & accessors.tame_mask)
            != 0
}

pub(super) fn mount_entity_is_ageable_baby(
    entity_type_id: i32,
    data_values: &[ProtocolEntityDataValue],
) -> bool {
    vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
        .map(|(data_id, default)| entity_data_bool(data_values, data_id, default))
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LocalContainerSlotAccess {
    pub(super) may_place: bool,
    pub(super) may_pickup: bool,
    pub(super) max_stack_size: i32,
    pub(super) takes_full_stack: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LocalMountSlotRules {
    entity_type_id: i32,
    equipment_slots: MountEquipmentSlotVisibility,
    player_start: i16,
}

pub(super) fn local_mount_slot_rules(
    ctx: InventoryCtx<'_>,
    container: &ContainerState,
    equipment_slots: Option<MountEquipmentSlotVisibility>,
) -> Option<LocalMountSlotRules> {
    Some(LocalMountSlotRules {
        entity_type_id: ctx.entities.entity_type_id(container.mount?.entity_id)?,
        equipment_slots: equipment_slots?,
        player_start: slot_state::player_section_start(container)?,
    })
}

impl LocalContainerSlotAccess {
    fn unrestricted(max_stack_size: i32) -> Self {
        Self {
            may_place: true,
            may_pickup: true,
            max_stack_size,
            takes_full_stack: false,
        }
    }

    fn input(may_place: bool, max_stack_size: i32) -> Self {
        Self {
            may_place,
            may_pickup: true,
            max_stack_size,
            takes_full_stack: false,
        }
    }

    fn result(may_pickup: bool, max_stack_size: i32) -> Self {
        Self {
            may_place: false,
            may_pickup,
            max_stack_size,
            takes_full_stack: false,
        }
    }

    fn crafting_result(may_pickup: bool, max_stack_size: i32) -> Self {
        Self {
            may_place: false,
            may_pickup,
            max_stack_size,
            takes_full_stack: true,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn local_container_slot_access(
    ctx: InventoryCtx<'_>,
    container_id: i32,
    menu_type_id: Option<i32>,
    mount_rules: Option<LocalMountSlotRules>,
    data_values: &[ContainerDataValue],
    slot_num: i16,
    slot_item: &ProtocolItemStackSummary,
    stack_to_place: &ProtocolItemStackSummary,
) -> Option<LocalContainerSlotAccess> {
    let item_max_stack_size = ctx.items.item_stack_max_stack_size(stack_to_place);
    if container_id == INVENTORY_MENU_CONTAINER_ID {
        return local_inventory_menu_slot_access(
            ctx,
            slot_num,
            slot_item,
            stack_to_place,
            item_max_stack_size,
        );
    }
    if let Some(mount_rules) = mount_rules {
        return local_mount_slot_access(
            ctx,
            mount_rules,
            slot_num,
            slot_item,
            stack_to_place,
            item_max_stack_size,
        );
    }
    if menu_type_has_unrestricted_slot_rules(menu_type_id) {
        return Some(LocalContainerSlotAccess::unrestricted(item_max_stack_size));
    }

    let item_tags = ctx.registries.tags.get("minecraft:item");
    let access = match menu_type_id? {
        VANILLA_MENU_TYPE_SHULKER_BOX_ID => match slot_num {
            0..SHULKER_BOX_CONTAINER_SLOT_COUNT => LocalContainerSlotAccess::input(
                item_can_fit_inside_container_items(stack_to_place),
                item_max_stack_size,
            ),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_CRAFTER_3X3_ID => match slot_num {
            0..=8 => LocalContainerSlotAccess::input(
                !crafter_disabled_slots(data_values).contains(&slot_num),
                item_max_stack_size,
            ),
            CRAFTER_RESULT_SLOT => LocalContainerSlotAccess::result(false, item_max_stack_size),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_ANVIL_ID => match slot_num {
            ANVIL_INPUT_SLOT | ANVIL_ADDITIONAL_SLOT => {
                LocalContainerSlotAccess::input(true, item_max_stack_size)
            }
            ANVIL_RESULT_SLOT => LocalContainerSlotAccess::result(
                anvil_result_may_pickup(
                    data_values,
                    ctx.local_player.abilities,
                    ctx.local_player.experience,
                ),
                item_max_stack_size,
            ),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_BEACON_ID => match slot_num {
            BEACON_PAYMENT_SLOT => LocalContainerSlotAccess::input(
                item_stack_in_item_tag(stack_to_place, item_tags, BEACON_PAYMENT_ITEM_TAG),
                item_max_stack_size.min(1),
            ),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_BLAST_FURNACE_ID
        | VANILLA_MENU_TYPE_FURNACE_ID
        | VANILLA_MENU_TYPE_SMOKER_ID => match slot_num {
            FURNACE_INPUT_SLOT => LocalContainerSlotAccess::input(true, item_max_stack_size),
            FURNACE_FUEL_SLOT => LocalContainerSlotAccess::input(
                furnace_is_fuel(stack_to_place, &ctx.items.furnace_fuel_item_ids)
                    || stack_to_place.item_id == Some(VANILLA_ITEM_BUCKET_ID),
                if stack_to_place.item_id == Some(VANILLA_ITEM_BUCKET_ID) {
                    item_max_stack_size.min(1)
                } else {
                    item_max_stack_size
                },
            ),
            FURNACE_RESULT_SLOT => LocalContainerSlotAccess::result(true, item_max_stack_size),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_BREWING_STAND_ID => match slot_num {
            BREWING_STAND_BOTTLE_SLOT_START..BREWING_STAND_BOTTLE_SLOT_END => {
                LocalContainerSlotAccess::input(
                    item_stack_item_id_in_set(stack_to_place, &ctx.items.brewing_potion_item_ids),
                    item_max_stack_size.min(1),
                )
            }
            BREWING_STAND_INGREDIENT_SLOT => LocalContainerSlotAccess::input(
                item_stack_item_id_in_set(stack_to_place, &ctx.items.brewing_ingredient_item_ids),
                item_max_stack_size,
            ),
            BREWING_STAND_FUEL_SLOT => LocalContainerSlotAccess::input(
                item_stack_in_item_tag(stack_to_place, item_tags, BREWING_STAND_FUEL_ITEM_TAG),
                item_max_stack_size,
            ),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_CRAFTING_ID => match slot_num {
            CRAFTING_MENU_RESULT_SLOT => {
                LocalContainerSlotAccess::crafting_result(true, item_max_stack_size)
            }
            CRAFTING_MENU_CRAFT_SLOT_START..CRAFTING_MENU_CRAFT_SLOT_END => {
                LocalContainerSlotAccess::input(true, item_max_stack_size)
            }
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_ENCHANTMENT_ID => match slot_num {
            ENCHANTMENT_INPUT_SLOT => {
                LocalContainerSlotAccess::input(true, item_max_stack_size.min(1))
            }
            ENCHANTMENT_LAPIS_SLOT => LocalContainerSlotAccess::input(
                item_stack_item_id_in_set(
                    stack_to_place,
                    &ctx.items.enchantment_lapis_lazuli_item_ids,
                ),
                item_max_stack_size,
            ),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_GRINDSTONE_ID => match slot_num {
            GRINDSTONE_INPUT_SLOT | GRINDSTONE_ADDITIONAL_SLOT => LocalContainerSlotAccess::input(
                item_stack_is_damageable_or_enchanted(stack_to_place, ctx.items),
                item_max_stack_size,
            ),
            GRINDSTONE_RESULT_SLOT => LocalContainerSlotAccess::result(true, item_max_stack_size),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_LECTERN_ID => LocalContainerSlotAccess::input(true, item_max_stack_size),
        VANILLA_MENU_TYPE_LOOM_ID => match slot_num {
            LOOM_BANNER_SLOT => LocalContainerSlotAccess::input(
                item_stack_in_item_tag(stack_to_place, item_tags, LOOM_BANNER_ITEM_TAG),
                item_max_stack_size,
            ),
            LOOM_DYE_SLOT => LocalContainerSlotAccess::input(
                item_stack_in_item_tag(stack_to_place, item_tags, LOOM_DYE_ITEM_TAG)
                    && ctx.items.item_has_effective_component(
                        stack_to_place.item_id,
                        &stack_to_place.component_patch,
                        VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                    ),
                item_max_stack_size,
            ),
            LOOM_PATTERN_SLOT => LocalContainerSlotAccess::input(
                item_stack_in_item_tag(stack_to_place, item_tags, LOOM_PATTERN_ITEM_TAG)
                    && ctx.items.item_has_effective_component(
                        stack_to_place.item_id,
                        &stack_to_place.component_patch,
                        VANILLA_DATA_COMPONENT_PROVIDES_BANNER_PATTERNS_TYPE_ID,
                    ),
                item_max_stack_size,
            ),
            LOOM_RESULT_SLOT => LocalContainerSlotAccess::result(true, item_max_stack_size),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_MERCHANT_ID => match slot_num {
            MERCHANT_PAYMENT_SLOT_1 | MERCHANT_PAYMENT_SLOT_2 => {
                LocalContainerSlotAccess::input(true, item_max_stack_size)
            }
            MERCHANT_RESULT_SLOT => LocalContainerSlotAccess::result(true, item_max_stack_size),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_SMITHING_ID => match slot_num {
            SMITHING_TEMPLATE_SLOT | SMITHING_BASE_SLOT | SMITHING_ADDITIONAL_SLOT => {
                LocalContainerSlotAccess::input(
                    smithing_input_slot_accepts_stack(
                        slot_num,
                        stack_to_place,
                        &ctx.recipes.property_sets,
                    ),
                    item_max_stack_size,
                )
            }
            SMITHING_RESULT_SLOT => LocalContainerSlotAccess::result(true, item_max_stack_size),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID => match slot_num {
            CARTOGRAPHY_TABLE_MAP_SLOT => LocalContainerSlotAccess::input(
                item_stack_has_map_id(stack_to_place),
                item_max_stack_size,
            ),
            CARTOGRAPHY_TABLE_ADDITIONAL_SLOT => LocalContainerSlotAccess::input(
                stack_to_place.item_id.is_some_and(|item_id| {
                    stack_to_place.count > 0
                        && ctx
                            .items
                            .cartography_additional_items
                            .contains_key(&item_id)
                }),
                item_max_stack_size,
            ),
            CARTOGRAPHY_TABLE_RESULT_SLOT => {
                LocalContainerSlotAccess::result(true, item_max_stack_size)
            }
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        VANILLA_MENU_TYPE_STONECUTTER_ID => match slot_num {
            STONECUTTER_INPUT_SLOT => LocalContainerSlotAccess::input(true, item_max_stack_size),
            STONECUTTER_RESULT_SLOT => LocalContainerSlotAccess::result(true, item_max_stack_size),
            _ => LocalContainerSlotAccess::unrestricted(item_max_stack_size),
        },
        _ => return None,
    };
    Some(access)
}

pub(super) fn local_container_slot_accesses(
    ctx: InventoryCtx<'_>,
    container_id: i32,
    menu_type_id: Option<i32>,
    mount_rules: Option<LocalMountSlotRules>,
    data_values: &[ContainerDataValue],
    slots: &[ContainerSlot],
    stack_to_place: &ProtocolItemStackSummary,
) -> Option<BTreeMap<i16, LocalContainerSlotAccess>> {
    slots
        .iter()
        .map(|slot| {
            local_container_slot_access(
                ctx,
                container_id,
                menu_type_id,
                mount_rules,
                data_values,
                slot.slot,
                &slot.item,
                stack_to_place,
            )
            .map(|access| (slot.slot, access))
        })
        .collect()
}

fn local_inventory_menu_slot_access(
    ctx: InventoryCtx<'_>,
    slot_num: i16,
    slot_item: &ProtocolItemStackSummary,
    stack_to_place: &ProtocolItemStackSummary,
    item_max_stack_size: i32,
) -> Option<LocalContainerSlotAccess> {
    if slot_num == 0 {
        return Some(LocalContainerSlotAccess::crafting_result(
            true,
            item_max_stack_size,
        ));
    }
    let expected_slot = match slot_num {
        5 => EquipmentSlot::Head,
        6 => EquipmentSlot::Chest,
        7 => EquipmentSlot::Legs,
        8 => EquipmentSlot::Feet,
        _ => return Some(LocalContainerSlotAccess::unrestricted(item_max_stack_size)),
    };
    let may_place = entity_stack_equipment_slot(
        ctx,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
        stack_to_place,
        expected_slot,
    )? == Some(expected_slot);
    let may_pickup = if ctx
        .local_player
        .abilities
        .is_some_and(|abilities| abilities.instabuild)
        || item_stack_is_empty(slot_item)
    {
        true
    } else {
        let mut prevented = false;
        for enchantment in &slot_item.component_patch.enchantments {
            if ctx
                .registries
                .enchantment_prevents_armor_change(enchantment.holder_id)?
            {
                prevented = true;
                break;
            }
        }
        !prevented
    };
    Some(LocalContainerSlotAccess {
        may_place,
        may_pickup,
        max_stack_size: item_max_stack_size.min(1),
        takes_full_stack: false,
    })
}

fn local_mount_slot_access(
    ctx: InventoryCtx<'_>,
    rules: LocalMountSlotRules,
    slot_num: i16,
    slot_item: &ProtocolItemStackSummary,
    stack_to_place: &ProtocolItemStackSummary,
    item_max_stack_size: i32,
) -> Option<LocalContainerSlotAccess> {
    if (MOUNT_INVENTORY_START..rules.player_start + MOUNT_PLAYER_SLOT_COUNT).contains(&slot_num) {
        return Some(LocalContainerSlotAccess::unrestricted(item_max_stack_size));
    }
    let expected_slot = match slot_num {
        MOUNT_SADDLE_SLOT if rules.equipment_slots.saddle => EquipmentSlot::Saddle,
        MOUNT_BODY_ARMOR_SLOT if rules.equipment_slots.body.is_some() => EquipmentSlot::Body,
        MOUNT_SADDLE_SLOT | MOUNT_BODY_ARMOR_SLOT => {
            return Some(LocalContainerSlotAccess::input(false, 1));
        }
        _ => return None,
    };
    let may_place = mount_stack_equipment_slot(ctx, rules, stack_to_place, expected_slot)?
        == Some(expected_slot);
    let may_pickup = if ctx
        .local_player
        .abilities
        .is_some_and(|abilities| abilities.instabuild)
        || item_stack_is_empty(slot_item)
    {
        true
    } else {
        let mut prevented = false;
        for enchantment in &slot_item.component_patch.enchantments {
            if ctx
                .registries
                .enchantment_prevents_armor_change(enchantment.holder_id)?
            {
                prevented = true;
                break;
            }
        }
        !prevented
    };
    Some(LocalContainerSlotAccess {
        may_place,
        may_pickup,
        max_stack_size: item_max_stack_size.min(1),
        takes_full_stack: false,
    })
}

fn mount_stack_equipment_slot(
    ctx: InventoryCtx<'_>,
    rules: LocalMountSlotRules,
    stack: &ProtocolItemStackSummary,
    expected_slot: EquipmentSlot,
) -> Option<Option<EquipmentSlot>> {
    entity_stack_equipment_slot(ctx, rules.entity_type_id, stack, expected_slot)
}

fn entity_stack_equipment_slot(
    ctx: InventoryCtx<'_>,
    entity_type_id: i32,
    stack: &ProtocolItemStackSummary,
    expected_slot: EquipmentSlot,
) -> Option<Option<EquipmentSlot>> {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
    {
        return Some(None);
    }
    if !ctx.items.item_has_effective_component(
        stack.item_id,
        &stack.component_patch,
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
    ) {
        return Some(None);
    }
    let equippable = ctx
        .items
        .component_source(
            stack.item_id,
            &stack.component_patch,
            VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
        )
        .equippable
        .as_ref()?;
    let slot = vanilla_equipment_slot_by_id(equippable.slot_id)?.slot;
    let allowed = match equippable.allowed_entities.as_ref() {
        None => true,
        Some(allowed) => match allowed.tag.as_deref() {
            Some(tag) => ctx
                .registries
                .tags
                .get("minecraft:entity_type")?
                .tags
                .get(tag)?
                .contains(&entity_type_id),
            None => allowed.registry_ids.contains(&entity_type_id),
        },
    };
    Some((allowed && slot == expected_slot).then_some(slot))
}

pub(super) fn item_stack_is_damageable_or_enchanted(
    stack: &ProtocolItemStackSummary,
    profiles: &ItemProfiles,
) -> bool {
    if item_stack_is_empty(stack) {
        return false;
    }
    let patch = &stack.component_patch;
    ItemProfiles::damage_state(Some(profiles), stack.item_id, patch).is_some()
        || !patch.enchantments.is_empty()
        || !patch.stored_enchantments.is_empty()
}

pub(super) fn item_stack_is_non_empty(stack: &ProtocolItemStackSummary) -> bool {
    stack.item_id.is_some() && stack.count > 0
}

pub(super) fn item_stack_has_map_id(stack: &ProtocolItemStackSummary) -> bool {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID)
    {
        return false;
    }

    stack.component_patch.map_id.is_some()
        || stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_MAP_ID_TYPE_ID)
}

pub(super) fn item_stack_attack_range(
    stack: &ProtocolItemStackSummary,
    items: &crate::ItemProfiles,
) -> Option<ItemAttackRange> {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID)
    {
        return None;
    }

    items
        .component_source(
            stack.item_id,
            &stack.component_patch,
            VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
        )
        .attack_range
}

pub(super) fn item_stack_minimum_attack_charge(
    stack: &ProtocolItemStackSummary,
    items: &crate::ItemProfiles,
) -> f32 {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID)
    {
        return 0.0;
    }

    items
        .component_source(
            stack.item_id,
            &stack.component_patch,
            VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        )
        .minimum_attack_charge_bits
        .map(f32::from_bits)
        .unwrap_or(0.0)
}

pub(super) fn item_stack_swing_animation(
    stack: &ProtocolItemStackSummary,
    items: &crate::ItemProfiles,
) -> ProtocolSwingAnimationSummary {
    if item_stack_is_empty(stack)
        || stack
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID)
    {
        return default_swing_animation();
    }

    items
        .component_source(
            stack.item_id,
            &stack.component_patch,
            VANILLA_DATA_COMPONENT_SWING_ANIMATION_TYPE_ID,
        )
        .swing_animation
        .unwrap_or_else(default_swing_animation)
}

fn default_swing_animation() -> ProtocolSwingAnimationSummary {
    ProtocolSwingAnimationSummary {
        animation_type: bbb_protocol::packets::SwingAnimationTypeSummary::Whack,
        duration: ATTACK_SWING_DURATION,
    }
}

pub(super) fn item_stack_use_effects(
    stack: &ProtocolItemStackSummary,
    items: &crate::ItemProfiles,
) -> Option<ItemUseEffects> {
    if item_stack_is_empty(stack) {
        return None;
    }

    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID)
    {
        return Some(ItemUseEffects::default());
    }

    Some(
        items
            .component_source(
                stack.item_id,
                &stack.component_patch,
                VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
            )
            .use_effects
            .map(|effects| ItemUseEffects {
                can_sprint: effects.can_sprint,
                interact_vibrations: effects.interact_vibrations,
                speed_multiplier: effects.speed_multiplier,
            })
            .unwrap_or_default(),
    )
}

pub(super) fn inventory_menu_result_was_taken(
    before: Option<&ProtocolItemStackSummary>,
    after: &[ContainerSlot],
) -> bool {
    let Some(before_result) = before else {
        return false;
    };
    if item_stack_is_empty(before_result) {
        return false;
    }

    let Some(after_result) = container_slot_item(after, 0) else {
        return true;
    };
    if item_stack_is_empty(after_result) {
        return true;
    }
    !same_item_same_components(before_result, after_result)
        || after_result.count < before_result.count
}

pub(super) fn inventory_menu_result_click_requires_server_authority(
    container_id: i32,
    slot_num: i16,
    input: ProtocolContainerInput,
    slots: &[ContainerSlot],
    default_item_crafting_remainders_known: bool,
) -> bool {
    if container_id != INVENTORY_MENU_CONTAINER_ID
        || slot_num != 0
        || !matches!(
            input,
            ProtocolContainerInput::Pickup
                | ProtocolContainerInput::QuickMove
                | ProtocolContainerInput::Throw
        )
        || container_slot_item(slots, 0).is_none_or(item_stack_is_empty)
    {
        return false;
    }

    !inventory_menu_non_empty_crafting_slot_nums(slots).is_empty()
        && inventory_menu_predictable_input_slot_nums(slots, default_item_crafting_remainders_known)
            .is_none()
}

pub(super) fn apply_inventory_menu_result_take_side_effects(
    slots: &mut [ContainerSlot],
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
    selected_hotbar_slot: u8,
    profiles: &ItemProfiles,
) {
    let input_slot_nums = inventory_menu_non_empty_crafting_slot_nums(slots);
    apply_crafting_result_take_side_effects(
        INVENTORY_MENU_CONTAINER_ID,
        slots,
        &input_slot_nums,
        default_item_crafting_remainders,
        PlayerInventoryAddSlots::inventory_menu(),
        None,
        selected_hotbar_slot,
        profiles,
    )
}

pub(super) fn inventory_menu_non_empty_crafting_slot_nums(slots: &[ContainerSlot]) -> Vec<i16> {
    non_empty_slot_nums(slots, 1, 5)
}

pub(super) fn non_empty_slot_nums(
    slots: &[ContainerSlot],
    start_slot: i16,
    end_slot: i16,
) -> Vec<i16> {
    (start_slot..end_slot)
        .filter(|slot_num| {
            slots
                .iter()
                .find(|slot| slot.slot == *slot_num)
                .is_some_and(|slot| item_stack_is_non_empty(&slot.item))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PlayerInventoryAddSlots {
    main_start: i16,
    main_end: i16,
    hotbar_start: i16,
    hotbar_end: i16,
    offhand_slot: Option<i16>,
}

impl PlayerInventoryAddSlots {
    fn inventory_menu() -> Self {
        Self {
            main_start: INVENTORY_MENU_MAIN_START,
            main_end: INVENTORY_MENU_MAIN_END,
            hotbar_start: INVENTORY_MENU_HOTBAR_START,
            hotbar_end: INVENTORY_MENU_HOTBAR_END,
            offhand_slot: Some(INVENTORY_MENU_OFFHAND_SLOT),
        }
    }

    pub(super) fn crafting_menu() -> Self {
        Self {
            main_start: CRAFTING_MENU_PLAYER_MAIN_START,
            main_end: CRAFTING_MENU_PLAYER_MAIN_END,
            hotbar_start: CRAFTING_MENU_HOTBAR_START,
            hotbar_end: CRAFTING_MENU_HOTBAR_END,
            offhand_slot: None,
        }
    }

    pub(super) fn for_container(container: &ContainerState) -> Option<Self> {
        if container.container_id == INVENTORY_MENU_CONTAINER_ID {
            return Some(Self::inventory_menu());
        }
        let start = slot_state::player_section_start(container)?;
        Some(Self {
            main_start: start,
            main_end: start + MOUNT_PLAYER_MAIN_SLOT_COUNT,
            hotbar_start: start + MOUNT_PLAYER_MAIN_SLOT_COUNT,
            hotbar_end: start + GENERIC_CONTAINER_PLAYER_SLOT_COUNT,
            offhand_slot: None,
        })
    }

    pub(super) fn swap_slot(self, button: i8) -> Option<i16> {
        match button {
            0..=8 => Some(self.hotbar_start + i16::from(button)),
            value if i32::from(value) == VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT => self.offhand_slot,
            _ => None,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_crafting_result_take_side_effects(
    container_id: i32,
    slots: &mut [ContainerSlot],
    input_slot_nums: &[i16],
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
    player_inventory_add_slots: PlayerInventoryAddSlots,
    mut external_offhand: Option<&mut ProtocolItemStackSummary>,
    selected_hotbar_slot: u8,
    profiles: &ItemProfiles,
) {
    for slot_num in input_slot_nums {
        let Some(slot_index) = slots.iter().position(|slot| slot.slot == *slot_num) else {
            continue;
        };
        if item_stack_is_empty(&slots[slot_index].item) {
            continue;
        }
        let input_before = slots[slot_index].item.clone();
        let remainder = input_before
            .item_id
            .and_then(|item_id| default_item_crafting_remainders.get(&item_id).copied())
            .map(simple_item_stack);

        slots[slot_index].item.count -= 1;
        normalize_item_stack(&mut slots[slot_index].item);
        if let Some(mut replacement) = remainder {
            if item_stack_is_empty(&slots[slot_index].item) {
                slots[slot_index].item = replacement;
            } else if same_item_same_components(&slots[slot_index].item, &replacement) {
                replacement.count += slots[slot_index].item.count;
                normalize_item_stack(&mut replacement);
                slots[slot_index].item = replacement;
            } else {
                add_item_stack_to_player_inventory(
                    container_id,
                    slots,
                    &mut replacement,
                    player_inventory_add_slots,
                    external_offhand.as_deref_mut(),
                    selected_hotbar_slot,
                    profiles,
                );
            }
        }
        slots[slot_index].normalize_selection();
    }
}

pub(super) fn add_item_stack_to_player_inventory(
    container_id: i32,
    slots: &mut [ContainerSlot],
    moving: &mut ProtocolItemStackSummary,
    player_inventory_add_slots: PlayerInventoryAddSlots,
    external_offhand: Option<&mut ProtocolItemStackSummary>,
    selected_hotbar_slot: u8,
    profiles: &ItemProfiles,
) -> bool {
    let mut changed = false;
    let damaged =
        ItemProfiles::damage_state(Some(profiles), moving.item_id, &moving.component_patch)
            .is_some_and(|(damage, _)| damage > 0);
    if !damaged && profiles.item_stack_max_stack_size(moving) > 1 {
        let selected_hotbar_slot =
            player_inventory_add_slots.hotbar_start + i16::from(selected_hotbar_slot.min(8));
        changed |= merge_item_stack_into_container_slot(
            container_id,
            slots,
            moving,
            selected_hotbar_slot,
            profiles,
        );
        if let Some(offhand_slot) = player_inventory_add_slots.offhand_slot {
            changed |= merge_item_stack_into_container_slot(
                container_id,
                slots,
                moving,
                offhand_slot,
                profiles,
            );
        } else if let Some(offhand) = external_offhand {
            changed |= merge_item_stack_into_slot(moving, offhand, |item| {
                profiles.item_stack_max_stack_size(item)
            });
        }
        for dest_slot in player_inventory_remaining_merge_slot_order(
            player_inventory_add_slots,
            selected_hotbar_slot,
        ) {
            if item_stack_is_empty(moving) {
                break;
            }
            changed |= merge_item_stack_into_container_slot(
                container_id,
                slots,
                moving,
                dest_slot,
                profiles,
            );
        }
    }

    if item_stack_is_non_empty(moving) {
        for dest_slot in player_inventory_free_slot_order(player_inventory_add_slots) {
            let Some(dest_index) = slots.iter().position(|slot| slot.slot == dest_slot) else {
                continue;
            };
            if item_stack_is_non_empty(&slots[dest_index].item) {
                continue;
            }
            let max_stack_size =
                container_slot_max_stack_size(container_id, dest_slot, moving, profiles);
            let amount = moving.count.min(max_stack_size);
            if amount <= 0 {
                continue;
            }
            let slot = &mut slots[dest_index];
            move_stack_count(moving, &mut slot.item, amount);
            slot.normalize_selection();
            changed = true;
            break;
        }
    }

    changed
}

pub(super) fn merge_item_stack_into_container_slot(
    container_id: i32,
    slots: &mut [ContainerSlot],
    moving: &mut ProtocolItemStackSummary,
    dest_slot: i16,
    profiles: &ItemProfiles,
) -> bool {
    if item_stack_is_empty(moving) {
        return false;
    }
    let Some(dest_index) = slots.iter().position(|slot| slot.slot == dest_slot) else {
        return false;
    };
    let slot = &mut slots[dest_index];
    if !merge_item_stack_into_slot(moving, &mut slot.item, |item| {
        container_slot_max_stack_size(container_id, dest_slot, item, profiles)
    }) {
        return false;
    }
    slot.normalize_selection();
    true
}

pub(super) fn merge_item_stack_into_slot(
    moving: &mut ProtocolItemStackSummary,
    dest: &mut ProtocolItemStackSummary,
    max_stack_size: impl FnOnce(&ProtocolItemStackSummary) -> i32,
) -> bool {
    if item_stack_is_empty(moving)
        || item_stack_is_empty(dest)
        || !same_item_same_components(moving, dest)
    {
        return false;
    }
    let moved = moving.count.min((max_stack_size(dest) - dest.count).max(0));
    if moved <= 0 {
        return false;
    }
    dest.count += moved;
    moving.count -= moved;
    normalize_item_stack(moving);
    true
}

fn player_inventory_remaining_merge_slot_order(
    slots: PlayerInventoryAddSlots,
    selected_hotbar_slot: i16,
) -> impl Iterator<Item = i16> {
    (slots.hotbar_start..slots.hotbar_end)
        .filter(move |slot| *slot != selected_hotbar_slot)
        .chain(slots.main_start..slots.main_end)
}

fn player_inventory_free_slot_order(slots: PlayerInventoryAddSlots) -> impl Iterator<Item = i16> {
    (slots.hotbar_start..slots.hotbar_end).chain(slots.main_start..slots.main_end)
}

pub(super) fn inventory_menu_predictable_input_slot_nums(
    slots: &[ContainerSlot],
    default_item_crafting_remainders_known: bool,
) -> Option<Vec<i16>> {
    crafting_result_predictable_input_slot_nums(slots, 1, 5, default_item_crafting_remainders_known)
}

pub(super) fn simple_item_stack(item_id: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: ProtocolDataComponentPatchSummary::default().into(),
    }
}

pub(super) fn crafting_result_predictable_input_slot_nums(
    slots: &[ContainerSlot],
    start_slot: i16,
    end_slot: i16,
    default_item_crafting_remainders_known: bool,
) -> Option<Vec<i16>> {
    if !default_item_crafting_remainders_known {
        return None;
    }
    let input_slot_nums = non_empty_slot_nums(slots, start_slot, end_slot);
    let can_predict = !input_slot_nums.is_empty()
        && input_slot_nums.iter().all(|slot_num| {
            slots
                .iter()
                .find(|slot| slot.slot == *slot_num)
                .is_some_and(|slot| {
                    item_stack_is_non_empty(&slot.item)
                        && slot.item.item_id.is_some()
                        && slot.item.count > 0
                })
        });
    can_predict.then_some(input_slot_nums)
}

pub(super) fn container_slot_item(
    slots: &[ContainerSlot],
    slot_num: i16,
) -> Option<&ProtocolItemStackSummary> {
    slots
        .iter()
        .find(|slot| slot.slot == slot_num)
        .map(|slot| &slot.item)
}

pub(super) fn apply_pickup_click_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    slot_num: i16,
    button_num: i8,
    slot_access: Option<LocalContainerSlotAccess>,
    profiles: &ItemProfiles,
) {
    match slot_num {
        -999 => {
            apply_outside_pickup_click(cursor, button_num);
        }
        slot_num if slot_num >= 0 => {
            let Some(slot) = slots.iter_mut().find(|slot| slot.slot == slot_num) else {
                return;
            };
            let Some(slot_access) = slot_access else {
                return;
            };
            apply_slot_pickup_click(&mut slot.item, cursor, button_num, slot_access, profiles);
            slot.local_selected_bundle_item_index = normalize_local_selected_bundle_item_index(
                slot.local_selected_bundle_item_index,
                &slot.item,
            );
        }
        _ => {}
    }
}

pub(super) fn advance_quick_craft(
    ctx: InventoryCtx<'_>,
    container: &ContainerState,
    mount_rules: Option<LocalMountSlotRules>,
    slot: Option<ContainerSlotRef<'_>>,
    cursor: &ProtocolItemStackSummary,
    quick_craft: &mut LocalQuickCraftState,
    button_num: i8,
) -> bool {
    let expected_status = quick_craft.status;
    quick_craft.status = quickcraft_header(button_num);
    if (expected_status != QUICKCRAFT_HEADER_CONTINUE
        || quick_craft.status != QUICKCRAFT_HEADER_END)
        && expected_status != quick_craft.status
    {
        quick_craft.reset();
        return false;
    }
    if item_stack_is_empty(cursor) {
        quick_craft.reset();
        return false;
    }

    let instabuild = ctx
        .local_player
        .abilities
        .is_some_and(|abilities| abilities.instabuild);
    match quick_craft.status {
        QUICKCRAFT_HEADER_START => {
            quick_craft.quickcraft_type = quickcraft_type(button_num);
            if local_quickcraft_type_is_valid(quick_craft.quickcraft_type, instabuild) {
                quick_craft.status = QUICKCRAFT_HEADER_CONTINUE;
                quick_craft.slots.clear();
            } else {
                quick_craft.reset();
            }
        }
        QUICKCRAFT_HEADER_CONTINUE => {
            let Some(slot) = slot else {
                return false;
            };
            let Some(slot_access) = local_container_slot_access(
                ctx,
                container.container_id,
                container.menu_type_id,
                mount_rules,
                &container.data_values,
                slot.slot,
                slot.item,
                cursor,
            ) else {
                return false;
            };
            if quick_craft_slot_can_accept(slot.item, cursor, slot_access, ctx.items)
                && (quick_craft.quickcraft_type == QUICKCRAFT_TYPE_CLONE
                    || cursor.count > quick_craft.slots.len() as i32)
                && !quick_craft.slots.contains(&slot.slot)
            {
                quick_craft.slots.push(slot.slot);
            }
        }
        QUICKCRAFT_HEADER_END => return true,
        _ => quick_craft.reset(),
    }
    false
}

pub(super) fn apply_clone_click_to_slots(
    slots: &[ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    slot_num: i16,
    instabuild: bool,
    profiles: &ItemProfiles,
) {
    if !instabuild || !item_stack_is_empty(cursor) || slot_num < 0 {
        return;
    }
    let Some(slot_item) = container_slot_item(slots, slot_num) else {
        return;
    };
    if item_stack_is_empty(slot_item) {
        return;
    }

    *cursor = slot_item.clone();
    cursor.count = profiles.item_stack_max_stack_size(slot_item);
    normalize_item_stack(cursor);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn finish_quick_craft(
    ctx: InventoryCtx<'_>,
    container_id: i32,
    menu_type_id: Option<i32>,
    mount_rules: Option<LocalMountSlotRules>,
    data_values: &[ContainerDataValue],
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    quick_craft: &mut LocalQuickCraftState,
    profiles: &ItemProfiles,
) {
    let selected_slots = quick_craft.slots.clone();
    if selected_slots.is_empty() {
        quick_craft.reset();
        return;
    }

    let quickcraft_type = quick_craft.quickcraft_type;
    if selected_slots.len() == 1 {
        quick_craft.reset();
        let slot_num = selected_slots[0];
        let Some(slot_access) = local_container_slot_access(
            ctx,
            container_id,
            menu_type_id,
            mount_rules,
            data_values,
            slot_num,
            container_slot_item(slots, slot_num).unwrap_or(&ProtocolItemStackSummary::empty()),
            cursor,
        ) else {
            return;
        };
        if let Some(slot) = slots.iter_mut().find(|slot| slot.slot == slot_num) {
            let slot_item_snapshot = slot.item.clone();
            if matches!(
                try_apply_bundle_pickup_override(
                    slot,
                    cursor,
                    quickcraft_type,
                    slot_access,
                    profiles,
                    |candidate| {
                        local_container_slot_access(
                            ctx,
                            container_id,
                            menu_type_id,
                            mount_rules,
                            data_values,
                            slot_num,
                            &slot_item_snapshot,
                            candidate,
                        )
                    },
                ),
                Some((true, _)) | None
            ) {
                return;
            }
        }
        apply_pickup_click_to_slots(
            slots,
            cursor,
            slot_num,
            quickcraft_type,
            Some(slot_access),
            profiles,
        );
        return;
    }
    let instabuild = ctx
        .local_player
        .abilities
        .is_some_and(|abilities| abilities.instabuild);
    if !local_quickcraft_type_is_valid(quickcraft_type, instabuild) {
        quick_craft.reset();
        return;
    }

    let source = cursor.clone();
    if item_stack_is_empty(&source) {
        quick_craft.reset();
        return;
    }

    let slot_count = selected_slots.len() as i32;
    let mut remaining = cursor.count;
    for selected_slot in selected_slots {
        if quickcraft_type != QUICKCRAFT_TYPE_CLONE && cursor.count < slot_count {
            continue;
        }
        let Some(slot_index) = slots.iter().position(|slot| slot.slot == selected_slot) else {
            continue;
        };
        let Some(slot_access) = local_container_slot_access(
            ctx,
            container_id,
            menu_type_id,
            mount_rules,
            data_values,
            selected_slot,
            &slots[slot_index].item,
            cursor,
        ) else {
            continue;
        };
        if !quick_craft_slot_can_accept(&slots[slot_index].item, cursor, slot_access, profiles) {
            continue;
        }

        let carry = if item_stack_is_empty(&slots[slot_index].item) {
            0
        } else {
            slots[slot_index].item.count
        };
        let max_size = profiles
            .item_stack_max_stack_size(&source)
            .min(slot_access.max_stack_size);
        let place_count = quickcraft_place_count(slot_count, quickcraft_type, &source, profiles);
        let new_count = (place_count + carry).min(max_size);
        remaining -= new_count - carry;

        let mut replacement = source.clone();
        replacement.count = new_count;
        normalize_item_stack(&mut replacement);
        slots[slot_index].item = replacement;
        slots[slot_index].normalize_selection();
    }

    cursor.count = remaining;
    normalize_item_stack(cursor);
    quick_craft.reset();
}

pub(super) fn quick_craft_slot_can_accept(
    slot_item: &ProtocolItemStackSummary,
    cursor: &ProtocolItemStackSummary,
    slot_access: LocalContainerSlotAccess,
    profiles: &ItemProfiles,
) -> bool {
    if !slot_access.may_place || slot_access.max_stack_size <= 0 {
        return false;
    }
    if item_stack_is_empty(slot_item) {
        return true;
    }
    same_item_same_components(slot_item, cursor)
        && slot_item.count <= profiles.item_stack_max_stack_size(cursor)
}

pub(super) fn quickcraft_place_count(
    slot_count: i32,
    quickcraft_type: i8,
    source: &ProtocolItemStackSummary,
    profiles: &ItemProfiles,
) -> i32 {
    match quickcraft_type {
        QUICKCRAFT_TYPE_CHARITABLE => source.count / slot_count,
        QUICKCRAFT_TYPE_GREEDY => 1,
        QUICKCRAFT_TYPE_CLONE => profiles.item_stack_max_stack_size(source),
        _ => source.count,
    }
}

/// `AbstractContainerMenu.isValidQuickcraftType` (`AbstractContainerMenu.java:714-720`).
pub(super) fn local_quickcraft_type_is_valid(quickcraft_type: i8, instabuild: bool) -> bool {
    matches!(
        quickcraft_type,
        QUICKCRAFT_TYPE_CHARITABLE | QUICKCRAFT_TYPE_GREEDY
    ) || (quickcraft_type == QUICKCRAFT_TYPE_CLONE && instabuild)
}

pub(super) fn quickcraft_header(mask: i8) -> i8 {
    mask & 3
}

pub(super) fn quickcraft_type(mask: i8) -> i8 {
    (mask >> 2) & 3
}

pub(super) fn apply_throw_click_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &ProtocolItemStackSummary,
    slot_num: i16,
    button_num: i8,
    slot_access: Option<LocalContainerSlotAccess>,
) {
    if !item_stack_is_empty(cursor) || slot_num < 0 || !matches!(button_num, 0 | 1) {
        return;
    }
    let Some(slot) = slots.iter_mut().find(|slot| slot.slot == slot_num) else {
        return;
    };
    let Some(access) = slot_access else {
        return;
    };
    if item_stack_is_empty(&slot.item) || !access.may_pickup {
        return;
    }
    // `ResultSlot` uses `ResultContainer.removeItem` (`ResultContainer.java:37-39`),
    // which takes the entire crafted stack regardless of `button_num`.
    if button_num == 0 && !access.takes_full_stack {
        slot.item.count -= 1;
        normalize_item_stack(&mut slot.item);
    } else {
        slot.item = ProtocolItemStackSummary::empty();
    }
    slot.normalize_selection();
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_pickup_all_to_slots(
    slots: &mut [ContainerSlot],
    cursor: &mut ProtocolItemStackSummary,
    slot_num: i16,
    button_num: i8,
    slot_access: &BTreeMap<i16, LocalContainerSlotAccess>,
    container_id: i32,
    menu_type_id: Option<i32>,
    profiles: &ItemProfiles,
) {
    if slot_num < 0 || item_stack_is_empty(cursor) {
        return;
    }
    let Some(clicked_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    let clicked_may_pickup = slot_access
        .get(&slot_num)
        .is_some_and(|access| access.may_pickup);
    if !item_stack_is_empty(&slots[clicked_index].item) && clicked_may_pickup {
        return;
    }

    let max_stack_size = profiles.item_stack_max_stack_size(cursor);
    if cursor.count >= max_stack_size {
        return;
    }

    let mut indices = (0..slots.len()).collect::<Vec<_>>();
    if button_num != 0 {
        indices.reverse();
    }

    for pass in 0..2 {
        for index in indices.iter().copied() {
            if cursor.count >= max_stack_size {
                return;
            }
            let slot = &mut slots[index];
            if item_stack_is_empty(&slot.item) || !same_item_same_components(&slot.item, cursor) {
                continue;
            }
            if !slot_access
                .get(&slot.slot)
                .is_some_and(|access| access.may_pickup)
                || !menu_slot_can_take_item_for_pick_all(container_id, menu_type_id, slot.slot)
            {
                continue;
            }
            if pass == 0 && slot.item.count == profiles.item_stack_max_stack_size(&slot.item) {
                continue;
            }

            let moved = slot.item.count.min(max_stack_size - cursor.count);
            if moved <= 0 {
                continue;
            }
            slot.item.count -= moved;
            cursor.count += moved;
            normalize_item_stack(&mut slot.item);
            slot.normalize_selection();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_swap_click_to_slots(
    container_id: i32,
    player_slots: PlayerInventoryAddSlots,
    slots: &mut [ContainerSlot],
    slot_num: i16,
    button_num: i8,
    target_access: LocalContainerSlotAccess,
    external_offhand: Option<&mut ProtocolItemStackSummary>,
    selected_hotbar_slot: u8,
    profiles: &ItemProfiles,
) {
    // Vanilla `AbstractContainerMenu.doClick` (`AbstractContainerMenu.java:471-507`)
    // executes `SWAP` even while carrying an item on the cursor.
    if slot_num < 0 {
        return;
    }
    let Some(source_slot_num) = player_slots.swap_slot(button_num) else {
        return;
    };
    if source_slot_num == slot_num {
        return;
    }
    let Some(source_index) = slots.iter().position(|slot| slot.slot == source_slot_num) else {
        return;
    };
    let Some(target_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };

    let source_item = slots[source_index].item.clone();
    let target_item = slots[target_index].item.clone();
    if item_stack_is_empty(&source_item) && item_stack_is_empty(&target_item) {
        return;
    }

    if item_stack_is_empty(&source_item) {
        if !target_access.may_pickup {
            return;
        }
        slots[source_index].item = target_item;
        slots[target_index].item = ProtocolItemStackSummary::empty();
        slots[source_index].normalize_selection();
        slots[target_index].normalize_selection();
        return;
    }

    let target_max_stack_size = target_access.max_stack_size;
    if !target_access.may_place || target_max_stack_size <= 0 {
        return;
    }

    if item_stack_is_empty(&target_item) {
        move_between_container_slots(slots, source_index, target_index, target_max_stack_size);
        return;
    }

    let source_max_stack_size =
        container_slot_max_stack_size(container_id, source_slot_num, &target_item, profiles);
    if source_max_stack_size <= 0 {
        return;
    }

    if !target_access.may_pickup {
        return;
    }
    if source_item.count <= target_max_stack_size && target_item.count <= source_max_stack_size {
        slots[source_index].item = target_item;
        slots[target_index].item = source_item;
        slots[source_index].normalize_selection();
        slots[target_index].normalize_selection();
        return;
    }

    let moved = source_item.count.min(target_max_stack_size);
    let mut target_replacement = source_item.clone();
    target_replacement.count = moved;
    let mut source_remainder = source_item;
    source_remainder.count -= moved;
    normalize_item_stack(&mut source_remainder);
    slots[source_index].item = source_remainder;
    slots[target_index].item = target_replacement;
    slots[source_index].normalize_selection();
    slots[target_index].normalize_selection();

    // `AbstractContainerMenu.java:498`: displaced target item goes through `inventory.add`.
    let mut displaced = target_item;
    add_item_stack_to_player_inventory(
        container_id,
        slots,
        &mut displaced,
        player_slots,
        external_offhand,
        selected_hotbar_slot,
        profiles,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_external_offhand_swap_to_slots(
    container_id: i32,
    player_slots: Option<PlayerInventoryAddSlots>,
    slots: &mut [ContainerSlot],
    offhand: &mut ProtocolItemStackSummary,
    slot_num: i16,
    target_access: LocalContainerSlotAccess,
    selected_hotbar_slot: u8,
    profiles: &ItemProfiles,
) {
    if slot_num < 0 {
        return;
    }
    let Some(target_index) = slots.iter().position(|slot| slot.slot == slot_num) else {
        return;
    };
    let source_item = offhand.clone();
    let target_item = slots[target_index].item.clone();
    if item_stack_is_empty(&source_item) && item_stack_is_empty(&target_item) {
        return;
    }

    if item_stack_is_empty(&source_item) {
        if !target_access.may_pickup {
            return;
        }
        *offhand = target_item;
        slots[target_index].item = ProtocolItemStackSummary::empty();
        slots[target_index].normalize_selection();
        return;
    }
    if !target_access.may_place || target_access.max_stack_size <= 0 {
        return;
    }
    if item_stack_is_empty(&target_item) {
        let moved = source_item.count.min(target_access.max_stack_size);
        slots[target_index].item = source_item.clone();
        slots[target_index].item.count = moved;
        offhand.count -= moved;
        normalize_item_stack(offhand);
        slots[target_index].normalize_selection();
        return;
    }
    if !target_access.may_pickup {
        return;
    }
    if source_item.count <= target_access.max_stack_size {
        *offhand = target_item;
        slots[target_index].item = source_item;
        slots[target_index].normalize_selection();
        return;
    }

    let mut target_replacement = source_item;
    target_replacement.count = target_access.max_stack_size;
    offhand.count -= target_access.max_stack_size;
    slots[target_index].item = target_replacement;
    slots[target_index].normalize_selection();
    add_item_to_external_player_inventory(
        container_id,
        player_slots,
        slots,
        offhand,
        target_item,
        selected_hotbar_slot,
        profiles,
    );
}

fn add_item_to_external_player_inventory(
    container_id: i32,
    player_slots: Option<PlayerInventoryAddSlots>,
    slots: &mut [ContainerSlot],
    offhand: &mut ProtocolItemStackSummary,
    mut moving: ProtocolItemStackSummary,
    selected_hotbar_slot: u8,
    profiles: &ItemProfiles,
) {
    let Some(player_inventory_add_slots) = player_slots else {
        return;
    };
    add_item_stack_to_player_inventory(
        container_id,
        slots,
        &mut moving,
        player_inventory_add_slots,
        Some(offhand),
        selected_hotbar_slot,
        profiles,
    );
}

pub(super) fn menu_slot_can_take_item_for_pick_all(
    container_id: i32,
    menu_type_id: Option<i32>,
    slot_num: i16,
) -> bool {
    if container_id == INVENTORY_MENU_CONTAINER_ID && slot_num == 0 {
        return false;
    }
    match menu_type_id {
        Some(VANILLA_MENU_TYPE_MERCHANT_ID) => false,
        Some(VANILLA_MENU_TYPE_CRAFTING_ID) => slot_num != CRAFTING_MENU_RESULT_SLOT,
        Some(VANILLA_MENU_TYPE_SMITHING_ID) => slot_num != SMITHING_RESULT_SLOT,
        Some(VANILLA_MENU_TYPE_STONECUTTER_ID) => slot_num != STONECUTTER_RESULT_SLOT,
        Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID) => slot_num != CARTOGRAPHY_TABLE_RESULT_SLOT,
        _ => true,
    }
}

pub(super) fn move_between_container_slots(
    slots: &mut [ContainerSlot],
    source_index: usize,
    target_index: usize,
    max_count: i32,
) {
    let mut source = slots[source_index].item.clone();
    let amount = source.count.min(max_count);
    let mut target = source.clone();
    target.count = amount;
    source.count -= amount;
    normalize_item_stack(&mut source);
    slots[source_index].item = source;
    slots[target_index].item = target;
    slots[source_index].normalize_selection();
    slots[target_index].normalize_selection();
}

/// `BlockItem.canFitInsideContainerItems` (`BlockItem.java:194-196`).
pub(super) fn item_can_fit_inside_container_items(item: &ProtocolItemStackSummary) -> bool {
    let Some(item_id) = item.item_id else {
        return true;
    };
    let Some(behavior) = vanilla_item_type(item_id).and_then(|entry| entry.block_item_behavior)
    else {
        return true;
    };
    bbb_protocol::block_types::vanilla_block_type(behavior.block_type_id)
        .and_then(|block| block.block_entity_type_id)
        != Some(bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SHULKER_BOX_ID)
}

pub(super) fn menu_type_has_unrestricted_slot_rules(menu_type_id: Option<i32>) -> bool {
    generic_9x_container_slot_count(menu_type_id).is_some()
        || generic_3x3_container_slot_count(menu_type_id).is_some()
        || hopper_container_slot_count(menu_type_id).is_some()
}

pub(super) fn menu_type_has_known_slot_rules(menu_type_id: Option<i32>) -> bool {
    menu_type_has_unrestricted_slot_rules(menu_type_id)
        || matches!(
            menu_type_id,
            Some(
                VANILLA_MENU_TYPE_CRAFTER_3X3_ID
                    | VANILLA_MENU_TYPE_ANVIL_ID
                    | VANILLA_MENU_TYPE_BEACON_ID
                    | VANILLA_MENU_TYPE_BLAST_FURNACE_ID
                    | VANILLA_MENU_TYPE_BREWING_STAND_ID
                    | VANILLA_MENU_TYPE_CRAFTING_ID
                    | VANILLA_MENU_TYPE_ENCHANTMENT_ID
                    | VANILLA_MENU_TYPE_FURNACE_ID
                    | VANILLA_MENU_TYPE_GRINDSTONE_ID
                    | VANILLA_MENU_TYPE_LECTERN_ID
                    | VANILLA_MENU_TYPE_LOOM_ID
                    | VANILLA_MENU_TYPE_MERCHANT_ID
                    | VANILLA_MENU_TYPE_SHULKER_BOX_ID
                    | VANILLA_MENU_TYPE_SMITHING_ID
                    | VANILLA_MENU_TYPE_SMOKER_ID
                    | VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID
                    | VANILLA_MENU_TYPE_STONECUTTER_ID
            )
        )
}

pub(super) fn generic_9x_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    let menu_type_id = menu_type_id?;
    (VANILLA_MENU_TYPE_GENERIC_9X1_ID..=VANILLA_MENU_TYPE_GENERIC_9X6_ID)
        .contains(&menu_type_id)
        .then_some((menu_type_id - VANILLA_MENU_TYPE_GENERIC_9X1_ID + 1) as i16)
        .map(|rows| rows * GENERIC_CONTAINER_SLOT_COUNT_PER_ROW)
}

pub(super) fn generic_3x3_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    (menu_type_id == Some(VANILLA_MENU_TYPE_GENERIC_3X3_ID))
        .then_some(GENERIC_3X3_CONTAINER_SLOT_COUNT)
}

pub(super) fn hopper_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    (menu_type_id == Some(VANILLA_MENU_TYPE_HOPPER_ID)).then_some(HOPPER_CONTAINER_SLOT_COUNT)
}

pub(super) fn shulker_box_container_slot_count(menu_type_id: Option<i32>) -> Option<i16> {
    (menu_type_id == Some(VANILLA_MENU_TYPE_SHULKER_BOX_ID))
        .then_some(SHULKER_BOX_CONTAINER_SLOT_COUNT)
}

pub(super) fn is_interactive_menu_result_slot(menu_type_id: Option<i32>, slot_num: i16) -> bool {
    matches!(
        (menu_type_id, slot_num),
        (Some(VANILLA_MENU_TYPE_ANVIL_ID), ANVIL_RESULT_SLOT)
            | (
                Some(VANILLA_MENU_TYPE_CRAFTING_ID),
                CRAFTING_MENU_RESULT_SLOT
            )
            | (
                Some(VANILLA_MENU_TYPE_CARTOGRAPHY_TABLE_ID),
                CARTOGRAPHY_TABLE_RESULT_SLOT
            )
            | (Some(VANILLA_MENU_TYPE_LOOM_ID), LOOM_RESULT_SLOT)
            | (
                Some(VANILLA_MENU_TYPE_GRINDSTONE_ID),
                GRINDSTONE_RESULT_SLOT
            )
            | (Some(VANILLA_MENU_TYPE_SMITHING_ID), SMITHING_RESULT_SLOT)
            | (Some(VANILLA_MENU_TYPE_MERCHANT_ID), MERCHANT_RESULT_SLOT)
            | (
                Some(VANILLA_MENU_TYPE_STONECUTTER_ID),
                STONECUTTER_RESULT_SLOT
            )
    )
}

/// Top-level gate before local click dispatch (`AbstractContainerMenu.doClick`,
/// `Slot.mayPickup`).
///
/// Interactive result slots handle deterministic `Pickup`, `QuickMove`, `Swap`,
/// and `Throw` locally and fail closed inside their per-menu handlers when
/// inputs or state require server authority; `PickupAll` clicking directly on
/// a result slot never takes from that clicked slot (`AbstractContainerMenu.java:538`),
/// and `NonInteractiveResultSlot` (`CrafterMenu`) rejects `mayPickup`/`mayPlace`
/// while preserving server-authoritative `Pickup`/`QuickMove` errors.
pub(super) fn menu_result_slot_requires_server_authority(
    menu_type_id: Option<i32>,
    slot_num: i16,
    input: ProtocolContainerInput,
) -> bool {
    if (menu_type_id, slot_num) == (Some(VANILLA_MENU_TYPE_CRAFTER_3X3_ID), CRAFTER_RESULT_SLOT) {
        return matches!(
            input,
            ProtocolContainerInput::Pickup | ProtocolContainerInput::QuickMove
        );
    }
    if !is_interactive_menu_result_slot(menu_type_id, slot_num) {
        return false;
    }
    !matches!(
        input,
        ProtocolContainerInput::Pickup
            | ProtocolContainerInput::QuickMove
            | ProtocolContainerInput::Swap
            | ProtocolContainerInput::Throw
            | ProtocolContainerInput::PickupAll
            | ProtocolContainerInput::Clone
            | ProtocolContainerInput::QuickCraft
    )
}

/// Menus whose `canTakeItemForPickAll` does not exclude their interactive result
/// slot (`AnvilMenu`, `LoomMenu`, `GrindstoneMenu`), so a `PickupAll` scan that
/// matches a takeable result stack would trigger `Slot.onTake` side effects.
pub(super) fn menu_result_slot_pickup_all_scan_requires_server_authority(
    menu_type_id: Option<i32>,
    slot_num: i16,
) -> bool {
    matches!(
        (menu_type_id, slot_num),
        (Some(VANILLA_MENU_TYPE_ANVIL_ID), ANVIL_RESULT_SLOT)
            | (Some(VANILLA_MENU_TYPE_LOOM_ID), LOOM_RESULT_SLOT)
            | (
                Some(VANILLA_MENU_TYPE_GRINDSTONE_ID),
                GRINDSTONE_RESULT_SLOT
            )
    )
}

pub(super) fn furnace_family_menu_type(menu_type_id: Option<i32>) -> Option<i32> {
    let menu_type_id = menu_type_id?;
    matches!(
        menu_type_id,
        VANILLA_MENU_TYPE_BLAST_FURNACE_ID
            | VANILLA_MENU_TYPE_FURNACE_ID
            | VANILLA_MENU_TYPE_SMOKER_ID
    )
    .then_some(menu_type_id)
}

pub(super) fn apply_inventory_menu_result_quick_move_to_slots(
    slots: &mut [ContainerSlot],
    default_item_crafting_remainders: &BTreeMap<i32, i32>,
    selected_hotbar_slot: u8,
    profiles: &ItemProfiles,
) -> bool {
    let Some(result_index) = slots.iter().position(|slot| slot.slot == 0) else {
        return true;
    };
    if item_stack_is_empty(&slots[result_index].item) {
        return true;
    }

    let input_slot_nums = inventory_menu_non_empty_crafting_slot_nums(slots);
    let mut candidate_slots = slots.to_vec();
    let mut moving = candidate_slots[result_index].item.clone();
    if !move_item_stack_to_slots(
        INVENTORY_MENU_CONTAINER_ID,
        &mut candidate_slots,
        result_index,
        &mut moving,
        INVENTORY_MENU_MAIN_START,
        INVENTORY_MENU_HOTBAR_END,
        true,
        profiles,
    ) {
        return true;
    }

    // Vanilla `InventoryMenu.quickMoveStack` (`InventoryMenu.java:155-157`) drops
    // any remaining crafted stack, then `onTake` consumes inputs and the server
    // repopulates the result slot while `AbstractContainerMenu.doClick` loops.
    candidate_slots[result_index].item = ProtocolItemStackSummary::empty();
    candidate_slots[result_index].normalize_selection();
    if !input_slot_nums.is_empty() {
        apply_crafting_result_take_side_effects(
            INVENTORY_MENU_CONTAINER_ID,
            &mut candidate_slots,
            &input_slot_nums,
            default_item_crafting_remainders,
            PlayerInventoryAddSlots::inventory_menu(),
            None,
            selected_hotbar_slot,
            profiles,
        );
        if !inventory_menu_non_empty_crafting_slot_nums(&candidate_slots).is_empty() {
            // Remaining ingredients may repopulate slot 0 for another loop
            // iteration using server-side recipe lookup; defer to server authority.
            return false;
        }
    }
    slots.clone_from_slice(&candidate_slots);
    true
}

pub(super) fn furnace_can_smelt(
    menu_type_id: Option<i32>,
    stack: &ProtocolItemStackSummary,
    recipe_property_sets: &BTreeMap<String, Vec<i32>>,
) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    let Some(property_set) = furnace_input_property_set(menu_type_id) else {
        return false;
    };
    recipe_property_sets
        .get(property_set)
        .is_some_and(|items| items.contains(&item_id))
}

pub(super) fn furnace_input_property_set(menu_type_id: Option<i32>) -> Option<&'static str> {
    match menu_type_id {
        Some(VANILLA_MENU_TYPE_BLAST_FURNACE_ID) => Some("minecraft:blast_furnace_input"),
        Some(VANILLA_MENU_TYPE_FURNACE_ID) => Some("minecraft:furnace_input"),
        Some(VANILLA_MENU_TYPE_SMOKER_ID) => Some("minecraft:smoker_input"),
        _ => None,
    }
}

pub(super) fn furnace_is_fuel(
    stack: &ProtocolItemStackSummary,
    furnace_fuel_item_ids: &BTreeSet<i32>,
) -> bool {
    stack
        .item_id
        .is_some_and(|item_id| furnace_fuel_item_ids.contains(&item_id))
}

pub(super) fn item_stack_in_item_tag(
    stack: &ProtocolItemStackSummary,
    item_tags: Option<&RegistryTagState>,
    tag: &str,
) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_tags
        .and_then(|registry| registry.tags.get(tag))
        .is_some_and(|entries| entries.contains(&item_id))
}

pub(super) fn item_stack_item_id_in_set(
    stack: &ProtocolItemStackSummary,
    item_ids: &BTreeSet<i32>,
) -> bool {
    stack
        .item_id
        .is_some_and(|item_id| item_ids.contains(&item_id))
}

pub(super) fn inventory_menu_quick_move_target_range(
    slot_num: i16,
    source_item: &ProtocolItemStackSummary,
    slots: &[ContainerSlot],
    profiles: &ItemProfiles,
) -> Option<(i16, i16, bool)> {
    match slot_num {
        0 => Some((INVENTORY_MENU_MAIN_START, INVENTORY_MENU_HOTBAR_END, true)),
        1..=8 => Some((INVENTORY_MENU_MAIN_START, INVENTORY_MENU_HOTBAR_END, false)),
        INVENTORY_MENU_MAIN_START..=35 => {
            inventory_menu_equipment_quick_move_target(source_item, slots, profiles).or(Some((
                INVENTORY_MENU_HOTBAR_START,
                INVENTORY_MENU_HOTBAR_END,
                false,
            )))
        }
        INVENTORY_MENU_HOTBAR_START..=44 => {
            inventory_menu_equipment_quick_move_target(source_item, slots, profiles).or(Some((
                INVENTORY_MENU_MAIN_START,
                INVENTORY_MENU_MAIN_END,
                false,
            )))
        }
        INVENTORY_MENU_OFFHAND_SLOT => {
            inventory_menu_equipment_quick_move_target(source_item, slots, profiles).or(Some((
                INVENTORY_MENU_MAIN_START,
                INVENTORY_MENU_HOTBAR_END,
                false,
            )))
        }
        _ => None,
    }
}

pub(super) fn inventory_menu_equipment_quick_move_target(
    source_item: &ProtocolItemStackSummary,
    slots: &[ContainerSlot],
    profiles: &ItemProfiles,
) -> Option<(i16, i16, bool)> {
    let target_slot = inventory_menu_equipment_slot(profiles.item_stack_equipment(source_item)?.0)?;
    if inventory_menu_slot_has_item(slots, target_slot) {
        return None;
    }
    Some((target_slot, target_slot + 1, false))
}

pub(super) fn inventory_menu_equipment_slot(equipment_slot: EquipmentSlot) -> Option<i16> {
    match equipment_slot {
        EquipmentSlot::Head => Some(5),
        EquipmentSlot::Chest => Some(6),
        EquipmentSlot::Legs => Some(7),
        EquipmentSlot::Feet => Some(8),
        EquipmentSlot::OffHand => Some(INVENTORY_MENU_OFFHAND_SLOT),
        EquipmentSlot::MainHand | EquipmentSlot::Body | EquipmentSlot::Saddle => None,
    }
}

pub(super) fn inventory_menu_slot_has_item(slots: &[ContainerSlot], slot_num: i16) -> bool {
    slots
        .iter()
        .find(|slot| slot.slot == slot_num)
        .is_some_and(|slot| item_stack_is_non_empty(&slot.item))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn move_item_stack_to_slots(
    container_id: i32,
    slots: &mut [ContainerSlot],
    source_index: usize,
    moving: &mut ProtocolItemStackSummary,
    start_slot: i16,
    end_slot: i16,
    backwards: bool,
    profiles: &ItemProfiles,
) -> bool {
    move_item_stack_to_slots_where(
        container_id,
        slots,
        source_index,
        moving,
        start_slot,
        end_slot,
        backwards,
        |_| true,
        profiles,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn move_item_stack_to_slots_where(
    container_id: i32,
    slots: &mut [ContainerSlot],
    source_index: usize,
    moving: &mut ProtocolItemStackSummary,
    start_slot: i16,
    end_slot: i16,
    backwards: bool,
    may_use_slot: impl FnMut(i16) -> bool,
    profiles: &ItemProfiles,
) -> bool {
    move_item_stack_to_slots_where_with_limit(
        container_id,
        slots,
        source_index,
        moving,
        start_slot,
        end_slot,
        backwards,
        may_use_slot,
        |_, _, max_stack_size| max_stack_size,
        profiles,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn move_item_stack_to_slots_where_with_limit(
    container_id: i32,
    slots: &mut [ContainerSlot],
    source_index: usize,
    moving: &mut ProtocolItemStackSummary,
    start_slot: i16,
    end_slot: i16,
    backwards: bool,
    mut may_use_slot: impl FnMut(i16) -> bool,
    mut slot_max_stack_size: impl FnMut(i16, &ProtocolItemStackSummary, i32) -> i32,
    profiles: &ItemProfiles,
) -> bool {
    let mut changed = false;
    if profiles.item_stack_max_stack_size(moving) > 1 {
        for dest_slot in quick_move_slot_ids(start_slot, end_slot, backwards) {
            if !may_use_slot(dest_slot) {
                continue;
            }
            if item_stack_is_empty(moving) {
                break;
            }
            let Some(dest_index) = slots.iter().position(|slot| slot.slot == dest_slot) else {
                continue;
            };
            if dest_index == source_index {
                continue;
            }
            let slot = &mut slots[dest_index];
            if !merge_item_stack_into_slot(moving, &mut slot.item, |item| {
                let base_max_stack_size =
                    container_slot_max_stack_size(container_id, dest_slot, item, profiles);
                slot_max_stack_size(dest_slot, item, base_max_stack_size).max(0)
            }) {
                continue;
            }
            slot.normalize_selection();
            changed = true;
        }
    }

    if !item_stack_is_empty(moving) {
        for dest_slot in quick_move_slot_ids(start_slot, end_slot, backwards) {
            if !may_use_slot(dest_slot) {
                continue;
            }
            let Some(dest_index) = slots.iter().position(|slot| slot.slot == dest_slot) else {
                continue;
            };
            if dest_index == source_index || !item_stack_is_empty(&slots[dest_index].item) {
                continue;
            }
            let base_max_stack_size =
                container_slot_max_stack_size(container_id, dest_slot, moving, profiles);
            let max_stack_size = slot_max_stack_size(dest_slot, moving, base_max_stack_size).max(0);
            let amount = moving.count.min(max_stack_size);
            if amount <= 0 {
                continue;
            }
            let slot = &mut slots[dest_index];
            move_stack_count(moving, &mut slot.item, amount);
            slot.normalize_selection();
            changed = true;
            break;
        }
    }

    changed
}

fn quick_move_slot_ids(
    start_slot: i16,
    end_slot: i16,
    backwards: bool,
) -> impl Iterator<Item = i16> {
    let mut slots = start_slot..end_slot;
    std::iter::from_fn(move || {
        if backwards {
            slots.next_back()
        } else {
            slots.next()
        }
    })
}

pub(super) fn apply_outside_pickup_click(cursor: &mut ProtocolItemStackSummary, button_num: i8) {
    if item_stack_is_empty(cursor) {
        return;
    }
    if button_num == 0 {
        *cursor = ProtocolItemStackSummary::empty();
    } else if button_num == 1 {
        cursor.count -= 1;
        normalize_item_stack(cursor);
    }
}

pub(super) fn apply_slot_pickup_click(
    slot: &mut ProtocolItemStackSummary,
    cursor: &mut ProtocolItemStackSummary,
    button_num: i8,
    access: LocalContainerSlotAccess,
    profiles: &ItemProfiles,
) {
    if !matches!(button_num, 0 | 1) {
        return;
    }

    if item_stack_is_empty(slot) {
        if item_stack_is_empty(cursor) || !access.may_place {
            return;
        }
        let amount = if button_num == 0 { cursor.count } else { 1 };
        let amount = amount.min(access.max_stack_size);
        move_stack_count(cursor, slot, amount);
        return;
    }

    if !access.may_pickup {
        return;
    } else if item_stack_is_empty(cursor) {
        let amount = if button_num == 0 || access.takes_full_stack {
            slot.count
        } else {
            (slot.count + 1) / 2
        };
        move_stack_count(slot, cursor, amount);
        return;
    }

    if access.may_place {
        if same_item_same_components(slot, cursor) {
            let amount = if button_num == 0 { cursor.count } else { 1 };
            let moved = amount.min((access.max_stack_size - slot.count).max(0));
            if moved > 0 {
                slot.count += moved;
                cursor.count -= moved;
                normalize_item_stack(cursor);
            }
        } else if cursor.count <= access.max_stack_size {
            std::mem::swap(slot, cursor);
        }
    } else if same_item_same_components(slot, cursor) {
        let cursor_max_stack_size = profiles.item_stack_max_stack_size(cursor);
        let amount = slot
            .count
            .min((cursor_max_stack_size - cursor.count).max(0));
        if amount > 0 {
            slot.count -= amount;
            cursor.count += amount;
            normalize_item_stack(slot);
        }
    }
}

pub(super) fn move_stack_count(
    source: &mut ProtocolItemStackSummary,
    target: &mut ProtocolItemStackSummary,
    amount: i32,
) {
    let moved = amount.min(source.count).max(0);
    if moved <= 0 {
        return;
    }
    if moved == source.count {
        *target = std::mem::replace(source, ProtocolItemStackSummary::empty());
    } else {
        *target = source.clone();
        target.count = moved;
        source.count -= moved;
        normalize_item_stack(source);
    }
}

pub(super) fn normalize_item_stack(stack: &mut ProtocolItemStackSummary) {
    if item_stack_is_empty(stack) {
        *stack = ProtocolItemStackSummary::empty();
    }
}

pub(super) fn item_stack_is_empty(stack: &ProtocolItemStackSummary) -> bool {
    stack.item_id.is_none() || stack.count <= 0
}

pub(super) fn item_stack_has_component_patch(stack: &ProtocolItemStackSummary) -> bool {
    !stack.component_patch.is_empty()
}

pub(super) fn same_item_same_components(
    left: &ProtocolItemStackSummary,
    right: &ProtocolItemStackSummary,
) -> bool {
    left.item_id == right.item_id && left.component_patch == right.component_patch
}

pub(super) fn container_slot_max_stack_size(
    container_id: i32,
    slot_num: i16,
    stack: &ProtocolItemStackSummary,
    profiles: &ItemProfiles,
) -> i32 {
    let item_max_stack_size = profiles.item_stack_max_stack_size(stack);
    let slot_max_stack_size = if container_id == INVENTORY_MENU_CONTAINER_ID {
        match slot_num {
            0 => 0,
            5..=8 => 1,
            _ => VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE,
        }
    } else {
        VANILLA_ITEM_DEFAULT_MAX_STACK_SIZE
    };
    item_max_stack_size.min(slot_max_stack_size)
}

pub(super) fn apply_local_selected_bundle_item_index(
    item: &ProtocolItemStackSummary,
    current_selected_item_index: &mut i32,
    selected_item_index: i32,
) -> bool {
    let Some(contents) = item.component_patch.explicit_bundle_contents() else {
        return false;
    };
    let bundle_item_count = contents.len();

    *current_selected_item_index = if selected_item_index == NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
        || selected_item_index == *current_selected_item_index
        || usize::try_from(selected_item_index)
            .map(|index| index >= bundle_item_count)
            .unwrap_or(true)
    {
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    } else {
        selected_item_index
    };
    true
}

pub(super) fn normalize_local_selected_bundle_item_index(
    selected_item_index: i32,
    item: &ProtocolItemStackSummary,
) -> i32 {
    let Some(contents) = item.component_patch.explicit_bundle_contents() else {
        return NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
    };
    let bundle_item_count = contents.len();
    let Ok(selected_item_index_usize) = usize::try_from(selected_item_index) else {
        return NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX;
    };
    if selected_item_index_usize < bundle_item_count {
        selected_item_index
    } else {
        NO_LOCAL_SELECTED_BUNDLE_ITEM_INDEX
    }
}
