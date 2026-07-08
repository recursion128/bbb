use std::collections::BTreeMap;

use bbb_protocol::packets::{CraftingRecipeDisplaySummary, ItemStackSummary, RecipeDisplayEntry};
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

pub(crate) fn recipe_book_page_count(collection_count: usize) -> usize {
    collection_count.div_ceil(RECIPE_BOOK_ITEMS_PER_PAGE)
}

pub(crate) fn clamped_recipe_book_page(page: usize, collection_count: usize) -> usize {
    let page_count = recipe_book_page_count(collection_count);
    page.min(page_count.saturating_sub(1))
}

pub(crate) fn recipe_book_crafting_result_stack(
    entry: &RecipeDisplayEntry,
) -> Option<&ItemStackSummary> {
    match entry.display.crafting.as_ref()? {
        CraftingRecipeDisplaySummary::Shapeless { result, .. }
        | CraftingRecipeDisplaySummary::Shaped { result, .. } => result.item_stack.as_ref(),
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
