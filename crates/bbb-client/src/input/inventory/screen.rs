use super as inventory;
use super::super::{
    enchantment_screen, text_field, AnvilRenameInputSignature, RecipeBookOverlayHudState,
};
use super::super::{
    physical_position_floor_i32, RecipeBookPageHudState, RecipeBookSearchHudState,
    RecipeBookTabSelectionHudState,
};
use super::super::{screen::Screen, ClientInputState};
use super::creative::CreativeInventoryInputState;
use super::RecipeBookPresentationHudState;
use super::{
    beacon_data_effect_id, loom_pattern_max_scroll_row, loom_selectable_pattern_count,
    stonecutter_recipe_max_scroll_row, BEACON_PRIMARY_EFFECT_DATA_ID,
    BEACON_SECONDARY_EFFECT_DATA_ID, LOOM_SELECTED_PATTERN_DATA_ID, VANILLA_MENU_TYPE_BEACON_ID,
    VANILLA_MENU_TYPE_LOOM_ID, VANILLA_MENU_TYPE_STONECUTTER_ID,
};
use crate::control::NetCounters;
use bbb_client_clock::Instant;
use bbb_client_net::NetCommand;
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_world::WorldStore;
use bbb_protocol::packets::ItemStackSummary;
use tokio::sync::mpsc;
use winit::dpi::PhysicalPosition;

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ContainerInputState {
    pub(in crate::input) inventory_hovered_slot: Option<i16>,
    pub(in crate::input) inventory_last_click_slot: Option<i16>,
    pub(in crate::input) inventory_last_click_button_num: Option<i8>,
    pub(in crate::input) inventory_last_click_at: Option<Instant>,
    pub(in crate::input) inventory_double_click: bool,
    pub(in crate::input) inventory_skip_next_release: bool,
    pub(in crate::input) inventory_last_quick_moved: Option<ItemStackSummary>,
    pub(in crate::input) inventory_quick_craft_button_num: Option<i8>,
    pub(in crate::input) inventory_quick_craft_slots: Vec<i16>,
    pub(in crate::input) stonecutter_recipe_scroll_row: i32,
    pub(in crate::input) stonecutter_recipe_scroll_input_item_id: Option<i32>,
    pub(in crate::input) stonecutter_recipe_scrolling: bool,
    pub(in crate::input) enchantment_screen: Option<enchantment_screen::EnchantmentScreenState>,
    pub(in crate::input) loom_pattern_scroll_row: i32,
    pub(in crate::input) loom_pattern_scrolling: bool,
    pub(in crate::input) loom_pattern_selection_dirty: bool,
    pub(in crate::input) loom_selected_pattern_index: Option<i32>,
    pub(in crate::input) beacon_effect_selection_dirty: bool,
    pub(in crate::input) beacon_primary_effect: Option<i32>,
    pub(in crate::input) beacon_secondary_effect: Option<i32>,
    pub(in crate::input) anvil_rename_input: Option<AnvilRenameInputSignature>,
    pub(in crate::input) anvil_rename:
        text_field::BoundedTextField<{ super::ANVIL_RENAME_MAX_LENGTH }>,
    pub(in crate::input) anvil_rename_hover_name: String,
    pub(in crate::input) recipe_book_search:
        text_field::BoundedTextField<{ super::RECIPE_BOOK_SEARCH_MAX_LENGTH }>,
    pub(crate) recipe_book_search_focused: bool,
    pub(in crate::input) recipe_book_search_suppress_open_key_commit: bool,
    pub(in crate::input) recipe_book_presentation: super::RecipeBookPresentationState,
    pub(in crate::input) recipe_book_overlay: Option<RecipeBookOverlayHudState>,
    pub(in crate::input) recipe_book_last_placed_recipe: Option<(i32, i32)>,
    pub(in crate::input) merchant_trade_scrolling: bool,
    pub(in crate::input) merchant_focused_trade_row: Option<i32>,
}

impl ClientInputState {
    pub(crate) fn current_screen(&self) -> Option<&Screen> {
        self.screen.current()
    }

    pub(in crate::input) fn retained_container(&self) -> Option<&ContainerInputState> {
        self.screen.iter().find_map(|screen| match screen {
            Screen::Container(state) => Some(state),
            _ => None,
        })
    }
    pub(in crate::input) fn container(&self) -> &ContainerInputState {
        self.retained_container().expect("container component")
    }
    pub(in crate::input) fn container_mut(&mut self) -> &mut ContainerInputState {
        self.screen
            .iter_mut()
            .find_map(|screen| match screen {
                Screen::Container(state) => Some(state),
                _ => None,
            })
            .expect("container component")
    }
    pub(crate) fn inventory_screen_is_open(&self) -> bool {
        matches!(
            self.screen.current(),
            Some(Screen::Container(_) | Screen::Creative(_))
        )
    }

    pub(crate) fn creative_inventory_context(&self) -> Option<bool> {
        self.screen.iter().find_map(|screen| match screen {
            Screen::Creative(state) => Some(state.display_operator_tab),
            _ => None,
        })
    }

    fn load_saved_hotbar_rows(&mut self, world: &WorldStore) -> Vec<Vec<ItemStackSummary>> {
        let Some(store) = self.hotbar_preset_store.as_mut() else {
            return Vec::new();
        };
        (0..crate::hotbar_presets::HOTBAR_PRESET_COUNT)
            .map(|index| store.preset(index, world))
            .collect()
    }

    pub(in crate::input) fn open_inventory_screen(
        &mut self,
        world: &mut WorldStore,
        creative: Option<bool>,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        self.close_screens(counters, net_commands);
        let screen = if let Some(display_operator_tab) = creative {
            let mut creative = CreativeInventoryInputState::new(
                self.creative_selected_tab.take(),
                std::mem::take(&mut self.creative_drop_spam_count),
            );
            creative.display_operator_tab = display_operator_tab;
            if let Some(snapshot) = world.creative_inventory_snapshot() {
                creative.refresh_catalog(snapshot);
            }
            let saved_hotbars = self.load_saved_hotbar_rows(world);
            creative.set_saved_hotbars(saved_hotbars, self.key_bindings.clone());
            Screen::Creative(creative)
        } else {
            // Vanilla `RecipeBookComponent.initVisuals` (`RecipeBookComponent.java:115-134`)
            // persists only the recipe book search text across screen closes; search focus,
            // overlay state, and `lastPlacedRecipe` start cleared on each open.
            let _ = std::mem::take(&mut self.recipe_book_search_focused_preference);
            let _ = self.recipe_book_overlay_preference.take();
            let _ = self.recipe_book_last_placed_preference.take();
            Screen::Container(ContainerInputState {
                recipe_book_search: std::mem::take(&mut self.recipe_book_search_preference),
                ..ContainerInputState::default()
            })
        };
        self.set_screen(screen, counters, net_commands);
    }

    pub(in crate::input) fn close_retained_inventory(&mut self) {
        for screen in self
            .screen
            .remove_if(|screen| matches!(screen, Screen::Container(_) | Screen::Creative(_)))
        {
            self.remember_inventory_preferences(screen);
        }
    }
    pub(in crate::input) fn remember_inventory_preferences(&mut self, screen: Screen) {
        match screen {
            Screen::Creative(state) => {
                self.creative_selected_tab = state.selected_tab;
                self.creative_drop_spam_count = state.drop_spam_count;
            }
            Screen::Container(state) => {
                self.recipe_book_search_preference = state.recipe_book_search;
                self.recipe_book_search_focused_preference = false;
                self.recipe_book_overlay_preference = None;
                self.recipe_book_last_placed_preference = None;
            }
            _ => {}
        }
    }

    pub(crate) fn refresh_creative_inventory(&mut self, world: &mut WorldStore) {
        let Some(operator_tab) = self.creative_inventory_context() else {
            return;
        };
        match world.rebuild_creative_inventory(operator_tab) {
            Ok(true) => self.creative_catalog_changed(world),
            Ok(false) => {}
            Err(error) => tracing::error!(?error, "creative inventory rebuild failed"),
        }
    }

    pub(crate) fn creative_catalog_changed(&mut self, world: &WorldStore) {
        let Some(snapshot) = world.creative_inventory_snapshot() else {
            return;
        };
        for screen in self.screen.iter_mut() {
            if let Screen::Creative(state) = screen {
                state.refresh_catalog(snapshot);
            }
        }
    }
}

impl ContainerInputState {
    pub(in crate::input) fn clear_pressed(&mut self) {
        self.inventory_hovered_slot = None;
        self.inventory_last_click_slot = None;
        self.inventory_last_click_button_num = None;
        self.inventory_last_click_at = None;
        self.inventory_double_click = false;
        self.inventory_skip_next_release = false;
        self.inventory_last_quick_moved = None;
        self.inventory_quick_craft_button_num = None;
        self.inventory_quick_craft_slots.clear();
        self.loom_pattern_scrolling = false;
        self.merchant_trade_scrolling = false;
        self.merchant_focused_trade_row = None;
        self.stonecutter_recipe_scrolling = false;
        self.anvil_rename_input = None;
        self.anvil_rename.set_text("", None);
        self.anvil_rename_hover_name.clear();
        self.recipe_book_search_focused = false;
        self.recipe_book_search_suppress_open_key_commit = false;
        self.recipe_book_overlay = None;
        self.recipe_book_last_placed_recipe = None;
    }
}

impl ClientInputState {
    pub(crate) fn inventory_hovered_slot(&self) -> Option<i16> {
        self.screen
            .container()
            .and_then(|state| state.inventory_hovered_slot)
            .or_else(|| {
                self.screen
                    .creative()
                    .and_then(|state| state.hovered_slot())
                    .and_then(|slot| i16::try_from(slot.hud_id()).ok())
            })
    }

    pub(crate) fn inventory_cursor_position(&self) -> Option<(i32, i32)> {
        self.inventory_cursor_position
    }

    pub(crate) fn refresh_creative_visible_items(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&NativeItemRuntime>,
    ) {
        let Some(snapshot) = world.creative_inventory_snapshot() else {
            return;
        };
        let needs_hotbar_sync = self.screen.iter().any(|screen| {
            matches!(
                screen,
                Screen::Creative(state)
                    if !state.hotbar_presets_loaded()
                        || state.hotbar_key_bindings() != &self.key_bindings
            )
        });
        let refreshed_hotbars = needs_hotbar_sync.then(|| self.load_saved_hotbar_rows(world));
        let key_bindings = needs_hotbar_sync.then(|| self.key_bindings.clone());
        for screen in self.screen.iter_mut() {
            let Screen::Creative(state) = screen else {
                continue;
            };
            if let (Some(rows), Some(bindings)) = (&refreshed_hotbars, &key_bindings) {
                state.set_saved_hotbars(rows.clone(), bindings.clone());
            }
            let count = state
                .selected_tab(snapshot)
                .map(|tab| {
                    if tab.tab_type == bbb_client_world::CreativeInventoryTabType::Hotbar {
                        81
                    } else {
                        inventory::creative::creative_inventory_visible_items(
                            world,
                            item_runtime,
                            tab,
                            &state.search_text(),
                        )
                        .len()
                    }
                })
                .unwrap_or_default();
            state.set_visible_item_count(count);
        }
    }

    pub(crate) fn creative_inventory_hud_state(
        &self,
        world: &WorldStore,
    ) -> Option<inventory::creative::CreativeInventoryHudState> {
        let creative = self.screen.creative()?;
        let snapshot = world.creative_inventory_snapshot()?;
        creative.hud_state(snapshot, creative.visible_item_count())
    }

    pub(crate) fn merchant_focused_trade_row(&self) -> Option<i32> {
        self.screen
            .container()
            .and_then(|state| state.merchant_focused_trade_row)
    }

    pub(crate) fn update_screen_cursor_position(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
    ) {
        self.inventory_cursor_position = cursor_position.and_then(physical_position_floor_i32);
        if let Some(container) = self.screen.container_mut() {
            container.inventory_hovered_slot = None;
        }
    }

    pub(crate) fn inventory_quick_craft_button_num(&self) -> Option<i8> {
        self.screen
            .container()
            .and_then(|state| state.inventory_quick_craft_button_num)
    }

    pub(crate) fn inventory_quick_craft_slots(&self) -> &[i16] {
        self.screen
            .container()
            .map_or(&[], |state| state.inventory_quick_craft_slots.as_slice())
    }

    pub(crate) fn stonecutter_recipe_scroll_row(&self) -> i32 {
        self.screen
            .container()
            .map_or(0, |state| state.stonecutter_recipe_scroll_row)
    }

    pub(crate) fn loom_pattern_scroll_row(&self) -> i32 {
        self.screen
            .container()
            .map_or(0, |state| state.loom_pattern_scroll_row)
    }

    pub(crate) fn recipe_book_search_receipt(&self, world: &WorldStore) -> (bool, usize) {
        (
            inventory::recipe_book_search_is_active(self, world),
            self.retained_container()
                .map_or(&self.recipe_book_search_preference, |state| {
                    &state.recipe_book_search
                })
                .text()
                .chars()
                .count(),
        )
    }

    pub(crate) fn loom_selected_pattern_index(&self) -> Option<i32> {
        self.screen
            .container()
            .and_then(|state| state.loom_selected_pattern_index)
    }

    pub(crate) fn beacon_effect_selection(&self) -> (Option<i32>, Option<i32>) {
        self.retained_container().map_or((None, None), |state| {
            (state.beacon_primary_effect, state.beacon_secondary_effect)
        })
    }

    pub(crate) fn anvil_rename_text(&self) -> String {
        self.screen
            .container()
            .map_or_else(String::new, |state| state.anvil_rename.text())
    }

    pub(crate) fn recipe_book_search_hud_state(&self) -> RecipeBookSearchHudState {
        RecipeBookSearchHudState {
            edit: text_field::TextField::clone(
                self.screen
                    .container()
                    .map_or(&self.recipe_book_search_preference, |state| {
                        &state.recipe_book_search
                    }),
            ),
            focused: self
                .screen
                .container()
                .is_some_and(|state| state.recipe_book_search_focused),
        }
    }

    pub(crate) fn recipe_book_tab_selection_hud_state(&self) -> RecipeBookTabSelectionHudState {
        RecipeBookTabSelectionHudState {
            crafting: self.recipe_book_crafting_tab_index,
            furnace: self.recipe_book_furnace_tab_index,
            blast_furnace: self.recipe_book_blast_furnace_tab_index,
            smoker: self.recipe_book_smoker_tab_index,
        }
    }

    pub(crate) fn recipe_book_page_hud_state(&self) -> RecipeBookPageHudState {
        RecipeBookPageHudState {
            crafting: self.recipe_book_crafting_page,
            furnace: self.recipe_book_furnace_page,
            blast_furnace: self.recipe_book_blast_furnace_page,
            smoker: self.recipe_book_smoker_page,
        }
    }

    pub(crate) fn recipe_book_presentation_hud_state(&self) -> RecipeBookPresentationHudState {
        self.screen
            .container()
            .map_or_else(Default::default, |state| {
                state.recipe_book_presentation.hud_state()
            })
    }

    pub(crate) fn recipe_book_overlay_hud_state(&self) -> Option<RecipeBookOverlayHudState> {
        self.screen
            .container()
            .and_then(|state| state.recipe_book_overlay)
    }
}

pub(super) fn sync_stonecutter_recipe_scroll(input: &mut ClientInputState, world: &WorldStore) {
    let input_item_id = stonecutter_input_item_id(world);
    if input
        .container_mut()
        .stonecutter_recipe_scroll_input_item_id
        != input_item_id
    {
        input
            .container_mut()
            .stonecutter_recipe_scroll_input_item_id = input_item_id;
        input.container_mut().stonecutter_recipe_scroll_row = 0;
        input.container_mut().stonecutter_recipe_scrolling = false;
    }
    let max_scroll_row = stonecutter_recipe_max_scroll_row(world).unwrap_or_default();
    input.container_mut().stonecutter_recipe_scroll_row = input
        .container_mut()
        .stonecutter_recipe_scroll_row
        .clamp(0, max_scroll_row);
}

pub(super) fn sync_loom_pattern_scroll(input: &mut ClientInputState, world: &WorldStore) {
    let max_scroll_row = loom_pattern_max_scroll_row(world).unwrap_or_default();
    input.container_mut().loom_pattern_scroll_row = input
        .container_mut()
        .loom_pattern_scroll_row
        .clamp(0, max_scroll_row);
    if max_scroll_row <= 0 {
        input.container_mut().loom_pattern_scrolling = false;
    }
}

pub(super) fn sync_loom_pattern_state(input: &mut ClientInputState, world: &WorldStore) {
    let Some(container) = world.inventory().open_container.as_ref() else {
        input.container_mut().loom_pattern_selection_dirty = false;
        input.container_mut().loom_selected_pattern_index = None;
        input.container_mut().loom_pattern_scroll_row = 0;
        input.container_mut().loom_pattern_scrolling = false;
        return;
    };
    if container.menu_type_id != Some(VANILLA_MENU_TYPE_LOOM_ID) {
        input.container_mut().loom_pattern_selection_dirty = false;
        input.container_mut().loom_selected_pattern_index = None;
        input.container_mut().loom_pattern_scroll_row = 0;
        input.container_mut().loom_pattern_scrolling = false;
        return;
    }

    if !input.container_mut().loom_pattern_selection_dirty {
        input.container_mut().loom_selected_pattern_index = world
            .open_container_data_value(LOOM_SELECTED_PATTERN_DATA_ID)
            .and_then(|value| (value >= 0).then_some(i32::from(value)));
    }
    let selectable_count = loom_selectable_pattern_count(world).unwrap_or_default();
    if input
        .container_mut()
        .loom_selected_pattern_index
        .is_some_and(|index| index < 0 || index >= selectable_count)
    {
        input.container_mut().loom_selected_pattern_index = None;
        input.container_mut().loom_pattern_selection_dirty = false;
    }
    sync_loom_pattern_scroll(input, world);
}

pub(crate) fn sync_stonecutter_recipe_scroll_state(
    input: &mut ClientInputState,
    world: &WorldStore,
) {
    if input.retained_container().is_none() {
        return;
    }
    sync_stonecutter_recipe_scroll(input, world);
}

pub(crate) fn sync_loom_pattern_state_for_hud(input: &mut ClientInputState, world: &WorldStore) {
    if input.retained_container().is_none() {
        return;
    }
    sync_loom_pattern_state(input, world);
}

pub(crate) fn sync_beacon_effect_selection_state(input: &mut ClientInputState, world: &WorldStore) {
    if input.retained_container().is_none() {
        return;
    }
    sync_beacon_effect_selection(input, world);
}

fn stonecutter_input_item_id(world: &WorldStore) -> Option<i32> {
    let container = world.inventory().open_container.as_ref()?;
    if container.menu_type_id != Some(VANILLA_MENU_TYPE_STONECUTTER_ID) {
        return None;
    }
    world
        .inventory()
        .container_slot(container, 0)
        .and_then(|slot| slot.item.item_id)
}

pub(super) fn sync_beacon_effect_selection(input: &mut ClientInputState, world: &WorldStore) {
    let Some(container) = world.inventory().open_container.as_ref() else {
        input.container_mut().beacon_effect_selection_dirty = false;
        input.container_mut().beacon_primary_effect = None;
        input.container_mut().beacon_secondary_effect = None;
        return;
    };
    if container.menu_type_id != Some(VANILLA_MENU_TYPE_BEACON_ID) {
        input.container_mut().beacon_effect_selection_dirty = false;
        input.container_mut().beacon_primary_effect = None;
        input.container_mut().beacon_secondary_effect = None;
        return;
    }

    if !input.container_mut().beacon_effect_selection_dirty {
        input.container_mut().beacon_primary_effect =
            beacon_data_effect_id(world, BEACON_PRIMARY_EFFECT_DATA_ID);
        input.container_mut().beacon_secondary_effect =
            beacon_data_effect_id(world, BEACON_SECONDARY_EFFECT_DATA_ID);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_client_world::{LevelEventSoundRandomState, PlayApplyEffects};
    use bbb_protocol::packets::{
        ContainerClose, ContainerSetSlot, DialogHolder, ItemStackSummary, PlayClientbound,
        PlayerAbilities, SetCursorItem, ShowDialog, UpdateEnabledFeatures,
    };

    struct ScreenEffects<'a>(&'a ClientInputState);
    impl PlayApplyEffects for ScreenEffects<'_> {
        fn creative_inventory_context(&self) -> Option<bool> {
            self.0.creative_inventory_context()
        }
    }

    fn apply(input: &mut ClientInputState, world: &mut WorldStore, packet: PlayClientbound) {
        assert!(world
            .apply_play_packet(
                packet,
                &mut LevelEventSoundRandomState::default(),
                &mut ScreenEffects(input),
            )
            .is_none());
        input.apply_screen_requests(world, &mut NetCounters::default(), &None);
    }

    fn creative_screen() -> (ClientInputState, WorldStore) {
        let mut world = WorldStore::new();
        world.apply_update_enabled_features(UpdateEnabledFeatures {
            features: Vec::new(),
        });
        world.apply_player_abilities(abilities(true));
        world.open_local_inventory_with_operator_tab(false).unwrap();
        let mut input = ClientInputState::new(true);
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        assert!(matches!(input.current_screen(), Some(Screen::Creative(_))));
        (input, world)
    }

    fn abilities(instabuild: bool) -> PlayerAbilities {
        PlayerAbilities {
            invulnerable: instabuild,
            flying: false,
            can_fly: instabuild,
            instabuild,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }
    }

    fn item(count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count,
            ..ItemStackSummary::empty()
        }
    }

    fn show_dialog(input: &mut ClientInputState, world: &mut WorldStore) {
        world.apply_show_dialog(ShowDialog {
            dialog: DialogHolder::Reference { registry_id: 0 },
        });
        input.apply_screen_requests(world, &mut NetCounters::default(), &None);
        assert_eq!(input.creative_inventory_context(), Some(false));
    }

    #[test]
    fn dialog_retains_creative_packet_context_until_the_container_closes() {
        let (mut input, mut world) = creative_screen();
        show_dialog(&mut input, &mut world);
        apply(
            &mut input,
            &mut world,
            PlayClientbound::SetCursorItem(SetCursorItem { item: item(3) }),
        );
        assert_eq!(world.inventory().cursor_item, ItemStackSummary::empty());
        apply(
            &mut input,
            &mut world,
            PlayClientbound::ContainerSetSlot(ContainerSetSlot {
                container_id: 0,
                state_id: 1,
                slot: 36,
                item: item(2),
            }),
        );
        assert_eq!(world.inventory().player_slot(0).unwrap().item, item(2));
        apply(
            &mut input,
            &mut world,
            PlayClientbound::ContainerClose(ContainerClose { container_id: 0 }),
        );
        assert_eq!(input.creative_inventory_context(), None);
        world.apply_clear_dialog();
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        assert!(input.current_screen().is_none());
        apply(
            &mut input,
            &mut world,
            PlayClientbound::SetCursorItem(SetCursorItem { item: item(3) }),
        );
        assert_eq!(world.inventory().cursor_item, item(3));
    }

    #[test]
    fn losing_creative_ability_replaces_the_retained_screen_with_survival_inventory() {
        let (mut input, mut world) = creative_screen();
        show_dialog(&mut input, &mut world);
        apply(
            &mut input,
            &mut world,
            PlayClientbound::PlayerAbilities(abilities(false)),
        );
        assert!(matches!(input.current_screen(), Some(Screen::Container(_))));
        assert!(!input.dialog_screen_is_open());
        assert!(world.local_inventory_is_open());
        assert_eq!(input.creative_inventory_context(), None);
        assert!(world.screen_requests().is_empty());
        input.apply_screen_requests(&mut world, &mut NetCounters::default(), &None);
        assert!(matches!(input.current_screen(), Some(Screen::Container(_))));
    }
}
