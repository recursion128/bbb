use std::collections::BTreeMap;

use bbb_protocol::packets::{
    CraftingRecipeDisplaySummary, ItemStackSummary, RecipeDisplayEntry, RecipeDisplaySummary,
    SlotDisplaySummary,
};
use bbb_world::WorldStore;

pub(crate) const RECIPE_BOOK_ITEMS_PER_PAGE: usize = 20;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecipeBookCraftingGrid {
    pub(crate) width: i32,
    pub(crate) height: i32,
}

#[derive(Debug, Clone)]
pub(crate) struct RecipeBookUiCollection<'a> {
    entries: Vec<&'a RecipeDisplayEntry>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RecipeBookGhostSlot<'a> {
    pub(crate) slot_id: i16,
    pub(crate) stack: &'a ItemStackSummary,
    pub(crate) is_result: bool,
}

impl<'a> RecipeBookUiCollection<'a> {
    pub(crate) fn result_stack(&self) -> Option<&'a ItemStackSummary> {
        self.entries
            .first()
            .and_then(|entry| recipe_book_crafting_result_stack(entry))
    }

    pub(crate) fn first_recipe_index(&self) -> Option<i32> {
        self.entries.first().map(|entry| entry.id.index)
    }

    pub(crate) fn has_multiple_recipes(&self) -> bool {
        self.entries.len() > 1
    }
}

pub(crate) fn crafting_recipe_book_collections(
    world: &WorldStore,
    grid: RecipeBookCraftingGrid,
    selected_tab_index: usize,
) -> Vec<RecipeBookUiCollection<'_>> {
    let Some(categories) = crafting_tab_categories(selected_tab_index) else {
        return Vec::new();
    };
    let mut collections = Vec::new();
    for category_id in categories {
        push_crafting_category_collections(world, grid, *category_id, &mut collections);
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
            *index == 0 || !crafting_recipe_book_collections(world, grid, *index).is_empty()
        })
        .collect()
}

pub(crate) fn recipe_book_page_count(collection_count: usize) -> usize {
    collection_count.div_ceil(RECIPE_BOOK_ITEMS_PER_PAGE)
}

pub(crate) fn clamped_recipe_book_page(page: usize, collection_count: usize) -> usize {
    let page_count = recipe_book_page_count(collection_count);
    page.min(page_count.saturating_sub(1))
}

pub(crate) fn crafting_recipe_book_ghost_slots(
    display: &RecipeDisplaySummary,
    grid: RecipeBookCraftingGrid,
) -> Vec<RecipeBookGhostSlot<'_>> {
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
            push_recipe_book_ghost_result(result, &mut slots);
            let slot_count = ingredients
                .len()
                .min((grid.width * grid.height).max(0) as usize);
            for (index, ingredient) in ingredients.iter().take(slot_count).enumerate() {
                push_recipe_book_ghost_input(ingredient, 1 + index as i32, &mut slots);
            }
        }
        CraftingRecipeDisplaySummary::Shaped {
            width,
            height,
            ingredients,
            result,
            ..
        } => {
            push_recipe_book_ghost_result(result, &mut slots);
            place_shaped_recipe_ghost_inputs(grid, *width, *height, ingredients, &mut slots);
        }
    }
    slots
}

pub(crate) fn recipe_book_crafting_result_stack(
    entry: &RecipeDisplayEntry,
) -> Option<&ItemStackSummary> {
    match entry.display.crafting.as_ref()? {
        CraftingRecipeDisplaySummary::Shapeless { result, .. }
        | CraftingRecipeDisplaySummary::Shaped { result, .. } => result.item_stack.as_ref(),
    }
}

fn push_recipe_book_ghost_result<'a>(
    display: &'a SlotDisplaySummary,
    slots: &mut Vec<RecipeBookGhostSlot<'a>>,
) {
    let Some(stack) = display.item_stack.as_ref() else {
        return;
    };
    slots.push(RecipeBookGhostSlot {
        slot_id: 0,
        stack,
        is_result: true,
    });
}

fn push_recipe_book_ghost_input<'a>(
    display: &'a SlotDisplaySummary,
    slot_id: i32,
    slots: &mut Vec<RecipeBookGhostSlot<'a>>,
) {
    let (Ok(slot_id), Some(stack)) = (i16::try_from(slot_id), display.item_stack.as_ref()) else {
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
    slots: &mut Vec<RecipeBookGhostSlot<'a>>,
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
                push_recipe_book_ghost_input(ingredient, 1 + grid_index, slots);
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

fn push_crafting_category_collections<'a>(
    world: &'a WorldStore,
    grid: RecipeBookCraftingGrid,
    category_id: i32,
    collections: &mut Vec<RecipeBookUiCollection<'a>>,
) {
    let mut group_indexes: BTreeMap<i32, usize> = BTreeMap::new();
    for entry in world.recipe_book().known.values() {
        if entry.category_id != category_id || !crafting_recipe_fits_grid(entry, grid) {
            continue;
        }
        if let Some(group_id) = entry.group {
            if let Some(index) = group_indexes.get(&group_id).copied() {
                collections[index].entries.push(entry);
            } else {
                let index = collections.len();
                group_indexes.insert(group_id, index);
                collections.push(RecipeBookUiCollection {
                    entries: vec![entry],
                });
            }
        } else {
            collections.push(RecipeBookUiCollection {
                entries: vec![entry],
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
