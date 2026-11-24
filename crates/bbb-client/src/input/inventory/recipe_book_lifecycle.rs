use crate::control::NetCounters;
use bbb_client_net::NetCommand;
use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_world::WorldStore;
use bbb_protocol::packets::{RecipeBookSeenRecipeCommand, RecipeBookType, RecipeDisplayId};
use tokio::sync::mpsc;

use super::*;
use crate::recipe_book_ui::{
    clamped_recipe_book_page, crafting_recipe_book_tab_has_highlighted_recipe,
    furnace_recipe_book_tab_has_highlighted_recipe, RECIPE_BOOK_ITEMS_PER_PAGE,
};

const RECIPE_BOOK_ANIMATION_TICKS: u8 = 15;
const RECIPE_BOOK_TAB_COUNT: usize = 5;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RecipeBookPresentationHudState {
    pub(crate) tab_animation_ticks: [u8; RECIPE_BOOK_TAB_COUNT],
    pub(crate) button_animation_ticks: [u8; RECIPE_BOOK_ITEMS_PER_PAGE],
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RecipeBookPresentationState {
    last_recipe_revision: u64,
    last_filtering: bool,
    was_visible: bool,
    hud: RecipeBookPresentationHudState,
}

impl RecipeBookPresentationState {
    pub(crate) fn hud_state(&self) -> RecipeBookPresentationHudState {
        self.hud
    }

    fn advance_visible_animations(&mut self, ticks: u32) {
        let ticks = ticks.min(u32::from(u8::MAX)) as u8;
        for remaining in &mut self.hud.tab_animation_ticks {
            *remaining = remaining.saturating_sub(ticks);
        }
        for remaining in &mut self.hud.button_animation_ticks {
            *remaining = remaining.saturating_sub(ticks);
        }
    }
}

pub(crate) fn sync_recipe_book_presentation(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    net_commands: &Option<mpsc::Sender<NetCommand>>,
    item_runtime: Option<&NativeItemRuntime>,
    advanced_ticks: u32,
) {
    if input.retained_container().is_none() {
        return;
    }
    let Some(layout) = inventory_screen_layout(world) else {
        return;
    };
    let Some(book_type) = recipe_book_type_for_background(layout.background) else {
        return;
    };
    if !recipe_book_type_settings(world, book_type).open {
        input.container_mut().recipe_book_presentation.was_visible = false;
        return;
    }

    input
        .container_mut()
        .recipe_book_presentation
        .advance_visible_animations(advanced_ticks);
    let recipe_revision = world.recipe_book().revision;
    let filtering = recipe_book_type_settings(world, book_type).filtering;
    let refresh_tabs = !input.container_mut().recipe_book_presentation.was_visible
        || input
            .container_mut()
            .recipe_book_presentation
            .last_recipe_revision
            != recipe_revision
        || input
            .container_mut()
            .recipe_book_presentation
            .last_filtering
            != filtering;
    if refresh_tabs {
        start_highlighted_tab_animations(input, world, layout.background, book_type);
    }

    let visible_highlights =
        visible_highlighted_recipe_ids(input, world, item_runtime, layout.background, book_type);
    input.container_mut().recipe_book_presentation.was_visible = true;
    input
        .container_mut()
        .recipe_book_presentation
        .last_recipe_revision = recipe_revision;
    input
        .container_mut()
        .recipe_book_presentation
        .last_filtering = filtering;

    for (button_index, recipe) in visible_highlights {
        if !queue_recipe_book_seen_recipe_command(
            counters,
            net_commands,
            RecipeBookSeenRecipeCommand { recipe },
        ) {
            break;
        }
        if world.remove_recipe_highlight(recipe) {
            input
                .container_mut()
                .recipe_book_presentation
                .hud
                .button_animation_ticks[button_index] = RECIPE_BOOK_ANIMATION_TICKS;
        }
    }
}

fn start_highlighted_tab_animations(
    input: &mut ClientInputState,
    world: &WorldStore,
    background: InventoryScreenBackground,
    book_type: RecipeBookType,
) {
    if world.recipe_book().highlights.is_empty() {
        return;
    }
    let only_craftable = recipe_book_type_settings(world, book_type).filtering;
    for tab_index in recipe_book_visible_tab_indices(world, background) {
        let highlighted =
            recipe_book_crafting_grid_for_background(background).is_some_and(|grid| {
                crafting_recipe_book_tab_has_highlighted_recipe(
                    world,
                    grid,
                    tab_index,
                    only_craftable,
                )
            }) || recipe_book_furnace_family_for_background(background).is_some_and(|family| {
                furnace_recipe_book_tab_has_highlighted_recipe(
                    world,
                    family,
                    tab_index,
                    only_craftable,
                )
            });
        if highlighted {
            if let Some(remaining) = input
                .container_mut()
                .recipe_book_presentation
                .hud
                .tab_animation_ticks
                .get_mut(tab_index)
            {
                *remaining = RECIPE_BOOK_ANIMATION_TICKS;
            }
        }
    }
}

fn visible_highlighted_recipe_ids(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    background: InventoryScreenBackground,
    book_type: RecipeBookType,
) -> Vec<(usize, RecipeDisplayId)> {
    if world.recipe_book().highlights.is_empty() {
        return Vec::new();
    }
    let Some(collections) =
        recipe_book_collections_for_background(input, world, item_runtime, background, book_type)
    else {
        return Vec::new();
    };
    let page = clamped_recipe_book_page(
        selected_recipe_book_page_index(input, book_type),
        collections.len(),
    );
    collections
        .into_iter()
        .skip(page * RECIPE_BOOK_ITEMS_PER_PAGE)
        .take(RECIPE_BOOK_ITEMS_PER_PAGE)
        .enumerate()
        .flat_map(|(button_index, collection)| {
            collection
                .recipe_indices()
                .filter(|recipe| world.recipe_book().highlights.contains(recipe))
                .map(move |index| (button_index, RecipeDisplayId { index }))
                .collect::<Vec<_>>()
        })
        .collect()
}
