use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use bbb_item_model::NativeItemRuntime;
use bbb_protocol::packets::{
    CraftingRecipeDisplaySummary, DataComponentPatchSummary, IngredientSummary, ItemStackSummary,
    RecipeDisplayEntry, RecipeDisplaySummary, SlotDisplaySummary,
};
use bbb_world::WorldStore;

pub(crate) const RECIPE_BOOK_ITEMS_PER_PAGE: usize = 20;
pub(crate) const RECIPE_BOOK_OVERLAY_BUTTON_SIZE: i32 = 25;
pub(crate) const RECIPE_BOOK_OVERLAY_BUTTON_DRAW_SIZE: i32 = 24;
pub(crate) const RECIPE_BOOK_OVERLAY_BACKGROUND_BORDER: i32 = 8;
pub(crate) const RECIPE_BOOK_OVERLAY_MAX_ROW: usize = 4;
pub(crate) const RECIPE_BOOK_OVERLAY_MAX_ROW_LARGE: usize = 5;
pub(crate) const RECIPE_BOOK_OVERLAY_ITEM_SCALE: f32 = 0.375;

const RECIPE_BOOK_OVERLAY_GRID_ITEM_OFFSET: i32 = 2;
const RECIPE_BOOK_OVERLAY_GRID_ITEM_STRIDE: i32 = 7;

const CRAFTING_BUILDING_BLOCKS_CATEGORY_ID: i32 = 0;
const CRAFTING_REDSTONE_CATEGORY_ID: i32 = 1;
const CRAFTING_EQUIPMENT_CATEGORY_ID: i32 = 2;
const CRAFTING_MISC_CATEGORY_ID: i32 = 3;

const CRAFTING_SEARCH_TAB_CATEGORIES: [i32; 4] = [
    CRAFTING_EQUIPMENT_CATEGORY_ID,
    CRAFTING_BUILDING_BLOCKS_CATEGORY_ID,
    CRAFTING_MISC_CATEGORY_ID,
    CRAFTING_REDSTONE_CATEGORY_ID,
];
const CRAFTING_EQUIPMENT_TAB_CATEGORIES: [i32; 1] = [CRAFTING_EQUIPMENT_CATEGORY_ID];
const CRAFTING_BUILDING_BLOCKS_TAB_CATEGORIES: [i32; 1] = [CRAFTING_BUILDING_BLOCKS_CATEGORY_ID];
const CRAFTING_MISC_TAB_CATEGORIES: [i32; 1] = [CRAFTING_MISC_CATEGORY_ID];
const CRAFTING_REDSTONE_TAB_CATEGORIES: [i32; 1] = [CRAFTING_REDSTONE_CATEGORY_ID];

const FURNACE_FOOD_CATEGORY_ID: i32 = 4;
const FURNACE_BLOCKS_CATEGORY_ID: i32 = 5;
const FURNACE_MISC_CATEGORY_ID: i32 = 6;
const BLAST_FURNACE_BLOCKS_CATEGORY_ID: i32 = 7;
const BLAST_FURNACE_MISC_CATEGORY_ID: i32 = 8;
const SMOKER_FOOD_CATEGORY_ID: i32 = 9;

const FURNACE_SEARCH_TAB_CATEGORIES: [i32; 3] = [
    FURNACE_FOOD_CATEGORY_ID,
    FURNACE_BLOCKS_CATEGORY_ID,
    FURNACE_MISC_CATEGORY_ID,
];
const FURNACE_FOOD_TAB_CATEGORIES: [i32; 1] = [FURNACE_FOOD_CATEGORY_ID];
const FURNACE_BLOCKS_TAB_CATEGORIES: [i32; 1] = [FURNACE_BLOCKS_CATEGORY_ID];
const FURNACE_MISC_TAB_CATEGORIES: [i32; 1] = [FURNACE_MISC_CATEGORY_ID];
const BLAST_FURNACE_SEARCH_TAB_CATEGORIES: [i32; 2] = [
    BLAST_FURNACE_BLOCKS_CATEGORY_ID,
    BLAST_FURNACE_MISC_CATEGORY_ID,
];
const BLAST_FURNACE_BLOCKS_TAB_CATEGORIES: [i32; 1] = [BLAST_FURNACE_BLOCKS_CATEGORY_ID];
const BLAST_FURNACE_MISC_TAB_CATEGORIES: [i32; 1] = [BLAST_FURNACE_MISC_CATEGORY_ID];
const SMOKER_SEARCH_TAB_CATEGORIES: [i32; 1] = [SMOKER_FOOD_CATEGORY_ID];
const SMOKER_FOOD_TAB_CATEGORIES: [i32; 1] = [SMOKER_FOOD_CATEGORY_ID];

const PLAYER_INVENTORY_SLOT_START: i32 = 0;
const PLAYER_INVENTORY_SLOT_END: i32 = 36;
const LOCAL_INVENTORY_CRAFT_SLOT_START: i16 = 1;
const LOCAL_INVENTORY_CRAFT_SLOT_END: i16 = 5;
const LOCAL_INVENTORY_PLAYER_MAIN_START: i16 = 9;
const LOCAL_INVENTORY_PLAYER_MAIN_END: i16 = 36;
const LOCAL_INVENTORY_HOTBAR_START: i16 = 36;
const LOCAL_INVENTORY_HOTBAR_END: i16 = 45;
const CRAFTING_TABLE_CRAFT_SLOT_START: i16 = 1;
const CRAFTING_TABLE_CRAFT_SLOT_END: i16 = 10;
const CRAFTING_TABLE_PLAYER_MAIN_START: i16 = 10;
const CRAFTING_TABLE_PLAYER_MAIN_END: i16 = 37;
const CRAFTING_TABLE_HOTBAR_START: i16 = 37;
const CRAFTING_TABLE_HOTBAR_END: i16 = 46;
const FURNACE_INGREDIENT_SLOT: i16 = 0;
const FURNACE_FUEL_SLOT: i16 = 1;
const FURNACE_RESULT_SLOT: i16 = 2;
const FURNACE_CRAFT_SLOT_START: i16 = 0;
const FURNACE_CRAFT_SLOT_END: i16 = 3;
const FURNACE_PLAYER_MAIN_START: i16 = 3;
const FURNACE_PLAYER_MAIN_END: i16 = 30;
const FURNACE_HOTBAR_START: i16 = 30;
const FURNACE_HOTBAR_END: i16 = 39;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecipeBookCraftingGrid {
    pub(crate) width: i32,
    pub(crate) height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecipeBookFurnaceFamily {
    Furnace,
    BlastFurnace,
    Smoker,
}

#[derive(Debug, Clone)]
pub(crate) struct RecipeBookUiCollection<'a> {
    entries: Vec<&'a RecipeDisplayEntry>,
    craftable_entries: Vec<bool>,
    has_craftable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RecipeBookOverlayEntry<'a> {
    pub(crate) recipe_index: i32,
    pub(crate) items: Vec<RecipeBookOverlayItem<'a>>,
    pub(crate) craftable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RecipeBookOverlayItem<'a> {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) stack: Cow<'a, ItemStackSummary>,
}

#[derive(Debug, Clone)]
pub(crate) struct RecipeBookGhostSlot<'a> {
    pub(crate) slot_id: i16,
    pub(crate) stack: Cow<'a, ItemStackSummary>,
    pub(crate) is_result: bool,
}

impl<'a> RecipeBookUiCollection<'a> {
    pub(crate) fn result_stack(&self) -> Option<&'a ItemStackSummary> {
        self.entries
            .first()
            .and_then(|entry| recipe_book_result_stack(entry))
    }

    pub(crate) fn result_stack_at_slot_select_index(
        &self,
        slot_select_index: usize,
    ) -> Option<&'a ItemStackSummary> {
        self.entry_at_slot_select_index(slot_select_index)
            .and_then(recipe_book_result_stack)
    }

    pub(crate) fn recipe_index_and_craftable_at_slot_select_index(
        &self,
        slot_select_index: usize,
    ) -> Option<(i32, bool)> {
        if self.entries.is_empty() {
            return None;
        }
        let index = slot_select_index % self.entries.len();
        Some((
            self.entries[index].id.index,
            self.craftable_entries.get(index).copied().unwrap_or(false),
        ))
    }

    pub(crate) fn has_multiple_recipes(&self) -> bool {
        self.entries.len() > 1
    }

    pub(crate) fn has_craftable(&self) -> bool {
        self.has_craftable
    }

    pub(crate) fn all_result_stacks_same(&self) -> bool {
        let Some(first) = self.result_stack() else {
            return true;
        };
        self.entries
            .iter()
            .all(|entry| recipe_book_result_stack(entry) == Some(first))
    }

    pub(crate) fn overlay_entries(
        &self,
        item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
        slot_select_index: usize,
    ) -> Vec<RecipeBookOverlayEntry<'a>> {
        let mut entries = Vec::with_capacity(self.entries.len());
        self.push_overlay_entries_with_craftability(
            true,
            item_tag_entries,
            slot_select_index,
            &mut entries,
        );
        self.push_overlay_entries_with_craftability(
            false,
            item_tag_entries,
            slot_select_index,
            &mut entries,
        );
        entries
    }

    fn entry_at_slot_select_index(
        &self,
        slot_select_index: usize,
    ) -> Option<&'a RecipeDisplayEntry> {
        if self.entries.is_empty() {
            return None;
        }
        Some(self.entries[slot_select_index % self.entries.len()])
    }

    fn push_overlay_entries_with_craftability(
        &self,
        craftable: bool,
        item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
        slot_select_index: usize,
        out: &mut Vec<RecipeBookOverlayEntry<'a>>,
    ) {
        for (entry, entry_craftable) in self.entries.iter().zip(&self.craftable_entries) {
            if *entry_craftable != craftable {
                continue;
            }
            out.push(RecipeBookOverlayEntry {
                recipe_index: entry.id.index,
                items: recipe_book_overlay_items_for_entry(
                    entry,
                    item_tag_entries,
                    slot_select_index,
                ),
                craftable,
            });
        }
    }
}

pub(crate) fn recipe_book_slot_select_index(world: &WorldStore, partial_tick: f32) -> usize {
    let render_time = world.world_time().map_or(0.0, |time| time.game_time as f64)
        + f64::from(partial_tick.max(0.0));
    (render_time / 30.0).floor().max(0.0) as usize
}

pub(crate) fn recipe_book_overlay_max_row(entry_count: usize) -> usize {
    if entry_count <= 16 {
        RECIPE_BOOK_OVERLAY_MAX_ROW
    } else {
        RECIPE_BOOK_OVERLAY_MAX_ROW_LARGE
    }
}

pub(crate) fn recipe_book_overlay_rows(entry_count: usize) -> usize {
    entry_count.div_ceil(recipe_book_overlay_max_row(entry_count))
}

pub(crate) fn recipe_book_overlay_origin(
    button_x: i32,
    button_y: i32,
    entry_count: usize,
) -> (i32, i32) {
    let max_row = recipe_book_overlay_max_row(entry_count);
    let rows = recipe_book_overlay_rows(entry_count);
    let mut x = button_x;
    let mut y = button_y;
    let right_pos = x + i32::try_from(entry_count.min(max_row)).unwrap_or_default()
        * RECIPE_BOOK_OVERLAY_BUTTON_SIZE;
    let max_left_pos = 147 / 2 + 50;
    if right_pos > max_left_pos {
        x -= RECIPE_BOOK_OVERLAY_BUTTON_SIZE
            * ((right_pos - max_left_pos) / RECIPE_BOOK_OVERLAY_BUTTON_SIZE);
    }

    let bottom_pos = y + i32::try_from(rows).unwrap_or_default() * RECIPE_BOOK_OVERLAY_BUTTON_SIZE;
    let max_bottom_pos = 13 + 166 / 2 + 50;
    if bottom_pos > max_bottom_pos {
        y -= RECIPE_BOOK_OVERLAY_BUTTON_SIZE
            * ceil_div_i32(bottom_pos - max_bottom_pos, RECIPE_BOOK_OVERLAY_BUTTON_SIZE);
    }

    let max_top_pos = 13 + 166 / 2 - 100;
    if y < max_top_pos {
        y -= RECIPE_BOOK_OVERLAY_BUTTON_SIZE
            * ceil_div_i32(y - max_top_pos, RECIPE_BOOK_OVERLAY_BUTTON_SIZE);
    }
    (x, y)
}

pub(crate) fn recipe_book_overlay_entry_position(
    origin_x: i32,
    origin_y: i32,
    entry_index: usize,
    entry_count: usize,
) -> (i32, i32) {
    let max_row = recipe_book_overlay_max_row(entry_count);
    let column = entry_index % max_row;
    let row = entry_index / max_row;
    (
        origin_x + 4 + i32::try_from(column).unwrap_or_default() * RECIPE_BOOK_OVERLAY_BUTTON_SIZE,
        origin_y + 5 + i32::try_from(row).unwrap_or_default() * RECIPE_BOOK_OVERLAY_BUTTON_SIZE,
    )
}

fn ceil_div_i32(value: i32, divisor: i32) -> i32 {
    value.div_euclid(divisor) + i32::from(value.rem_euclid(divisor) != 0)
}

pub(crate) fn crafting_recipe_book_collections<'a>(
    world: &'a WorldStore,
    grid: RecipeBookCraftingGrid,
    selected_tab_index: usize,
    only_craftable: bool,
    search_text: &str,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<RecipeBookUiCollection<'a>> {
    let Some(categories) = crafting_tab_categories(selected_tab_index) else {
        return Vec::new();
    };
    let available_items = crafting_recipe_book_available_item_counts(world, grid);
    let item_tag_entries = world
        .registry_tags("minecraft:item")
        .map(|registry| &registry.tags);
    let mut collections = Vec::new();
    for category_id in categories {
        push_crafting_category_collections(
            world,
            grid,
            *category_id,
            only_craftable,
            &available_items,
            item_tag_entries,
            &mut collections,
        );
    }
    if let Some(search_text) = normalized_recipe_search_text(search_text) {
        collections.retain(|collection| {
            recipe_book_collection_matches_search(collection, &search_text, item_runtime)
        });
    }
    collections
}

pub(crate) fn crafting_recipe_book_visible_tab_indices(
    world: &WorldStore,
    grid: RecipeBookCraftingGrid,
    tab_count: usize,
) -> Vec<usize> {
    (0..tab_count)
        .filter(|index| {
            *index == 0
                || !crafting_recipe_book_collections(world, grid, *index, false, "", None)
                    .is_empty()
        })
        .collect()
}

pub(crate) fn furnace_recipe_book_collections<'a>(
    world: &'a WorldStore,
    family: RecipeBookFurnaceFamily,
    selected_tab_index: usize,
    only_craftable: bool,
    search_text: &str,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<RecipeBookUiCollection<'a>> {
    let Some(categories) = furnace_tab_categories(family, selected_tab_index) else {
        return Vec::new();
    };
    let available_items = furnace_recipe_book_available_item_counts(world);
    let item_tag_entries = world
        .registry_tags("minecraft:item")
        .map(|registry| &registry.tags);
    let mut collections = Vec::new();
    for category_id in categories {
        push_furnace_category_collections(
            world,
            *category_id,
            only_craftable,
            &available_items,
            item_tag_entries,
            &mut collections,
        );
    }
    if let Some(search_text) = normalized_recipe_search_text(search_text) {
        collections.retain(|collection| {
            recipe_book_collection_matches_search(collection, &search_text, item_runtime)
        });
    }
    collections
}

pub(crate) fn furnace_recipe_book_visible_tab_indices(
    world: &WorldStore,
    family: RecipeBookFurnaceFamily,
    tab_count: usize,
) -> Vec<usize> {
    (0..tab_count)
        .filter(|index| {
            *index == 0
                || !furnace_recipe_book_collections(world, family, *index, false, "", None)
                    .is_empty()
        })
        .collect()
}

pub(crate) fn crafting_recipe_book_tab_has_highlighted_recipe(
    world: &WorldStore,
    grid: RecipeBookCraftingGrid,
    tab_index: usize,
    only_craftable: bool,
) -> bool {
    if tab_index == 0 {
        return false;
    }
    crafting_recipe_book_collections(world, grid, tab_index, only_craftable, "", None)
        .iter()
        .flat_map(|collection| collection.entries.iter())
        .any(|entry| world.recipe_book().highlights.contains(&entry.id.index))
}

pub(crate) fn furnace_recipe_book_tab_has_highlighted_recipe(
    world: &WorldStore,
    family: RecipeBookFurnaceFamily,
    tab_index: usize,
    only_craftable: bool,
) -> bool {
    if tab_index == 0 {
        return false;
    }
    furnace_recipe_book_collections(world, family, tab_index, only_craftable, "", None)
        .iter()
        .flat_map(|collection| collection.entries.iter())
        .any(|entry| world.recipe_book().highlights.contains(&entry.id.index))
}

pub(crate) fn recipe_book_page_count(collection_count: usize) -> usize {
    collection_count.div_ceil(RECIPE_BOOK_ITEMS_PER_PAGE)
}

pub(crate) fn clamped_recipe_book_page(page: usize, collection_count: usize) -> usize {
    let page_count = recipe_book_page_count(collection_count);
    page.min(page_count.saturating_sub(1))
}

pub(crate) fn crafting_recipe_book_ghost_slots<'a>(
    display: &'a RecipeDisplaySummary,
    grid: RecipeBookCraftingGrid,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    slot_select_index: usize,
) -> Vec<RecipeBookGhostSlot<'a>> {
    let Some(crafting) = display.crafting.as_ref() else {
        return Vec::new();
    };
    let mut slots = Vec::new();
    match crafting {
        CraftingRecipeDisplaySummary::Shapeless {
            ingredients,
            result,
            ..
        } => {
            push_recipe_book_ghost_result_at_slot(
                0,
                result,
                item_tag_entries,
                slot_select_index,
                &mut slots,
            );
            let slot_count = ingredients
                .len()
                .min((grid.width * grid.height).max(0) as usize);
            for (index, ingredient) in ingredients.iter().take(slot_count).enumerate() {
                push_recipe_book_ghost_input(
                    ingredient,
                    1 + index as i32,
                    item_tag_entries,
                    slot_select_index,
                    &mut slots,
                );
            }
        }
        CraftingRecipeDisplaySummary::Shaped {
            width,
            height,
            ingredients,
            result,
            ..
        } => {
            push_recipe_book_ghost_result_at_slot(
                0,
                result,
                item_tag_entries,
                slot_select_index,
                &mut slots,
            );
            place_shaped_recipe_ghost_inputs(
                grid,
                *width,
                *height,
                ingredients,
                item_tag_entries,
                slot_select_index,
                &mut slots,
            );
        }
    }
    slots
}

pub(crate) fn furnace_recipe_book_ghost_slots<'a>(
    display: &'a RecipeDisplaySummary,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    slot_select_index: usize,
    fuel_slot_empty: bool,
) -> Vec<RecipeBookGhostSlot<'a>> {
    let Some(furnace) = display.furnace.as_ref() else {
        return Vec::new();
    };
    let mut slots = Vec::new();
    push_recipe_book_ghost_result_at_slot(
        FURNACE_RESULT_SLOT,
        &furnace.result,
        item_tag_entries,
        slot_select_index,
        &mut slots,
    );
    push_recipe_book_ghost_input(
        &furnace.ingredient,
        i32::from(FURNACE_INGREDIENT_SLOT),
        item_tag_entries,
        slot_select_index,
        &mut slots,
    );
    if fuel_slot_empty {
        push_recipe_book_ghost_input(
            &furnace.fuel,
            i32::from(FURNACE_FUEL_SLOT),
            item_tag_entries,
            slot_select_index,
            &mut slots,
        );
    }
    slots
}

fn recipe_book_overlay_items_for_entry<'a>(
    entry: &'a RecipeDisplayEntry,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    slot_select_index: usize,
) -> Vec<RecipeBookOverlayItem<'a>> {
    let mut items = Vec::new();
    if let Some(crafting) = entry.display.crafting.as_ref() {
        match crafting {
            CraftingRecipeDisplaySummary::Shapeless { ingredients, .. } => {
                for (index, ingredient) in ingredients.iter().enumerate() {
                    push_recipe_book_overlay_item(
                        ingredient,
                        i32::try_from(index % 3).unwrap_or_default(),
                        i32::try_from(index / 3).unwrap_or_default(),
                        item_tag_entries,
                        slot_select_index,
                        &mut items,
                    );
                }
            }
            CraftingRecipeDisplaySummary::Shaped {
                width,
                height,
                ingredients,
                ..
            } => {
                for_each_shaped_recipe_grid_slot(
                    RecipeBookCraftingGrid {
                        width: 3,
                        height: 3,
                    },
                    *width,
                    *height,
                    ingredients,
                    |ingredient, _grid_index, grid_x, grid_y| {
                        push_recipe_book_overlay_item(
                            ingredient,
                            grid_x,
                            grid_y,
                            item_tag_entries,
                            slot_select_index,
                            &mut items,
                        );
                    },
                );
            }
        }
    } else if let Some(furnace) = entry.display.furnace.as_ref() {
        push_recipe_book_overlay_item(
            &furnace.ingredient,
            1,
            1,
            item_tag_entries,
            slot_select_index,
            &mut items,
        );
    }
    items
}

fn push_recipe_book_overlay_item<'a>(
    display: &'a SlotDisplaySummary,
    grid_x: i32,
    grid_y: i32,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    slot_select_index: usize,
    items: &mut Vec<RecipeBookOverlayItem<'a>>,
) {
    let Some(stack) =
        slot_display_item_stack_at_index(display, item_tag_entries, slot_select_index)
    else {
        return;
    };
    items.push(RecipeBookOverlayItem {
        x: RECIPE_BOOK_OVERLAY_GRID_ITEM_OFFSET + grid_x * RECIPE_BOOK_OVERLAY_GRID_ITEM_STRIDE,
        y: RECIPE_BOOK_OVERLAY_GRID_ITEM_OFFSET + grid_y * RECIPE_BOOK_OVERLAY_GRID_ITEM_STRIDE,
        stack,
    });
}

pub(crate) fn recipe_book_crafting_result_stack(
    entry: &RecipeDisplayEntry,
) -> Option<&ItemStackSummary> {
    match entry.display.crafting.as_ref()? {
        CraftingRecipeDisplaySummary::Shapeless { result, .. }
        | CraftingRecipeDisplaySummary::Shaped { result, .. } => result.item_stack.as_ref(),
    }
}

fn recipe_book_result_stack(entry: &RecipeDisplayEntry) -> Option<&ItemStackSummary> {
    recipe_book_crafting_result_stack(entry).or_else(|| recipe_book_furnace_result_stack(entry))
}

fn recipe_book_furnace_result_stack(entry: &RecipeDisplayEntry) -> Option<&ItemStackSummary> {
    entry.display.furnace.as_ref()?.result.item_stack.as_ref()
}

fn push_recipe_book_ghost_result_at_slot<'a>(
    slot_id: i16,
    display: &'a SlotDisplaySummary,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    slot_select_index: usize,
    slots: &mut Vec<RecipeBookGhostSlot<'a>>,
) {
    let Some(stack) =
        slot_display_item_stack_at_index(display, item_tag_entries, slot_select_index)
    else {
        return;
    };
    slots.push(RecipeBookGhostSlot {
        slot_id,
        stack,
        is_result: true,
    });
}

fn push_recipe_book_ghost_input<'a>(
    display: &'a SlotDisplaySummary,
    slot_id: i32,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    slot_select_index: usize,
    slots: &mut Vec<RecipeBookGhostSlot<'a>>,
) {
    let (Ok(slot_id), Some(stack)) = (
        i16::try_from(slot_id),
        slot_display_item_stack_at_index(display, item_tag_entries, slot_select_index),
    ) else {
        return;
    };
    slots.push(RecipeBookGhostSlot {
        slot_id,
        stack,
        is_result: false,
    });
}

fn place_shaped_recipe_ghost_inputs<'a>(
    grid: RecipeBookCraftingGrid,
    recipe_width: i32,
    recipe_height: i32,
    ingredients: &'a [SlotDisplaySummary],
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    slot_select_index: usize,
    slots: &mut Vec<RecipeBookGhostSlot<'a>>,
) {
    for_each_shaped_recipe_grid_slot(
        grid,
        recipe_width,
        recipe_height,
        ingredients,
        |ingredient, grid_index, _grid_x, _grid_y| {
            push_recipe_book_ghost_input(
                ingredient,
                1 + grid_index,
                item_tag_entries,
                slot_select_index,
                slots,
            );
        },
    );
}

fn for_each_shaped_recipe_grid_slot<'a>(
    grid: RecipeBookCraftingGrid,
    recipe_width: i32,
    recipe_height: i32,
    ingredients: &'a [SlotDisplaySummary],
    mut visit: impl FnMut(&'a SlotDisplaySummary, i32, i32, i32),
) {
    if grid.width <= 0 || grid.height <= 0 || recipe_width <= 0 || recipe_height <= 0 {
        return;
    }
    let mut ingredients = ingredients.iter().peekable();
    let mut grid_index = 0;
    let mut grid_y = 0;
    while grid_y < grid.height {
        let should_center_y = (recipe_height as f32) < (grid.height as f32 / 2.0);
        let start_y = (grid.height as f32 / 2.0 - recipe_height as f32 / 2.0).floor() as i32;
        if should_center_y && start_y > grid_y {
            grid_index += grid.width;
            grid_y += 1;
        }

        let mut grid_x = 0;
        while grid_x < grid.width {
            if ingredients.peek().is_none() {
                return;
            }
            let should_center_x = (recipe_width as f32) < (grid.width as f32 / 2.0);
            let start_x = (grid.width as f32 / 2.0 - recipe_width as f32 / 2.0).floor() as i32;
            let mut total_recipe_width_in_grid = recipe_width;
            let mut add_ingredient_to_slot = grid_x < recipe_width;
            if should_center_x {
                total_recipe_width_in_grid = start_x + recipe_width;
                add_ingredient_to_slot = start_x <= grid_x && grid_x < start_x + recipe_width;
            }

            if add_ingredient_to_slot {
                let ingredient = ingredients.next().expect("ingredient presence checked");
                visit(ingredient, grid_index, grid_x, grid_y);
            } else if total_recipe_width_in_grid == grid_x {
                grid_index += grid.width - grid_x;
                break;
            }

            grid_index += 1;
            grid_x += 1;
        }
        grid_y += 1;
    }
}

fn slot_display_item_stack_at_index<'a>(
    display: &'a SlotDisplaySummary,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    slot_select_index: usize,
) -> Option<Cow<'a, ItemStackSummary>> {
    if let Some(stack) = display.item_stack.as_ref() {
        return Some(Cow::Borrowed(stack));
    }
    let mut stacks = Vec::new();
    push_slot_display_item_stack_candidates(display, item_tag_entries, &mut stacks);
    if stacks.is_empty() {
        return None;
    }
    Some(Cow::Owned(stacks[slot_select_index % stacks.len()].clone()))
}

fn push_slot_display_item_stack_candidates(
    display: &SlotDisplaySummary,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    stacks: &mut Vec<ItemStackSummary>,
) {
    if let Some(stack) = display.item_stack.as_ref() {
        stacks.push(stack.clone());
        return;
    }
    if let Some(entries) = display
        .tag
        .as_ref()
        .and_then(|tag| item_tag_entries.and_then(|tags| tags.get(tag)))
    {
        if !entries.is_empty() {
            stacks.extend(entries.iter().map(|item_id| ItemStackSummary {
                item_id: Some(*item_id),
                count: 1,
                component_patch: DataComponentPatchSummary::default(),
            }));
            return;
        }
    }
    for child in display.stack_resolving_children() {
        push_slot_display_item_stack_candidates(&child, item_tag_entries, stacks);
    }
}

fn normalized_recipe_search_text(search_text: &str) -> Option<String> {
    (!search_text.is_empty()).then(|| search_text.to_lowercase())
}

fn recipe_book_collection_matches_search(
    collection: &RecipeBookUiCollection<'_>,
    search_text: &str,
    item_runtime: Option<&NativeItemRuntime>,
) -> bool {
    let Some(item_runtime) = item_runtime else {
        return false;
    };
    let result_stacks = || {
        collection
            .entries
            .iter()
            .filter_map(|entry| recipe_book_result_stack(entry))
    };
    if let Some((namespace, path)) = search_text.split_once(':') {
        let namespace = namespace.trim();
        let path = path.trim();
        let namespace_matches = result_stacks().any(|stack| {
            recipe_book_result_stack_resource_namespace_matches(stack, namespace, item_runtime)
        });
        let path_or_name_matches = result_stacks().any(|stack| {
            recipe_book_result_stack_resource_path_matches(stack, path, item_runtime)
                || recipe_book_result_stack_tooltip_matches(stack, path, item_runtime)
        });
        return namespace_matches && path_or_name_matches;
    }
    result_stacks()
        .any(|stack| recipe_book_result_stack_tooltip_matches(stack, search_text, item_runtime))
}

fn recipe_book_result_stack_resource_namespace_matches(
    stack: &ItemStackSummary,
    search_text: &str,
    item_runtime: &NativeItemRuntime,
) -> bool {
    recipe_book_result_stack_resource_id_part_matches(stack, search_text, item_runtime, true)
}

fn recipe_book_result_stack_resource_path_matches(
    stack: &ItemStackSummary,
    search_text: &str,
    item_runtime: &NativeItemRuntime,
) -> bool {
    recipe_book_result_stack_resource_id_part_matches(stack, search_text, item_runtime, false)
}

fn recipe_book_result_stack_resource_id_part_matches(
    stack: &ItemStackSummary,
    search_text: &str,
    item_runtime: &NativeItemRuntime,
    namespace: bool,
) -> bool {
    let Some(item_id) = stack.item_id else {
        return false;
    };
    item_runtime
        .item_resource_id(item_id)
        .is_some_and(|resource_id| {
            let (resource_namespace, resource_path) =
                resource_id.split_once(':').unwrap_or(("", resource_id));
            let resource_part = if namespace {
                resource_namespace
            } else {
                resource_path
            };
            resource_part.to_lowercase().contains(search_text)
        })
}

fn recipe_book_result_stack_tooltip_matches(
    stack: &ItemStackSummary,
    search_text: &str,
    item_runtime: &NativeItemRuntime,
) -> bool {
    item_runtime
        .tooltip_lines_for_stack(stack)
        .is_some_and(|lines| {
            lines
                .iter()
                .any(|line| line.text.to_lowercase().contains(search_text))
        })
}

fn push_crafting_category_collections<'a>(
    world: &'a WorldStore,
    grid: RecipeBookCraftingGrid,
    category_id: i32,
    only_craftable: bool,
    available_items: &BTreeMap<i32, i32>,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    collections: &mut Vec<RecipeBookUiCollection<'a>>,
) {
    let mut group_indexes: BTreeMap<i32, usize> = BTreeMap::new();
    for entry in world.recipe_book().known.values() {
        if entry.category_id != category_id || !crafting_recipe_fits_grid(entry, grid) {
            continue;
        }
        let craftable = recipe_book_entry_is_craftable(entry, available_items, item_tag_entries);
        if only_craftable && !craftable {
            continue;
        }
        if let Some(group_id) = entry.group {
            if let Some(index) = group_indexes.get(&group_id).copied() {
                collections[index].entries.push(entry);
                collections[index].craftable_entries.push(craftable);
                collections[index].has_craftable |= craftable;
            } else {
                let index = collections.len();
                group_indexes.insert(group_id, index);
                collections.push(RecipeBookUiCollection {
                    entries: vec![entry],
                    craftable_entries: vec![craftable],
                    has_craftable: craftable,
                });
            }
        } else {
            collections.push(RecipeBookUiCollection {
                entries: vec![entry],
                craftable_entries: vec![craftable],
                has_craftable: craftable,
            });
        }
    }
}

fn push_furnace_category_collections<'a>(
    world: &'a WorldStore,
    category_id: i32,
    only_craftable: bool,
    available_items: &BTreeMap<i32, i32>,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
    collections: &mut Vec<RecipeBookUiCollection<'a>>,
) {
    let mut group_indexes: BTreeMap<i32, usize> = BTreeMap::new();
    for entry in world.recipe_book().known.values() {
        if entry.category_id != category_id || entry.display.furnace.is_none() {
            continue;
        }
        let craftable = recipe_book_entry_is_craftable(entry, available_items, item_tag_entries);
        if only_craftable && !craftable {
            continue;
        }
        if let Some(group_id) = entry.group {
            if let Some(index) = group_indexes.get(&group_id).copied() {
                collections[index].entries.push(entry);
                collections[index].craftable_entries.push(craftable);
                collections[index].has_craftable |= craftable;
            } else {
                let index = collections.len();
                group_indexes.insert(group_id, index);
                collections.push(RecipeBookUiCollection {
                    entries: vec![entry],
                    craftable_entries: vec![craftable],
                    has_craftable: craftable,
                });
            }
        } else {
            collections.push(RecipeBookUiCollection {
                entries: vec![entry],
                craftable_entries: vec![craftable],
                has_craftable: craftable,
            });
        }
    }
}

fn crafting_tab_categories(selected_tab_index: usize) -> Option<&'static [i32]> {
    match selected_tab_index {
        0 => Some(&CRAFTING_SEARCH_TAB_CATEGORIES),
        1 => Some(&CRAFTING_EQUIPMENT_TAB_CATEGORIES),
        2 => Some(&CRAFTING_BUILDING_BLOCKS_TAB_CATEGORIES),
        3 => Some(&CRAFTING_MISC_TAB_CATEGORIES),
        4 => Some(&CRAFTING_REDSTONE_TAB_CATEGORIES),
        _ => None,
    }
}

fn furnace_tab_categories(
    family: RecipeBookFurnaceFamily,
    selected_tab_index: usize,
) -> Option<&'static [i32]> {
    match (family, selected_tab_index) {
        (RecipeBookFurnaceFamily::Furnace, 0) => Some(&FURNACE_SEARCH_TAB_CATEGORIES),
        (RecipeBookFurnaceFamily::Furnace, 1) => Some(&FURNACE_FOOD_TAB_CATEGORIES),
        (RecipeBookFurnaceFamily::Furnace, 2) => Some(&FURNACE_BLOCKS_TAB_CATEGORIES),
        (RecipeBookFurnaceFamily::Furnace, 3) => Some(&FURNACE_MISC_TAB_CATEGORIES),
        (RecipeBookFurnaceFamily::BlastFurnace, 0) => Some(&BLAST_FURNACE_SEARCH_TAB_CATEGORIES),
        (RecipeBookFurnaceFamily::BlastFurnace, 1) => Some(&BLAST_FURNACE_BLOCKS_TAB_CATEGORIES),
        (RecipeBookFurnaceFamily::BlastFurnace, 2) => Some(&BLAST_FURNACE_MISC_TAB_CATEGORIES),
        (RecipeBookFurnaceFamily::Smoker, 0) => Some(&SMOKER_SEARCH_TAB_CATEGORIES),
        (RecipeBookFurnaceFamily::Smoker, 1) => Some(&SMOKER_FOOD_TAB_CATEGORIES),
        _ => None,
    }
}

fn crafting_recipe_fits_grid(entry: &RecipeDisplayEntry, grid: RecipeBookCraftingGrid) -> bool {
    match entry.display.crafting.as_ref() {
        Some(CraftingRecipeDisplaySummary::Shapeless { ingredients, .. }) => {
            i32::try_from(ingredients.len()).is_ok_and(|count| count <= grid.width * grid.height)
        }
        Some(CraftingRecipeDisplaySummary::Shaped { width, height, .. }) => {
            *width > 0 && *height > 0 && *width <= grid.width && *height <= grid.height
        }
        None => false,
    }
}

fn crafting_recipe_book_available_item_counts(
    world: &WorldStore,
    grid: RecipeBookCraftingGrid,
) -> BTreeMap<i32, i32> {
    let mut counts = BTreeMap::new();
    let mut canonical_player_slots = BTreeSet::new();
    for slot in &world.inventory().player_slots {
        if (PLAYER_INVENTORY_SLOT_START..PLAYER_INVENTORY_SLOT_END).contains(&slot.slot) {
            canonical_player_slots.insert(slot.slot);
            add_item_stack_count(&mut counts, &slot.item);
        }
    }

    if world.local_inventory_is_open() && grid.width == 2 && grid.height == 2 {
        let container = &world.inventory().inventory_menu;
        add_container_slot_range_counts(
            &mut counts,
            container,
            LOCAL_INVENTORY_CRAFT_SLOT_START,
            LOCAL_INVENTORY_CRAFT_SLOT_END,
        );
        add_mapped_container_player_slot_counts(
            &mut counts,
            container,
            LOCAL_INVENTORY_PLAYER_MAIN_START,
            LOCAL_INVENTORY_PLAYER_MAIN_END,
            |slot| i32::from(slot),
            &canonical_player_slots,
        );
        add_mapped_container_player_slot_counts(
            &mut counts,
            container,
            LOCAL_INVENTORY_HOTBAR_START,
            LOCAL_INVENTORY_HOTBAR_END,
            |slot| i32::from(slot - LOCAL_INVENTORY_HOTBAR_START),
            &canonical_player_slots,
        );
        return counts;
    }

    if grid.width == 3 && grid.height == 3 {
        let Some(container) = world.inventory().open_container.as_ref() else {
            return counts;
        };
        add_container_slot_range_counts(
            &mut counts,
            container,
            CRAFTING_TABLE_CRAFT_SLOT_START,
            CRAFTING_TABLE_CRAFT_SLOT_END,
        );
        add_mapped_container_player_slot_counts(
            &mut counts,
            container,
            CRAFTING_TABLE_PLAYER_MAIN_START,
            CRAFTING_TABLE_PLAYER_MAIN_END,
            |slot| i32::from(slot - 1),
            &canonical_player_slots,
        );
        add_mapped_container_player_slot_counts(
            &mut counts,
            container,
            CRAFTING_TABLE_HOTBAR_START,
            CRAFTING_TABLE_HOTBAR_END,
            |slot| i32::from(slot - CRAFTING_TABLE_HOTBAR_START),
            &canonical_player_slots,
        );
    }

    counts
}

fn furnace_recipe_book_available_item_counts(world: &WorldStore) -> BTreeMap<i32, i32> {
    let mut counts = BTreeMap::new();
    let mut canonical_player_slots = BTreeSet::new();
    for slot in &world.inventory().player_slots {
        if (PLAYER_INVENTORY_SLOT_START..PLAYER_INVENTORY_SLOT_END).contains(&slot.slot) {
            canonical_player_slots.insert(slot.slot);
            add_item_stack_count(&mut counts, &slot.item);
        }
    }

    let Some(container) = world.inventory().open_container.as_ref() else {
        return counts;
    };
    add_container_slot_range_counts(
        &mut counts,
        container,
        FURNACE_CRAFT_SLOT_START,
        FURNACE_CRAFT_SLOT_END,
    );
    add_mapped_container_player_slot_counts(
        &mut counts,
        container,
        FURNACE_PLAYER_MAIN_START,
        FURNACE_PLAYER_MAIN_END,
        |slot| i32::from(slot - FURNACE_PLAYER_MAIN_START + 9),
        &canonical_player_slots,
    );
    add_mapped_container_player_slot_counts(
        &mut counts,
        container,
        FURNACE_HOTBAR_START,
        FURNACE_HOTBAR_END,
        |slot| i32::from(slot - FURNACE_HOTBAR_START),
        &canonical_player_slots,
    );
    counts
}

fn add_container_slot_range_counts(
    counts: &mut BTreeMap<i32, i32>,
    container: &bbb_world::ContainerState,
    start: i16,
    end: i16,
) {
    for slot in &container.slots {
        if (start..end).contains(&slot.slot) {
            add_item_stack_count(counts, &slot.item);
        }
    }
}

fn add_mapped_container_player_slot_counts(
    counts: &mut BTreeMap<i32, i32>,
    container: &bbb_world::ContainerState,
    start: i16,
    end: i16,
    player_slot: impl Fn(i16) -> i32,
    canonical_player_slots: &BTreeSet<i32>,
) {
    for slot in &container.slots {
        if !(start..end).contains(&slot.slot) {
            continue;
        }
        if canonical_player_slots.contains(&player_slot(slot.slot)) {
            continue;
        }
        add_item_stack_count(counts, &slot.item);
    }
}

fn add_item_stack_count(counts: &mut BTreeMap<i32, i32>, stack: &ItemStackSummary) {
    let (Some(item_id), count) = (stack.item_id, stack.count) else {
        return;
    };
    if count <= 0 {
        return;
    }
    *counts.entry(item_id).or_default() += count;
}

fn recipe_book_entry_is_craftable(
    entry: &RecipeDisplayEntry,
    available_items: &BTreeMap<i32, i32>,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
) -> bool {
    let Some(requirements) = entry.crafting_requirements.as_ref() else {
        return false;
    };
    let mut options = Vec::with_capacity(requirements.len());
    for requirement in requirements {
        let item_options = recipe_book_ingredient_item_options(requirement, item_tag_entries);
        if item_options.is_empty() {
            return false;
        }
        options.push(item_options);
    }
    options.sort_by_key(Vec::len);
    let mut remaining = available_items.clone();
    recipe_book_can_satisfy_ingredients(&options, &mut remaining)
}

fn recipe_book_ingredient_item_options(
    ingredient: &IngredientSummary,
    item_tag_entries: Option<&BTreeMap<String, Vec<i32>>>,
) -> Vec<i32> {
    if let Some(tag) = ingredient.tag.as_ref() {
        return item_tag_entries
            .and_then(|tags| tags.get(tag))
            .map(|item_ids| normalized_item_options(item_ids.clone()))
            .unwrap_or_default();
    }
    normalized_item_options(ingredient.item_ids.clone())
}

fn normalized_item_options(mut item_ids: Vec<i32>) -> Vec<i32> {
    item_ids.sort_unstable();
    item_ids.dedup();
    item_ids
}

fn recipe_book_can_satisfy_ingredients(
    options: &[Vec<i32>],
    remaining: &mut BTreeMap<i32, i32>,
) -> bool {
    let Some((first, rest)) = options.split_first() else {
        return true;
    };
    for item_id in first {
        let available = remaining.get(item_id).copied().unwrap_or_default();
        if available <= 0 {
            continue;
        }
        remaining.insert(*item_id, available - 1);
        if recipe_book_can_satisfy_ingredients(rest, remaining) {
            return true;
        }
        remaining.insert(*item_id, available);
    }
    false
}
