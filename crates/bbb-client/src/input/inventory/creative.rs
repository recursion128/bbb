use crate::control::NetCounters;
use crate::item_models::tooltip::{tooltip_lines_for_stack_with_context, NativeItemTooltipOptions};
use crate::options::key_bindings::{BoundInput, KeyBindings};
use bbb_client_net::NetCommand;
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::{
    hud_creative_inventory_slot_layouts, hud_creative_inventory_tab_hit_y,
    hud_creative_inventory_tab_x, HudCreativeInventorySlot as CreativeInventorySlot,
    HudCreativeInventorySlotLayoutKind, HudCreativeInventoryTabRow, HUD_CREATIVE_INVENTORY_HEIGHT,
    HUD_CREATIVE_INVENTORY_WIDTH, HUD_CREATIVE_PICKER_COLUMNS, HUD_CREATIVE_PICKER_ROWS,
    HUD_CREATIVE_SCROLLER_X, HUD_CREATIVE_SCROLLER_Y, HUD_CREATIVE_SEARCH_HEIGHT,
    HUD_CREATIVE_SEARCH_WIDTH, HUD_CREATIVE_SEARCH_X, HUD_CREATIVE_SEARCH_Y,
    HUD_CREATIVE_TAB_HEIGHT, HUD_CREATIVE_TAB_WIDTH,
};
use bbb_client_world::{
    CreativeInventoryAction, CreativeInventorySnapshot, CreativeInventoryTabRow,
    CreativeInventoryTabSnapshot, CreativeInventoryTabType, WorldStore,
};
use bbb_protocol::item_types::vanilla_item_type;
use bbb_protocol::packets::{ContainerInput, ItemStackSummary};
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
    text_field::{BoundedTextField, TextField, TextFieldModifiers},
    DebugClipboard,
};

const CREATIVE_SEARCH_MAX_LENGTH: usize = 50;
const CREATIVE_SCROLLBAR_HIT_WIDTH: i32 = 14;
const CREATIVE_SCROLLBAR_HIT_HEIGHT: i32 = 112;
const CREATIVE_SCROLLER_HEIGHT: i32 = 15;
const CREATIVE_SCROLL_DRAG_RANGE: f64 = 97.0;
const CREATIVE_DROP_THROTTLE_INCREMENT: u16 = 20;
const CREATIVE_DROP_THROTTLE_LIMIT: u16 = 1280;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreativeInventoryHover {
    Slot(CreativeInventorySlot),
    Tab(usize),
    Search,
    Scroller,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CreativeInventoryHudState {
    pub(crate) selected_tab: String,
    pub(crate) search: TextField,
    pub(crate) search_focused: bool,
    pub(crate) scroll_row: usize,
    pub(crate) hovered: Option<CreativeInventoryHover>,
    pub(crate) cursor_position: Option<(i32, i32)>,
    pub(crate) saved_hotbars: Vec<Vec<ItemStackSummary>>,
    pub(crate) hotbar_key_bindings: KeyBindings,
    pub(crate) quick_craft_button_num: Option<i8>,
    pub(crate) quick_craft_slots: Vec<i16>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct CreativeInventoryInputState {
    pub(crate) display_operator_tab: bool,
    pub(crate) selected_tab: Option<String>,
    search: BoundedTextField<CREATIVE_SEARCH_MAX_LENGTH>,
    search_focused: bool,
    suppress_open_search_key_commit: Option<KeyCode>,
    scroll_row: usize,
    scrolling: bool,
    scroll_accumulator: f64,
    hovered: Option<CreativeInventoryHover>,
    cursor_position: Option<(i32, i32)>,
    visible_item_count: usize,
    quick_craft_button_num: Option<i8>,
    quick_craft_slots: Vec<i16>,
    pub(crate) drop_spam_count: u16,
    pending_action: Option<CreativeInventoryAction>,
    suppress_hotbar_key_commit: bool,
    saved_hotbars: Vec<Vec<ItemStackSummary>>,
    hotbar_key_bindings: KeyBindings,
    hotbar_presets_loaded: bool,
}

impl CreativeInventoryInputState {
    pub(in crate::input) fn search_mut(&mut self) -> &mut TextField {
        &mut self.search
    }

    pub(crate) fn new(selected_tab: Option<String>, drop_spam_count: u16) -> Self {
        Self {
            selected_tab,
            drop_spam_count,
            ..Self::default()
        }
    }

    pub(crate) fn refresh_catalog(&mut self, snapshot: &CreativeInventorySnapshot) {
        let selected_is_valid = self
            .selected_tab
            .as_deref()
            .is_some_and(|selected| snapshot.tabs.iter().any(|tab| tab.resource_id == selected));
        if !selected_is_valid {
            self.selected_tab = default_tab(snapshot).map(|tab| tab.resource_id.clone());
            self.search.set_text("", None);
            self.search_focused = false;
            self.scroll_row = 0;
        }

        self.hovered = None;
        self.scrolling = false;
        self.scroll_accumulator = 0.0;
        self.suppress_open_search_key_commit = None;
        self.quick_craft_button_num = None;
        self.quick_craft_slots.clear();
        self.pending_action = None;
        self.suppress_hotbar_key_commit = false;
        self.hotbar_presets_loaded = false;
        match self.selected_tab(snapshot).map(|tab| tab.tab_type) {
            Some(CreativeInventoryTabType::Search) => self.search_focused = true,
            _ => {
                self.search.set_text("", None);
                self.search_focused = false;
            }
        }
    }

    pub(crate) fn set_saved_hotbars(
        &mut self,
        rows: Vec<Vec<ItemStackSummary>>,
        key_bindings: KeyBindings,
    ) {
        self.saved_hotbars = rows;
        self.hotbar_key_bindings = key_bindings;
        self.hotbar_presets_loaded = true;
    }

    pub(crate) fn hotbar_presets_loaded(&self) -> bool {
        self.hotbar_presets_loaded
    }

    pub(crate) fn hotbar_key_bindings(&self) -> &KeyBindings {
        &self.hotbar_key_bindings
    }

    pub(crate) fn hud_state(
        &self,
        snapshot: &CreativeInventorySnapshot,
        visible_item_count: usize,
    ) -> Option<CreativeInventoryHudState> {
        let selected = self.selected_tab(snapshot)?;
        let max_row = creative_inventory_max_scroll_row(visible_item_count);
        Some(CreativeInventoryHudState {
            selected_tab: selected.resource_id.clone(),
            search: TextField::clone(&self.search),
            search_focused: self.search_focused
                && selected.tab_type == CreativeInventoryTabType::Search,
            scroll_row: self.scroll_row.min(max_row),
            hovered: self.hovered,
            cursor_position: self.cursor_position,
            saved_hotbars: self.saved_hotbars.clone(),
            hotbar_key_bindings: self.hotbar_key_bindings.clone(),
            quick_craft_button_num: self.quick_craft_button_num,
            quick_craft_slots: self.quick_craft_slots.clone(),
        })
    }

    pub(crate) fn selected_tab<'a>(
        &self,
        snapshot: &'a CreativeInventorySnapshot,
    ) -> Option<&'a CreativeInventoryTabSnapshot> {
        let selected = self.selected_tab.as_deref()?;
        snapshot.tabs.iter().find(|tab| tab.resource_id == selected)
    }

    pub(crate) fn search_text(&self) -> String {
        self.search.text()
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
        _code: KeyCode,
    ) -> bool {
        self.search_is_active(snapshot)
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
    }

    pub(crate) fn update_quick_craft_drag(
        &mut self,
        world: &WorldStore,
        snapshot: &CreativeInventorySnapshot,
    ) {
        let Some(selected) = self.selected_tab(snapshot) else {
            return;
        };
        self.maybe_add_quick_craft_slot(world, selected);
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
                    self.search.focus();
                    let end = self.search.value().len();
                    self.search.move_cursor(end, false, None);
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
        if matches!(button_num, 0..=2)
            && !shift_down
            && !item_stack_is_empty(&world.inventory().cursor_item)
            && self.slot_can_quick_craft(selected.tab_type, slot)
        {
            // Vanilla `AbstractContainerScreen.mouseClicked` (`AbstractContainerScreen.java:369-379`):
            // arm quick-craft without adding the pressed slot so a plain click
            // falls through to `Pickup` or `OutsideClick` on release.
            self.quick_craft_button_num = Some(button_num);
            self.quick_craft_slots.clear();
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
        bindings: &KeyBindings,
        code: KeyCode,
        control_down: bool,
    ) -> bool {
        let Some(selected) = self.selected_tab(snapshot) else {
            return false;
        };
        let Some(CreativeInventoryHover::Slot(slot)) = self.hovered else {
            return false;
        };
        let search_active = self.search_is_active(snapshot);
        if let Some(hotbar_index) = hotbar_slot_for_key(bindings, code) {
            // Vanilla `CreativeModeInventoryScreen.keyPressed` (`CreativeModeInventoryScreen.java:413-418`)
            // and `AbstractContainerScreen.checkHotbarKeyPressed` (`AbstractContainerScreen.java:614-627`).
            if !item_stack_is_empty(&world.inventory().cursor_item) {
                return false;
            }
            if search_active && !key_code_is_numeric(code) {
                return false;
            }
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
        if search_active {
            return false;
        }
        if bindings.input("key.swapOffhand") == BoundInput::Key(code) {
            if !item_stack_is_empty(&world.inventory().cursor_item) {
                return false;
            }
            let Some(action) = self.action_for_slot(
                world,
                item_runtime,
                selected,
                slot,
                40,
                ContainerInput::Swap,
            ) else {
                return false;
            };
            return self.submit_action(world, counters, net_commands, action);
        }
        if bindings.input("key.drop") != BoundInput::Key(code) {
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
                let item = self
                    .picker_item(world, item_runtime, selected, index)
                    .unwrap_or_else(ItemStackSummary::empty);
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
            let row = index / HUD_CREATIVE_PICKER_COLUMNS;
            let column = index % HUD_CREATIVE_PICKER_COLUMNS;
            if row >= crate::hotbar_presets::HOTBAR_PRESET_COUNT {
                return None;
            }
            if let Some(saved_row) = self
                .saved_hotbars
                .get(row)
                .filter(|row| row.iter().any(|stack| !item_stack_is_empty(stack)))
            {
                return saved_row.get(column).cloned();
            }
            if column != row {
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
        creative_inventory_visible_items(world, item_runtime, selected, &self.search.text())
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

    fn maybe_add_quick_craft_slot(
        &mut self,
        world: &WorldStore,
        selected: &CreativeInventoryTabSnapshot,
    ) {
        let Some(button_num) = self.quick_craft_button_num else {
            return;
        };
        let cursor_item = &world.inventory().cursor_item;
        if item_stack_is_empty(cursor_item) {
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
        if slot_num == 0 || self.quick_craft_slots.contains(&slot_num) {
            return;
        }
        if button_num != 2
            && cursor_item.count <= i32::try_from(self.quick_craft_slots.len()).unwrap_or(i32::MAX)
        {
            return;
        }
        if !world.active_container_slot_can_quick_craft(slot_num) {
            return;
        }
        self.quick_craft_slots.push(slot_num);
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
        let Ok(plan) = world.plan_creative_inventory_action(&action, can_drop) else {
            return false;
        };
        let drop_command_count = plan.drop_command_count();
        match queue_set_creative_mode_slot_commands_atomically(
            counters,
            net_commands,
            plan.commands(),
        ) {
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
        plan.apply();
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
        if let Some(open_code) = self.suppress_open_search_key_commit.take() {
            if crate::input::key_code_matches_opening_text(open_code, text) {
                return true;
            }
        }
        if self.search.type_text(text, None) {
            self.scroll_row = 0;
        }
        true
    }

    pub(crate) fn handle_key(
        &mut self,
        snapshot: &CreativeInventorySnapshot,
        key_bindings: &KeyBindings,
        code: KeyCode,
        modifiers: TextFieldModifiers,
        clipboard: &mut Option<&mut dyn DebugClipboard>,
    ) -> bool {
        if !self.search_is_active(snapshot)
            && key_bindings.input("key.chat") == BoundInput::Key(code)
        {
            if let Some(search) = snapshot
                .tabs
                .iter()
                .find(|tab| tab.tab_type == CreativeInventoryTabType::Search)
            {
                self.select_tab(search);
                self.suppress_open_search_key_commit = Some(code);
                return true;
            }
        }
        if !self.search_is_active(snapshot) {
            return false;
        }
        let before = self.search.value().to_vec();
        self.search.key(code, modifiers, clipboard, None);
        if self.search.value() != before {
            self.scroll_row = 0;
        }
        true
    }

    pub(crate) fn search_is_active(&self, snapshot: &CreativeInventorySnapshot) -> bool {
        self.search_focused
            && self
                .selected_tab(snapshot)
                .is_some_and(|tab| tab.tab_type == CreativeInventoryTabType::Search)
    }

    fn select_tab(&mut self, tab: &CreativeInventoryTabSnapshot) {
        let changed = self.selected_tab.as_deref() != Some(tab.resource_id.as_str());
        if changed && tab.tab_type == CreativeInventoryTabType::Search {
            self.search.set_text("", None);
        }
        self.selected_tab = Some(tab.resource_id.clone());
        self.scroll_row = 0;
        self.scrolling = false;
        self.hovered = None;
        if tab.tab_type == CreativeInventoryTabType::Search {
            self.search_focused = true;
            self.search.focus();
        } else {
            self.search.set_text("", None);
            self.search_focused = false;
            self.suppress_open_search_key_commit = None;
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
}

#[derive(PartialEq, Eq)]
struct CreativeSearchIndexKey {
    level_generation: u64,
    snapshot_generation: u64,
    snapshot_ptr: usize,
    resource_generation: u64,
    item_runtime_ptr: usize,
    tab_resource_id: String,
    item_count: usize,
    items_fingerprint: u64,
}

#[derive(PartialEq, Eq)]
struct CreativeTagSearchKey {
    world_ptr: usize,
    level_generation: u64,
    snapshot_generation: u64,
    snapshot_ptr: usize,
    tags_revision: u64,
    tags_fingerprint: u64,
    tab_resource_id: String,
    item_count: usize,
    items_fingerprint: u64,
    tag_query: String,
}

struct CreativeSearchEntry {
    namespace: Option<std::borrow::Cow<'static, str>>,
    path: Option<std::borrow::Cow<'static, str>>,
    tooltip_lower: Vec<String>,
}

#[derive(Default)]
struct CreativeSearchCache {
    snapshot_guard: Option<std::sync::Arc<CreativeInventorySnapshot>>,
    index_key: Option<CreativeSearchIndexKey>,
    entries: Vec<CreativeSearchEntry>,
    last_query: Option<String>,
    last_matched_indices: Vec<usize>,
    last_tag_key: Option<CreativeTagSearchKey>,
    last_tag_matched_indices: Vec<usize>,
    #[cfg(test)]
    index_builds: usize,
    #[cfg(test)]
    tag_scans: usize,
}

thread_local! {
    static CREATIVE_SEARCH_CACHE: std::cell::RefCell<CreativeSearchCache> =
        std::cell::RefCell::new(CreativeSearchCache::default());
}

fn creative_items_fingerprint(items: &[ItemStackSummary]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    items.len().hash(&mut hasher);
    for item in items {
        item.item_id.hash(&mut hasher);
        item.count.hash(&mut hasher);
        item.component_patch.custom_name.hash(&mut hasher);
        item.component_patch.item_name.hash(&mut hasher);
        item.component_patch.enchantments.len().hash(&mut hasher);
        item.component_patch
            .stored_enchantments
            .len()
            .hash(&mut hasher);
    }
    hasher.finish()
}

fn creative_item_tags_fingerprint(world: &WorldStore) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    if let Some(registry) = world.registry_tags("minecraft:item") {
        registry.tags.len().hash(&mut hasher);
        if let Some((key, ids)) = registry.tags.first_key_value() {
            key.hash(&mut hasher);
            ids.len().hash(&mut hasher);
            ids.first().hash(&mut hasher);
            ids.last().hash(&mut hasher);
        }
        if let Some((key, ids)) = registry.tags.last_key_value() {
            key.hash(&mut hasher);
            ids.len().hash(&mut hasher);
            ids.first().hash(&mut hasher);
            ids.last().hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn build_creative_search_entries(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    items: &[ItemStackSummary],
) -> Vec<CreativeSearchEntry> {
    items
        .iter()
        .map(|stack| {
            let (namespace, path) = match stack.item_id {
                Some(id) => {
                    if let Some(item) = vanilla_item_type(id) {
                        match item.resource_id.split_once(':') {
                            Some((ns, p)) => (
                                Some(std::borrow::Cow::Borrowed(ns)),
                                Some(std::borrow::Cow::Borrowed(p)),
                            ),
                            None => (None, Some(std::borrow::Cow::Borrowed(item.resource_id))),
                        }
                    } else if let Some(resource_id) = item_runtime
                        .and_then(|runtime| runtime.item_resource_id(id))
                        .or_else(|| {
                            world
                                .content_registry()
                                .item_by_id(id)
                                .map(|spec| spec.resource_id.as_str())
                        })
                    {
                        match resource_id.split_once(':') {
                            Some((ns, p)) => (
                                Some(std::borrow::Cow::Owned(ns.to_string())),
                                Some(std::borrow::Cow::Owned(p.to_string())),
                            ),
                            None => (None, Some(std::borrow::Cow::Owned(resource_id.to_string()))),
                        }
                    } else {
                        (None, None)
                    }
                }
                None => (None, None),
            };
            let effective = world.effective_item_stack(stack);
            let tooltip_lower = item_runtime
                .and_then(|runtime| {
                    tooltip_lines_for_stack_with_context(
                        runtime,
                        effective.as_ref(),
                        NativeItemTooltipOptions {
                            item_profiles: Some(world.item_profiles()),
                            creative: true,
                            ..NativeItemTooltipOptions::default()
                        },
                    )
                })
                .map(|lines| {
                    lines
                        .into_iter()
                        .map(|line| line.text.to_lowercase())
                        .collect()
                })
                .unwrap_or_default();
            CreativeSearchEntry {
                namespace,
                path,
                tooltip_lower,
            }
        })
        .collect()
}

fn creative_search_entry_matches(entry: &CreativeSearchEntry, query_lower: &str) -> bool {
    let (namespace_search, text_search) = query_lower
        .split_once(':')
        .map_or((None, query_lower), |(namespace, text)| {
            (Some(namespace.trim()), text.trim())
        });
    if let Some(namespace_search) = namespace_search {
        if !entry
            .namespace
            .as_deref()
            .is_some_and(|ns| ns.contains(namespace_search))
        {
            return false;
        }
    }
    if entry
        .path
        .as_deref()
        .is_some_and(|p| p.contains(text_search))
    {
        return true;
    }
    entry
        .tooltip_lower
        .iter()
        .any(|line| line.contains(text_search))
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
    let query_lower = query.to_lowercase();
    let live_snapshot = world.creative_inventory_snapshot().filter(|snapshot| {
        snapshot
            .tabs
            .iter()
            .any(|candidate| std::ptr::eq(candidate, tab))
    });
    // Live snapshots are immutable behind PublicationShared (Arc), so holding a
    // snapshot guard prevents address reuse and avoids hashing ~1,500 items per frame.
    let (snapshot_generation, snapshot_ptr, items_fingerprint) = match live_snapshot {
        Some(snapshot) => (
            snapshot.generation,
            std::ptr::from_ref(snapshot) as usize,
            0,
        ),
        None => (0, 0, creative_items_fingerprint(&tab.items)),
    };

    CREATIVE_SEARCH_CACHE.with(|cache_cell| {
        let mut cache = cache_cell.borrow_mut();
        if let Some(snapshot) = live_snapshot {
            let same_snapshot = cache
                .snapshot_guard
                .as_deref()
                .is_some_and(|cached| std::ptr::eq(cached, snapshot));
            if !same_snapshot {
                cache.snapshot_guard = world.creative_inventory_snapshot_shared();
                cache.index_key = None;
                cache.last_query = None;
                cache.last_matched_indices.clear();
                cache.last_tag_key = None;
                cache.last_tag_matched_indices.clear();
            }
        }

        if let Some(tag_search) = query_lower.strip_prefix('#') {
            let tag_key = CreativeTagSearchKey {
                world_ptr: std::ptr::from_ref(world) as usize,
                level_generation: world.client_level_generation(),
                snapshot_generation,
                snapshot_ptr,
                tags_revision: world.registry_access_revision().tags,
                tags_fingerprint: creative_item_tags_fingerprint(world),
                tab_resource_id: tab.resource_id.clone(),
                item_count: tab.items.len(),
                items_fingerprint,
                tag_query: tag_search.to_string(),
            };
            if cache.last_tag_key.as_ref() != Some(&tag_key) {
                cache.last_tag_matched_indices.clear();
                for (index, stack) in tab.items.iter().enumerate() {
                    if stack
                        .item_id
                        .is_some_and(|id| world.creative_item_matches_tag_search(id, tag_search))
                    {
                        cache.last_tag_matched_indices.push(index);
                    }
                }
                cache.last_tag_key = Some(tag_key);
                #[cfg(test)]
                {
                    cache.tag_scans += 1;
                }
            }
            return cache
                .last_tag_matched_indices
                .iter()
                .filter_map(|&index| tab.items.get(index))
                .collect();
        }

        let index_key = CreativeSearchIndexKey {
            level_generation: world.client_level_generation(),
            snapshot_generation,
            snapshot_ptr,
            resource_generation: item_runtime.map_or(0, NativeItemRuntime::resource_generation),
            item_runtime_ptr: item_runtime
                .map_or(0, |runtime| std::ptr::from_ref(runtime) as usize),
            tab_resource_id: tab.resource_id.clone(),
            item_count: tab.items.len(),
            items_fingerprint,
        };

        if cache.index_key.as_ref() != Some(&index_key) {
            cache.entries = build_creative_search_entries(world, item_runtime, &tab.items);
            cache.index_key = Some(index_key);
            cache.last_query = None;
            cache.last_matched_indices.clear();
            #[cfg(test)]
            {
                cache.index_builds += 1;
            }
        }
        let CreativeSearchCache {
            entries,
            last_query,
            last_matched_indices,
            ..
        } = &mut *cache;
        if last_query.as_deref() != Some(query_lower.as_str()) {
            last_matched_indices.clear();
            for (index, entry) in entries.iter().enumerate() {
                if creative_search_entry_matches(entry, &query_lower) {
                    last_matched_indices.push(index);
                }
            }
            *last_query = Some(query_lower);
        }
        last_matched_indices
            .iter()
            .filter_map(|&index| tab.items.get(index))
            .collect()
    })
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

fn key_code_is_numeric(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::Digit0
            | KeyCode::Digit1
            | KeyCode::Digit2
            | KeyCode::Digit3
            | KeyCode::Digit4
            | KeyCode::Digit5
            | KeyCode::Digit6
            | KeyCode::Digit7
            | KeyCode::Digit8
            | KeyCode::Digit9
            | KeyCode::Numpad0
            | KeyCode::Numpad1
            | KeyCode::Numpad2
            | KeyCode::Numpad3
            | KeyCode::Numpad4
            | KeyCode::Numpad5
            | KeyCode::Numpad6
            | KeyCode::Numpad7
            | KeyCode::Numpad8
            | KeyCode::Numpad9
    )
}

fn item_stack_is_empty(item: &ItemStackSummary) -> bool {
    item.item_id.is_none() || item.count <= 0
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use bbb_client_render::hud_creative_inventory_scroller_y;
    use bbb_client_world::{
        CreativeInventoryBackground, CreativeInventoryMaterializationRevision,
        CreativeInventoryTabType, RegistryAccessRevision,
    };
    use bbb_protocol::{
        item_types::VANILLA_ITEM_STONE_ID,
        packets::{
            PlayerAbilities, RegistryTags, TagNetworkPayload, UpdateEnabledFeatures, UpdateTags,
        },
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

    fn reopen(state: CreativeInventoryInputState) -> CreativeInventoryInputState {
        let mut input = crate::input::ClientInputState::new(true);
        input.set_screen_for_test(crate::input::Screen::Creative(state));
        input.close_screens(&mut NetCounters::default(), &None);
        assert!(input.current_screen().is_none());
        CreativeInventoryInputState::new(
            input.creative_selected_tab.take(),
            input.creative_drop_spam_count,
        )
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
        state.refresh_catalog(&original);
        assert_eq!(
            state.selected_tab(&original).unwrap().resource_id,
            "minecraft:building_blocks"
        );
        state.select_tab(&original.tabs[1]);
        state.search.set_text("stone", None);

        state.refresh_catalog(&original);
        assert_eq!(
            state.selected_tab(&original).unwrap().resource_id,
            "minecraft:search"
        );
        assert_eq!(state.search.text(), "stone");
        assert!(state.hovered.is_none());

        let refreshed = snapshot(2, true);
        state.refresh_catalog(&refreshed);
        assert_eq!(
            state.selected_tab(&refreshed).unwrap().resource_id,
            "minecraft:search"
        );
        assert_eq!(state.search.text(), "stone");

        let mut invalidated = snapshot(3, false);
        invalidated.tabs.push(tab(
            "minecraft:natural_blocks",
            CreativeInventoryTabRow::Top,
            2,
            CreativeInventoryTabType::Category,
            12,
        ));
        state.selected_tab = Some("minecraft:missing".to_string());
        state.refresh_catalog(&invalidated);
        assert_eq!(
            state.selected_tab(&invalidated).unwrap().resource_id,
            "minecraft:natural_blocks"
        );
        assert!(state.search.is_empty());

        state.scroll_row = 4;
        state.scroll_accumulator = 0.5;
        state = reopen(state);
        assert!(state.selected_tab(&invalidated).is_some());
        assert_eq!(state.scroll_row, 0);
        assert_eq!(state.scroll_accumulator, 0.0);
    }

    #[test]
    fn search_editing_is_bounded_and_tab_changes_clear_incompatible_text() {
        let snapshot = snapshot(1, true);
        let mut state = CreativeInventoryInputState::default();
        state.refresh_catalog(&snapshot);
        state.select_tab(&snapshot.tabs[1]);
        assert!(state.handle_text(&snapshot, &"a".repeat(80)));
        assert_eq!(state.search.value().len(), 50);
        assert!(state.handle_key(
            &snapshot,
            &KeyBindings::default(),
            KeyCode::KeyA,
            TextFieldModifiers {
                shortcut: true,
                ..TextFieldModifiers::default()
            },
            &mut None
        ));
        assert!(state.handle_text(&snapshot, "stone"));
        assert_eq!(state.search.text(), "stone");
        state.select_tab(&snapshot.tabs[0]);
        assert!(state.search.is_empty());
        assert!(!state.search_focused);

        state.select_tab(&snapshot.tabs[1]);
        state = reopen(state);
        assert!(!state.search_focused);
        state.refresh_catalog(&snapshot);
        assert!(state.search_focused);
        assert!(state.search.is_empty());
    }

    #[test]
    fn search_open_key_pointer_drag_and_wheel_follow_screen_gestures() {
        let snapshot = snapshot(1, true);
        let mut state = CreativeInventoryInputState::default();
        state.refresh_catalog(&snapshot);
        state.set_visible_item_count(100);

        assert!(state.handle_key(
            &snapshot,
            &KeyBindings::default(),
            KeyCode::KeyT,
            TextFieldModifiers::default(),
            &mut None
        ));
        assert_eq!(
            state.selected_tab(&snapshot).unwrap().resource_id,
            "minecraft:search"
        );
        assert!(state.search_focused);
        assert!(state.handle_text(&snapshot, "t"));
        assert!(state.search.is_empty());
        assert!(state.handle_text(&snapshot, "stone"));
        assert_eq!(state.search.text(), "stone");

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
        state.refresh_catalog(&catalog);
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
            &KeyBindings::default(),
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
            &KeyBindings::default(),
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
    fn creative_drop_throttler_limits_decays_and_resets_when_the_level_closes() {
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

        let mut input = crate::input::ClientInputState::new(true);
        input.set_screen_for_test(crate::input::Screen::Creative(state));
        world.clear_client_level();
        input.apply_screen_requests(&mut world, &mut counters, &commands);
        assert!(input.current_screen().is_none());
        assert_eq!(input.creative_drop_spam_count, 0);
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

        state = reopen(state);
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
            .join(bbb_client_assets::MC_VERSION)
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
            NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap())
                .unwrap();
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

    #[test]
    fn creative_saved_hotbars_search_key_gating_offhand_and_empty_picker_click_match_vanilla() {
        let mut world = creative_action_world();
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(16);
        let commands = Some(tx);
        let hotbars = tab(
            "minecraft:hotbar",
            CreativeInventoryTabRow::Top,
            5,
            CreativeInventoryTabType::Hotbar,
            0,
        );
        let search = CreativeInventoryTabSnapshot {
            items: vec![stack(VANILLA_ITEM_STONE_ID)],
            ..tab(
                "minecraft:search",
                CreativeInventoryTabRow::Top,
                6,
                CreativeInventoryTabType::Search,
                0,
            )
        };
        let mut catalog = snapshot(1, false);
        catalog.tabs = vec![hotbars, search];
        let mut state = CreativeInventoryInputState::default();
        state.refresh_catalog(&catalog);
        state.set_saved_hotbars(
            vec![vec![stack(
                bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID,
            )]],
            KeyBindings::default(),
        );

        // Clicking slot 0 on Saved Hotbars picks up the saved diamond item.
        state.hovered = Some(CreativeInventoryHover::Slot(CreativeInventorySlot::Picker(
            0,
        )));
        assert!(state.handle_action_press(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            0,
            false,
        ));
        assert_eq!(
            world.inventory().cursor_item,
            stack(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
        );

        // Clicking empty picker cell with non-empty cursor clears the cursor.
        state.hovered = Some(CreativeInventoryHover::Slot(CreativeInventorySlot::Picker(
            1,
        )));
        assert!(state.handle_action_press(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            0,
            false,
        ));
        assert_eq!(world.inventory().cursor_item, ItemStackSummary::empty());

        // key.swapOffhand (F) works on non-search tab (slot 45 offhand), blocked on Search tab.
        state.hovered = Some(CreativeInventoryHover::Slot(CreativeInventorySlot::Picker(
            0,
        )));
        let keys = KeyBindings::default();
        assert!(state.handle_action_key(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            &keys,
            KeyCode::KeyF,
            false,
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            NetCommand::SetCreativeModeSlot(cmd)
                if cmd.slot_num == 45
                    && cmd.item.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
                    && cmd.item.count == 64
        ));

        state.select_tab(&catalog.tabs[1]);
        state.hovered = Some(CreativeInventoryHover::Slot(CreativeInventorySlot::Picker(
            0,
        )));
        assert!(!state.handle_action_key(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            &keys,
            KeyCode::KeyQ,
            false,
        ));
        assert!(!state.handle_action_key(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            &keys,
            KeyCode::KeyF,
            false,
        ));
    }

    #[test]
    fn creative_search_cache_reuses_live_snapshot_and_caches_tag_queries() {
        let mut world = WorldStore::new();
        world.begin_registry_configuration();
        for data in bbb_client_render::scene::synchronized_registry_test_fixture::minimal_non_empty_synchronized_registry_data() {
            world.record_registry_data(data);
        }
        world
            .finish_registry_configuration()
            .expect("finish registry configuration");
        world.apply_update_tags(bbb_protocol::packets::UpdateTags {
            registries: vec![
                bbb_protocol::packets::RegistryTags {
                    registry: bbb_client_world::PAINTING_VARIANT_REGISTRY.to_string(),
                    tags: vec![bbb_protocol::packets::TagNetworkPayload {
                        tag: "minecraft:placeable".to_string(),
                        entries: vec![0],
                    }],
                },
                bbb_protocol::packets::RegistryTags {
                    registry: bbb_client_world::INSTRUMENT_REGISTRY.to_string(),
                    tags: vec![
                        bbb_protocol::packets::TagNetworkPayload {
                            tag: "minecraft:goat_horns".to_string(),
                            entries: vec![0],
                        },
                        bbb_protocol::packets::TagNetworkPayload {
                            tag: "minecraft:regular_goat_horns".to_string(),
                            entries: vec![0],
                        },
                        bbb_protocol::packets::TagNetworkPayload {
                            tag: "minecraft:screaming_goat_horns".to_string(),
                            entries: vec![],
                        },
                    ],
                },
            ],
        });
        world
            .rebuild_creative_inventory(false)
            .expect("rebuild creative inventory");
        let shared_a = world
            .creative_inventory_snapshot_shared()
            .expect("shared snapshot");
        let shared_b = world
            .creative_inventory_snapshot_shared()
            .expect("shared snapshot");
        assert!(std::ptr::eq(&*shared_a, &*shared_b));

        let search_tab = world
            .creative_inventory_snapshot()
            .and_then(|snapshot| snapshot.tab("minecraft:search"))
            .expect("search tab");

        let builds_before = CREATIVE_SEARCH_CACHE.with(|cell| cell.borrow().index_builds);
        let first = creative_inventory_visible_items(&world, None, search_tab, "diamond");
        let builds_after_first = CREATIVE_SEARCH_CACHE.with(|cell| cell.borrow().index_builds);
        let second = creative_inventory_visible_items(&world, None, search_tab, "diamond");
        let third = creative_inventory_visible_items(&world, None, search_tab, "iron");
        let builds_after_third = CREATIVE_SEARCH_CACHE.with(|cell| cell.borrow().index_builds);
        assert!(!first.is_empty());
        assert_eq!(first.len(), second.len());
        assert!(!third.is_empty());
        assert_eq!(builds_after_first, builds_before + 1);
        assert_eq!(builds_after_third, builds_after_first);

        world.apply_update_tags(bbb_protocol::packets::UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![bbb_protocol::packets::TagNetworkPayload {
                    tag: "example:gems".to_string(),
                    entries: vec![bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID],
                }],
            }],
        });
        let search_tab = world
            .creative_inventory_snapshot()
            .and_then(|snapshot| snapshot.tab("minecraft:search"))
            .expect("search tab");
        let scans_before = CREATIVE_SEARCH_CACHE.with(|cell| cell.borrow().tag_scans);
        let tag_first = creative_inventory_visible_items(&world, None, search_tab, "#gems");
        let scans_after_first = CREATIVE_SEARCH_CACHE.with(|cell| cell.borrow().tag_scans);
        let tag_second = creative_inventory_visible_items(&world, None, search_tab, "#gems");
        let scans_after_second = CREATIVE_SEARCH_CACHE.with(|cell| cell.borrow().tag_scans);
        assert_eq!(tag_first.len(), 1);
        assert_eq!(tag_second.len(), 1);
        assert_eq!(scans_after_first, scans_before + 1);
        assert_eq!(scans_after_second, scans_after_first);

        world.apply_update_tags(bbb_protocol::packets::UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![bbb_protocol::packets::TagNetworkPayload {
                    tag: "example:gems".to_string(),
                    entries: vec![
                        bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID,
                        bbb_protocol::item_types::VANILLA_ITEM_EMERALD_ID,
                    ],
                }],
            }],
        });
        let search_tab = world
            .creative_inventory_snapshot()
            .and_then(|snapshot| snapshot.tab("minecraft:search"))
            .expect("search tab");
        let tag_updated = creative_inventory_visible_items(&world, None, search_tab, "#gems");
        let scans_after_update = CREATIVE_SEARCH_CACHE.with(|cell| cell.borrow().tag_scans);
        assert_eq!(tag_updated.len(), 2);
        assert_eq!(scans_after_update, scans_after_second + 1);
    }

    #[test]
    fn creative_click_without_drag_performs_pickup_instead_of_quick_craft() {
        let mut world = creative_action_world();
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(16);
        let commands = Some(tx);
        let inventory_tab = tab(
            "minecraft:inventory",
            CreativeInventoryTabRow::Bottom,
            0,
            CreativeInventoryTabType::Inventory,
            0,
        );
        let mut catalog = snapshot(1, false);
        catalog.tabs = vec![inventory_tab];
        let mut state = CreativeInventoryInputState::default();
        state.refresh_catalog(&catalog);

        world.apply_container_set_content(bbb_protocol::packets::ContainerSetContent {
            container_id: 0,
            state_id: 1,
            items: {
                let mut items = vec![ItemStackSummary::empty(); 46];
                items[36] = stack(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID);
                items
            },
            carried_item: stack(VANILLA_ITEM_STONE_ID),
        });
        world.open_local_inventory();
        state.hovered = Some(CreativeInventoryHover::Slot(
            CreativeInventorySlot::Inventory(36),
        ));

        assert!(state.handle_action_press(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            0,
            false,
        ));
        assert!(
            state.quick_craft_slots.is_empty(),
            "mouse down alone must not add a quick-craft slot"
        );
        assert!(state.handle_action_release(
            &mut world,
            &mut counters,
            &commands,
            None,
            &catalog,
            0,
        ));
        assert_eq!(
            world.inventory().inventory_menu_slot_item(36),
            Some(&stack(VANILLA_ITEM_STONE_ID)),
            "clicking an occupied different-item slot must swap via Pickup"
        );
        assert_eq!(
            world.inventory().cursor_item,
            stack(bbb_protocol::item_types::VANILLA_ITEM_DIAMOND_ID)
        );
        assert!(rx.try_recv().is_ok());
    }
}
