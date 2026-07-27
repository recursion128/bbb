use bbb_control::NetCounters;
use bbb_item_model::{NativeItemRuntime, NativeItemTooltipOptions};
use bbb_net::NetCommand;
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::{ContainerInput, ItemStackSummary};
use bbb_renderer::{
    hud_creative_inventory_slot_layouts, hud_creative_inventory_tab_hit_y,
    hud_creative_inventory_tab_x, HudCreativeInventorySlot as CreativeInventorySlot,
    HudCreativeInventorySlotLayoutKind, HudCreativeInventoryTabRow, HUD_CREATIVE_INVENTORY_HEIGHT,
    HUD_CREATIVE_INVENTORY_WIDTH, HUD_CREATIVE_PICKER_COLUMNS, HUD_CREATIVE_PICKER_ROWS,
    HUD_CREATIVE_SCROLLER_X, HUD_CREATIVE_SCROLLER_Y, HUD_CREATIVE_SEARCH_HEIGHT,
    HUD_CREATIVE_SEARCH_WIDTH, HUD_CREATIVE_SEARCH_X, HUD_CREATIVE_SEARCH_Y,
    HUD_CREATIVE_TAB_HEIGHT, HUD_CREATIVE_TAB_WIDTH,
};
use bbb_world::{
    CreativeInventoryAction, CreativeInventoryActionPlanError, CreativeInventoryScreenState,
    CreativeInventorySnapshot, CreativeInventoryTabRow, CreativeInventoryTabSnapshot,
    CreativeInventoryTabType, WorldStore,
};
use tokio::sync::mpsc;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::MouseScrollDelta,
    keyboard::KeyCode,
};

use super::super::{
    commands::{
        hotbar_slot_for_key, queue_set_creative_mode_slot_commands_atomically,
        QueueCreativeModeSlotBatchError,
    },
    text_edit,
};

const CREATIVE_SEARCH_MAX_CHARS: usize = 50;
const CREATIVE_SCROLLBAR_HIT_WIDTH: i32 = 14;
const CREATIVE_SCROLLBAR_HIT_HEIGHT: i32 = 112;
const CREATIVE_SCROLLER_HEIGHT: i32 = 15;
const CREATIVE_SCROLL_DRAG_RANGE: f64 = 97.0;
const CREATIVE_DROP_THROTTLE_INCREMENT: u16 = 20;
const CREATIVE_DROP_THROTTLE_LIMIT: u16 = 1280;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreativeInventoryOwner {
    pub(crate) client_level_generation: u64,
    pub(crate) catalog_generation: u64,
    pub(crate) resource_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreativeInventoryHover {
    Slot(CreativeInventorySlot),
    Tab(usize),
    Search,
    Scroller,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreativeInventoryHudState {
    pub(crate) selected_tab: String,
    pub(crate) search_text: String,
    pub(crate) search_cursor: usize,
    pub(crate) search_selection: usize,
    pub(crate) search_focused: bool,
    pub(crate) scroll_row: usize,
    pub(crate) hovered: Option<CreativeInventoryHover>,
    pub(crate) cursor_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CreativeInventoryInputState {
    owner: Option<CreativeInventoryOwner>,
    selected_tab: Option<String>,
    search_text: String,
    search_cursor: usize,
    search_selection: usize,
    search_focused: bool,
    suppress_open_search_key_commit: bool,
    scroll_row: usize,
    scrolling: bool,
    scroll_accumulator: f64,
    hovered: Option<CreativeInventoryHover>,
    cursor_position: Option<(i32, i32)>,
    visible_item_count: usize,
    quick_craft_button_num: Option<i8>,
    quick_craft_slots: Vec<i16>,
    drop_spam_count: u16,
    pending_action: Option<CreativeInventoryAction>,
    suppress_hotbar_key_commit: bool,
    screen_open: bool,
}

impl CreativeInventoryInputState {
    pub(crate) fn sync(
        &mut self,
        screen: Option<&CreativeInventoryScreenState>,
        snapshot: Option<&CreativeInventorySnapshot>,
        resource_generation: u64,
    ) {
        let (Some(screen), Some(snapshot)) = (screen, snapshot) else {
            if self.screen_open {
                self.search_text.clear();
                self.search_cursor = 0;
                self.search_selection = 0;
            }
            self.screen_open = false;
            self.search_focused = false;
            self.suppress_open_search_key_commit = false;
            self.scroll_row = 0;
            self.scrolling = false;
            self.scroll_accumulator = 0.0;
            self.hovered = None;
            self.cursor_position = None;
            self.visible_item_count = 0;
            self.quick_craft_button_num = None;
            self.quick_craft_slots.clear();
            self.pending_action = None;
            self.suppress_hotbar_key_commit = false;
            return;
        };
        self.screen_open = true;

        let next_owner = CreativeInventoryOwner {
            client_level_generation: screen.client_level_generation,
            catalog_generation: screen.catalog_generation,
            resource_generation,
        };
        let previous_owner = self.owner;
        let level_changed = previous_owner.is_some_and(|owner| {
            owner.client_level_generation != next_owner.client_level_generation
        });
        let catalog_changed = previous_owner
            .is_some_and(|owner| owner.catalog_generation != next_owner.catalog_generation);
        let resources_changed = previous_owner
            .is_some_and(|owner| owner.resource_generation != next_owner.resource_generation);
        self.owner = Some(next_owner);

        if level_changed {
            self.selected_tab = None;
            self.search_text.clear();
            self.search_cursor = 0;
            self.search_selection = 0;
            self.scroll_row = 0;
            self.suppress_open_search_key_commit = false;
            self.scrolling = false;
            self.scroll_accumulator = 0.0;
            self.hovered = None;
            self.cursor_position = None;
            self.quick_craft_button_num = None;
            self.quick_craft_slots.clear();
            self.pending_action = None;
            self.drop_spam_count = 0;
            self.suppress_hotbar_key_commit = false;
        }

        let selected_is_valid = self
            .selected_tab
            .as_deref()
            .is_some_and(|selected| snapshot.tabs.iter().any(|tab| tab.resource_id == selected));
        if !selected_is_valid {
            self.selected_tab = default_tab(snapshot).map(|tab| tab.resource_id.clone());
            self.search_text.clear();
            self.search_cursor = 0;
            self.search_selection = 0;
            self.search_focused = false;
            self.scroll_row = 0;
        }

        if catalog_changed || resources_changed {
            self.hovered = None;
            self.scrolling = false;
            self.scroll_accumulator = 0.0;
            self.suppress_open_search_key_commit = false;
            self.quick_craft_button_num = None;
            self.quick_craft_slots.clear();
            self.pending_action = None;
            self.suppress_hotbar_key_commit = false;
        }
        match self.selected_tab(snapshot).map(|tab| tab.tab_type) {
            Some(CreativeInventoryTabType::Search) => self.search_focused = true,
            _ => {
                self.search_text.clear();
                self.search_cursor = 0;
                self.search_selection = 0;
                self.search_focused = false;
            }
        }
    }

    pub(crate) fn hud_state(
        &self,
        snapshot: &CreativeInventorySnapshot,
        visible_item_count: usize,
    ) -> Option<CreativeInventoryHudState> {
        if !self.screen_open {
            return None;
        }
        let selected = self.selected_tab(snapshot)?;
        let max_row = creative_inventory_max_scroll_row(visible_item_count);
        Some(CreativeInventoryHudState {
            selected_tab: selected.resource_id.clone(),
            search_text: self.search_text.clone(),
            search_cursor: self
                .search_cursor
                .min(text_edit::char_len(&self.search_text)),
            search_selection: self
                .search_selection
                .min(text_edit::char_len(&self.search_text)),
            search_focused: self.search_focused
                && selected.tab_type == CreativeInventoryTabType::Search,
            scroll_row: self.scroll_row.min(max_row),
            hovered: self.hovered,
            cursor_position: self.cursor_position,
        })
    }

    pub(crate) fn selected_tab<'a>(
        &self,
        snapshot: &'a CreativeInventorySnapshot,
    ) -> Option<&'a CreativeInventoryTabSnapshot> {
        let selected = self.selected_tab.as_deref()?;
        snapshot.tabs.iter().find(|tab| tab.resource_id == selected)
    }

    pub(crate) fn search_text(&self) -> &str {
        &self.search_text
    }

    pub(crate) fn cursor_position(&self) -> Option<(i32, i32)> {
        self.cursor_position
    }

    pub(crate) fn hovered_slot(&self) -> Option<CreativeInventorySlot> {
        match self.hovered {
            Some(CreativeInventoryHover::Slot(slot)) => Some(slot),
            _ => None,
        }
    }

    pub(crate) fn visible_item_count(&self) -> usize {
        self.visible_item_count
    }

    pub(crate) fn set_visible_item_count(&mut self, count: usize) {
        self.visible_item_count = count;
        self.scroll_row = self
            .scroll_row
            .min(creative_inventory_max_scroll_row(count));
    }

    pub(crate) fn search_entry_consumes_key(
        &self,
        snapshot: &CreativeInventorySnapshot,
        code: KeyCode,
    ) -> bool {
        matches!(code, KeyCode::KeyE) && self.search_is_active(snapshot)
    }

    pub(crate) fn set_cursor_position(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        snapshot: &CreativeInventorySnapshot,
        visible_item_count: usize,
    ) {
        self.cursor_position =
            creative_inventory_relative_cursor_position(cursor_position, surface_size);
        let Some((x, y)) = self.cursor_position else {
            self.hovered = None;
            return;
        };
        let Some(selected) = self.selected_tab(snapshot) else {
            self.hovered = None;
            return;
        };
        if self.scrolling {
            self.set_scroll_from_cursor(y, visible_item_count);
        }
        self.hovered = creative_inventory_hover_at(snapshot, selected, x, y);
        self.maybe_add_quick_craft_slot(selected);
    }

    pub(crate) fn handle_primary_press(
        &mut self,
        snapshot: &CreativeInventorySnapshot,
        visible_item_count: usize,
    ) -> bool {
        let Some(hovered) = self.hovered else {
            if self
                .selected_tab(snapshot)
                .is_none_or(|tab| tab.tab_type != CreativeInventoryTabType::Search)
            {
                self.search_focused = false;
            }
            return true;
        };
        match hovered {
            CreativeInventoryHover::Tab(_) => {}
            CreativeInventoryHover::Search => {
                if self
                    .selected_tab(snapshot)
                    .is_some_and(|tab| tab.tab_type == CreativeInventoryTabType::Search)
                {
                    self.search_focused = true;
                    self.search_cursor = text_edit::char_len(&self.search_text);
                    self.search_selection = self.search_cursor;
                }
            }
            CreativeInventoryHover::Scroller
                if creative_inventory_max_scroll_row(visible_item_count) > 0 =>
            {
                self.scrolling = true;
                if let Some((_, y)) = self.cursor_position {
                    self.set_scroll_from_cursor(y, visible_item_count);
                }
            }
            CreativeInventoryHover::Slot(_) | CreativeInventoryHover::Scroller => {
                if self
                    .selected_tab(snapshot)
                    .is_none_or(|tab| tab.tab_type != CreativeInventoryTabType::Search)
                {
                    self.search_focused = false;
                }
            }
        }
        true
    }

    pub(crate) fn handle_primary_release(&mut self, snapshot: &CreativeInventorySnapshot) {
        self.scrolling = false;
        let Some((x, y)) = self.cursor_position else {
            return;
        };
        if let Some((_, tab)) = creative_inventory_clicked_tab_at(snapshot, x, y) {
            self.select_tab(tab);
        }
    }

    pub(crate) fn cancel_primary_interaction(&mut self) {
        self.scrolling = false;
        self.quick_craft_button_num = None;
        self.quick_craft_slots.clear();
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_action_press(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        item_runtime: Option<&NativeItemRuntime>,
        snapshot: &CreativeInventorySnapshot,
        button_num: i8,
        shift_down: bool,
    ) -> bool {
        let Some(selected) = self.selected_tab(snapshot) else {
            return false;
        };
        let Some(hovered) = self.hovered else {
            if self.cursor_is_outside_screen(snapshot)
                && !item_stack_is_empty(&world.inventory().cursor_item)
            {
                return self.submit_action(
                    world,
                    counters,
                    net_commands,
                    CreativeInventoryAction::OutsideClick { button_num },
                );
            }
            return false;
        };
        let CreativeInventoryHover::Slot(slot) = hovered else {
            return false;
        };
        if matches!(button_num, 0 | 1)
            && !shift_down
            && !item_stack_is_empty(&world.inventory().cursor_item)
            && self.slot_can_quick_craft(selected.tab_type, slot)
        {
            self.quick_craft_button_num = Some(button_num);
            self.quick_craft_slots.clear();
            self.maybe_add_quick_craft_slot(selected);
            return true;
        }
        let input = if button_num == 2 {
            ContainerInput::Clone
        } else if shift_down {
            ContainerInput::QuickMove
        } else {
            ContainerInput::Pickup
        };
        let Some(action) =
            self.action_for_slot(world, item_runtime, selected, slot, button_num, input)
        else {
            return false;
        };
        self.submit_action(world, counters, net_commands, action)
    }

    pub(crate) fn handle_action_release(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        item_runtime: Option<&NativeItemRuntime>,
        snapshot: &CreativeInventorySnapshot,
        button_num: i8,
    ) -> bool {
        let Some(quick_craft_button_num) = self.quick_craft_button_num.take() else {
            return false;
        };
        if quick_craft_button_num != button_num {
            self.quick_craft_slots.clear();
            return true;
        }
        let Some(selected) = self.selected_tab(snapshot) else {
            self.quick_craft_slots.clear();
            return true;
        };
        self.maybe_add_quick_craft_slot(selected);
        let slots = std::mem::take(&mut self.quick_craft_slots);
        if !slots.is_empty() {
            return self.submit_action(
                world,
                counters,
                net_commands,
                CreativeInventoryAction::QuickCraft {
                    button_num,
                    slots,
                    sync_all_hotbar_slots: selected.tab_type != CreativeInventoryTabType::Inventory,
                },
            );
        }
        let action = match self.hovered {
            Some(CreativeInventoryHover::Slot(slot)) => self.action_for_slot(
                world,
                item_runtime,
                selected,
                slot,
                button_num,
                ContainerInput::Pickup,
            ),
            None if self.cursor_is_outside_screen(snapshot) => {
                Some(CreativeInventoryAction::OutsideClick { button_num })
            }
            _ => None,
        };
        action.is_some_and(|action| self.submit_action(world, counters, net_commands, action))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn handle_action_key(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        item_runtime: Option<&NativeItemRuntime>,
        snapshot: &CreativeInventorySnapshot,
        code: KeyCode,
        control_down: bool,
    ) -> bool {
        let Some(selected) = self.selected_tab(snapshot) else {
            return false;
        };
        let Some(CreativeInventoryHover::Slot(slot)) = self.hovered else {
            return false;
        };
        if let Some(hotbar_index) = hotbar_slot_for_key(code) {
            let Some(action) = self.action_for_slot(
                world,
                item_runtime,
                selected,
                slot,
                hotbar_index as i8,
                ContainerInput::Swap,
            ) else {
                return false;
            };
            if self.submit_action(world, counters, net_commands, action) {
                self.suppress_hotbar_key_commit = true;
                return true;
            }
            return false;
        }
        if code != KeyCode::KeyQ {
            return false;
        }
        let Some(action) = self.action_for_slot(
            world,
            item_runtime,
            selected,
            slot,
            i8::from(control_down),
            ContainerInput::Throw,
        ) else {
            return false;
        };
        self.submit_action(world, counters, net_commands, action)
    }

    pub(crate) fn advance_commands(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        ticks: u32,
    ) {
        let ticks = u16::try_from(ticks).unwrap_or(u16::MAX);
        self.drop_spam_count = self.drop_spam_count.saturating_sub(ticks);
        self.retry_pending_action(world, counters, net_commands);
    }

    fn action_for_slot(
        &self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        selected: &CreativeInventoryTabSnapshot,
        slot: CreativeInventorySlot,
        button_num: i8,
        input: ContainerInput,
    ) -> Option<CreativeInventoryAction> {
        match slot {
            CreativeInventorySlot::Picker(index) => {
                let item = self.picker_item(world, item_runtime, selected, index)?;
                Some(CreativeInventoryAction::PickerClick {
                    item,
                    button_num,
                    input,
                })
            }
            CreativeInventorySlot::Hotbar(index) => Some(CreativeInventoryAction::HotbarClick {
                hotbar_index: u8::try_from(index).ok()?,
                button_num,
                input,
            }),
            CreativeInventorySlot::Inventory(slot_num) => {
                Some(CreativeInventoryAction::InventoryClick {
                    slot_num,
                    button_num,
                    input,
                })
            }
            CreativeInventorySlot::Trash => Some(CreativeInventoryAction::Trash {
                quick_move: input == ContainerInput::QuickMove,
            }),
        }
    }

    fn picker_item(
        &self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
        selected: &CreativeInventoryTabSnapshot,
        picker_index: usize,
    ) -> Option<ItemStackSummary> {
        let index = self.scroll_row * HUD_CREATIVE_PICKER_COLUMNS + picker_index;
        if selected.tab_type == CreativeInventoryTabType::Hotbar {
            if index % 10 != 0 {
                return None;
            }
            let mut placeholder = ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_PAPER_ID),
                count: 1,
                component_patch: Default::default(),
            };
            placeholder.component_patch.creative_slot_lock = true;
            return Some(placeholder);
        }
        creative_inventory_visible_items(world, item_runtime, selected, &self.search_text)
            .get(index)
            .map(|item| (*item).clone())
    }

    fn slot_can_quick_craft(
        &self,
        tab_type: CreativeInventoryTabType,
        slot: CreativeInventorySlot,
    ) -> bool {
        match (tab_type, slot) {
            (CreativeInventoryTabType::Inventory, CreativeInventorySlot::Inventory(_)) => true,
            (CreativeInventoryTabType::Inventory, _) => false,
            (_, CreativeInventorySlot::Hotbar(_)) => true,
            _ => false,
        }
    }

    fn maybe_add_quick_craft_slot(&mut self, selected: &CreativeInventoryTabSnapshot) {
        if self.quick_craft_button_num.is_none() {
            return;
        }
        let slot_num = match (selected.tab_type, self.hovered) {
            (
                CreativeInventoryTabType::Inventory,
                Some(CreativeInventoryHover::Slot(CreativeInventorySlot::Inventory(slot_num))),
            ) => slot_num,
            (
                tab_type,
                Some(CreativeInventoryHover::Slot(CreativeInventorySlot::Hotbar(index))),
            ) if tab_type != CreativeInventoryTabType::Inventory => {
                let Ok(index) = i16::try_from(index) else {
                    return;
                };
                36 + index
            }
            _ => return,
        };
        if !self.quick_craft_slots.contains(&slot_num) {
            self.quick_craft_slots.push(slot_num);
        }
    }

    fn cursor_is_outside_screen(&self, snapshot: &CreativeInventorySnapshot) -> bool {
        let Some((x, y)) = self.cursor_position else {
            return false;
        };
        creative_inventory_clicked_tab_at(snapshot, x, y).is_none()
            && (x < 0
                || y < 0
                || x >= i32::try_from(HUD_CREATIVE_INVENTORY_WIDTH).unwrap_or(i32::MAX)
                || y >= i32::try_from(HUD_CREATIVE_INVENTORY_HEIGHT).unwrap_or(i32::MAX))
    }

    fn submit_action(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        action: CreativeInventoryAction,
    ) -> bool {
        self.retry_pending_action(world, counters, net_commands);
        if self.pending_action.is_some() {
            return false;
        }
        self.submit_action_inner(world, counters, net_commands, action)
    }

    fn retry_pending_action(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let Some(action) = self.pending_action.take() else {
            return;
        };
        self.submit_action_inner(world, counters, net_commands, action);
    }

    fn submit_action_inner(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        action: CreativeInventoryAction,
    ) -> bool {
        let can_drop = self.drop_spam_count < CREATIVE_DROP_THROTTLE_LIMIT;
        let plan = match world.plan_creative_inventory_action(&action, can_drop) {
            Ok(plan) => plan,
            Err(
                CreativeInventoryActionPlanError::NoCreativeScreen
                | CreativeInventoryActionPlanError::NoInfiniteMaterials
                | CreativeInventoryActionPlanError::OtherContainerOpen
                | CreativeInventoryActionPlanError::DropThrottled
                | CreativeInventoryActionPlanError::InvalidSlot(_)
                | CreativeInventoryActionPlanError::InvalidButton(_)
                | CreativeInventoryActionPlanError::UnsupportedInput(_)
                | CreativeInventoryActionPlanError::EmptyItem
                | CreativeInventoryActionPlanError::DisabledItem
                | CreativeInventoryActionPlanError::LockedItem
                | CreativeInventoryActionPlanError::InvalidItemCount(_)
                | CreativeInventoryActionPlanError::InventoryClick(_),
            ) => return false,
        };
        let commands = plan.commands().to_vec();
        let drop_command_count = plan.drop_command_count();
        match queue_set_creative_mode_slot_commands_atomically(counters, net_commands, &commands) {
            Ok(()) => {}
            Err(QueueCreativeModeSlotBatchError::Full) => {
                self.pending_action = Some(action);
                return true;
            }
            Err(
                QueueCreativeModeSlotBatchError::Unavailable
                | QueueCreativeModeSlotBatchError::Closed,
            ) => return false,
        }
        let applied = world.apply_creative_inventory_action_plan(plan);
        debug_assert!(
            applied,
            "creative action state changed after queue admission"
        );
        if !applied {
            return false;
        }
        let increment = u16::try_from(drop_command_count)
            .unwrap_or(u16::MAX)
            .saturating_mul(CREATIVE_DROP_THROTTLE_INCREMENT);
        self.drop_spam_count = self.drop_spam_count.saturating_add(increment);
        true
    }

    #[cfg(test)]
    fn drop_spam_count(&self) -> u16 {
        self.drop_spam_count
    }

    #[cfg(test)]
    fn has_pending_action(&self) -> bool {
        self.pending_action.is_some()
    }

    pub(crate) fn handle_wheel(&mut self, delta: MouseScrollDelta, visible_item_count: usize) {
        let max_row = creative_inventory_max_scroll_row(visible_item_count);
        if max_row == 0 {
            self.scroll_row = 0;
            self.scroll_accumulator = 0.0;
            return;
        }
        let raw = match delta {
            MouseScrollDelta::LineDelta(_, y) => f64::from(y),
            MouseScrollDelta::PixelDelta(position) => position.y / 10.0,
        };
        self.scroll_accumulator += raw;
        let whole = self.scroll_accumulator.trunc() as i32;
        self.scroll_accumulator -= f64::from(whole);
        if whole != 0 {
            self.scroll_row = (i32::try_from(self.scroll_row).unwrap_or(i32::MAX) - whole)
                .clamp(0, i32::try_from(max_row).unwrap_or(i32::MAX))
                as usize;
        }
    }

    pub(crate) fn handle_text(&mut self, snapshot: &CreativeInventorySnapshot, text: &str) -> bool {
        if !self.search_is_active(snapshot) {
            return false;
        }
        if self.suppress_hotbar_key_commit {
            self.suppress_hotbar_key_commit = false;
            if text.chars().count() == 1 && text.chars().all(|ch| ch.is_ascii_digit()) {
                return true;
            }
        }
        if self.suppress_open_search_key_commit {
            self.suppress_open_search_key_commit = false;
            if matches!(text, "t" | "T") {
                return true;
            }
        }
        let filtered = text
            .chars()
            .filter(|ch| !ch.is_control())
            .collect::<String>();
        if filtered.is_empty() {
            return true;
        }
        delete_selection(
            &mut self.search_text,
            &mut self.search_cursor,
            &mut self.search_selection,
        );
        let remaining =
            CREATIVE_SEARCH_MAX_CHARS.saturating_sub(text_edit::char_len(&self.search_text));
        let insert = filtered.chars().take(remaining).collect::<String>();
        let byte = text_edit::byte_index(&self.search_text, self.search_cursor);
        self.search_text.insert_str(byte, &insert);
        self.search_cursor += text_edit::char_len(&insert);
        self.search_selection = self.search_cursor;
        self.scroll_row = 0;
        true
    }

    pub(crate) fn handle_key(
        &mut self,
        snapshot: &CreativeInventorySnapshot,
        code: KeyCode,
        control_down: bool,
        shift_down: bool,
    ) -> bool {
        if !self.search_is_active(snapshot) && code == KeyCode::KeyT {
            if let Some(search) = snapshot
                .tabs
                .iter()
                .find(|tab| tab.tab_type == CreativeInventoryTabType::Search)
            {
                self.select_tab(search);
                self.suppress_open_search_key_commit = true;
                return true;
            }
        }
        if !self.search_is_active(snapshot) {
            return false;
        }
        let length = text_edit::char_len(&self.search_text);
        match code {
            KeyCode::KeyA if control_down => {
                self.search_cursor = length;
                self.search_selection = 0;
            }
            KeyCode::ArrowLeft => {
                let cursor = if control_down {
                    text_edit::word_position(&self.search_text, self.search_cursor, -1)
                } else {
                    self.search_cursor.saturating_sub(1)
                };
                self.move_cursor(cursor, shift_down);
            }
            KeyCode::ArrowRight => {
                let cursor = if control_down {
                    text_edit::word_position(&self.search_text, self.search_cursor, 1)
                } else {
                    self.search_cursor.saturating_add(1).min(length)
                };
                self.move_cursor(cursor, shift_down);
            }
            KeyCode::Home => self.move_cursor(0, shift_down),
            KeyCode::End => self.move_cursor(length, shift_down),
            KeyCode::Backspace => {
                if !delete_selection(
                    &mut self.search_text,
                    &mut self.search_cursor,
                    &mut self.search_selection,
                ) {
                    if control_down {
                        text_edit::remove_word_before_cursor(
                            &mut self.search_text,
                            &mut self.search_cursor,
                        );
                    } else {
                        remove_char_before(&mut self.search_text, &mut self.search_cursor);
                    }
                    self.search_selection = self.search_cursor;
                }
                self.scroll_row = 0;
            }
            KeyCode::Delete => {
                if !delete_selection(
                    &mut self.search_text,
                    &mut self.search_cursor,
                    &mut self.search_selection,
                ) {
                    if control_down {
                        text_edit::remove_word_at_cursor(&mut self.search_text, self.search_cursor);
                    } else {
                        remove_char_at(&mut self.search_text, self.search_cursor);
                    }
                    self.search_selection = self.search_cursor;
                }
                self.scroll_row = 0;
            }
            _ => {}
        }
        true
    }

    fn search_is_active(&self, snapshot: &CreativeInventorySnapshot) -> bool {
        self.search_focused
            && self
                .selected_tab(snapshot)
                .is_some_and(|tab| tab.tab_type == CreativeInventoryTabType::Search)
    }

    fn select_tab(&mut self, tab: &CreativeInventoryTabSnapshot) {
        let changed = self.selected_tab.as_deref() != Some(tab.resource_id.as_str());
        if changed && tab.tab_type == CreativeInventoryTabType::Search {
            self.search_text.clear();
            self.search_cursor = 0;
            self.search_selection = 0;
        }
        self.selected_tab = Some(tab.resource_id.clone());
        self.scroll_row = 0;
        self.scrolling = false;
        self.hovered = None;
        if tab.tab_type == CreativeInventoryTabType::Search {
            self.search_focused = true;
        } else {
            self.search_text.clear();
            self.search_cursor = 0;
            self.search_selection = 0;
            self.search_focused = false;
            self.suppress_open_search_key_commit = false;
        }
    }

    fn set_scroll_from_cursor(&mut self, cursor_y: i32, visible_item_count: usize) {
        let max_row = creative_inventory_max_scroll_row(visible_item_count);
        if max_row == 0 {
            self.scroll_row = 0;
            return;
        }
        let fraction = ((f64::from(cursor_y)
            - f64::from(HUD_CREATIVE_SCROLLER_Y)
            - f64::from(CREATIVE_SCROLLER_HEIGHT) * 0.5)
            / CREATIVE_SCROLL_DRAG_RANGE)
            .clamp(0.0, 1.0);
        self.scroll_row = ((fraction * max_row as f64) + 0.5).floor() as usize;
    }

    fn move_cursor(&mut self, cursor: usize, keep_selection: bool) {
        self.search_cursor = cursor.min(text_edit::char_len(&self.search_text));
        if !keep_selection {
            self.search_selection = self.search_cursor;
        }
    }
}

pub(crate) fn creative_inventory_visible_items<'a>(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    tab: &'a CreativeInventoryTabSnapshot,
    query: &str,
) -> Vec<&'a ItemStackSummary> {
    if tab.tab_type != CreativeInventoryTabType::Search || query.is_empty() {
        return tab.items.iter().collect();
    }
    let query = query.to_lowercase();
    if let Some(tag_search) = query.strip_prefix('#') {
        return tab
            .items
            .iter()
            .filter(|stack| {
                stack
                    .item_id
                    .is_some_and(|id| world.creative_item_matches_tag_search(id, tag_search))
            })
            .collect();
    }
    tab.items
        .iter()
        .filter(|stack| creative_item_matches_text(item_runtime, stack, &query))
        .collect()
}

pub(crate) fn creative_inventory_matching_item_tags<'a>(
    world: &'a WorldStore,
    item: &ItemStackSummary,
    query: &str,
) -> Vec<&'a str> {
    let Some(item_id) = item.item_id else {
        return Vec::new();
    };
    let query = query
        .to_lowercase()
        .strip_prefix('#')
        .unwrap_or_default()
        .to_string();
    world
        .registry_tag_names_containing_holder_id("minecraft:item", item_id)
        .into_iter()
        .flatten()
        .filter(|tag| resource_id_matches_search(tag, &query))
        .collect()
}

pub(crate) fn creative_inventory_max_scroll_row(item_count: usize) -> usize {
    item_count
        .div_ceil(HUD_CREATIVE_PICKER_COLUMNS)
        .saturating_sub(HUD_CREATIVE_PICKER_ROWS)
}

fn default_tab(snapshot: &CreativeInventorySnapshot) -> Option<&CreativeInventoryTabSnapshot> {
    snapshot
        .tabs
        .iter()
        .find(|tab| tab.tab_type == CreativeInventoryTabType::Category)
        .or_else(|| snapshot.tabs.first())
}

fn creative_tab_x(tab: &CreativeInventoryTabSnapshot) -> i32 {
    hud_creative_inventory_tab_x(tab.column, tab.aligned_right)
}

fn creative_tab_hit_y(tab: &CreativeInventoryTabSnapshot) -> i32 {
    hud_creative_inventory_tab_hit_y(match tab.row {
        CreativeInventoryTabRow::Top => HudCreativeInventoryTabRow::Top,
        CreativeInventoryTabRow::Bottom => HudCreativeInventoryTabRow::Bottom,
    })
}

fn creative_inventory_hover_at(
    snapshot: &CreativeInventorySnapshot,
    selected: &CreativeInventoryTabSnapshot,
    x: i32,
    y: i32,
) -> Option<CreativeInventoryHover> {
    if let Some((index, _)) = snapshot.tabs.iter().enumerate().find(|(_, tab)| {
        rect_contains(
            creative_tab_x(tab) + 3,
            creative_tab_hit_y(tab) + 3,
            21,
            27,
            x,
            y,
        )
    }) {
        return Some(CreativeInventoryHover::Tab(index));
    }
    if selected.tab_type == CreativeInventoryTabType::Search
        && rect_contains(
            HUD_CREATIVE_SEARCH_X,
            HUD_CREATIVE_SEARCH_Y,
            i32::try_from(HUD_CREATIVE_SEARCH_WIDTH).unwrap_or_default(),
            i32::try_from(HUD_CREATIVE_SEARCH_HEIGHT).unwrap_or_default(),
            x,
            y,
        )
    {
        return Some(CreativeInventoryHover::Search);
    }
    if selected.can_scroll
        && rect_contains(
            HUD_CREATIVE_SCROLLER_X,
            HUD_CREATIVE_SCROLLER_Y,
            CREATIVE_SCROLLBAR_HIT_WIDTH,
            CREATIVE_SCROLLBAR_HIT_HEIGHT,
            x,
            y,
        )
    {
        return Some(CreativeInventoryHover::Scroller);
    }
    hud_creative_inventory_slot_layouts(
        if selected.tab_type == CreativeInventoryTabType::Inventory {
            HudCreativeInventorySlotLayoutKind::Inventory
        } else {
            HudCreativeInventorySlotLayoutKind::Picker
        },
    )
    .into_iter()
    .find(|slot| rect_contains(slot.x, slot.y, 16, 16, x, y))
    .map(|slot| CreativeInventoryHover::Slot(slot.slot))
}

fn creative_inventory_clicked_tab_at(
    snapshot: &CreativeInventorySnapshot,
    x: i32,
    y: i32,
) -> Option<(usize, &CreativeInventoryTabSnapshot)> {
    snapshot.tabs.iter().enumerate().find(|(_, tab)| {
        let tab_x = creative_tab_x(tab);
        let tab_y = creative_tab_hit_y(tab);
        x >= tab_x
            && x <= tab_x + i32::try_from(HUD_CREATIVE_TAB_WIDTH).unwrap_or_default()
            && y >= tab_y
            && y <= tab_y + i32::try_from(HUD_CREATIVE_TAB_HEIGHT).unwrap_or_default()
    })
}

fn creative_inventory_relative_cursor_position(
    cursor_position: Option<PhysicalPosition<f64>>,
    surface_size: PhysicalSize<u32>,
) -> Option<(i32, i32)> {
    let cursor = cursor_position?;
    if !cursor.x.is_finite() || !cursor.y.is_finite() {
        return None;
    }
    let origin_x = (f64::from(surface_size.width) - f64::from(HUD_CREATIVE_INVENTORY_WIDTH)) * 0.5;
    let origin_y =
        (f64::from(surface_size.height) - f64::from(HUD_CREATIVE_INVENTORY_HEIGHT)) * 0.5;
    Some((
        (cursor.x - origin_x).floor() as i32,
        (cursor.y - origin_y).floor() as i32,
    ))
}

fn creative_item_matches_text(
    item_runtime: Option<&NativeItemRuntime>,
    stack: &ItemStackSummary,
    query: &str,
) -> bool {
    let item_resource_id = stack
        .item_id
        .and_then(vanilla_item_type)
        .map(|item| item.resource_id);
    let (namespace_search, text_search) = query
        .split_once(':')
        .map_or((None, query), |(namespace, text)| {
            (Some(namespace.trim()), text.trim())
        });
    let namespace_matches = namespace_search.is_none_or(|namespace_search| {
        item_resource_id
            .and_then(|resource_id| resource_id.split_once(':'))
            .is_some_and(|(namespace, _)| namespace.contains(namespace_search))
    });
    if !namespace_matches {
        return false;
    }
    let path_matches = item_resource_id
        .map(|resource_id| {
            resource_id
                .split_once(':')
                .map_or(resource_id, |(_, path)| path)
        })
        .is_some_and(|path| path.contains(text_search));
    path_matches
        || item_runtime
            .and_then(|runtime| {
                runtime.tooltip_lines_for_stack_with_context(
                    stack,
                    NativeItemTooltipOptions {
                        creative: true,
                        ..NativeItemTooltipOptions::default()
                    },
                )
            })
            .is_some_and(|lines| {
                lines
                    .iter()
                    .any(|line| line.text.to_lowercase().contains(text_search))
            })
}

fn resource_id_matches_search(resource_id: &str, search_term: &str) -> bool {
    let (namespace, path) = resource_id
        .split_once(':')
        .unwrap_or(("minecraft", resource_id));
    match search_term.split_once(':') {
        Some((namespace_search, path_search)) => {
            namespace.to_lowercase().contains(namespace_search.trim())
                && path.to_lowercase().contains(path_search.trim())
        }
        None => path.to_lowercase().contains(search_term),
    }
}

fn rect_contains(x: i32, y: i32, width: i32, height: i32, px: i32, py: i32) -> bool {
    px >= x && px < x + width && py >= y && py < y + height
}

fn item_stack_is_empty(item: &ItemStackSummary) -> bool {
    item.item_id.is_none() || item.count <= 0
}

fn delete_selection(text: &mut String, cursor: &mut usize, selection: &mut usize) -> bool {
    if *cursor == *selection {
        return false;
    }
    let start = (*cursor).min(*selection);
    let end = (*cursor).max(*selection);
    let start_byte = text_edit::byte_index(text, start);
    let end_byte = text_edit::byte_index(text, end);
    text.replace_range(start_byte..end_byte, "");
    *cursor = start;
    *selection = start;
    true
}

fn remove_char_before(text: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = text_edit::byte_index(text, *cursor - 1);
    let end = text_edit::byte_index(text, *cursor);
    text.replace_range(start..end, "");
    *cursor -= 1;
}

fn remove_char_at(text: &mut String, cursor: usize) {
    if cursor >= text_edit::char_len(text) {
        return;
    }
    let start = text_edit::byte_index(text, cursor);
    let end = text_edit::byte_index(text, cursor + 1);
    text.replace_range(start..end, "");
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use bbb_protocol::{
        item_types::VANILLA_ITEM_STONE_ID,
        packets::{
            PlayerAbilities, RegistryTags, TagNetworkPayload, UpdateEnabledFeatures, UpdateTags,
        },
    };
    use bbb_renderer::hud_creative_inventory_scroller_y;
    use bbb_world::{
        CreativeInventoryBackground, CreativeInventoryMaterializationRevision,
        CreativeInventoryTabType, RegistryAccessRevision,
    };

    use super::*;

    fn stack(item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        }
    }

    fn tab(
        resource_id: &str,
        row: CreativeInventoryTabRow,
        column: u8,
        tab_type: CreativeInventoryTabType,
        item_count: usize,
    ) -> CreativeInventoryTabSnapshot {
        CreativeInventoryTabSnapshot {
            resource_id: resource_id.to_string(),
            row,
            column,
            title_key: format!("itemGroup.{resource_id}"),
            icon: stack(0),
            tab_type,
            background: match tab_type {
                CreativeInventoryTabType::Search => CreativeInventoryBackground::ItemSearch,
                CreativeInventoryTabType::Inventory => CreativeInventoryBackground::Inventory,
                _ => CreativeInventoryBackground::Items,
            },
            aligned_right: column >= 6,
            show_title: tab_type != CreativeInventoryTabType::Search,
            can_scroll: tab_type != CreativeInventoryTabType::Inventory,
            items: (0..item_count)
                .map(|index| stack(i32::try_from(index).unwrap()))
                .collect(),
        }
    }

    fn snapshot(generation: u64, include_building: bool) -> CreativeInventorySnapshot {
        let mut tabs = Vec::new();
        if include_building {
            tabs.push(tab(
                "minecraft:building_blocks",
                CreativeInventoryTabRow::Top,
                0,
                CreativeInventoryTabType::Category,
                46,
            ));
        }
        tabs.push(tab(
            "minecraft:search",
            CreativeInventoryTabRow::Top,
            6,
            CreativeInventoryTabType::Search,
            100,
        ));
        tabs.push(tab(
            "minecraft:inventory",
            CreativeInventoryTabRow::Bottom,
            6,
            CreativeInventoryTabType::Inventory,
            0,
        ));
        CreativeInventorySnapshot {
            generation,
            revision: CreativeInventoryMaterializationRevision {
                registry: RegistryAccessRevision {
                    session: 1,
                    contents: 1,
                    tags: 1,
                },
                enabled_features: vec!["minecraft:vanilla".to_string()],
                has_permissions: false,
            },
            tabs,
        }
    }

    fn screen(generation: u64) -> CreativeInventoryScreenState {
        CreativeInventoryScreenState {
            client_level_generation: 9,
            catalog_generation: generation,
            display_operator_tab: false,
        }
    }

    fn creative_action_world() -> WorldStore {
        let mut world = WorldStore::new();
        world.apply_update_enabled_features(UpdateEnabledFeatures {
            features: Vec::new(),
        });
        world.apply_player_abilities(PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        assert!(world.open_local_inventory_with_operator_tab(false).unwrap());
        world.apply_update_enabled_features(UpdateEnabledFeatures {
            features: vec!["minecraft:vanilla".to_string()],
        });
        world
    }

    #[test]
    fn row_count_and_thumb_cover_zero_one_and_many_extra_rows() {
        assert_eq!(creative_inventory_max_scroll_row(45), 0);
        assert_eq!(creative_inventory_max_scroll_row(46), 1);
        assert_eq!(creative_inventory_max_scroll_row(99), 6);
        assert_eq!(hud_creative_inventory_scroller_y(0, 6), 18);
        assert_eq!(hud_creative_inventory_scroller_y(3, 6), 65);
        assert_eq!(hud_creative_inventory_scroller_y(6, 6), 113);
    }

    #[test]
    fn selection_survives_resource_and_catalog_refresh_but_invalidates_to_first_category() {
        let original = snapshot(1, true);
        let mut state = CreativeInventoryInputState::default();
        state.sync(Some(&screen(1)), Some(&original), 10);
        assert_eq!(
            state.selected_tab(&original).unwrap().resource_id,
            "minecraft:building_blocks"
        );
        state.select_tab(&original.tabs[1]);
        state.search_text = "stone".to_string();

        state.sync(Some(&screen(1)), Some(&original), 11);
        assert_eq!(
            state.selected_tab(&original).unwrap().resource_id,
            "minecraft:search"
        );
        assert_eq!(state.search_text, "stone");
        assert!(state.hovered.is_none());

        let refreshed = snapshot(2, true);
        state.sync(Some(&screen(2)), Some(&refreshed), 11);
        assert_eq!(
            state.selected_tab(&refreshed).unwrap().resource_id,
            "minecraft:search"
        );
        assert_eq!(state.search_text, "stone");

        let mut invalidated = snapshot(3, false);
        invalidated.tabs.push(tab(
            "minecraft:natural_blocks",
            CreativeInventoryTabRow::Top,
            2,
            CreativeInventoryTabType::Category,
            12,
        ));
        state.selected_tab = Some("minecraft:missing".to_string());
        state.sync(Some(&screen(3)), Some(&invalidated), 11);
        assert_eq!(
            state.selected_tab(&invalidated).unwrap().resource_id,
            "minecraft:natural_blocks"
        );
        assert!(state.search_text.is_empty());

        state.scroll_row = 4;
        state.scroll_accumulator = 0.5;
        state.sync(None, Some(&invalidated), 11);
        assert!(state.hud_state(&invalidated, 12).is_none());
        assert_eq!(state.scroll_row, 0);
        assert_eq!(state.scroll_accumulator, 0.0);
    }

    #[test]
    fn search_editing_is_bounded_and_tab_changes_clear_incompatible_text() {
        let snapshot = snapshot(1, true);
        let mut state = CreativeInventoryInputState::default();
        state.sync(Some(&screen(1)), Some(&snapshot), 1);
        state.select_tab(&snapshot.tabs[1]);
        assert!(state.handle_text(&snapshot, &"a".repeat(80)));
        assert_eq!(text_edit::char_len(&state.search_text), 50);
        assert!(state.handle_key(&snapshot, KeyCode::KeyA, true, false));
        assert!(state.handle_text(&snapshot, "stone"));
        assert_eq!(state.search_text, "stone");
        state.select_tab(&snapshot.tabs[0]);
        assert!(state.search_text.is_empty());
        assert!(!state.search_focused);

        state.select_tab(&snapshot.tabs[1]);
        state.sync(None, Some(&snapshot), 1);
        assert!(!state.search_focused);
        state.sync(Some(&screen(1)), Some(&snapshot), 1);
        assert!(state.search_focused);
        assert!(state.search_text.is_empty());
    }

    #[test]
    fn search_open_key_pointer_drag_and_wheel_follow_screen_gestures() {
        let snapshot = snapshot(1, true);
        let mut state = CreativeInventoryInputState::default();
        state.sync(Some(&screen(1)), Some(&snapshot), 1);
        state.set_visible_item_count(100);

        assert!(state.handle_key(&snapshot, KeyCode::KeyT, false, false));
        assert_eq!(
            state.selected_tab(&snapshot).unwrap().resource_id,
            "minecraft:search"
        );
        assert!(state.search_focused);
        assert!(state.handle_text(&snapshot, "t"));
        assert!(state.search_text.is_empty());
        assert!(state.handle_text(&snapshot, "stone"));
        assert_eq!(state.search_text, "stone");

        let surface = PhysicalSize::new(395, 336);
        state.set_cursor_position(
            Some(PhysicalPosition::new(288.0, 229.0)),
            surface,
            &snapshot,
            100,
        );
        assert_eq!(state.hovered, Some(CreativeInventoryHover::Scroller));
        assert!(state.handle_primary_press(&snapshot, 100));
        assert!(state.scrolling);
        assert_eq!(state.scroll_row, 7);

        state.handle_primary_release(&snapshot);
        assert!(!state.scrolling);
        state.handle_wheel(MouseScrollDelta::LineDelta(0.0, 1.0), 100);
        assert_eq!(state.scroll_row, 6);
        state.handle_wheel(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -20.0)),
            100,
        );
        assert_eq!(state.scroll_row, 7);

        state.set_cursor_position(
            Some(PhysicalPosition::new(100.0, 70.0)),
            surface,
            &snapshot,
            100,
        );
        assert_eq!(state.hovered, None, "tab edge is outside hover tooltip");
        state.handle_primary_release(&snapshot);
        assert_eq!(
            state.selected_tab(&snapshot).unwrap().resource_id,
            "minecraft:building_blocks",
            "the inclusive tab click target still selects from its sprite edge"
        );
    }

    #[test]
    fn resource_id_tag_matching_trims_namespace_and_path_terms() {
        assert!(resource_id_matches_search(
            "minecraft:stone_tool_materials",
            "stone"
        ));
        assert!(resource_id_matches_search(
            "minecraft:stone_tool_materials",
            "mine: tool"
        ));
        assert!(!resource_id_matches_search(
            "example:stone_tool_materials",
            "mine:tool"
        ));
    }

    #[test]
    fn small_surfaces_keep_centered_relative_cursor_geometry() {
        assert_eq!(
            creative_inventory_relative_cursor_position(
                Some(PhysicalPosition::new(0.0, 0.0)),
                PhysicalSize::new(100, 80),
            ),
            Some((47, 28))
        );
        assert_eq!(
            creative_inventory_relative_cursor_position(
                Some(PhysicalPosition::new(100.0, 100.0)),
                PhysicalSize::new(395, 336),
            ),
            Some((0, 0))
        );
        assert_eq!(
            creative_inventory_relative_cursor_position(
                Some(PhysicalPosition::new(22.5, 32.0)),
                PhysicalSize::new(240, 200),
            ),
            Some((0, 0)),
            "odd surface deltas use the renderer's half-pixel centered origin"
        );
    }

    #[test]
    fn saturated_action_retries_without_mutating_or_duplicating_the_hotbar() {
        let mut world = creative_action_world();
        let mut state = CreativeInventoryInputState::default();
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(1);
        tx.try_send(NetCommand::ClientTickEnd).unwrap();
        let commands = Some(tx);
        let action = CreativeInventoryAction::PickerClick {
            item: stack(VANILLA_ITEM_STONE_ID),
            button_num: 0,
            input: ContainerInput::Swap,
        };

        assert!(state.submit_action(&mut world, &mut counters, &commands, action));
        assert!(state.has_pending_action());
        assert!(world.inventory().hotbar_items()[0].item_id.is_none());
        assert_eq!(counters.set_creative_mode_slot_commands_queued, 0);

        assert_eq!(rx.try_recv().unwrap(), NetCommand::ClientTickEnd);
        state.advance_commands(&mut world, &mut counters, &commands, 1);
        assert!(!state.has_pending_action());
        assert_eq!(
            world.inventory().hotbar_items()[0],
            ItemStackSummary {
                item_id: Some(VANILLA_ITEM_STONE_ID),
                count: 64,
                component_patch: Default::default(),
            }
        );
        assert_eq!(counters.set_creative_mode_slot_commands_queued, 1);
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::SetCreativeModeSlot(command)
                if command.slot_num == 36 && command.item.count == 64
        ));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn saturated_shift_trash_retries_all_forty_six_packets_atomically() {
        let mut world = creative_action_world();
        let mut state = CreativeInventoryInputState::default();
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(46);
        let commands = Some(tx);
        assert!(state.submit_action(
            &mut world,
            &mut counters,
            &commands,
            CreativeInventoryAction::PickerClick {
                item: stack(VANILLA_ITEM_STONE_ID),
                button_num: 0,
                input: ContainerInput::Swap,
            },
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::SetCreativeModeSlot(command) if command.slot_num == 36
        ));
        commands
            .as_ref()
            .unwrap()
            .try_send(NetCommand::ClientTickEnd)
            .unwrap();

        assert!(state.submit_action(
            &mut world,
            &mut counters,
            &commands,
            CreativeInventoryAction::Trash { quick_move: true },
        ));
        assert!(state.has_pending_action());
        assert_eq!(world.inventory().hotbar_items()[0].count, 64);
        assert_eq!(rx.try_recv().unwrap(), NetCommand::ClientTickEnd);
        assert!(rx.try_recv().is_err());

        state.advance_commands(&mut world, &mut counters, &commands, 0);
        assert!(!state.has_pending_action());
        assert!(world.inventory().hotbar_items()[0].item_id.is_none());
        for expected_slot in 0..=45 {
            assert!(matches!(
                rx.try_recv().unwrap(),
                NetCommand::SetCreativeModeSlot(command)
                    if command.slot_num == expected_slot && command.item.item_id.is_none()
            ));
        }
        assert!(rx.try_recv().is_err());
        assert_eq!(counters.set_creative_mode_slot_commands_queued, 47);
    }

    #[test]
    fn pointer_clone_right_pickup_number_key_and_throw_emit_creative_actions() {
        let mut world = creative_action_world();
        let mut catalog = snapshot(1, true);
        catalog.tabs[0].items[0] = stack(VANILLA_ITEM_STONE_ID);
        let mut state = CreativeInventoryInputState::default();
        state.sync(Some(&screen(1)), Some(&catalog), 1);
        state.hovered = Some(CreativeInventoryHover::Slot(CreativeInventorySlot::Picker(
            0,
        )));
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);

        assert!(state.handle_action_press(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            2,
            false,
        ));
        assert_eq!(world.inventory().cursor_item.count, 64);
        assert!(state.handle_action_press(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            1,
            false,
        ));
        assert_eq!(world.inventory().cursor_item.count, 63);

        state.hovered = Some(CreativeInventoryHover::Slot(CreativeInventorySlot::Trash));
        assert!(state.handle_action_press(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            0,
            false,
        ));
        assert!(world.inventory().cursor_item.item_id.is_none());

        state.hovered = Some(CreativeInventoryHover::Slot(CreativeInventorySlot::Picker(
            0,
        )));
        assert!(state.handle_action_key(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            KeyCode::Digit3,
            false,
        ));
        assert_eq!(world.inventory().hotbar_items()[2].count, 64);
        assert!(state.handle_action_key(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            KeyCode::KeyQ,
            false,
        ));
        assert_eq!(state.drop_spam_count(), 20);

        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::SetCreativeModeSlot(command)
                if command.slot_num == 38 && command.item.count == 64
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::SetCreativeModeSlot(command)
                if command.slot_num == -1 && command.item.count == 1
        ));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn creative_drop_throttler_limits_decays_and_resets_for_a_new_level_owner() {
        let mut world = creative_action_world();
        let mut state = CreativeInventoryInputState::default();
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(128);
        let commands = Some(tx);
        let drop = CreativeInventoryAction::PickerClick {
            item: stack(VANILLA_ITEM_STONE_ID),
            button_num: 0,
            input: ContainerInput::Throw,
        };

        for _ in 0..64 {
            assert!(state.submit_action(&mut world, &mut counters, &commands, drop.clone(),));
        }
        assert_eq!(state.drop_spam_count(), 1280);
        assert!(!state.submit_action(&mut world, &mut counters, &commands, drop.clone(),));
        assert_eq!(counters.set_creative_mode_slot_commands_queued, 64);

        state.advance_commands(&mut world, &mut counters, &commands, 1);
        assert_eq!(state.drop_spam_count(), 1279);
        assert!(state.submit_action(&mut world, &mut counters, &commands, drop));
        assert_eq!(state.drop_spam_count(), 1299);
        assert_eq!(counters.set_creative_mode_slot_commands_queued, 65);
        for _ in 0..65 {
            assert!(matches!(
                rx.try_recv().unwrap(),
                NetCommand::SetCreativeModeSlot(command) if command.slot_num == -1
            ));
        }

        let current = world.creative_inventory_screen().cloned().unwrap();
        let catalog = world.creative_inventory_snapshot().cloned().unwrap();
        state.sync(Some(&current), Some(&catalog), 1);
        let next_owner = CreativeInventoryScreenState {
            client_level_generation: current.client_level_generation + 1,
            ..current
        };
        state.sync(Some(&next_owner), Some(&catalog), 1);
        assert_eq!(state.drop_spam_count(), 0);
    }

    #[test]
    fn screen_replacement_clears_a_saturated_pending_action() {
        let mut world = creative_action_world();
        let mut state = CreativeInventoryInputState::default();
        let mut counters = NetCounters::default();
        let (tx, _rx) = mpsc::channel(1);
        tx.try_send(NetCommand::ClientTickEnd).unwrap();
        let commands = Some(tx);
        assert!(state.submit_action(
            &mut world,
            &mut counters,
            &commands,
            CreativeInventoryAction::PickerClick {
                item: stack(VANILLA_ITEM_STONE_ID),
                button_num: 0,
                input: ContainerInput::Swap,
            },
        ));
        assert!(state.has_pending_action());

        let catalog = world.creative_inventory_snapshot().cloned().unwrap();
        state.sync(None, Some(&catalog), 1);
        assert!(!state.has_pending_action());
    }

    #[test]
    fn vanilla_item_ids_are_registry_holder_ids_for_tag_search() {
        assert_eq!(
            vanilla_item_type(1).map(|item| item.id),
            Some(1),
            "creative tag filtering passes item protocol ids to the item registry"
        );
    }

    #[test]
    fn search_matches_localized_names_and_live_namespaced_tags_in_catalog_order() {
        let root = std::env::temp_dir().join(format!(
            "bbb-creative-search-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let lang = root
            .join("sources")
            .join(bbb_pack::MC_VERSION)
            .join("assets/minecraft/lang");
        fs::create_dir_all(&lang).unwrap();
        fs::write(
            lang.join("en_us.json"),
            r#"{
                "item.minecraft.stone": "Polished Stone",
                "item.minecraft.dirt": "Soft Dirt"
            }"#,
        )
        .unwrap();
        let assets = lang.parent().unwrap();
        let atlases = assets.join("atlases");
        fs::create_dir_all(&atlases).unwrap();
        fs::write(
            atlases.join("items.json"),
            r#"{
                "sources": [{
                    "type": "minecraft:directory",
                    "prefix": "item/",
                    "source": "item"
                }]
            }"#,
        )
        .unwrap();
        fs::write(atlases.join("blocks.json"), r#"{"sources":[]}"#).unwrap();
        for (item, rgba) in [
            ("stone", [128, 128, 128, 255]),
            ("dirt", [120, 80, 48, 255]),
        ] {
            let items = assets.join("items");
            let models = assets.join("models/item");
            let textures = assets.join("textures/item");
            fs::create_dir_all(&items).unwrap();
            fs::create_dir_all(&models).unwrap();
            fs::create_dir_all(&textures).unwrap();
            fs::write(
                items.join(format!("{item}.json")),
                format!(
                    r#"{{
                        "model": {{
                            "type": "minecraft:model",
                            "model": "minecraft:item/{item}"
                        }}
                    }}"#
                ),
            )
            .unwrap();
            fs::write(
                models.join(format!("{item}.json")),
                format!(
                    r#"{{
                        "textures": {{
                            "layer0": "minecraft:item/{item}"
                        }}
                    }}"#
                ),
            )
            .unwrap();
            image::save_buffer(
                textures.join(format!("{item}.png")),
                &rgba,
                1,
                1,
                image::ColorType::Rgba8,
            )
            .unwrap();
        }
        crate::item_registry_test_fixture::write_item_registry_fixture(&root, &["stone", "dirt"]);
        let runtime =
            NativeItemRuntime::load(&bbb_pack::PackRoots::from_root(&root).unwrap()).unwrap();
        fs::remove_dir_all(&root).unwrap();

        let search = tab(
            "minecraft:search",
            CreativeInventoryTabRow::Top,
            6,
            CreativeInventoryTabType::Search,
            0,
        );
        let search = CreativeInventoryTabSnapshot {
            items: vec![stack(0), stack(1)],
            ..search
        };
        let mut world = WorldStore::default();
        assert_eq!(
            creative_inventory_visible_items(&world, Some(&runtime), &search, "polished",)
                .iter()
                .filter_map(|stack| stack.item_id)
                .collect::<Vec<_>>(),
            vec![0]
        );
        assert_eq!(
            creative_inventory_visible_items(
                &world,
                Some(&runtime),
                &search,
                "minecraft:polished",
            )
            .iter()
            .filter_map(|stack| stack.item_id)
            .collect::<Vec<_>>(),
            vec![0],
            "namespaced full-text search intersects the item namespace with names and paths"
        );
        assert!(creative_inventory_visible_items(
            &world,
            Some(&runtime),
            &search,
            "example:polished",
        )
        .is_empty());

        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![
                    TagNetworkPayload {
                        tag: "mine:soft_soil".to_string(),
                        entries: vec![1],
                    },
                    TagNetworkPayload {
                        tag: "minecraft:stone_family".to_string(),
                        entries: vec![0],
                    },
                ],
            }],
        });
        assert_eq!(
            creative_inventory_visible_items(&world, Some(&runtime), &search, "#mine: soil",)
                .iter()
                .filter_map(|stack| stack.item_id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            creative_inventory_matching_item_tags(&world, &stack(1), "#mine: soil",),
            vec!["mine:soft_soil"]
        );
    }
}
