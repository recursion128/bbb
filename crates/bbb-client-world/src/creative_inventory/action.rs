use std::collections::BTreeSet;

use bbb_protocol::{
    item_types::{vanilla_item_type, VANILLA_ITEM_AIR_ID},
    packets::{
        ContainerInput, ItemStackSummary, SetCreativeModeSlot as ProtocolSetCreativeModeSlot,
    },
};
use serde::{Deserialize, Serialize};

use crate::{
    ContainerClickBuildError, ContainerClickSlotRequest, ContainerSlot, InventoryState,
    PublicationShared, WorldCounters, WorldStore,
};

const INVENTORY_MENU_RESULT_SLOT: i16 = 0;
const INVENTORY_MENU_FIRST_WRITABLE_SLOT: i16 = 1;
const INVENTORY_MENU_LAST_WRITABLE_SLOT: i16 = 45;
const INVENTORY_MENU_HOTBAR_START: i16 = 36;
const INVENTORY_MENU_HOTBAR_END: i16 = 44;
const PLAYER_HOTBAR_SIZE: u8 = 9;
const OUTSIDE_DROP_SLOT: i16 = -1;
const OUTSIDE_CLICK_SLOT: i16 = -999;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreativeInventoryAction {
    PickerClick {
        item: ItemStackSummary,
        button_num: i8,
        input: ContainerInput,
    },
    InventoryClick {
        slot_num: i16,
        button_num: i8,
        input: ContainerInput,
    },
    HotbarClick {
        hotbar_index: u8,
        button_num: i8,
        input: ContainerInput,
    },
    OutsideClick {
        button_num: i8,
    },
    Trash {
        quick_move: bool,
    },
    QuickCraft {
        button_num: i8,
        slots: Vec<i16>,
        sync_all_hotbar_slots: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreativeInventoryActionPlanError {
    NoCreativeScreen,
    NoInfiniteMaterials,
    OtherContainerOpen,
    InvalidSlot(i16),
    InvalidButton(i8),
    UnsupportedInput(ContainerInput),
    EmptyItem,
    DisabledItem,
    LockedItem,
    InvalidItemCount(i32),
    DropThrottled,
    InventoryClick(ContainerClickBuildError),
}

#[derive(Debug)]
pub struct CreativeInventoryActionPlan<'a> {
    inventory: &'a mut PublicationShared<InventoryState>,
    counters: &'a mut WorldCounters,
    inventory_after: PublicationShared<InventoryState>,
    commands: Vec<ProtocolSetCreativeModeSlot>,
}

impl CreativeInventoryActionPlan<'_> {
    pub fn commands(&self) -> &[ProtocolSetCreativeModeSlot] {
        &self.commands
    }

    pub fn drop_command_count(&self) -> usize {
        self.commands
            .iter()
            .filter(|command| command.slot_num == OUTSIDE_DROP_SLOT)
            .count()
    }

    pub fn apply(self) {
        *self.inventory = self.inventory_after;
        self.inventory.update_inventory_slot_count(self.counters);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreativeHotbarPresetError {
    NoInfiniteMaterials,
}

impl WorldStore {
    pub fn plan_creative_inventory_action(
        &mut self,
        action: &CreativeInventoryAction,
        can_drop: bool,
    ) -> Result<CreativeInventoryActionPlan<'_>, CreativeInventoryActionPlanError> {
        self.validate_creative_action_owner()?;
        let mut inventory_after = self.inventory.clone();
        let mut commands = Vec::new();

        match action {
            CreativeInventoryAction::PickerClick {
                item,
                button_num,
                input,
            } => self.plan_picker_click(
                &mut inventory_after,
                &mut commands,
                item,
                *button_num,
                *input,
                can_drop,
            )?,
            CreativeInventoryAction::InventoryClick {
                slot_num,
                button_num,
                input,
            } => self.plan_inventory_click(
                &mut inventory_after,
                &mut commands,
                *slot_num,
                *button_num,
                *input,
                can_drop,
            )?,
            CreativeInventoryAction::HotbarClick {
                hotbar_index,
                button_num,
                input,
            } => self.plan_hotbar_click(
                &mut inventory_after,
                &mut commands,
                *hotbar_index,
                *button_num,
                *input,
                can_drop,
            )?,
            CreativeInventoryAction::OutsideClick { button_num } => {
                self.plan_outside_click(&mut inventory_after, &mut commands, *button_num, can_drop)?
            }
            CreativeInventoryAction::Trash { quick_move } => {
                self.plan_trash(&mut inventory_after, &mut commands, *quick_move)?
            }
            CreativeInventoryAction::QuickCraft {
                button_num,
                slots,
                sync_all_hotbar_slots,
            } => self.plan_quick_craft(
                &mut inventory_after,
                &mut commands,
                *button_num,
                slots,
                *sync_all_hotbar_slots,
            )?,
        }

        for command in &commands {
            self.validate_creative_command(command)?;
        }
        Ok(CreativeInventoryActionPlan {
            inventory: &mut self.inventory,
            counters: &mut self.counters,
            inventory_after,
            commands,
        })
    }

    /// The save half of Vanilla
    /// `CreativeModeInventoryScreen.handleHotbarLoadOrSave`:
    /// `Hotbar.storeFrom` snapshots the player inventory's selection row.
    /// The `hasInfiniteMaterials` gate lives in `Minecraft.handleKeybinds`.
    pub fn creative_hotbar_preset_snapshot(
        &self,
    ) -> Result<Vec<ItemStackSummary>, CreativeHotbarPresetError> {
        self.validate_infinite_materials()?;
        Ok(self.inventory.hotbar_items().into())
    }

    /// The load half of `handleHotbarLoadOrSave`: `inventory.setItem(i, ...)`
    /// plus `handleCreativeModeItemAdd(stack, 36 + i)` per hotbar column,
    /// with no screen requirement (the keybind path runs without one).
    pub fn creative_hotbar_preset_commands(
        &self,
        items: &[ItemStackSummary],
    ) -> Result<Vec<ProtocolSetCreativeModeSlot>, CreativeHotbarPresetError> {
        self.validate_infinite_materials()?;
        let mut commands = Vec::with_capacity(usize::from(PLAYER_HOTBAR_SIZE));
        for (index, item) in items
            .iter()
            .take(usize::from(PLAYER_HOTBAR_SIZE))
            .enumerate()
        {
            let slot_num = INVENTORY_MENU_HOTBAR_START + index as i16;
            commands.push(ProtocolSetCreativeModeSlot {
                slot_num,
                item: item.clone(),
            });
        }
        Ok(commands)
    }

    pub fn load_local_creative_hotbar_preset(
        &mut self,
        items: &[ItemStackSummary],
    ) -> Result<(), CreativeHotbarPresetError> {
        self.validate_infinite_materials()?;
        for (index, item) in items
            .iter()
            .take(usize::from(PLAYER_HOTBAR_SIZE))
            .enumerate()
        {
            let slot_num = INVENTORY_MENU_HOTBAR_START + index as i16;
            set_inventory_menu_slot(&mut self.inventory, slot_num, item.clone());
        }
        self.inventory
            .update_inventory_slot_count(&mut self.counters);
        Ok(())
    }

    fn validate_infinite_materials(&self) -> Result<(), CreativeHotbarPresetError> {
        if self
            .local_player
            .abilities
            .as_ref()
            .is_some_and(|abilities| abilities.instabuild)
        {
            Ok(())
        } else {
            Err(CreativeHotbarPresetError::NoInfiniteMaterials)
        }
    }

    fn validate_creative_action_owner(&self) -> Result<(), CreativeInventoryActionPlanError> {
        if !self.inventory.local_inventory_open {
            return Err(CreativeInventoryActionPlanError::NoCreativeScreen);
        }
        if !self
            .local_player
            .abilities
            .as_ref()
            .is_some_and(|abilities| abilities.instabuild)
        {
            return Err(CreativeInventoryActionPlanError::NoInfiniteMaterials);
        }
        if self.inventory.open_container.is_some() {
            return Err(CreativeInventoryActionPlanError::OtherContainerOpen);
        }
        Ok(())
    }

    fn plan_picker_click(
        &self,
        inventory_after: &mut InventoryState,
        commands: &mut Vec<ProtocolSetCreativeModeSlot>,
        item: &ItemStackSummary,
        button_num: i8,
        input: ContainerInput,
        can_drop: bool,
    ) -> Result<(), CreativeInventoryActionPlanError> {
        if item_stack_is_empty(item) {
            if matches!(
                input,
                ContainerInput::Pickup | ContainerInput::QuickMove | ContainerInput::PickupAll
            ) && !item_stack_is_empty(&inventory_after.cursor_item)
            {
                validate_pickup_button(button_num)?;
                if button_num == 0 {
                    inventory_after.cursor_item = ItemStackSummary::empty();
                } else {
                    shrink_stack(&mut inventory_after.cursor_item, 1);
                }
                return Ok(());
            }
            return Err(CreativeInventoryActionPlanError::EmptyItem);
        }
        self.validate_enabled_nonempty_item(item)?;
        if item.component_patch.creative_slot_lock {
            return Err(CreativeInventoryActionPlanError::LockedItem);
        }
        let max_stack_size = self.creative_item_max_stack_size(item)?;
        match input {
            ContainerInput::Swap => {
                // Vanilla `CreativeModeInventoryScreen.slotClicked` (`CreativeModeInventoryScreen.java:250-256`)
                // forwards both hotbar indices (`0..9`) and offhand (`40`, menu slot `45`) via `Inventory.setItem`.
                let slot_num = if button_num == 40 {
                    INVENTORY_MENU_LAST_WRITABLE_SLOT
                } else {
                    let hotbar_index = u8::try_from(button_num)
                        .ok()
                        .filter(|index| *index < PLAYER_HOTBAR_SIZE)
                        .ok_or(CreativeInventoryActionPlanError::InvalidButton(button_num))?;
                    INVENTORY_MENU_HOTBAR_START + i16::from(hotbar_index)
                };
                let mut replacement = item.clone();
                replacement.count = max_stack_size;
                set_inventory_menu_slot(inventory_after, slot_num, replacement.clone());
                commands.push(ProtocolSetCreativeModeSlot {
                    slot_num,
                    item: replacement,
                });
            }
            ContainerInput::Clone => {
                if item_stack_is_empty(&inventory_after.cursor_item) {
                    let mut carried = item.clone();
                    carried.count = max_stack_size;
                    inventory_after.cursor_item = carried;
                }
            }
            ContainerInput::Throw => {
                validate_pickup_button(button_num)?;
                if !can_drop {
                    return Err(CreativeInventoryActionPlanError::DropThrottled);
                }
                let mut dropped = item.clone();
                dropped.count = if button_num == 0 { 1 } else { max_stack_size };
                commands.push(ProtocolSetCreativeModeSlot {
                    slot_num: OUTSIDE_DROP_SLOT,
                    item: dropped,
                });
            }
            ContainerInput::Pickup | ContainerInput::QuickMove | ContainerInput::PickupAll => {
                validate_pickup_button(button_num)?;
                let quick_move = input == ContainerInput::QuickMove;
                let carried = &mut inventory_after.cursor_item;
                if !item_stack_is_empty(carried)
                    && item_stacks_match_by_item_and_components(carried, item)
                {
                    if button_num == 0 {
                        if quick_move {
                            carried.count = max_stack_size;
                        } else if carried.count < max_stack_size {
                            carried.count += 1;
                        }
                    } else {
                        shrink_stack(carried, 1);
                    }
                } else if item_stack_is_empty(carried) {
                    let mut replacement = item.clone();
                    replacement.count = if quick_move {
                        max_stack_size
                    } else {
                        item.count
                    };
                    *carried = replacement;
                } else if button_num == 0 {
                    *carried = ItemStackSummary::empty();
                } else {
                    shrink_stack(carried, 1);
                }
            }
            input => {
                return Err(CreativeInventoryActionPlanError::UnsupportedInput(input));
            }
        }
        Ok(())
    }

    fn plan_inventory_click(
        &self,
        inventory_after: &mut InventoryState,
        commands: &mut Vec<ProtocolSetCreativeModeSlot>,
        slot_num: i16,
        button_num: i8,
        input: ContainerInput,
        can_drop: bool,
    ) -> Result<(), CreativeInventoryActionPlanError> {
        if !(5..=INVENTORY_MENU_LAST_WRITABLE_SLOT).contains(&slot_num) {
            return Err(CreativeInventoryActionPlanError::InvalidSlot(slot_num));
        }
        self.validate_click_items(inventory_after, slot_num)?;
        if input == ContainerInput::Throw {
            return self.plan_real_slot_throw(
                inventory_after,
                commands,
                slot_num,
                button_num,
                can_drop,
            );
        }
        self.preview_creative_inventory_click(
            inventory_after,
            ContainerClickSlotRequest {
                slot_num,
                button_num,
                input,
            },
        )
        .map_err(CreativeInventoryActionPlanError::InventoryClick)?;
        push_changed_slot_commands(&self.inventory, inventory_after, commands);
        Ok(())
    }

    fn plan_hotbar_click(
        &self,
        inventory_after: &mut InventoryState,
        commands: &mut Vec<ProtocolSetCreativeModeSlot>,
        hotbar_index: u8,
        button_num: i8,
        input: ContainerInput,
        can_drop: bool,
    ) -> Result<(), CreativeInventoryActionPlanError> {
        if hotbar_index >= PLAYER_HOTBAR_SIZE {
            return Err(CreativeInventoryActionPlanError::InvalidSlot(i16::from(
                hotbar_index,
            )));
        }
        let slot_num = INVENTORY_MENU_HOTBAR_START + i16::from(hotbar_index);
        if input == ContainerInput::QuickMove {
            self.validate_click_items(inventory_after, slot_num)?;
            if item_stack_is_empty(&inventory_menu_item(inventory_after, slot_num)) {
                return Ok(());
            }
            set_inventory_menu_slot(inventory_after, slot_num, ItemStackSummary::empty());
            commands.push(ProtocolSetCreativeModeSlot {
                slot_num,
                item: ItemStackSummary::empty(),
            });
            return Ok(());
        }
        self.plan_inventory_click(
            inventory_after,
            commands,
            slot_num,
            button_num,
            input,
            can_drop,
        )
    }

    fn plan_real_slot_throw(
        &self,
        inventory_after: &mut InventoryState,
        commands: &mut Vec<ProtocolSetCreativeModeSlot>,
        slot_num: i16,
        button_num: i8,
        can_drop: bool,
    ) -> Result<(), CreativeInventoryActionPlanError> {
        validate_pickup_button(button_num)?;
        if !can_drop {
            return Err(CreativeInventoryActionPlanError::DropThrottled);
        }
        let mut source = inventory_menu_item(inventory_after, slot_num);
        if item_stack_is_empty(&source) {
            return Err(CreativeInventoryActionPlanError::EmptyItem);
        }
        self.validate_enabled_nonempty_item(&source)?;
        let removed = if button_num == 0 { 1 } else { source.count };
        let mut dropped = source.clone();
        dropped.count = removed;
        shrink_stack(&mut source, removed);
        set_inventory_menu_slot(inventory_after, slot_num, source.clone());
        commands.push(ProtocolSetCreativeModeSlot {
            slot_num: OUTSIDE_DROP_SLOT,
            item: dropped,
        });
        commands.push(ProtocolSetCreativeModeSlot {
            slot_num,
            item: source,
        });
        Ok(())
    }

    fn plan_outside_click(
        &self,
        inventory_after: &mut InventoryState,
        commands: &mut Vec<ProtocolSetCreativeModeSlot>,
        button_num: i8,
        can_drop: bool,
    ) -> Result<(), CreativeInventoryActionPlanError> {
        validate_pickup_button(button_num)?;
        if !can_drop {
            return Err(CreativeInventoryActionPlanError::DropThrottled);
        }
        let carried = &mut inventory_after.cursor_item;
        self.validate_enabled_nonempty_item(carried)?;
        let removed = if button_num == 0 { carried.count } else { 1 };
        let mut dropped = carried.clone();
        dropped.count = removed;
        shrink_stack(carried, removed);
        commands.push(ProtocolSetCreativeModeSlot {
            slot_num: OUTSIDE_DROP_SLOT,
            item: dropped,
        });
        Ok(())
    }

    fn plan_trash(
        &self,
        inventory_after: &mut InventoryState,
        commands: &mut Vec<ProtocolSetCreativeModeSlot>,
        quick_move: bool,
    ) -> Result<(), CreativeInventoryActionPlanError> {
        if !quick_move {
            inventory_after.cursor_item = ItemStackSummary::empty();
            return Ok(());
        }
        for slot_num in INVENTORY_MENU_RESULT_SLOT..=INVENTORY_MENU_LAST_WRITABLE_SLOT {
            set_inventory_menu_slot(inventory_after, slot_num, ItemStackSummary::empty());
            commands.push(ProtocolSetCreativeModeSlot {
                slot_num,
                item: ItemStackSummary::empty(),
            });
        }
        Ok(())
    }

    fn plan_quick_craft(
        &self,
        inventory_after: &mut InventoryState,
        commands: &mut Vec<ProtocolSetCreativeModeSlot>,
        button_num: i8,
        slots: &[i16],
        sync_all_hotbar_slots: bool,
    ) -> Result<(), CreativeInventoryActionPlanError> {
        if !matches!(button_num, 0..=2) {
            return Err(CreativeInventoryActionPlanError::InvalidButton(button_num));
        }
        self.validate_enabled_nonempty_item(&inventory_after.cursor_item)?;
        let allowed = if sync_all_hotbar_slots {
            INVENTORY_MENU_HOTBAR_START..=INVENTORY_MENU_HOTBAR_END
        } else {
            5..=INVENTORY_MENU_LAST_WRITABLE_SLOT
        };
        let mut unique = BTreeSet::new();
        for slot in slots {
            if !allowed.contains(slot) || !unique.insert(*slot) {
                return Err(CreativeInventoryActionPlanError::InvalidSlot(*slot));
            }
            self.validate_click_items(inventory_after, *slot)?;
        }
        for request in std::iter::once(ContainerClickSlotRequest {
            slot_num: OUTSIDE_CLICK_SLOT,
            button_num: quick_craft_mask(0, button_num),
            input: ContainerInput::QuickCraft,
        })
        .chain(slots.iter().map(|slot_num| ContainerClickSlotRequest {
            slot_num: *slot_num,
            button_num: quick_craft_mask(1, button_num),
            input: ContainerInput::QuickCraft,
        }))
        .chain(std::iter::once(ContainerClickSlotRequest {
            slot_num: OUTSIDE_CLICK_SLOT,
            button_num: quick_craft_mask(2, button_num),
            input: ContainerInput::QuickCraft,
        })) {
            self.preview_creative_inventory_click(inventory_after, request)
                .map_err(CreativeInventoryActionPlanError::InventoryClick)?;
        }
        if sync_all_hotbar_slots {
            for slot_num in INVENTORY_MENU_HOTBAR_START..=INVENTORY_MENU_HOTBAR_END {
                commands.push(ProtocolSetCreativeModeSlot {
                    slot_num,
                    item: inventory_menu_item(inventory_after, slot_num),
                });
            }
        } else {
            push_changed_slot_commands(&self.inventory, inventory_after, commands);
        }
        Ok(())
    }

    fn validate_click_items(
        &self,
        inventory: &InventoryState,
        slot_num: i16,
    ) -> Result<(), CreativeInventoryActionPlanError> {
        for item in [
            &inventory.cursor_item,
            inventory
                .inventory_menu_slot_item(slot_num)
                .unwrap_or(&ItemStackSummary::empty()),
        ] {
            if !item_stack_is_empty(item) {
                self.validate_enabled_nonempty_item(item)?;
            }
        }
        Ok(())
    }

    fn validate_enabled_nonempty_item(
        &self,
        item: &ItemStackSummary,
    ) -> Result<(), CreativeInventoryActionPlanError> {
        if item_stack_is_empty(item) {
            return Err(CreativeInventoryActionPlanError::EmptyItem);
        }
        let Some(item_id) = item.item_id else {
            return Err(CreativeInventoryActionPlanError::DisabledItem);
        };
        let enabled = if vanilla_item_type(item_id).is_some() {
            self.is_feature_enabled("minecraft:vanilla")
        } else {
            !self.content_registry().is_vanilla_only()
                && self.content_registry().item_by_id(item_id).is_some()
        };
        if !enabled {
            return Err(CreativeInventoryActionPlanError::DisabledItem);
        }
        self.creative_item_max_stack_size(item)?;
        Ok(())
    }

    fn creative_item_max_stack_size(
        &self,
        item: &ItemStackSummary,
    ) -> Result<i32, CreativeInventoryActionPlanError> {
        if item_stack_is_empty(item) {
            return Err(CreativeInventoryActionPlanError::EmptyItem);
        }
        let max_stack_size = self.items.item_stack_max_stack_size(item);
        if item.count <= 0 || item.count > max_stack_size {
            return Err(CreativeInventoryActionPlanError::InvalidItemCount(
                item.count,
            ));
        }
        Ok(max_stack_size)
    }

    fn validate_creative_command(
        &self,
        command: &ProtocolSetCreativeModeSlot,
    ) -> Result<(), CreativeInventoryActionPlanError> {
        if command.slot_num == OUTSIDE_DROP_SLOT {
            return self.validate_enabled_nonempty_item(&command.item);
        }
        if command.slot_num == INVENTORY_MENU_RESULT_SLOT {
            if item_stack_is_empty(&command.item) {
                return Ok(());
            }
            return Err(CreativeInventoryActionPlanError::InvalidSlot(
                command.slot_num,
            ));
        }
        if !(INVENTORY_MENU_FIRST_WRITABLE_SLOT..=INVENTORY_MENU_LAST_WRITABLE_SLOT)
            .contains(&command.slot_num)
        {
            return Err(CreativeInventoryActionPlanError::InvalidSlot(
                command.slot_num,
            ));
        }
        if item_stack_is_empty(&command.item) {
            Ok(())
        } else {
            self.validate_enabled_nonempty_item(&command.item)
        }
    }
}

fn validate_pickup_button(button_num: i8) -> Result<(), CreativeInventoryActionPlanError> {
    if matches!(button_num, 0 | 1) {
        Ok(())
    } else {
        Err(CreativeInventoryActionPlanError::InvalidButton(button_num))
    }
}

fn quick_craft_mask(header: i8, quick_craft_type: i8) -> i8 {
    (header & 3) | ((quick_craft_type & 3) << 2)
}

fn item_stack_is_empty(item: &ItemStackSummary) -> bool {
    item.item_id.is_none() || item.item_id == Some(VANILLA_ITEM_AIR_ID) || item.count <= 0
}

fn item_stacks_match_by_item_and_components(
    left: &ItemStackSummary,
    right: &ItemStackSummary,
) -> bool {
    left.item_id == right.item_id && left.component_patch == right.component_patch
}

fn shrink_stack(item: &mut ItemStackSummary, amount: i32) {
    item.count = item.count.saturating_sub(amount);
    if item.count <= 0 {
        *item = ItemStackSummary::empty();
    }
}

fn inventory_menu_item(inventory: &InventoryState, slot_num: i16) -> ItemStackSummary {
    inventory
        .inventory_menu_slot_item(slot_num)
        .cloned()
        .unwrap_or_else(ItemStackSummary::empty)
}

fn set_inventory_menu_slot(inventory: &mut InventoryState, slot_num: i16, item: ItemStackSummary) {
    inventory.set_menu_slot(0, ContainerSlot::new(slot_num, item));
}

fn push_changed_slot_commands(
    before: &InventoryState,
    after: &InventoryState,
    commands: &mut Vec<ProtocolSetCreativeModeSlot>,
) {
    let empty = ItemStackSummary::empty();
    for slot_num in INVENTORY_MENU_RESULT_SLOT..=INVENTORY_MENU_LAST_WRITABLE_SLOT {
        let before_item = before.inventory_menu_slot_item(slot_num).unwrap_or(&empty);
        let after_item = after.inventory_menu_slot_item(slot_num).unwrap_or(&empty);
        if before_item != after_item {
            commands.push(ProtocolSetCreativeModeSlot {
                slot_num,
                item: after_item.clone(),
            });
        }
    }
}
